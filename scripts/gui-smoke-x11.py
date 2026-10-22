#!/usr/bin/env python3
"""Black-box X11 smoke test. Invoke via test-gui-x11.sh, not on a live desktop.

No application test hooks: observe window title and clipboard through X11, send
real keyboard/mouse input, and compare exact UTF-8 bytes. This is not Wayland,
portal, pixel-layout, IME or Wayland window-close verification.
"""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time
import uuid

ROOT = Path(__file__).resolve().parent.parent


def isolated_environment(parent, scratch):
    env = dict(parent)
    for name in ("HYPRLAND_INSTANCE_SIGNATURE", "WAYLAND_DISPLAY", "WAYLAND_SOCKET",
                 "DBUS_SESSION_BUS_ADDRESS", "DBUS_SESSION_BUS_PID"):
        env.pop(name, None)
    for key, directory in (("XDG_CONFIG_HOME", "config"), ("XDG_CACHE_HOME", "cache"),
                           ("XDG_DATA_HOME", "data"), ("XDG_STATE_HOME", "state"),
                           ("XDG_RUNTIME_DIR", "runtime")):
        path = scratch / directory
        path.mkdir(mode=0o700)
        env[key] = str(path)
    env.update(ICED_BACKEND="tiny-skia", XDG_SESSION_TYPE="x11", WINIT_UNIX_BACKEND="x11")
    return env


class Desktop:
    def __init__(self, app, env, log):
        self.app = app
        self.env = env
        self.log = log
        self.window = None
        self.owners = []
        self.passed = []
        self.native_exits = 0

    def command(self, *args, timeout=3, **kwargs):
        return subprocess.run(args, env=self.env, check=True, timeout=timeout,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, **kwargs).stdout

    def wait(self, description, condition, seconds=12):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            if self.app.poll() is not None:
                raise RuntimeError(f"Application exited ({self.app.returncode}): {description}")
            if condition():
                return
            time.sleep(0.1)
        raise TimeoutError(description)

    def passed_check(self, description):
        self.passed.append(description)
        print(f"PASS: {description}", flush=True)

    def find_window(self):
        def find():
            try:
                found = self.command("xdotool", "search", "--onlyvisible", "--pid", str(self.app.pid))
                self.window = found.decode("ascii").splitlines()[0]
                return True
            except (subprocess.CalledProcessError, IndexError):
                return False
        self.wait("Visible application window", find, seconds=30)
        self.command("xdotool", "windowfocus", "--sync", self.window)
        # Source is the configured initial mode, at a fixed 1200x800 window.
        # Mouse coordinates are relative to THIS client, not the root desktop.
        self.command("xdotool", "mousemove", "--window", self.window, "600", "300", "click", "1")

    def key(self, combination):
        self.command("xdotool", "key", "--clearmodifiers", "--delay", "40", combination)
        # Iced publishes SelectAll as a Message, while Copy reads selection
        # directly in the widget event handler. Allow a frame/update between
        # human-paced shortcuts; otherwise Copy can see the preceding click's
        # selection when both key sequences land in one event batch. Assertions
        # still require a fresh clipboard response and exact expected bytes.
        time.sleep(0.25)

    def type_ascii(self, text):
        if not text.isascii():
            raise ValueError("Use clipboard Paste for UTF-8 fixtures, not xdotool type")
        self.command("xdotool", "type", "--clearmodifiers", "--delay", "30", text)
        time.sleep(0.25)

    def title_is_dirty(self, expected):
        def matches():
            title = self.command("xdotool", "getwindowname", self.window).decode("utf-8")
            return title.endswith(" — Hylite\n") and (" •" in title) == expected
        self.wait(f"Window dirty title = {expected}", matches)

    def clipboard(self):
        try:
            return self.command("xclip", "-selection", "clipboard", "-out", "-target", "UTF8_STRING")
        except subprocess.CalledProcessError:
            return None

    def put_clipboard(self, data):
        # -quiet keeps xclip in the foreground so every owner can be reaped.
        owner = subprocess.Popen(["xclip", "-selection", "clipboard", "-in", "-quiet"],
                                 env=self.env, stdin=subprocess.PIPE,
                                 stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.owners.append(owner)
        owner.stdin.write(data)
        owner.stdin.close()
        observed = None

        def owned():
            nonlocal observed
            observed = self.clipboard()
            return observed == data
        try:
            self.wait("Clipboard fixture ownership", owned)
        except TimeoutError as error:
            raise TimeoutError(f"Clipboard ownership: owner status={owner.poll()}, expected {data[:300]!r}, got {observed[:300] if observed is not None else None!r}") from error

    def expect_copy(self, description, expected, *, refocus=True):
        # Changing ownership BEFORE Ctrl+C prevents a false pass from simply
        # reading the bytes we staged for Ctrl+V. Never strip whitespace or LF.
        marker = f"hylite-copy-not-acknowledged-{uuid.uuid4()}".encode("ascii")
        self.put_clipboard(marker)
        # Dispatch Copy ONCE, then only observe. Repeated Copy in the poll
        # could leave another asynchronous clipboard write queued after success,
        # stealing ownership from the next Paste fixture.
        if refocus:
            self.command("xdotool", "mousemove", "--window", self.window, "600", "300", "click", "1")
        self.key("ctrl+a")
        self.key("ctrl+c")
        actual = None

        def copied():
            nonlocal actual
            actual = self.clipboard()
            return actual == expected

        try:
            self.wait(description, copied)
        except TimeoutError as error:
            raise AssertionError(f"{description}: expected {expected[:300]!r}, got {actual[:300] if actual is not None else None!r}") from error
        self.passed_check(description)

    def paste_from_clean(self, data):
        self.title_is_dirty(False)
        self.put_clipboard(data)
        self.key("ctrl+a")
        self.key("ctrl+v")
        # A new dirty title acknowledges that Paste has CONSUMED the clipboard.
        # Only then may expect_copy replace the external clipboard owner.
        self.title_is_dirty(True)

    def confirmation(self, action):
        def matches():
            title = self.command("xdotool", "getwindowname", self.window).decode("utf-8")
            if action is None:
                return "[confirm " not in title
            return f"[confirm {action}]" in title
        self.wait(f"Confirmation title = {action}", matches)
        self.passed_check(f"Confirmation title: {action or 'closed'}")

    def request_close(self):
        # WM_DELETE_WINDOW is the same client protocol a window manager uses.
        # Do NOT use xdotool windowclose: it calls XDestroyWindow directly,
        # bypassing the application's unsaved-changes guard.
        from Xlib import X, display, protocol
        connection = display.Display(self.env["DISPLAY"])
        try:
            window = connection.create_resource_object("window", int(self.window))
            delete = connection.intern_atom("WM_DELETE_WINDOW")
            if delete not in (window.get_wm_protocols() or []):
                raise RuntimeError("Application does not advertise WM_DELETE_WINDOW")
            message = protocol.event.ClientMessage(
                window=int(self.window), client_type=connection.intern_atom("WM_PROTOCOLS"),
                data=(32, [delete, X.CurrentTime, 0, 0, 0]),
            )
            window.send_event(message, event_mask=0, propagate=False)
            connection.sync()
        finally:
            connection.close()

    def expect_native_exit(self, description):
        code = self.app.wait(timeout=12)
        if code != 0:
            raise AssertionError(f"{description}: expected natural exit 0, got {code}")
        self.native_exits += 1
        self.passed_check(description)

    def restart_after_native_exit(self, binary):
        if self.app.poll() != 0:
            raise RuntimeError("Restart requires a previously observed successful native exit")
        self.cleanup()
        self.owners = []
        self.window = None
        self.app = subprocess.Popen([str(binary)], env=self.env, stdout=self.log, stderr=self.log)
        self.find_window()

    def screenshot(self, path):
        if self.window:
            self.command("import", "-silent", "-limit", "thread", "1", "-window", self.window, str(path), timeout=15)

    def cleanup(self):
        # Fallback resource cleanup, never counted as a successful native exit.
        # Lifecycle checks must observe exit 0 BEFORE this can terminate a child.
        for process in [self.app, *self.owners]:
            if process.poll() is None:
                process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=3)


def exercise(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Initial Source load and clipboard copy", welcome)
    fixtures = [
        ("Unicode and trailing tabs", "café 世界🙂\n- [x] item\t\n\t".encode("utf-8")),
        ("Terminal empty lines", b"alpha\n\n"),
    ]
    for name, source in fixtures:
        desktop.paste_from_clean(source)
        desktop.expect_copy(f"{name}: paste", source)
        desktop.screenshot(artifacts / ("tabs.png" if "tabs" in name else "empty-lines.png"))
        desktop.key("ctrl+z")
        desktop.title_is_dirty(False)
        desktop.expect_copy(f"{name}: undo restores clean baseline", welcome)
        desktop.key("ctrl+shift+z")
        desktop.title_is_dirty(True)
        desktop.expect_copy(f"{name}: redo", source)
        desktop.key("ctrl+z")
        desktop.title_is_dirty(False)
        desktop.expect_copy(f"{name}: return to baseline", welcome)

    # Unlike clipboard insertion, this goes through individual key events and
    # typing coalescing. Copy closes the group but does not become a text edit.
    desktop.key("ctrl+End")
    desktop.type_ascii("XYZ")
    desktop.title_is_dirty(True)
    desktop.expect_copy("Keyboard typing at document end", welcome + b"XYZ")
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("One undo removes the keyboard typing burst", welcome)


def exercise_navigation(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = "head\ncafé🙂世界\nsame needle\nother needle\n".encode("utf-8")
    desktop.paste_from_clean(source)
    desktop.expect_copy("Navigation fixture: keyboard-only Source copy", source, refocus=False)

    # Every copy below deliberately omits mouse/focus operations. Query copies
    # prove the panel took keyboard focus; sentinel edits prove automatic return
    # to Source AND exact caret position, not merely an unchanged document.
    desktop.key("ctrl+Home")
    desktop.key("ctrl+g")
    desktop.expect_copy("Go to Line: query automatically focused", b"1:1", refocus=False)
    desktop.type_ascii("2:6")
    desktop.expect_copy("Go to Line: address field edited", b"2:6", refocus=False)
    desktop.key("Return")
    desktop.type_ascii("X")
    positioned = "head\ncafé🙂X世界\nsame needle\nother needle\n".encode("utf-8")
    desktop.expect_copy("Go to Line: Unicode scalar caret and automatic Source focus", positioned, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Go to Line: sentinel undo leaves fixture unchanged", source, refocus=False)

    desktop.key("ctrl+Home")
    desktop.key("ctrl+g")
    desktop.expect_copy("Go to Line: reopening focuses the query", b"1:1", refocus=False)
    desktop.type_ascii("999:1")
    desktop.key("Return")
    desktop.expect_copy("Go to Line: invalid submit keeps query focus", b"999:1", refocus=False)
    desktop.key("Escape")
    desktop.type_ascii("Z")
    desktop.expect_copy("Go to Line: Escape returns focus and invalid address preserves caret", b"Z" + source, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Go to Line: invalid navigation adds no text edit", source, refocus=False)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Go to Line: navigation adds no undo entry", welcome, refocus=False)

    desktop.paste_from_clean(source)
    cases = [
        ("first", ["Return"], "head\ncafé🙂世界\nsame Qneedle\nother needle\n"),
        ("next", ["Return", "F3"], "head\ncafé🙂世界\nsame needle\nother Qneedle\n"),
        ("wrap", ["Return", "F3", "F3"], "head\ncafé🙂世界\nsame Qneedle\nother needle\n"),
        ("backwards", ["shift+F3"], "head\ncafé🙂世界\nsame needle\nother Qneedle\n"),
    ]
    for name, keys, expected in cases:
        desktop.key("ctrl+f")
        # Ctrl+A replaces the query (not the document) after the focus task.
        desktop.key("ctrl+a")
        desktop.type_ascii("needle")
        desktop.expect_copy(f"Find {name}: query automatically focused", b"needle", refocus=False)
        for key in keys:
            desktop.key(key)
        desktop.key("Escape")
        desktop.type_ascii("Q")
        desktop.expect_copy(f"Find {name}: match caret and automatic Source focus", expected.encode("utf-8"), refocus=False)
        desktop.key("ctrl+z")
        desktop.expect_copy(f"Find {name}: undo restores fixture without query edits", source, refocus=False)
    desktop.screenshot(artifacts / "navigation.png")
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Find: all panel navigation leaves document undo history intact", welcome, refocus=False)



def exercise_preview(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = ("# Native **preview**\n\n"
              "café🙂 *italic* ~~strike~~ `code` [inert link](https://example.invalid)\n\n"
              "> Quote\n>\n> 3. ordered\n>    - [x] nested task\n\n"
              "```rust\n\tlet x = 1;  \n\n```\n\n"
              "| a | b |\n| --- | --- |\n| c | d |\n\n"
              "<script>inert source</script>\n\n"
              "![not fetched](https://example.invalid/image.png) $x+1$\n\t").encode("utf-8")
    desktop.paste_from_clean(source)
    desktop.expect_copy("Preview fixture: exact Source before mode change", source, refocus=False)
    desktop.key("ctrl+shift+m")  # Source -> Preview; no editable text widget.
    desktop.screenshot(artifacts / "preview-native.png")
    desktop.type_ascii("LEAK")
    for key in ["Return", "ctrl+shift+b", "ctrl+z"]:
        desktop.key(key)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "950", "650",
                    "click", "--repeat", "8", "--delay", "70", "5")
    desktop.screenshot(artifacts / "preview-fallbacks.png")
    desktop.key("ctrl+shift+m")  # Preview -> Split.
    desktop.screenshot(artifacts / "preview-split.png")
    # Mode switching is not a focus-restoration claim: explicitly click the
    # Source half of Split before taking a fresh, byte-exact clipboard sample.
    desktop.expect_copy("Preview is read-only and Split retains exact Source", source)
    desktop.key("ctrl+shift+m")  # Split -> Source.
    desktop.expect_copy("Full preview cycle preserves terminal whitespace", source)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Preview rendering and blocked input add no undo entry", welcome, refocus=False)



def exercise_tables(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    wide_row = "|" + " cell |" * 13 + "\n"
    wide_delimiter = "|" + " --- |" * 13 + "\n"
    source = ("# Native tables\n\n"
              "| **Name** | Left | Center | Right | Plain | Code | Link | Empty |\n"
              "| --- | :--- | :---: | ---: | --- | --- | --- | --- |\n"
              "| café🙂 | *left* | ~~center~~ | 42 | a\\|b &amp; | `c\\|d` | [inert](https://example.invalid) | |\n"
              "| short | padded |\n"
              "| last | b | c | d | e | f | g | h | EXTRA SOURCE ONLY |\n\n"
              "> | quoted | count |\n> | --- | ---: |\n> | child | 3 |\n\n"
              + wide_row + wide_delimiter + wide_row + "\nterminal\n\t").encode("utf-8")
    desktop.paste_from_clean(source)
    desktop.expect_copy("Table fixture: Source with alignment, ragged rows and limit fallback", source, refocus=False)
    desktop.key("ctrl+shift+m")
    desktop.screenshot(artifacts / "tables-native.png")
    # Send real horizontal wheel events inside the table viewport. Screenshots
    # record runtime behavior, not an automatic assertion of scrollbar position.
    # The new block-source bars put the grid lower than its Stage 5b position.
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "900", "300",
                    "click", "--repeat", "6", "--delay", "70", "7")
    desktop.screenshot(artifacts / "tables-horizontal.png")
    desktop.type_ascii("LEAK")
    desktop.key("ctrl+shift+m")
    desktop.screenshot(artifacts / "tables-split.png")
    desktop.expect_copy("Native table preview and wheel input leave Split Source byte-exact", source)
    desktop.key("ctrl+shift+m")
    desktop.expect_copy("Table mode cycle preserves all source including excess cells and terminal tab", source)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Table rendering adds no undo entries or document edits", welcome, refocus=False)



def exercise_preview_cache(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = ("# Cache\n\n| Name | Count |\n| --- | ---: |\n| café🙂 | 3 |\n\n"
              "```rs\n\tcode  \n\n```\n\t").encode("utf-8")
    edited = source.replace(b"| 3 |", b"| 93 |")
    desktop.paste_from_clean(source)
    desktop.expect_copy("Cache fixture: exact Source before warming preview", source, refocus=False)
    desktop.key("ctrl+shift+m")  # Warm Preview, then keep the model in Split.
    desktop.key("ctrl+shift+m")
    desktop.screenshot(artifacts / "cache-split.png")
    # Public navigation rebuilds the Source native buffer without changing
    # its content revision. Typing then edits the cached table's Count cell.
    desktop.key("ctrl+g")
    desktop.key("ctrl+a")
    desktop.type_ascii("5:11")  # Unicode scalar position immediately before 3.
    desktop.key("Return")
    desktop.type_ascii("9")
    desktop.expect_copy("Warm Split: table cell edit is byte-exact after Go to Line", edited, refocus=False)
    desktop.screenshot(artifacts / "cache-split-edited.png")
    desktop.key("ctrl+z")
    desktop.expect_copy("Warm Split: undo restores the original table cell", source, refocus=False)
    desktop.key("ctrl+shift+z")
    desktop.expect_copy("Warm Split: redo restores the edited table cell", edited, refocus=False)
    desktop.key("ctrl+z")
    desktop.key("ctrl+shift+m")  # Split -> Source.
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Cached preview and navigation add no undo entries", welcome, refocus=False)



def exercise_preview_reveal(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = ("# café🙂\n\nbody **bold**\n\n"
              "| Name | Count |\n| --- | ---: |\n| item | 3 |\n\n"
              "```rs\n\tcode  \n```\n\t").encode("utf-8")
    desktop.paste_from_clean(source)
    desktop.expect_copy("Reveal fixture: exact Source before preview", source, refocus=False)
    desktop.key("ctrl+shift+m")  # Source -> Preview.
    desktop.screenshot(artifacts / "reveal-preview.png")
    # Intentional click on the first block's explicit Show block source bar,
    # NOT a Source editor click/refocus. Fixed 1200x800 client, default config.
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "600", "151", "click", "1")
    time.sleep(0.25)  # Let the navigation message and scoped focus task run.
    desktop.type_ascii("X")
    desktop.expect_copy("Preview source action opens Split and focuses exact block-start caret", b"X" + source, refocus=False)
    desktop.screenshot(artifacts / "reveal-split.png")
    desktop.key("ctrl+z")
    desktop.expect_copy("Reveal: sentinel undo restores exact source and terminal whitespace", source, refocus=False)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Reveal navigation adds no document undo entry", welcome, refocus=False)
    desktop.key("ctrl+shift+m")  # Split -> Source for the lifecycle probe.


def exercise_lifecycle(desktop, artifacts, binary):
    source = "lifecycle café🙂\nkeep me\t".encode("utf-8")
    desktop.paste_from_clean(source)
    desktop.expect_copy("Lifecycle: dirty fixture", source, refocus=False)
    desktop.key("ctrl+End")
    desktop.key("ctrl+n")
    desktop.confirmation("New")
    desktop.screenshot(artifacts / "confirmation-new.png")
    desktop.type_ascii("LEAK")
    for key in ["ctrl+z", "ctrl+shift+z", "ctrl+g", "ctrl+f", "ctrl+o", "ctrl+alt+d"]:
        desktop.key(key)
    desktop.confirmation("New")
    desktop.key("Escape")
    desktop.confirmation(None)
    desktop.type_ascii("X")
    desktop.expect_copy("New Cancel: blocked edits did not leak, caret and keyboard focus restored", source + b"X", refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("New Cancel: undo restores the original dirty document", source, refocus=False)

    desktop.key("ctrl+f")
    desktop.key("ctrl+a")
    desktop.type_ascii("keep")
    desktop.expect_copy("Cancel context: Find query focused before confirmation", b"keep", refocus=False)
    desktop.key("ctrl+n")
    desktop.confirmation("New")
    desktop.key("Escape")
    desktop.confirmation(None)
    desktop.expect_copy("New Cancel restores Find query focus and value", b"keep", refocus=False)
    desktop.type_ascii("me")
    desktop.expect_copy("New Cancel: subsequent query typing stays in Find", b"me", refocus=False)
    desktop.key("Escape")
    desktop.expect_copy("New Cancel from Find preserves source text", source, refocus=False)

    desktop.key("ctrl+Home")
    desktop.key("ctrl+g")
    desktop.expect_copy("Cancel context: Go to Line focused before confirmation", b"1:1", refocus=False)
    desktop.key("ctrl+n")
    desktop.confirmation("New")
    desktop.key("Escape")
    desktop.confirmation(None)
    desktop.expect_copy("New Cancel restores Go to Line focus and value", b"1:1", refocus=False)
    desktop.key("Escape")
    desktop.expect_copy("New Cancel from Go to Line preserves source text", source, refocus=False)

    desktop.key("ctrl+n")
    desktop.confirmation("New")
    desktop.key("alt+d")
    desktop.confirmation(None)
    desktop.title_is_dirty(False)
    desktop.type_ascii("Fresh")
    desktop.expect_copy("New Discard: empty clean document receives fresh keyboard input", b"Fresh", refocus=False)
    # Contextual confirmation keys do not save/discard outside a prompt.
    desktop.key("alt+d")
    desktop.key("alt+s")
    desktop.expect_copy("Confirmation keys outside a prompt preserve the document", b"Fresh", refocus=False)

    desktop.key("ctrl+End")
    desktop.request_close()
    desktop.confirmation("Close")
    desktop.screenshot(artifacts / "confirmation-close.png")
    desktop.type_ascii("LEAK")
    desktop.key("Escape")
    desktop.confirmation(None)
    desktop.type_ascii("Z")
    desktop.expect_copy("Native Close Cancel: document, caret and focus survive", b"FreshZ", refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Native Close Cancel: undo preserves unsaved text", b"Fresh", refocus=False)
    desktop.request_close()
    desktop.confirmation("Close")
    desktop.key("alt+d")
    desktop.expect_native_exit("Native dirty Close Discard exits normally without force-kill")

    # A second clean launch independently covers close without a confirmation.
    desktop.restart_after_native_exit(binary)
    desktop.title_is_dirty(False)
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    desktop.expect_copy("Clean close: fresh process has original clean document", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Native clean Close exits normally without force-kill")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/hylite")
    parser.add_argument("--artifacts", type=Path, default=ROOT / "target/gui-smoke")
    args = parser.parse_args()
    if not os.environ.get("DISPLAY") or os.environ.get("HYLITE_SMOKE_DISPLAY") != os.environ["DISPLAY"]:
        parser.error("Use scripts/test-gui-x11.sh to create an isolated display")
    binary = args.binary.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        parser.error(f"Build the executable first: {binary}")
    artifacts = args.artifacts.resolve()
    artifacts.mkdir(parents=True, exist_ok=True)
    if any(artifacts.iterdir()):
        parser.error("Evidence directory must be empty; use --artifacts with a fresh directory")
    status = "failed"
    desktop = None
    error_message = None

    def timed_out(_signum, _frame):
        raise TimeoutError("GUI smoke exceeded its 180 second global budget")

    signal.signal(signal.SIGALRM, timed_out)
    signal.alarm(180)
    with tempfile.TemporaryDirectory(prefix="hylite-gui-") as temp:
        env = isolated_environment(os.environ, Path(temp))
        with (artifacts / "application.log").open("wb") as log:
            try:
                app = subprocess.Popen([str(binary)], env=env, stdout=log, stderr=log)
                desktop = Desktop(app, env, log)
                exercise(desktop, artifacts)
                exercise_navigation(desktop, artifacts)
                exercise_preview(desktop, artifacts)
                exercise_tables(desktop, artifacts)
                exercise_preview_cache(desktop, artifacts)
                exercise_preview_reveal(desktop, artifacts)
                exercise_lifecycle(desktop, artifacts, binary)
                status = "passed"
                print(f"GUI X11/tiny-skia smoke passed: {len(desktop.passed)} checks (not a Hyprland/Wayland test).", flush=True)
            except Exception as error:
                error_message = str(error)
                if desktop:
                    try:
                        desktop.screenshot(artifacts / "failure.png")
                    except (subprocess.SubprocessError, OSError):
                        pass
                raise
            finally:
                signal.alarm(0)
                if desktop:
                    desktop.cleanup()
                (artifacts / "summary.json").write_text(json.dumps({
                    "status": status, "backend": "X11 / Xvfb / tiny-skia",
                    "checks": desktop.passed if desktop else [], "error": error_message,
                    "native_exits": desktop.native_exits if desktop else 0,
                    "scope": "Source clipboard, navigation focus, read-only Preview/Split and table cycles, warm Split edits/undo/redo, block-source reveal, New/Cancel/Discard and WM_DELETE_WINDOW lifecycle; screenshots without pixel assertions; no file portal or Wayland",
                }, ensure_ascii=False, indent=2) + "\n")
                if status != "passed":
                    log.flush()
                    print((artifacts / "application.log").read_bytes()[-16000:].decode("utf-8", errors="replace"), flush=True)


if __name__ == "__main__":
    main()
