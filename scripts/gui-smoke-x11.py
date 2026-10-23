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
    def __init__(self, app, env, log, arguments=()):
        self.app = app
        self.env = env
        self.log = log
        # Launch arguments (e.g. the workspace folder) are identical for the
        # first start and every restart, so no suite silently changes the state
        # the application boots into.
        self.arguments = [str(argument) for argument in arguments]
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

    def title_ends_with(self, suffix, seconds=12):
        def matches():
            title = self.command("xdotool", "getwindowname", self.window).decode("utf-8")
            return title.endswith(suffix)
        self.wait(f"Window title ends with {suffix!r}", matches, seconds=seconds)
        self.passed_check(f"Window title: {suffix.strip()!r}")

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
        self.app = subprocess.Popen([str(binary), *self.arguments], env=self.env,
                                    stdout=self.log, stderr=self.log)
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



def exercise_cell_reveal(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = ("# Cell source café🙂\n\n"
              "|Name|Count|\n|---|---:|\n|café🙂|**3**|\n|short|\n\n"
              "```rs\n\tcode  \n```\n\t").encode("utf-8")
    edited = source.replace(b"|**3**|", b"|X**3**|")
    desktop.paste_from_clean(source)
    desktop.expect_copy("Cell fixture: exact UTF-8 Source, raw strong markup and terminal tab", source, refocus=False)
    desktop.key("ctrl+shift+m")
    desktop.screenshot(artifacts / "cell-preview.png")
    # Default 1200x800 client: first body row, second column. Left click must
    # remain read-only; only the deliberate right click should navigate.
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "490", "316", "click", "1")
    time.sleep(0.25)
    desktop.type_ascii("LEAK")
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "490", "316", "click", "3")
    time.sleep(0.25)
    desktop.type_ascii("X")
    desktop.expect_copy("Right-click cell opens Split at raw cell start, not styled 3 or a byte/scalar mixup", edited, refocus=False)
    desktop.screenshot(artifacts / "cell-split.png")
    desktop.key("ctrl+z")
    desktop.expect_copy("Cell sentinel undo restores exact source including trailing whitespace", source, refocus=False)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Cell navigation adds no document undo entry", welcome, refocus=False)
    desktop.key("ctrl+shift+m")  # Split -> Source.



def exercise_task_toggle(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = ("- [ ] café🙂 task\n- [X] keep case\n\n"
              "```md\n- [ ] literal code\n```\n\nterminal\n\t").encode("utf-8")
    checked = source.replace(b"- [ ]", b"- [x]", 1)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Task fixture: UTF-8, uppercase X, fenced literal and terminal tab", source, refocus=False)
    desktop.key("ctrl+shift+m")  # Source -> Preview.
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "272", "153", "click", "1")
    time.sleep(0.25)
    desktop.screenshot(artifacts / "task-preview.png")
    desktop.key("ctrl+shift+m")  # Preview -> Split; Source must receive focus.
    desktop.expect_copy("Preview checkbox changes only its canonical Source marker", checked, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("One Undo restores unchecked task byte-exact", source, refocus=False)
    desktop.key("ctrl+shift+z")
    desktop.expect_copy("Redo restores checked task and preserves other raw markers", checked, refocus=False)
    # Click the same actual checkbox in the right-hand Split preview, not Source.
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "762", "153", "click", "1")
    time.sleep(0.25)
    desktop.expect_copy("Split checkbox unchecks and restores Source keyboard focus", source, refocus=False)
    desktop.screenshot(artifacts / "task-split.png")
    desktop.key("ctrl+z")
    desktop.expect_copy("Split task edit is exactly one Undo transaction", checked, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Undo first task edit restores exact original fixture", source, refocus=False)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Undo fixture returns clean welcome without extra checkbox history", welcome, refocus=False)
    desktop.key("ctrl+shift+m")  # Split -> Source for lifecycle.



def exercise_preview_task_history(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = ("- [ ] café🙂 task\n- [X] keep case\n\n"
              "```md\n- [ ] code literal\n```\n\nterminal\n\t").encode("utf-8")
    checked = source.replace(b"- [ ]", b"- [x]", 1)
    def back_to_preview():
        desktop.key("ctrl+shift+m")  # Split -> Source.
        desktop.key("ctrl+shift+m")  # Source -> Preview; no Source refocus click.
    def inspect(label, expected):
        desktop.key("ctrl+shift+m")  # Preview -> Split.
        desktop.expect_copy(label, expected, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Preview history fixture: exact raw Source before task edit", source, refocus=False)
    desktop.key("ctrl+shift+m")
    desktop.key("ctrl+z")  # Source paste is not eligible for Preview history.
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "272", "153", "click", "1")
    time.sleep(0.25)
    desktop.key("ctrl+z")
    inspect("Preview Ctrl+Z undoes only the checkbox transaction", source)
    back_to_preview()
    desktop.key("ctrl+shift+z")
    inspect("Preview Ctrl+Shift+Z redoes checkbox without touching raw body", checked)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("Preview repeated Undo returns exact fixture", source)
    back_to_preview()
    desktop.key("ctrl+y")
    inspect("Preview Ctrl+Y redoes the task transition", checked)
    back_to_preview()
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "40", "62", "click", "1")
    time.sleep(0.25)
    inspect("Preview toolbar Undo follows task-only history guard", source)
    back_to_preview()
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "95", "62", "click", "1")
    time.sleep(0.25)
    inspect("Preview toolbar Redo follows the same task-only guard", checked)
    back_to_preview()
    desktop.screenshot(artifacts / "task-history-preview.png")
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")  # Must stop at the Source paste, not erase fixture.
    desktop.type_ascii("LEAK")
    desktop.key("ctrl+shift+b")
    inspect("Preview history stops at Source boundary and other editing remains blocked", source)
    desktop.key("ctrl+z")  # Source Undo is captured and must execute exactly once.
    desktop.title_is_dirty(False)
    desktop.expect_copy("Source Undo still restores clean welcome after Preview task history", welcome, refocus=False)
    desktop.screenshot(artifacts / "task-history-source.png")
    desktop.key("ctrl+shift+m")  # Split -> Source for lifecycle.


def exercise_preview_heading(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = ("##\tcafé🙂 **raw** &amp; ##  \n\n"
              "- [X] untouched\n\n```md\n## literal\n```\n\t").encode("utf-8")
    demoted = b"#" + source
    desktop.paste_from_clean(source)
    desktop.expect_copy("Heading fixture: exact raw Source with closing hashes and terminal tab", source, refocus=False)
    desktop.key("ctrl+shift+m")

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")  # Preview -> Split; automatic Source focus.
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")  # Split -> Source.
        desktop.key("ctrl+shift+m")  # Source -> Preview.

    # The heading's existing provenance bar keeps its height; H-/H+ are at
    # its right edge. These are product clicks, never Source refocus clicks.
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "984", "151", "click", "1")
    time.sleep(0.25)
    inspect("Preview H+ changes exactly one opening hash", demoted)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("Preview Undo restores raw heading without normalizing its body", source)
    back_to_preview()
    desktop.key("ctrl+y")
    inspect("Preview Redo restores one-hash heading edit", demoted)
    back_to_preview()
    desktop.screenshot(artifacts / "heading-preview.png")
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "932", "151", "click", "1")
    time.sleep(0.25)
    inspect("Preview H- preserves closing hashes, raw inline markup and whitespace", source)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1144", "151", "click", "1")
    time.sleep(0.25)
    desktop.type_ascii("X")
    desktop.expect_copy("Split H+ restores Source keyboard focus at marker end", demoted[:3] + b"X" + demoted[3:], refocus=False)
    desktop.screenshot(artifacts / "heading-split.png")
    desktop.key("ctrl+z")  # Only the native Source sentinel.
    desktop.key("ctrl+z")  # Only the Split heading transaction.
    desktop.expect_copy("Split heading and Source typing remain separate Undo transactions", source, refocus=False)
    desktop.key("ctrl+z")  # Preview H-.
    desktop.key("ctrl+z")  # Preview H+.
    desktop.key("ctrl+z")  # Source paste; no extra history from rendering/navigation.
    desktop.title_is_dirty(False)
    desktop.expect_copy("Heading edits Undo back to exact clean welcome without extra history", welcome, refocus=False)
    desktop.key("ctrl+shift+m")  # Split -> Source for lifecycle.


def exercise_preview_setext(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = ("café🙂 **raw** &amp;  \ncontinued `code`\n   =====  \t\n\n"
              "- [X] untouched\n\n```md\nliteral\n=====\n```\n\t").encode("utf-8")
    demoted = source.replace(b"=====", b"-----", 1)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Setext fixture: multiline raw Unicode body, underline indent and terminal tab", source, refocus=False)
    desktop.key("ctrl+shift+m")

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")  # Preview -> Split, automatic Source focus.
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")  # Split -> Source.
        desktop.key("ctrl+shift+m")  # Source -> Preview.

    desktop.command("xdotool", "mousemove", "--window", desktop.window, "984", "151", "click", "1")
    time.sleep(0.25)
    inspect("Preview Setext H+ changes only the underline run", demoted)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("Preview Setext Undo restores exact raw heading and underline", source)
    back_to_preview()
    desktop.key("ctrl+shift+z")
    desktop.screenshot(artifacts / "setext-preview.png")
    inspect("Preview Setext Redo preserves body, marker count and whitespace", demoted)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1092", "151", "click", "1")
    time.sleep(0.25)
    desktop.type_ascii("X")
    sentinel = source.replace(b"=====  \t", b"=====X  \t", 1)
    desktop.expect_copy("Split Setext H- restores keyboard focus at underline end", sentinel, refocus=False)
    desktop.screenshot(artifacts / "setext-split.png")
    desktop.key("ctrl+z")  # Native Source typing only.
    desktop.key("ctrl+z")  # Split Setext heading only.
    desktop.expect_copy("Setext underline change and native Source typing remain separate transactions", demoted, refocus=False)
    desktop.key("ctrl+z")  # Preview Setext heading.
    desktop.key("ctrl+z")  # Source paste.
    desktop.title_is_dirty(False)
    desktop.expect_copy("Setext edits Undo to exact clean welcome without extra history", welcome, refocus=False)
    desktop.key("ctrl+shift+m")  # Split -> Source for lifecycle.


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


def exercise_task_batch(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = ("- [ ] café🙂 **raw**  \n- [X] keep\n> - [ ] quoted\n"
              "- [\t] unsupported\n\n```md\n- [ ] literal\n```\n\t").encode("utf-8")
    checked = source.replace(b"- [ ]", b"- [x]", 1).replace(b"> - [ ]", b"> - [x]", 1)
    cleared = checked.replace(b"- [x]", b"- [ ]", 1).replace(b"[X]", b"[ ]", 1).replace(b"> - [x]", b"> - [ ]", 1)
    sentinel = cleared.replace(b"> - [ ]", b"> - [ X]", 1)
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Batch suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Batch fixture: mixed case, Unicode, unsupported tab and fenced literal", source, refocus=False)
    desktop.key("ctrl+shift+m")

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")  # Preview -> Split; logical Source focus.
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")  # Split -> Source.
        desktop.key("ctrl+shift+m")  # Source -> Preview.

    desktop.command("xdotool", "mousemove", "--window", desktop.window, "185", "62", "click", "1")
    time.sleep(0.25)
    inspect("Check tasks changes only supported unchecked markers and preserves uppercase X", checked)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("One Preview Undo restores the complete batch fixture byte-exact", source)
    back_to_preview()
    desktop.key("ctrl+shift+z")
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "185", "62", "click", "1")
    time.sleep(0.25)  # Already checked: disabled/no-op must add no history.
    desktop.screenshot(artifacts / "batch-preview.png")
    inspect("Preview batch Redo and already-checked no-op preserve exact bytes", checked)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "305", "62", "click", "1")
    time.sleep(0.25)
    desktop.type_ascii("X")
    desktop.expect_copy("Split Clear tasks restores Source keyboard focus after last changed marker byte", sentinel, refocus=False)
    desktop.screenshot(artifacts / "batch-split.png")
    desktop.key("ctrl+z")  # Native sentinel only.
    desktop.key("ctrl+z")  # Clear batch only, including capital X restoration.
    desktop.expect_copy("Clear batch and native typing are separate Undo transactions", checked, refocus=False)
    desktop.key("ctrl+z")  # Check batch only.
    desktop.key("ctrl+z")  # Source paste, no extra no-op/render history.
    desktop.title_is_dirty(False)
    desktop.expect_copy("Batch Undo returns exact clean welcome with no extra steps", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Batch suite clean client Close exits naturally")


def exercise_table_alignment(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = ("| Name | Count |\n  | ---  | --: |\n"
              "| café🙂 **raw** | a\\|b &amp; |\n| short |\n\nterminal\n\t").encode("utf-8")
    centered = source.replace(b"---", b":---:", 1)
    sentinel = centered.replace(b"| --: |", b"| :--X |", 1)
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Table alignment suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Alignment fixture: raw UTF-8, escaped pipe, ragged row and terminal tab", source, refocus=False)
    desktop.key("ctrl+shift+m")
    # Table action-bar height stays 24 px. Pick-list rows are 11 px text +
    # 2 px vertical padding (18.3 px line boxes); current alignments omitted.
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "984", "151", "click", "1")
    time.sleep(0.25)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "900", "189", "click", "1")
    time.sleep(0.25)  # Column 1: Center, second menu row.

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")
        desktop.key("ctrl+shift+m")

    inspect("Preview alignment changes only the selected delimiter token", centered)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("Preview alignment Undo restores raw delimiter whitespace and dash counts", source)
    back_to_preview()
    desktop.key("ctrl+y")
    desktop.screenshot(artifacts / "alignment-preview.png")
    inspect("Preview alignment Redo preserves all raw cell source", centered)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1144", "151", "click", "1")
    time.sleep(0.25)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1080", "244", "click", "1")
    time.sleep(0.25)  # Column 2: Left, fifth row after Column 1 became Center.
    desktop.type_ascii("X")
    desktop.expect_copy("Split column alignment restores Source focus at delimiter token end", sentinel, refocus=False)
    desktop.screenshot(artifacts / "alignment-split.png")
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    desktop.expect_copy("Alignment and native Source typing are separate Undo transactions", centered, refocus=False)
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Alignment history returns exact clean welcome without extra steps", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Alignment suite clean client Close exits naturally")


def exercise_table_rows(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    row = "  | café🙂 **raw** | a\\|b &amp; |\n".encode("utf-8")
    source = b"| Name | Count |\n  | ---  | --: |\n" + row + b"| short |\n\nafter\n\t"
    duplicated = source.replace(row, row + row, 1)
    sentinel = source.replace(b"| short |\n", b"X| short |\n", 1)
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Table row suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Row fixture preserves UTF-8, escaped pipes, ragged rows and terminal tab", source, refocus=False)
    desktop.key("ctrl+shift+m")
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "984", "151", "click", "1")
    time.sleep(0.25)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "900", "281", "click", "1")
    time.sleep(0.25)  # Six alignment options, then Row 1: Duplicate.

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")
        desktop.key("ctrl+shift+m")

    inspect("Preview Duplicate copies exactly one raw body line without normalizing cells", duplicated)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("One Preview Undo restores the original complete table source", source)
    back_to_preview()
    desktop.key("ctrl+y")
    desktop.screenshot(artifacts / "rows-preview.png")
    inspect("Preview row Redo restores raw duplicate and untouched suffix", duplicated)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1144", "151", "click", "1")
    time.sleep(0.25)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1080", "336", "click", "1")
    time.sleep(0.25)  # Row 2: Delete, tenth option.
    desktop.type_ascii("X")
    desktop.expect_copy("Split Delete restores Source focus at the following physical line start", sentinel, refocus=False)
    desktop.screenshot(artifacts / "rows-split.png")
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    desktop.expect_copy("Row Delete and native Source typing are separate Undo transactions", duplicated, refocus=False)
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Row history returns exact clean welcome without extra steps", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Row suite clean client Close exits naturally")


def exercise_table_cells(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = ("| Name | Count |\n  | ---  | --: |\n"
              " | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
    initial = "café🙂 **raw**".encode("utf-8")
    value = "new **bold** é🙂".encode("utf-8")
    edited = source.replace(initial, value, 1)
    sentinel = edited.replace(b"| a\\|b &amp; |", b"| nextX |", 1)
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Cell suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Cell fixture: canonical root table, UTF-8/raw markup/escaped pipe/terminal tab", source, refocus=False)
    desktop.key("ctrl+shift+m")

    def draft_title(expected):
        desktop.wait(f"Cell draft title = {expected}", lambda: ("[cell draft]" in desktop.command("xdotool", "getwindowname", desktop.window).decode("utf-8")) == expected)

    def open_cell(split=False, second=False):
        desktop.command("xdotool", "mousemove", "--window", desktop.window, "1144" if split else "984", "151", "click", "1")
        time.sleep(0.25)
        desktop.command("xdotool", "mousemove", "--window", desktop.window, "1080" if split else "900", "372" if second else "354", "click", "1")
        draft_title(True)  # Alignment 6 + row actions 2 + header cells 2 precede body cells.

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")
        desktop.key("ctrl+shift+m")

    open_cell()
    desktop.expect_copy("Cell draft receives exact raw payload and native input focus", initial, refocus=False)
    desktop.type_ascii("discard me")
    desktop.key("Escape")
    draft_title(False)
    inspect("Escape Cancel keeps every Source byte and creates no cell edit", source)
    back_to_preview()
    open_cell()
    desktop.put_clipboard(value)
    desktop.key("ctrl+a")
    desktop.key("ctrl+v")
    desktop.expect_copy("Unicode Markdown draft paste is consumed by draft input, not Source", value, refocus=False)
    desktop.key("Return")
    draft_title(False)
    desktop.screenshot(artifacts / "cells-preview.png")
    inspect("Explicit Apply replaces only raw cell payload preserving all outer bytes", edited)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("One Preview Undo restores complete pre-Apply source", source)
    back_to_preview()
    desktop.key("ctrl+y")
    inspect("Preview Redo restores exact Unicode cell edit", edited)
    open_cell(split=True)
    desktop.key("Return")
    draft_title(False)
    desktop.expect_copy("Unchanged Apply closes draft without source normalization or history step", edited, refocus=False)
    open_cell(split=True, second=True)
    desktop.key("ctrl+a")
    desktop.type_ascii("bad|pipe")
    desktop.key("Return")
    draft_title(True)
    desktop.screenshot(artifacts / "cells-invalid.png")
    desktop.key("Escape")
    draft_title(False)
    desktop.expect_copy("Unsafe unescaped pipe Apply is refused and Cancel keeps table structure", edited, refocus=False)
    open_cell(split=True, second=True)
    desktop.key("ctrl+a")
    desktop.type_ascii("next")
    desktop.key("Return")
    draft_title(False)
    desktop.type_ascii("X")
    desktop.expect_copy("Split Apply restores Source focus exactly after the new payload", sentinel, refocus=False)
    desktop.screenshot(artifacts / "cells-split.png")
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    desktop.expect_copy("Cell Apply and native typing are distinct Atomic Undo transactions", edited, refocus=False)
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Cancel/no-op/invalid drafts leave no hidden history before clean welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Cell suite clean client Close exits naturally")


def exercise_cell_navigation(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    initial = "café🙂 **raw**".encode("utf-8")
    value = "applied **é🙂**".encode("utf-8")
    second = b"a\\|b &amp;"
    source = b"| Name | Count |\n  | ---  | --: |\n | " + initial + b" | " + second + b" |\n\nafter\n\t"
    edited = source.replace(initial, value, 1)
    sentinel = source.replace(initial, b"finalX", 1)
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Cell navigation suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Navigation fixture retains raw UTF-8, spacing, escaped pipe and suffix", source, refocus=False)
    desktop.key("ctrl+shift+m")

    def position(row, column):
        label = f"[cell {row}:{column}]"
        desktop.wait(f"Draft position {label}", lambda: label in desktop.command("xdotool", "getwindowname", desktop.window).decode("utf-8"))

    def click(x):
        desktop.command("xdotool", "mousemove", "--window", desktop.window, str(x), "244", "click", "1")
        time.sleep(0.25)

    def open_body(split=False):
        desktop.command("xdotool", "mousemove", "--window", desktop.window, "1144" if split else "984", "151", "click", "1")
        time.sleep(0.25)
        desktop.command("xdotool", "mousemove", "--window", desktop.window, "1080" if split else "900", "354", "click", "1")
        position(1, 1)

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")
        desktop.key("ctrl+shift+m")

    open_body()
    click(240)  # Previous crosses body/header boundary, no Source edit.
    position(0, 2)
    desktop.expect_copy("Previous opens the real raw header cell across a row boundary", b"Count", refocus=False)
    click(360)
    position(1, 1)
    desktop.expect_copy("Next returns to body raw payload with input focus", initial, refocus=False)
    desktop.type_ascii("pending")
    click(360)
    position(1, 1)
    desktop.expect_copy("Dirty Next refuses navigation and keeps unapplied draft text focused", b"pending", refocus=False)
    desktop.put_clipboard(value)
    desktop.key("ctrl+a")
    desktop.key("ctrl+v")
    desktop.expect_copy("Corrected Unicode draft remains valid after dirty navigation refusal", value, refocus=False)
    click(500)  # Apply & Next: commit first body payload and reopen last cell.
    position(1, 2)
    desktop.expect_copy("Apply and Next opens fresh raw adjacent payload after UTF-8 offset changes", second, refocus=False)
    click(360)  # Last-cell Next is disabled; never wrap to another table/header.
    click(500)  # Last-cell Apply & Next is disabled and must not commit.
    position(1, 2)
    click(240)
    position(1, 1)
    desktop.expect_copy("Previous after last-cell boundary returns the applied raw payload", value, refocus=False)
    click(500)  # Unchanged Apply & Next must add no history.
    position(1, 2)
    desktop.expect_copy("Unchanged Apply and Next reopens raw next cell without another transaction", second, refocus=False)
    desktop.screenshot(artifacts / "cell-navigation-draft.png")
    desktop.type_ascii("discard me")
    desktop.key("Escape")
    inspect("Cancel discards only current draft and keeps the earlier Apply and Next commit", edited)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("One Preview Undo reverses the only effective Apply and Next", source)
    back_to_preview()
    desktop.key("ctrl+y")
    inspect("Preview Redo restores exact prior cell bytes after navigation", edited)
    open_body(split=True)
    desktop.key("ctrl+a")
    desktop.type_ascii("final")
    click(500)
    position(1, 2)
    desktop.key("Escape")
    desktop.type_ascii("X")
    desktop.expect_copy("Split Cancel after Apply and Next restores Source focus at applied payload end", sentinel, refocus=False)
    desktop.screenshot(artifacts / "cell-navigation-split.png")
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    desktop.expect_copy("Apply and Next remains separate from subsequent native Source typing", edited, refocus=False)
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Read-only navigation, refusals and no-op add no hidden history before welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Cell navigation suite clean client Close exits naturally")



def exercise_table_insert(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    row = " | café🙂 **raw** | a\\|b &amp; |\n".encode("utf-8")
    source = b"| Name | Count |\n  | ---  | --: |\n" + row + b"\nafter\n\t"
    before = source.replace(row, b" |  |  |\n" + row, 1)
    after = source.replace(row, row + b" |  |  |\n", 1)
    typed = source.replace(row, row + b" | X |  |\n", 1)
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Insert suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Insert fixture retains raw Unicode, escaped pipe, entity and terminal tab", source, refocus=False)
    desktop.key("ctrl+shift+m")
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "984", "151", "click", "1")
    time.sleep(0.25)
    # Alignment(6), old row(2), cell(4), then explicit Insert blank before.
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "900", "391", "click", "1")
    time.sleep(0.25)

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")
        desktop.key("ctrl+shift+m")

    inspect("Preview Insert before adds only one canonical blank row with selected indent", before)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("One Preview Undo removes the complete inserted row byte-exact", source)
    back_to_preview()
    desktop.key("ctrl+y")
    desktop.screenshot(artifacts / "insert-preview.png")
    inspect("Preview Redo restores the same blank row without normalizing old bytes", before)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("Undo before Split restores the original table and menu prefix", source)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1144", "151", "click", "1")
    time.sleep(0.25)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1080", "409", "click", "1")
    time.sleep(0.25)  # Original row: Insert blank after, fourteenth option.
    desktop.type_ascii("X")
    desktop.expect_copy("Split Insert after restores Source focus inside first new blank cell", typed, refocus=False)
    desktop.screenshot(artifacts / "insert-split.png")
    desktop.key("ctrl+z")
    desktop.expect_copy("Native typing Undo does not undo the separate inserted row", after, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("A second Undo removes only Insert after and keeps the fixture", source, refocus=False)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Both insert directions leave no hidden history before clean welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Insert suite clean client Close exits naturally")


def exercise_table_columns(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = ("| Name | Count |\n  | ---  | --: |\n"
              " | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
    before = ("|  | Name | Count |\n  | --- | ---  | --: |\n"
              " |  | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
    after = ("| Name | Count |  |\n  | ---  | --: | --- |\n"
             " | café🙂 **raw** | a\\|b &amp; |  |\n\nafter\n\t").encode("utf-8")
    typed = ("| Name | Count | X |\n  | ---  | --: | --- |\n"
             " | café🙂 **raw** | a\\|b &amp; |  |\n\nafter\n\t").encode("utf-8")
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Column suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Column fixture retains raw Unicode, spacing, escaped pipe, entity and tab", source, refocus=False)
    desktop.key("ctrl+shift+m")
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "984", "151", "click", "1")
    time.sleep(0.25)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "900", "428", "click", "1")
    time.sleep(0.25)  # All 14 old choices, then Column 1: Insert before.

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")
        desktop.key("ctrl+shift+m")

    inspect("Preview Insert column before adds blank header delimiter and body without changing old bytes", before)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("One Preview Undo removes the entire new column, not a partial line", source)
    back_to_preview()
    desktop.key("ctrl+y")
    desktop.screenshot(artifacts / "columns-preview.png")
    inspect("Preview Redo restores blank column while preserving original right alignment", before)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("Undo before Split restores original table and complete old menu prefix", source)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1144", "151", "click", "1")
    time.sleep(0.25)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1080", "483", "click", "1")
    time.sleep(0.25)  # Column 2: Insert after, eighteenth option.
    desktop.type_ascii("X")
    desktop.expect_copy("Split Insert after last column restores Source focus inside new header cell", typed, refocus=False)
    desktop.screenshot(artifacts / "columns-split.png")
    desktop.key("ctrl+z")
    desktop.expect_copy("Undo native header typing preserves the separate complete column insertion", after, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Second Undo removes Insert column after without touching the original fixture", source, refocus=False)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Column edits add no hidden history before clean welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Column suite clean client Close exits naturally")


def exercise_table_column_delete(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = ("| Name | Count |\n  | ---  | --: |\n"
              " | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
    first_deleted = b"| Count |\n  | --: |\n | a\\|b &amp; |\n\nafter\n\t"
    last_deleted = ("| Name |\n  | ---  |\n | café🙂 **raw** |\n\nafter\n\t").encode("utf-8")
    typed = ("| XName |\n  | ---  |\n | café🙂 **raw** |\n\nafter\n\t").encode("utf-8")
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Delete column suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Delete fixture retains raw UTF-8 markup escaped pipe entity and terminal tab", source, refocus=False)
    desktop.key("ctrl+shift+m")
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "984", "151", "click", "1")
    time.sleep(0.25)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "900", "501", "click", "1")
    time.sleep(0.25)  # All eighteen old choices, then Column 1: Delete.

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")
        desktop.key("ctrl+shift+m")

    inspect("Preview Delete first column removes header delimiter body together preserving retained raw bytes", first_deleted)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("One Preview Undo restores every deleted header delimiter and body byte", source)
    back_to_preview()
    desktop.key("ctrl+y")
    desktop.screenshot(artifacts / "column-delete-preview.png")
    inspect("Preview Redo restores one-column table with original right alignment and raw entity", first_deleted)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("Undo before Split restores full original table and eighteen-choice prefix", source)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1144", "151", "click", "1")
    time.sleep(0.25)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1080", "520", "click", "1")
    time.sleep(0.25)  # Column 2: Delete, twentieth option.
    desktop.type_ascii("X")
    desktop.expect_copy("Split Delete last column restores Source focus at surviving header payload start", typed, refocus=False)
    desktop.screenshot(artifacts / "column-delete-split.png")
    desktop.key("ctrl+z")
    desktop.expect_copy("Undo native header typing keeps separate complete column deletion", last_deleted, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Second Undo restores complete deleted last column without normalizing retained bytes", source, refocus=False)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Column deletion adds no hidden history before clean welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Delete column suite clean client Close exits naturally")


def exercise_table_row_move(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    header = b"| Name | Count |\n  | ---  | --: |\n"
    first = "  | café🙂 **raw** | a\\|b &amp; |\n".encode("utf-8")
    last = b"| short |\n"
    source = header + first + last + b"\nafter\n\t"
    moved = header + last + first + b"\nafter\n\t"
    typed = header + b"X| short |\n" + first + b"\nafter\n\t"
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Row move suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Move fixture retains Unicode markup escaped pipe entity ragged row and terminal tab", source, refocus=False)
    desktop.key("ctrl+shift+m")
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "984", "151", "click", "1")
    time.sleep(0.25)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "900", "428", "click", "1")
    time.sleep(0.25)  # Ragged fixture: fourteen old choices, then Row 1: Move down.

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")
        desktop.key("ctrl+shift+m")

    inspect("Preview Move down swaps exactly two raw body lines without changing header or suffix", moved)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("One Preview Undo restores the complete original row ordering byte-exact", source)
    back_to_preview()
    desktop.key("ctrl+y")
    desktop.screenshot(artifacts / "row-move-preview.png")
    inspect("Preview Redo restores exact moved Unicode markup indentation and ragged line", moved)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("Undo before Split restores original body rows and old action menu prefix", source)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1144", "151", "click", "1")
    time.sleep(0.25)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1080", "446", "click", "1")
    time.sleep(0.25)  # Row 2: Move up, sixteenth option; never moves the header.
    desktop.type_ascii("X")
    desktop.expect_copy("Split Move up restores Source focus at the moved physical row start", typed, refocus=False)
    desktop.screenshot(artifacts / "row-move-split.png")
    desktop.key("ctrl+z")
    desktop.expect_copy("Undo native Source typing keeps the separate complete row move", moved, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Second Undo restores both raw rows and their original order", source, refocus=False)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Row moves add no hidden history before clean welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Row move suite clean client Close exits naturally")


def exercise_table_column_move(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = ("| Name | Count |\n  | ---  | --: |\n"
              " | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
    moved = ("| Count | Name |\n  | --: | ---  |\n"
             " | a\\|b &amp; | café🙂 **raw** |\n\nafter\n\t").encode("utf-8")
    typed = ("| XCount | Name |\n  | --: | ---  |\n"
             " | a\\|b &amp; | café🙂 **raw** |\n\nafter\n\t").encode("utf-8")
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Column move suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Column move fixture retains Unicode raw markup escaped pipe entity spacing and tab", source, refocus=False)
    desktop.key("ctrl+shift+m")
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "984", "151", "click", "1")
    time.sleep(0.25)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "900", "538", "click", "1")
    time.sleep(0.25)  # Twenty old single-body choices, then Column 1: Move right.

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")
        desktop.key("ctrl+shift+m")

    inspect("Preview Move right swaps whole raw columns including delimiter alignment and body", moved)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("One Preview Undo restores the entire original column order byte-exact", source)
    back_to_preview()
    desktop.key("ctrl+y")
    desktop.screenshot(artifacts / "column-move-preview.png")
    inspect("Preview Redo restores exact moved column raw segments indentation and alignment", moved)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("Undo before Split restores full original table and old action menu prefix", source)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1144", "151", "click", "1")
    time.sleep(0.25)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1080", "556", "click", "1")
    time.sleep(0.25)  # Column 2: Move left, twenty-second option; no wrap.
    desktop.type_ascii("X")
    desktop.expect_copy("Split Move left restores Source focus at selected moved header payload start", typed, refocus=False)
    desktop.screenshot(artifacts / "column-move-split.png")
    desktop.key("ctrl+z")
    desktop.expect_copy("Undo native header typing retains the separate complete column move", moved, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Second Undo restores both columns and original alignment without normalization", source, refocus=False)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Column moves add no hidden history before clean welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Column move suite clean client Close exits naturally")

def exercise_table_column_duplicate(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    source = ("| Name | Count |\n  | ---  | --: |\n"
              " | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
    first = ("| Name | Name | Count |\n  | ---  | ---  | --: |\n"
             " | café🙂 **raw** | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
    last = ("| Name | Count | Count |\n  | ---  | --: | --: |\n"
            " | café🙂 **raw** | a\\|b &amp; | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
    typed = ("| Name | Count | XCount |\n  | ---  | --: | --: |\n"
             " | café🙂 **raw** | a\\|b &amp; | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Column duplicate suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Column duplicate fixture retains Unicode raw markup escaped pipe entity spacing and tab", source, refocus=False)
    desktop.key("ctrl+shift+m")
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "984", "151", "click", "1")
    time.sleep(0.25)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "900", "575", "click", "1")
    time.sleep(0.25)  # All twenty-two old choices, then Column 1: Duplicate.

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")
        desktop.key("ctrl+shift+m")

    inspect("Preview Duplicate copies entire first raw column including delimiter alignment and body", first)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("One Preview Undo removes the complete duplicate column byte-exact", source)
    back_to_preview()
    desktop.key("ctrl+y")
    desktop.screenshot(artifacts / "column-duplicate-preview.png")
    inspect("Preview Redo restores exact copied column raw segments indentation and alignment", first)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("Undo before Split restores full original table and old action menu prefix", source)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1144", "151", "click", "1")
    time.sleep(0.25)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "1080", "593", "click", "1")
    time.sleep(0.25)  # Column 2: Duplicate; caret in the new third header, not original.
    desktop.type_ascii("X")
    desktop.expect_copy("Split Duplicate restores Source focus at new copied header payload start", typed, refocus=False)
    desktop.screenshot(artifacts / "column-duplicate-split.png")
    desktop.key("ctrl+z")
    desktop.expect_copy("Undo native header typing retains the separate complete column duplication", last, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Second Undo removes copied column and restores original alignment without normalization", source, refocus=False)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Column duplication adds no hidden history before clean welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Column duplicate suite clean client Close exits naturally")


def exercise_cell_apply_previous(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    initial = "café🙂 **raw**".encode("utf-8")
    second = b"a\\|b &amp;"
    value = "applied **é🙂**".encode("utf-8")
    source = b"| Name | Count |\n  | ---  | --: |\n | " + initial + b" | " + second + b" |\n\nafter\n\t"
    edited = source.replace(second, value, 1)
    sentinel = source.replace(second, b"finalX", 1)
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Apply Previous suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Backward fixture retains Unicode raw markup escaped pipe alignment spacing and suffix", source, refocus=False)
    desktop.key("ctrl+shift+m")

    def position(row, column):
        label = f"[cell {row}:{column}]"
        desktop.wait(f"Backward draft position {label}", lambda: label in desktop.command("xdotool", "getwindowname", desktop.window).decode("utf-8"))

    def apply_previous():
        desktop.command("xdotool", "mousemove", "--window", desktop.window, "650", "244", "click", "1")
        time.sleep(0.25)  # New fourth button, all three old navigation positions fixed.

    def open_last(split=False):
        desktop.command("xdotool", "mousemove", "--window", desktop.window, "1144" if split else "984", "151", "click", "1")
        time.sleep(0.25)
        desktop.command("xdotool", "mousemove", "--window", desktop.window, "1080" if split else "900", "373", "click", "1")
        position(1, 2)

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")
        desktop.key("ctrl+shift+m")

    open_last()
    desktop.expect_copy("Backward last body draft receives exact escaped pipe entity payload", second, refocus=False)
    desktop.key("ctrl+a")
    desktop.type_ascii("bad|pipe")
    desktop.expect_copy("Invalid raw draft stays in input before disabled Apply Previous", b"bad|pipe", refocus=False)
    apply_previous()
    position(1, 2)
    desktop.key("Escape")
    inspect("Disabled invalid Apply Previous and Cancel leave Source byte-exact", source)
    back_to_preview()
    open_last()
    desktop.put_clipboard(value)
    desktop.key("ctrl+a")
    desktop.key("ctrl+v")
    apply_previous()
    position(1, 1)
    desktop.expect_copy("Apply Previous commits Unicode and focuses fresh preceding raw body payload", initial, refocus=False)
    desktop.screenshot(artifacts / "cell-apply-previous-draft.png")
    apply_previous()
    position(0, 2)
    desktop.expect_copy("Unchanged Apply Previous crosses delimiter to real Count header without history", b"Count", refocus=False)
    apply_previous()
    position(0, 1)
    desktop.expect_copy("Backward header transition opens Name without wrapping", b"Name", refocus=False)
    desktop.key("ctrl+a")
    desktop.type_ascii("first pending")
    desktop.expect_copy("Modified first-cell draft stays unapplied before disabled backward boundary", b"first pending", refocus=False)
    apply_previous()
    position(0, 1)
    desktop.key("Escape")
    inspect("Cancel keeps only the earlier applied second body payload byte-exact", edited)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("One Preview Undo reverses only effective Apply Previous", source)
    back_to_preview()
    desktop.key("ctrl+y")
    inspect("Preview Redo restores exact Unicode backward cell edit", edited)
    open_last(split=True)
    desktop.key("ctrl+a")
    desktop.type_ascii("final")
    apply_previous()
    position(1, 1)
    desktop.key("Escape")
    desktop.type_ascii("X")
    desktop.expect_copy("Split Cancel after Apply Previous restores Source focus at applied payload end", sentinel, refocus=False)
    desktop.screenshot(artifacts / "cell-apply-previous-split.png")
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    desktop.expect_copy("Apply Previous and native Source typing are separate atomic history edges", edited, refocus=False)
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Backward no-op invalid first-boundary and Cancel add no hidden history before welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Apply Previous suite clean client Close exits naturally")


def exercise_paragraph_draft(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    initial = "café🙂 **raw** &amp; a\\|b  ".encode("utf-8")
    value = "edited **é🙂** &amp; a\\|b".encode("utf-8")
    suffix = b"\n\n# untouched\n\nterminal\n\t"
    source, edited, typed = initial + suffix, value + suffix, b"finalX" + suffix
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Paragraph suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Paragraph fixture preserves raw Unicode markup entity escaped pipe spaces and suffix", source, refocus=False)
    desktop.key("ctrl+shift+m")

    def draft_title(active):
        desktop.wait("Paragraph draft title transition", lambda: ("[paragraph draft]" in desktop.command("xdotool", "getwindowname", desktop.window).decode("utf-8")) == active)

    def open_paragraph(split=False):
        desktop.command("xdotool", "mousemove", "--window", desktop.window, "1144" if split else "984", "151", "click", "1")
        time.sleep(0.25)
        draft_title(True)

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")
        desktop.key("ctrl+shift+m")

    open_paragraph()
    desktop.expect_copy("Root paragraph draft receives exact raw payload and native input focus", initial, refocus=False)
    desktop.type_ascii("discard me")
    desktop.key("Escape")
    draft_title(False)
    inspect("Paragraph Cancel preserves every Source byte and creates no history", source)
    back_to_preview()
    open_paragraph()
    desktop.put_clipboard(value)
    desktop.key("ctrl+a")
    desktop.key("ctrl+v")
    desktop.expect_copy("Unicode paragraph paste stays in draft until explicit Apply", value, refocus=False)
    desktop.key("Return")
    draft_title(False)
    desktop.screenshot(artifacts / "paragraph-preview.png")
    inspect("Paragraph Apply replaces one payload preserving LF neighboring blocks and terminal tab", edited)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("One Preview Undo restores full original raw paragraph", source)
    back_to_preview()
    desktop.key("ctrl+y")
    inspect("Preview Redo restores exact Unicode paragraph edit", edited)
    open_paragraph(split=True)
    desktop.key("Return")
    draft_title(False)
    desktop.expect_copy("Unchanged paragraph Apply closes without normalization or a history edge", edited, refocus=False)
    open_paragraph(split=True)
    desktop.key("ctrl+a")
    desktop.type_ascii("# heading")
    desktop.key("Return")
    draft_title(True)
    desktop.expect_copy("Parser refuses paragraph to heading conversion while retaining raw draft", b"# heading", refocus=False)
    desktop.screenshot(artifacts / "paragraph-invalid.png")
    desktop.key("Escape")
    draft_title(False)
    desktop.expect_copy("Rejected paragraph structure change and Cancel preserve Source", edited, refocus=False)
    open_paragraph(split=True)
    desktop.key("ctrl+a")
    desktop.type_ascii("final")
    desktop.key("Return")
    draft_title(False)
    desktop.type_ascii("X")
    desktop.expect_copy("Split paragraph Apply restores Source focus at exact applied payload end", typed, refocus=False)
    desktop.screenshot(artifacts / "paragraph-split.png")
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    desktop.expect_copy("Paragraph Apply and native typing are separate atomic history edges", edited, refocus=False)
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Paragraph Cancel unchanged and rejected drafts add no hidden history before welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Paragraph suite clean client Close exits naturally")


def exercise_paragraph_multiline(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    initial = "café🙂 **raw**  \na\\|b &amp;".encode("utf-8")
    value = "edited **é🙂**\nsecond  \nthird &amp;".encode("utf-8")
    suffix = b"\n\n# untouched\n\nterminal\n\t"
    source, edited, typed = initial + suffix, value + suffix, b"final\nnextX" + suffix
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Multiline suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Multiline fixture preserves Unicode raw hard break spaces escaped pipe entity and suffix", source, refocus=False)
    desktop.key("ctrl+shift+m")

    def draft_title(active):
        desktop.wait("Multiline draft title transition", lambda: ("[multiline paragraph draft]" in desktop.command("xdotool", "getwindowname", desktop.window).decode("utf-8")) == active)

    def open_paragraph(split=False):
        desktop.command("xdotool", "mousemove", "--window", desktop.window, "1144" if split else "984", "151", "click", "1")
        time.sleep(0.25)
        draft_title(True)

    def apply():
        desktop.command("xdotool", "mousemove", "--window", desktop.window, "225", "364", "click", "1")
        time.sleep(0.25)

    def paste(value):
        desktop.put_clipboard(value)
        desktop.key("ctrl+a")
        desktop.key("ctrl+v")

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_preview():
        desktop.key("ctrl+shift+m")
        desktop.key("ctrl+shift+m")

    open_paragraph()
    desktop.expect_copy("Multiline raw draft receives exact hard break payload and scoped native editor focus", initial, refocus=False)
    desktop.type_ascii("discard")
    desktop.key("Return")
    desktop.type_ascii("pending")
    draft_title(True)
    desktop.expect_copy("Enter inserts a draft newline instead of applying Source", b"discard\npending", refocus=False)
    desktop.key("Escape")
    draft_title(False)
    inspect("Multiline Cancel preserves Source and creates no history despite native Enter", source)
    back_to_preview()
    open_paragraph()
    paste(value)
    desktop.expect_copy("Unicode three line Markdown paste stays local until explicit Apply", value, refocus=False)
    apply()
    draft_title(False)
    desktop.screenshot(artifacts / "multiline-preview.png")
    inspect("Multiline Apply replaces one complete raw payload keeping final LF neighbors and suffix", edited)
    back_to_preview()
    desktop.key("ctrl+z")
    inspect("One Preview Undo restores original soft and hard break bytes", source)
    back_to_preview()
    desktop.key("ctrl+y")
    inspect("Preview Redo restores exact multiline Unicode paragraph", edited)
    open_paragraph(split=True)
    apply()
    draft_title(False)
    desktop.expect_copy("Unchanged multiline Apply closes without another history edge", edited, refocus=False)
    open_paragraph(split=True)
    paste(b"first\n# heading")
    apply()
    draft_title(True)
    desktop.expect_copy("Parser refuses multiline paragraph split and restores draft input focus", b"first\n# heading", refocus=False)
    desktop.screenshot(artifacts / "multiline-invalid.png")
    desktop.key("Escape")
    draft_title(False)
    desktop.expect_copy("Rejected multiline split and Cancel keep Source byte-exact", edited, refocus=False)
    open_paragraph(split=True)
    paste(b"final\nnext")
    apply()
    draft_title(False)
    desktop.type_ascii("X")
    desktop.expect_copy("Split multiline Apply restores Source focus at last applied line payload end", typed, refocus=False)
    desktop.screenshot(artifacts / "multiline-split.png")
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    desktop.expect_copy("Multiline Apply and subsequent native Source typing are separate atomic edges", edited, refocus=False)
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Multiline Cancel unchanged and refused drafts leave no hidden history before welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Multiline paragraph suite clean client Close exits naturally")


def exercise_paragraph_draft_history(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    initial = "café🙂 **raw**  \na\\|b &amp;".encode("utf-8")
    alpha = "alpha é🙂  \nbeta\nlast &amp;".encode("utf-8")
    invalid, branch = b"bad\n\ntrailing\t", b"branch\nkept"
    final = "final é🙂\nnext  ".encode("utf-8")
    suffix = b"\n\n# untouched\n\nterminal\n\t"
    source, applied, typed = initial + suffix, final + suffix, final + b"X" + suffix
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Draft history suite starts with clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Draft history fixture preserves raw Source bytes", source, refocus=False)
    desktop.key("ctrl+shift+m")

    def draft_title(active):
        desktop.wait("Local history draft title transition", lambda: ("[multiline paragraph draft]" in desktop.command("xdotool", "getwindowname", desktop.window).decode("utf-8")) == active)

    def open_paragraph(split=False):
        desktop.command("xdotool", "mousemove", "--window", desktop.window, "1144" if split else "984", "151", "click", "1")
        time.sleep(0.25)
        draft_title(True)

    def paste(value):
        desktop.put_clipboard(value)
        desktop.key("ctrl+a")
        desktop.key("ctrl+v")

    def copy(description, value):
        desktop.expect_copy(description, value, refocus=False)
        draft_title(True)

    open_paragraph()
    desktop.key("ctrl+z")
    copy("Empty draft Undo is captured without undoing the Source fixture", initial)
    paste(alpha)
    copy("Unicode three-line paste creates one local draft history edit", alpha)
    desktop.key("ctrl+z")
    copy("Local Undo restores initial exact hard break paragraph", initial)
    desktop.key("ctrl+y")
    copy("Ctrl Y restores complete Unicode paste in one local Redo", alpha)
    desktop.key("ctrl+z")
    desktop.key("ctrl+shift+z")
    copy("Ctrl Shift Z is also contextual draft Redo", alpha)
    paste(invalid)
    copy("Invalid blank and final tab bytes stay in local draft history", invalid)
    desktop.key("ctrl+z")
    copy("Undo of invalid raw draft restores valid prior text", alpha)
    desktop.key("ctrl+y")
    copy("Redo restores invalid final tab without normalization or Source mutation", invalid)
    desktop.key("ctrl+z")
    paste(branch)
    copy("New native edit after Undo starts a local branch", branch)
    desktop.key("ctrl+y")
    copy("Abandoned draft Redo cannot reappear or leak into Source history", branch)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "425", "364", "click", "1")
    time.sleep(0.25)
    copy("Undo draft button restores previous payload and native input focus without refocus", alpha)
    desktop.screenshot(artifacts / "draft-history-preview.png")
    desktop.key("Escape")
    draft_title(False)
    desktop.key("ctrl+shift+m")
    desktop.expect_copy("Cancel discards entire local history and preserves Source fixture", source, refocus=False)
    open_paragraph(split=True)
    desktop.key("ctrl+y")
    copy("Reopened Split draft has no previous session Redo", initial)
    paste(final)
    copy("Split final draft remains staged before explicit Apply", final)
    desktop.key("ctrl+z")
    copy("Split local Undo still leaves complete Source transaction untouched", initial)
    desktop.key("ctrl+y")
    copy("Split local Redo restores trailing spaces exactly", final)
    desktop.command("xdotool", "mousemove", "--window", desktop.window, "225", "364", "click", "1")
    time.sleep(0.25)
    draft_title(False)
    desktop.type_ascii("X")
    desktop.expect_copy("Apply after local Redo restores Source focus at final payload end", typed, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Source Undo first removes native typing not any local draft edit", applied, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("One more Source Undo removes entire Apply regardless of draft history", source, refocus=False)
    desktop.key("ctrl+shift+m")  # Split -> Source
    desktop.key("ctrl+shift+m")  # Source -> Preview; redo must use Preview history
    desktop.key("ctrl+y")
    desktop.key("ctrl+shift+m")
    desktop.expect_copy("Preview Redo restores only committed paragraph not draft history", applied, refocus=False)
    desktop.screenshot(artifacts / "draft-history-source.png")
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Cancelled and applied local histories leave no hidden Source Undo before welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Draft history suite clean client Close exits naturally")


def exercise_paragraph_navigation(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    first = "café🙂 **first**  \na\\|b &amp;".encode("utf-8")
    middle = b"\n\n# gap\n\nsingle line\n\n> nested\n> paragraph\n\n```text\nliteral\nparagraph\n```\n\n"
    second = "second é🙂\nraw **tail**  ".encode("utf-8")
    suffix = b"\n\nterminal\n\t"
    value = "edited é🙂\nlast  ".encode("utf-8")
    source = first + middle + second + suffix
    applied = first + middle + value + suffix
    typed = first + middle + value + b"X" + suffix
    pending = b"changed\npending"
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Paragraph navigation starts at clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Navigation fixture includes raw Unicode roots heading single nested and fenced blocks", source, refocus=False)
    desktop.key("ctrl+shift+m")  # Source -> Preview

    def draft_title(active):
        desktop.wait("Navigation draft title transition", lambda: ("[multiline paragraph draft]" in desktop.command("xdotool", "getwindowname", desktop.window).decode("utf-8")) == active)

    def click(x, y):
        desktop.command("xdotool", "mousemove", "--window", desktop.window, str(x), str(y), "click", "1")
        time.sleep(0.25)

    def open_paragraph(split=False):
        click(1144 if split else 984, 151)
        draft_title(True)

    def copy(description, expected):
        desktop.expect_copy(description, expected, refocus=False)
        draft_title(True)

    def paste(raw):
        desktop.put_clipboard(raw)
        desktop.key("ctrl+a")
        desktop.key("ctrl+v")

    open_paragraph()
    copy("First multiline draft opens with native input focus and exact hard break bytes", first)
    paste(pending)
    copy("Changed first draft remains local before navigation", pending)
    click(450, 408)  # Next multiline
    copy("Dirty Next refuses to discard text and restores draft selection and input focus", pending)
    desktop.key("ctrl+z")
    copy("Local Undo restores unchanged first payload enabling explicit navigation", first)
    click(450, 408)
    copy("Next skips heading single nested and fenced content to exact second root payload", second)
    desktop.key("ctrl+y")
    copy("Successful navigation starts fresh local history without previous session Redo", second)
    click(280, 408)  # Previous multiline
    copy("Previous returns to first eligible root without Source change", first)
    desktop.screenshot(artifacts / "paragraph-navigation-preview.png")
    # Boundary buttons can blur input; inspect before click, then product Cancel.
    # Do not add a draft/Source refocus workaround for disabled native controls.
    click(280, 408)
    desktop.key("Escape")
    draft_title(False)
    desktop.key("ctrl+shift+m")  # Preview -> Split
    desktop.expect_copy("First boundary click and Cancel leave every Source byte and history intact", source, refocus=False)
    open_paragraph(split=True)
    click(450, 408)
    copy("Split navigation opens exact last supported paragraph with input focus", second)
    click(450, 408)  # last boundary disabled
    desktop.key("Escape")
    draft_title(False)
    desktop.expect_copy("Last boundary click and Cancel do not wrap or apply any text", source, refocus=False)
    open_paragraph(split=True)
    click(450, 408)
    paste(value)
    copy("Edit second root after navigation stays staged and retains Unicode trailing spaces", value)
    click(280, 408)
    copy("Dirty Previous also keeps second draft and leaves explicit Apply available", value)
    desktop.screenshot(artifacts / "paragraph-navigation-dirty.png")
    click(225, 364)  # existing explicit Apply
    draft_title(False)
    desktop.type_ascii("X")
    desktop.expect_copy("Apply after refused navigation edits only second payload and restores Source final-line caret focus", typed, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Source typing remains separate from navigated paragraph Apply", applied, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("One atomic Undo restores both original roots and every skipped neighbor byte", source, refocus=False)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Previous Next Cancel and boundaries create no hidden Source history before welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Paragraph navigation clean client Close exits naturally")


def exercise_paragraph_apply_next(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    first = "café🙂 **first**  \na\\|b &amp;".encode("utf-8")
    middle = b"\n\n# gap\n\nsingle line\n\n> nested\n> paragraph\n\n```text\nliteral\nparagraph\n```\n\n"
    second = "second é🙂\nraw **tail**  ".encode("utf-8")
    suffix = b"\n\nterminal\n\t"
    value = "grown **é🙂**\nline two  \nthird &amp;".encode("utf-8")
    final = "final é🙂\nnext  ".encode("utf-8")
    source, applied = first + middle + second + suffix, value + middle + second + suffix
    final_source, typed = final + middle + second + suffix, final + b"X" + middle + second + suffix
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Apply Next paragraph suite starts at clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Apply Next fixture preserves both raw roots and unsupported neighbors", source, refocus=False)
    desktop.key("ctrl+shift+m")

    def draft_title(active):
        desktop.wait("Apply Next draft title transition", lambda: ("[multiline paragraph draft]" in desktop.command("xdotool", "getwindowname", desktop.window).decode("utf-8")) == active)

    def click(x, y):
        desktop.command("xdotool", "mousemove", "--window", desktop.window, str(x), str(y), "click", "1")
        time.sleep(0.25)

    def open_paragraph(split=False):
        click(1144 if split else 984, 151)
        draft_title(True)

    def paste(raw):
        desktop.put_clipboard(raw)
        desktop.key("ctrl+a")
        desktop.key("ctrl+v")

    def copy(description, expected):
        desktop.expect_copy(description, expected, refocus=False)
        draft_title(True)

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")  # Preview -> Split
        desktop.expect_copy(description, expected, refocus=False)

    def preview():
        desktop.key("ctrl+shift+m")  # Split -> Source
        desktop.key("ctrl+shift+m")  # Source -> Preview

    open_paragraph()
    copy("Initial Apply Next draft receives exact raw hard break text and input focus", first)
    paste(value)
    copy("Three line Unicode raw replacement is staged before explicit Apply Next", value)
    click(625, 408)
    copy("Apply Next commits first payload and opens exact next root with fresh offsets and native focus", second)
    desktop.key("ctrl+z")
    copy("New next session has empty local Undo and cannot undo the Source commit", second)
    click(280, 408)
    copy("Previous after Apply Next authenticates fresh grown first paragraph bytes", value)
    click(625, 408)
    copy("Unchanged Apply Next moves forward without another Source history edge", second)
    paste(b"last\npending")
    copy("Last supported paragraph changed draft remains staged before disabled boundary action", b"last\npending")
    click(625, 408)  # disabled last boundary; Copy first then Cancel, no refocus hack
    desktop.key("Escape")
    draft_title(False)
    inspect("Last boundary and Cancel leave only the earlier first paragraph Apply committed", applied)
    preview()
    desktop.key("ctrl+z")
    inspect("One Preview Undo removes exactly the first Apply Next despite unchanged transition and Cancel", source)
    preview()
    desktop.key("ctrl+y")
    inspect("Preview Redo restores exact grown first root and every skipped neighbor byte", applied)
    open_paragraph(split=True)
    paste(b"one\n# heading")
    copy("Structural replacement is staged locally before Apply Next parser preflight", b"one\n# heading")
    click(625, 408)
    copy("Apply Next parser refusal retains raw draft and restores input focus without partial Apply", b"one\n# heading")
    desktop.screenshot(artifacts / "paragraph-apply-next-refused.png")
    desktop.key("Escape")
    draft_title(False)
    desktop.expect_copy("Cancel after failed Apply Next keeps complete previously committed Source", applied, refocus=False)
    open_paragraph(split=True)
    paste(final)
    copy("Split final first paragraph replacement is still a local draft", final)
    click(625, 408)
    copy("Split Apply Next loads raw second paragraph as a fresh focused native session", second)
    desktop.screenshot(artifacts / "paragraph-apply-next-split.png")
    desktop.key("Escape")
    draft_title(False)
    desktop.type_ascii("X")
    desktop.expect_copy("Cancel next draft restores Source focus at prior applied first payload end not next paragraph", typed, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Source typing remains a separate edge after Apply Next and Cancel", final_source, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("One Source Undo restores previous grown paragraph before final Apply Next", applied, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Prior Apply Next is its own atomic edge preserving original complete Source", source, refocus=False)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Noop transition boundary rejection and Cancel create no hidden history before welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Paragraph Apply Next clean client Close exits naturally")


def exercise_paragraph_apply_previous(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    first = "café🙂 **first**  \na\\|b &amp;".encode("utf-8")
    middle = b"\n\n# gap\n\nsingle line\n\n> nested\n> paragraph\n\n```text\nliteral\nparagraph\n```\n\n"
    second = "second é🙂\nraw **tail**  ".encode("utf-8")
    between = b"\n\n## second gap\n\n- item\n  - nested item\n\n"
    third = "third 🙂\nraw final  ".encode("utf-8")
    suffix = b"\n\nterminal\n\t"
    third2 = "growth **é🙂**\nfinal two  \nthird  ".encode("utf-8")
    first2 = "first edited é🙂\nsecond line  ".encode("utf-8")
    final = "final é🙂\nsecond two  ".encode("utf-8")
    source = first + middle + second + between + third + suffix
    applied = first + middle + second + between + third2 + suffix
    applied2 = first + middle + final + between + third2 + suffix
    typed = first + middle + final + b"X" + between + third2 + suffix
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Apply Previous paragraph suite starts at clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Apply Previous fixture keeps three raw roots and unsupported neighbors", source, refocus=False)
    desktop.key("ctrl+shift+m")

    def draft_title(active):
        desktop.wait("Apply Previous draft title transition", lambda: ("[multiline paragraph draft]" in desktop.command("xdotool", "getwindowname", desktop.window).decode("utf-8")) == active)

    def click(x, y):
        desktop.command("xdotool", "mousemove", "--window", desktop.window, str(x), str(y), "click", "1")
        time.sleep(0.25)

    def open_paragraph(split=False):
        click(1144 if split else 984, 151)
        draft_title(True)

    def paste(raw):
        desktop.put_clipboard(raw)
        desktop.key("ctrl+a")
        desktop.key("ctrl+v")

    def copy(description, expected):
        desktop.expect_copy(description, expected, refocus=False)
        draft_title(True)

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")  # Preview -> Split
        desktop.expect_copy(description, expected, refocus=False)

    open_paragraph()
    copy("First raw root opens with native focus for Apply Previous suite", first)
    click(450, 408)  # Next multiline
    copy("Next move reaches the unchanged middle root before any Source edge", second)
    click(450, 408)
    copy("Second Next move reaches the third eligible root with exact raw bytes", third)
    paste(third2)
    copy("Three line Unicode third replacement is staged before explicit Apply Previous", third2)
    click(302, 452)
    copy("Apply Previous commits the third payload and opens exact middle root with fresh offsets and focus", second)
    desktop.key("ctrl+z")
    copy("New previous session has empty local Undo and cannot undo the Source commit", second)
    click(302, 452)
    copy("Unchanged Apply Previous reaches the first root without another Source history edge", first)
    paste(first2)
    copy("First root replacement is staged locally before the disabled first boundary action", first2)
    click(302, 452)  # disabled first boundary blurs the input; Copy after it is not a focus claim
    desktop.key("Escape")
    draft_title(False)
    inspect("First boundary plus Cancel leave only the earlier third paragraph Apply committed", applied)
    desktop.key("ctrl+z")
    desktop.expect_copy("One Source Undo removes exactly the third Apply Previous and no boundary edge", source, refocus=False)
    desktop.key("ctrl+y")
    desktop.expect_copy("Source Redo restores the exact grown third root with every skipped byte", applied, refocus=False)
    open_paragraph(split=True)
    click(450, 408)  # Next multiline
    copy("Split middle root receives the unchanged raw payload before a structural draft", second)
    paste(b"one\n# heading")
    copy("Structural replacement is staged locally before Apply Previous parser preflight", b"one\n# heading")
    click(302, 452)
    copy("Apply Previous parser refusal retains the raw draft and native focus without partial Apply", b"one\n# heading")
    desktop.screenshot(artifacts / "paragraph-apply-previous-refused.png")
    desktop.key("Escape")
    draft_title(False)
    desktop.expect_copy("Cancel after refused Apply Previous keeps every committed Source byte", applied, refocus=False)
    open_paragraph(split=True)
    click(450, 408)
    copy("Split middle root is reopened unchanged after the refusal was cancelled", second)
    paste(final)
    copy("Final middle replacement is a local draft only", final)
    click(302, 452)
    copy("Apply Previous loads the raw first paragraph as a fresh focused native session", first)
    desktop.screenshot(artifacts / "paragraph-apply-previous-split.png")
    desktop.key("Escape")
    draft_title(False)
    desktop.type_ascii("X")
    desktop.expect_copy("Cancel previous draft restores Source focus at prior applied middle payload end not first", typed, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Typing stays a separate Source edge after Apply Previous and Cancel", applied2, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Another Source Undo removes only the final Apply Previous", applied, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("The first Apply Previous is its own atomic edge back to the clean welcome fixture", source, refocus=False)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Boundary refusal and unchanged transitions leave no hidden history before welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Paragraph Apply Previous clean client Close exits naturally")


def exercise_paragraph_draft_shortcuts(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    first = "café🙂 **first**  \na\\|b &amp;".encode("utf-8")
    middle = b"\n\n# gap\n\nsingle line\n\n> nested\n> paragraph\n\n```text\nliteral\nparagraph\n```\n\n"
    second = "second é🙂\nraw **tail**  ".encode("utf-8")
    suffix = b"\n\nterminal\n\t"
    edited = "edited **é🙂**\nsecond  \nthird &amp;".encode("utf-8")
    changed = "changed 🙂\nlast draft  ".encode("utf-8")
    source = first + middle + second + suffix
    applied = edited + middle + second + suffix
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Draft shortcut suite starts at clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Draft shortcut fixture keeps both raw multiline roots", source, refocus=False)
    desktop.key("ctrl+shift+m")

    def draft_title(active):
        desktop.wait("Draft shortcut title transition", lambda: ("[multiline paragraph draft]" in desktop.command("xdotool", "getwindowname", desktop.window).decode("utf-8")) == active)

    def click(x, y):
        desktop.command("xdotool", "mousemove", "--window", desktop.window, str(x), str(y), "click", "1")
        time.sleep(0.25)

    def open_paragraph(split=False):
        click(1144 if split else 984, 151)
        draft_title(True)

    def paste(raw):
        desktop.put_clipboard(raw)
        desktop.key("ctrl+a")
        desktop.key("ctrl+v")

    def copy(description, expected):
        desktop.expect_copy(description, expected, refocus=False)

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")  # Preview -> Split refocuses the Source editor
        desktop.expect_copy(description, expected, refocus=False)

    open_paragraph()
    copy("First raw root is focused before any commit shortcut", first)
    paste(edited)
    copy("Three line replacement stays a local draft until Ctrl+Enter", edited)
    desktop.key("ctrl+Return")
    draft_title(False)
    inspect("Ctrl+Enter applies exactly one atomic Source edge and closes the draft", applied)
    open_paragraph(split=True)
    copy("Reopened committed root shows the applied raw bytes", edited)
    desktop.key("ctrl+Next")
    copy("Ctrl+PageDown applies and opens the exact next root without changing Source", second)
    desktop.key("ctrl+Prior")
    copy("Ctrl+PageUp applies and opens the exact previous root with fresh offsets", edited)
    desktop.key("ctrl+Prior")
    copy("Ctrl+PageUp at the first boundary keeps the draft and never wraps", edited)
    desktop.key("ctrl+Next")
    copy("Ctrl+PageDown returns to the last root unchanged with no Source edge", second)
    paste(changed)
    copy("Changed last draft is staged before the disabled commit shortcut", changed)
    desktop.key("ctrl+Next")
    copy("Ctrl+PageDown at the last boundary refuses and keeps the changed draft", changed)
    desktop.key("Escape")
    draft_title(False)
    copy("Escape cancels the boundary draft without touching committed Source", applied)
    desktop.key("ctrl+z")
    copy("One Source Undo removes only the Ctrl+Enter Apply", source)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    copy("Second Source Undo returns the clean welcome document", welcome)
    desktop.request_close()
    desktop.expect_native_exit("Draft commit shortcut clean client Close exits naturally")


def exercise_single_paragraph_navigation(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    first = "café🙂 **first**  ".encode("utf-8")
    gap = b"\n\n# gap\n\n> nested\n> quote\n\n```text\ncode\nblock\n```\n\n"
    second = "second é🙂 &amp;".encode("utf-8")
    between = b"\n\n## second gap\n\n- item\n  - nested item\n\n"
    third = "third 🙂  ".encode("utf-8")
    suffix = b"\n\nterminal\n\t"
    third2 = "growth **é🙂**  ".encode("utf-8")
    first2 = "first edited 🙂".encode("utf-8")
    final = "final é🙂".encode("utf-8")
    source = first + gap + second + between + third + suffix
    applied = first + gap + second + between + third2 + suffix
    final_source = final + gap + second + between + third2 + suffix
    typed = final + b"X" + gap + second + between + third2 + suffix
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Single paragraph navigation suite starts at clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(source)
    desktop.expect_copy("Single paragraph fixture keeps three eligible roots and unsupported blocks", source, refocus=False)
    desktop.key("ctrl+shift+m")

    def draft_title(active):
        desktop.wait("Single paragraph draft title transition", lambda: ("[paragraph draft]" in desktop.command("xdotool", "getwindowname", desktop.window).decode("utf-8")) == active)

    def click(x, y):
        desktop.command("xdotool", "mousemove", "--window", desktop.window, str(x), str(y), "click", "1")
        time.sleep(0.25)

    def open_paragraph(split=False):
        click(1144 if split else 984, 151)
        draft_title(True)

    def paste(raw):
        desktop.put_clipboard(raw)
        desktop.key("ctrl+a")
        desktop.key("ctrl+v")

    def copy(description, expected):
        desktop.expect_copy(description, expected, refocus=False)
        draft_title(True)

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")  # Preview -> Split refocuses the Source editor
        desktop.expect_copy(description, expected, refocus=False)

    open_paragraph()
    copy("First single-line root opens with exact raw payload and native focus", first)
    click(430, 244)  # Next paragraph
    copy("Next skips heading quote fence and list blocks to the exact second root", second)
    click(430, 244)
    copy("Second Next reaches the third root with its raw trailing spaces", third)
    paste(third2)
    copy("Two-space Unicode replacement stays a local draft until Apply & Previous", third2)
    click(270, 288)  # Apply & Previous
    copy("Apply & Previous commits the third payload and opens the exact middle root", second)
    click(270, 288)
    copy("Unchanged Apply & Previous reaches the first root without a Source edge", first)
    desktop.key("ctrl+Prior")
    copy("Ctrl+PageUp is not a commit route for the text_input draft", first)
    paste(first2)
    copy("First root replacement is staged before the disabled first boundary action", first2)
    click(270, 288)  # disabled Apply & Previous at the first root
    desktop.key("Escape")
    draft_title(False)
    inspect("First boundary plus Cancel keep only the earlier third Apply in Source", applied)
    desktop.key("ctrl+z")
    desktop.expect_copy("One Source Undo removes exactly the Apply & Previous commit", source, refocus=False)
    desktop.key("ctrl+y")
    desktop.expect_copy("Source Redo restores the exact grown third root and every skipped byte", applied, refocus=False)
    open_paragraph(split=True)
    click(430, 244)
    copy("Split middle root receives the unchanged raw payload before a structural draft", second)
    paste(b"# heading")
    copy("Structural single-line replacement is staged before Apply & Next preflight", b"# heading")
    click(428, 288)  # Apply & Next
    copy("Apply & Next parse refusal retains the raw draft and native focus", b"# heading")
    desktop.screenshot(artifacts / "single-paragraph-navigation-refused.png")
    desktop.key("Escape")
    draft_title(False)
    desktop.expect_copy("Cancel after refused Apply & Next keeps every committed Source byte", applied, refocus=False)
    open_paragraph(split=True)
    paste(final)
    copy("Final first-root replacement is a local draft in Split", final)
    click(428, 288)
    copy("Apply & Next opens the raw middle paragraph as a fresh focused draft", second)
    desktop.screenshot(artifacts / "single-paragraph-navigation-split.png")
    desktop.key("Escape")
    draft_title(False)
    desktop.type_ascii("X")
    desktop.expect_copy("Cancel restores Source focus at the prior applied payload end not the middle root", typed, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("Typing stays a separate Source edge after Apply & Next and Cancel", final_source, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("One Source Undo removes only the final Apply & Next", applied, refocus=False)
    desktop.key("ctrl+z")
    desktop.expect_copy("The earlier Apply & Previous is its own atomic edge back to the fixture", source, refocus=False)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("Boundary refusal and unchanged moves leave no hidden history before welcome", welcome, refocus=False)
    desktop.request_close()
    desktop.expect_native_exit("Single paragraph navigation clean client Close exits naturally")


def exercise_draft_history_single(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    payload = "café🙂 **raw** &amp;  ".encode("utf-8")
    para_source = payload + b"\n\n# untouched\n\ntail \xf0\x9f\x99\x82\n\t"
    first = "first edited 🙂".encode("utf-8")
    kept = "kept 🙂".encode("utf-8")
    kept2 = "kept 🙂 two".encode("utf-8")
    applied = para_source.replace(payload, kept, 1)
    table = ("| Name | Count |\n  | ---  | --: |\n | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
    cell_initial = "café🙂 **raw**".encode("utf-8")
    cell_a = "café🙂 **raw** A".encode("utf-8")
    cell_kept = "kept cell 🙂".encode("utf-8")
    cell_applied = table.replace(cell_initial, cell_kept, 1)
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Draft history suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(para_source)
    desktop.expect_copy("Paragraph fixture keeps raw Unicode markup and the untouched suffix", para_source, refocus=False)
    desktop.key("ctrl+shift+m")  # Preview -> Split; Source keeps focus for later copies.

    def draft_title(marker, active):
        desktop.wait("Draft history title transition", lambda: ((marker in desktop.command("xdotool", "getwindowname", desktop.window).decode("utf-8")) == active))

    def click(x, y):
        desktop.command("xdotool", "mousemove", "--window", desktop.window, str(x), str(y), "click", "1")
        time.sleep(0.25)

    def open_paragraph():
        click(984, 151)
        draft_title("[paragraph draft]", True)

    def open_cell():
        click(984, 151)
        click(900, 354)
        draft_title("[cell draft]", True)

    def paste(raw):
        desktop.put_clipboard(raw)
        desktop.key("ctrl+a")
        desktop.key("ctrl+v")

    def copy(description, expected):
        desktop.expect_copy(description, expected, refocus=False)

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")  # Split -> Source refocuses the editor
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_split():
        desktop.key("ctrl+shift+m")  # Source -> Preview
        desktop.key("ctrl+shift+m")  # Preview -> Split

    open_paragraph()
    copy("Paragraph draft opens with the exact raw payload and native focus", payload)
    paste(first)
    copy("Paragraph replacement is only a local draft snapshot", first)
    click(566, 288)  # Undo draft
    copy("Undo draft restores the previous local snapshot without touching Source", payload)
    click(694, 288)  # Redo draft
    copy("Redo draft reapplies the reverted local value", first)
    click(566, 288)
    copy("Second Undo draft returns to the original raw payload", payload)
    desktop.key("Escape")
    draft_title("[paragraph draft]", False)
    inspect("Local paragraph Undo and Redo never created a Source edit", para_source)
    back_to_split()
    open_paragraph()
    paste(kept)
    paste(kept2)
    copy("Two paragraph snapshots are staged before a local restore", kept2)
    click(566, 288)
    copy("Paragraph Undo draft keeps the first staged value", kept)
    desktop.key("Return")
    draft_title("[paragraph draft]", False)
    inspect("Paragraph Apply commits the restored local value as one Source edge", applied)
    desktop.key("ctrl+z")
    copy("One Source Undo removes only that paragraph edge", para_source)
    paste(table)  # Whole-document replace keeps one Source edge for phase two.
    copy("Table fixture replaces the paragraph document byte for byte", table)
    back_to_split()
    open_cell()
    copy("Cell draft opens with the exact raw cell payload and native focus", cell_initial)
    paste(cell_a)
    copy("Cell replacement is only a local draft snapshot", cell_a)
    click(486, 200)  # Undo draft
    copy("Cell Undo draft restores the raw cell without touching Source", cell_initial)
    click(614, 200)  # Redo draft
    copy("Cell Redo draft reapplies the local value", cell_a)
    click(486, 200)
    copy("Second cell Undo draft returns to the committed payload", cell_initial)
    desktop.key("Escape")
    draft_title("[cell draft]", False)
    inspect("Cell draft Undo and Redo leave every Source byte untouched", table)
    back_to_split()
    open_cell()
    paste(cell_kept)
    paste(cell_kept + b" 2")
    click(486, 200)
    copy("Cell Undo draft keeps the first staged value", cell_kept)
    desktop.key("Return")
    draft_title("[cell draft]", False)
    inspect("Cell Apply commits only the restored local value", cell_applied)
    desktop.key("ctrl+z")
    copy("One Source Undo removes the single cell edge", table)
    desktop.key("ctrl+z")
    copy("A second Source Undo removes the whole-document replace", para_source)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    copy("Both single-line panels left no hidden history behind welcome", welcome)
    desktop.request_close()
    desktop.expect_native_exit("Draft history suite clean client Close exits naturally")


def exercise_draft_history_keys(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    payload = "café🙂 **raw** &amp;  ".encode("utf-8")
    para_source = payload + b"\n\n# untouched\n\ntail \xf0\x9f\x99\x82\n\t"
    first = "first edited 🙂".encode("utf-8")
    table = ("| Name | Count |\n  | ---  | --: |\n | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
    cell_initial = "café🙂 **raw**".encode("utf-8")
    cell_a = "café🙂 **raw** A".encode("utf-8")
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.expect_copy("Keyboard history suite: fresh clean welcome", welcome, refocus=False)
    desktop.paste_from_clean(para_source)
    desktop.expect_copy("Keyboard history fixture keeps raw Unicode and the untouched suffix", para_source, refocus=False)
    desktop.key("ctrl+shift+m")  # Preview -> Split; Source keeps focus for later copies.

    def draft_title(marker, active):
        desktop.wait(
            "Keyboard history title transition",
            lambda: (marker in desktop.command("xdotool", "getwindowname", desktop.window).decode("utf-8")) == active,
        )

    def click(x, y):
        desktop.command("xdotool", "mousemove", "--window", desktop.window, str(x), str(y), "click", "1")
        time.sleep(0.25)

    def open_paragraph():
        click(984, 151)
        draft_title("[paragraph draft]", True)

    def open_cell():
        click(984, 151)
        click(900, 354)
        draft_title("[cell draft]", True)

    def paste(raw):
        desktop.put_clipboard(raw)
        desktop.key("ctrl+a")
        desktop.key("ctrl+v")

    def copy(description, expected):
        desktop.expect_copy(description, expected, refocus=False)

    def inspect(description, expected):
        desktop.key("ctrl+shift+m")  # Split -> Source refocuses the editor
        desktop.expect_copy(description, expected, refocus=False)

    def back_to_split():
        desktop.key("ctrl+shift+m")  # Source -> Preview
        desktop.key("ctrl+shift+m")  # Preview -> Split

    open_paragraph()
    copy("Keyboard history: paragraph draft opens with the exact raw payload", payload)
    paste(first)
    copy("Typing a local change is only a draft snapshot", first)
    desktop.key("ctrl+z")
    copy("Ctrl+Z undoes the local paragraph snapshot and nothing else", payload)
    desktop.key("ctrl+y")
    copy("Ctrl+Y reapplies the local paragraph snapshot", first)
    desktop.key("ctrl+z")
    desktop.key("ctrl+shift+z")
    copy("Ctrl+Shift+Z redoes after a local Undo", first)
    desktop.key("ctrl+z")
    desktop.key("ctrl+z")
    copy("A second local Undo without snapshots is a no-op", payload)
    desktop.key("Escape")
    draft_title("[paragraph draft]", False)
    inspect("Keyboard history never created a paragraph Source edit", para_source)
    paste(table)  # Still in Source: whole-document replace keeps one edge.
    copy("Keyboard history table fixture replaces the document byte for byte", table)
    back_to_split()
    open_cell()
    copy("Keyboard history: cell draft opens with the exact raw payload", cell_initial)
    paste(cell_a)
    copy("Cell typing stays a local draft snapshot", cell_a)
    desktop.key("ctrl+z")
    copy("Ctrl+Z in the cell draft restores the committed payload", cell_initial)
    desktop.key("ctrl+shift+z")
    copy("Ctrl+Shift+Z in the cell draft reapplies the local value", cell_a)
    desktop.key("Escape")
    draft_title("[cell draft]", False)
    inspect("Cell keyboard history leaves every Source byte untouched", table)
    desktop.key("ctrl+z")
    copy("Source Undo after the cell panel removes the table replace", para_source)
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    copy("Both panels left no hidden Source history behind welcome", welcome)
    desktop.request_close()
    desktop.expect_native_exit("Keyboard history suite clean client Close exits naturally")


def exercise_tabs(desktop, artifacts):
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    one = "tab one 🙂 **raw**".encode("utf-8")
    two = "tab two é &amp;".encode("utf-8")
    desktop.find_window()
    desktop.title_is_dirty(False)

    def copy(description, expected):
        desktop.expect_copy(description, expected, refocus=False)

    def paste(raw):
        desktop.put_clipboard(raw)
        desktop.key("ctrl+a")
        desktop.key("ctrl+v")

    copy("Tabs suite starts at the clean welcome buffer", welcome)
    # Ctrl+T opens a truly empty buffer: the first fixture is the only content.
    desktop.key("ctrl+t")
    paste(one)
    copy("Ctrl+T parks the welcome buffer and starts an empty tab", one)
    desktop.title_is_dirty(True)
    # Alt+1 returns to the parked buffer byte for byte.
    desktop.key("alt+1")
    copy("Alt+1 restores the parked welcome buffer unchanged", welcome)
    desktop.key("alt+2")
    copy("Alt+2 restores the second tab with its own text", one)
    # A local undo only touches the active tab.
    desktop.key("ctrl+z")
    paste(two)
    copy("Undo in the second tab never reaches the first buffer", two)
    desktop.key("alt+1")
    copy("The first buffer is still untouched after the second tab undo", welcome)
    # Ctrl+W closes the clean first tab and falls back to its neighbour.
    desktop.key("ctrl+w")
    copy("Ctrl+W closes the clean first tab and activates the survivor", two)
    # The last tab is protected, so the editor keeps one buffer and stays open.
    desktop.key("ctrl+w")
    copy("Ctrl+W on the last tab is refused and keeps the buffer", two)
    desktop.key("ctrl+z")
    # The surviving tab returns to its empty saved checkpoint, so the clean
    # title (an existing oracle) proves the undo landed in this buffer.
    desktop.title_is_dirty(False)
    desktop.request_close()
    desktop.expect_native_exit("Tabs suite clean client Close exits naturally")


def workspace_fixture(root):
    """The on-disk workspace the tree suite navigates. Bytes are exact UTF-8."""
    root = Path(root)
    documents = {
        Path("alpha.md"): "alpha tree 🙂".encode("utf-8"),
        Path("beta.md"): "beta tree é".encode("utf-8"),
        Path("notes") / "nested.md": "nested tree ✓".encode("utf-8"),
    }
    for name, data in documents.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    return documents


def exercise_workspace(desktop, artifacts, workspace):
    workspace = Path(workspace)
    documents = workspace_fixture(workspace)
    welcome = (ROOT / "src" / "welcome.md").read_bytes()
    desktop.find_window()
    desktop.title_is_dirty(False)

    def copy(description, expected):
        # Keyboard-only: no click can move the selection or the sidebar width.
        desktop.expect_copy(description, expected, refocus=False)

    def cycle(key, times):
        for _ in range(times):
            desktop.key(key)

    copy("Workspace suite starts at the clean welcome buffer", welcome)
    # Ten moves from the first row clamp on the last row, so the initial
    # selection position does not depend on scan timing.
    cycle("alt+j", 10)
    copy("Alt+J clamps on the last row and never edits the document", welcome)
    desktop.key("alt+Return")
    copy("Alt+Enter opens the selected tree document", documents[Path("beta.md")])
    desktop.title_ends_with("beta.md — Hylite\n")

    # A collapsed folder hides its children from the tree and from navigation.
    cycle("alt+k", 3)
    desktop.key("alt+Return")
    copy("Collapsing a folder opens nothing", documents[Path("beta.md")])
    desktop.key("alt+j")
    desktop.key("alt+Return")
    copy("After collapse the next row is the next document", documents[Path("alpha.md")])
    desktop.title_ends_with("alpha.md — Hylite\n")

    # Expanding restores the row that was hidden.
    desktop.key("alt+k")
    desktop.key("alt+Return")
    desktop.key("alt+j")
    desktop.key("alt+Return")
    copy("Expanding the folder restores its nested document",
         documents[Path("notes") / "nested.md"])
    desktop.title_ends_with("nested.md — Hylite\n")

    # No watcher: a file written after the scan appears only after F5.
    gamma = "gamma tree ✓".encode("utf-8")
    (workspace / "gamma.md").write_bytes(gamma)
    desktop.key("F5")
    cycle("alt+j", 10)
    desktop.key("alt+Return")
    copy("F5 re-scan lists a new file without a watcher", gamma)
    desktop.title_ends_with("gamma.md — Hylite\n")

    desktop.title_is_dirty(False)
    desktop.request_close()
    desktop.expect_native_exit("Workspace suite closes naturally after tree opens")


def tiny_png(width=32, height=16):
    """A real, decodable PNG so the Preview can actually render it."""
    import struct
    import zlib

    rows = b"".join(b"\x00" + b"\xff\x00\x00" * width for _ in range(height))

    def chunk(tag, data):
        crc = zlib.crc32(tag + data) & 0xFFFFFFFF
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", crc)

    header = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header)
            + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))


IMAGE_DOCUMENT = b"""# Images

![local pic](pic.png)

![remote](https://example.invalid/x.png)

![missing](gone.png)

![too big](big.png)

before ![inline](pic.png) after
"""


def images_fixture(root):
    """One local picture, one remote URL, one missing file, one oversized file.

    Returns the document path; the document bytes are read back from disk by the
    suite so the clipboard oracle is the file, not a copy held in Python.
    """
    root = Path(root)
    root.mkdir(parents=True, exist_ok=True)
    document = root / "doc.md"
    document.write_bytes(IMAGE_DOCUMENT)
    (root / "pic.png").write_bytes(tiny_png())
    # Header says 16x16 but the file is one byte over the 4 MiB gate.
    oversized = bytearray(tiny_png(16, 16))
    oversized.extend(b"\x00" * (4 * 1024 * 1024 + 1 - len(oversized)))
    (root / "big.png").write_bytes(bytes(oversized))
    return document


def exercise_images(desktop, artifacts, document):
    document = Path(document)
    source = document.read_bytes()
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.title_ends_with("doc.md — Hylite\n")

    def copy(description):
        desktop.expect_copy(description, source, refocus=False)

    copy("Launch argument opens the image document byte for byte")
    # Preview is read-only, so the marker paste must never reach the document.
    desktop.key("ctrl+shift+m")
    desktop.screenshot(artifacts / "images-preview.png")
    desktop.put_clipboard(b"must not land\n")
    desktop.key("ctrl+v")
    desktop.key("ctrl+shift+m")
    desktop.screenshot(artifacts / "images-split.png")
    desktop.key("ctrl+shift+m")
    copy("A paste made in Preview never edits the image document")

    # Split renders the same local picture and every rejected marker again.
    desktop.key("ctrl+shift+m")
    desktop.key("ctrl+shift+m")
    desktop.key("ctrl+shift+m")
    desktop.title_is_dirty(False)
    copy("Source bytes are unchanged after Preview and Split")

    desktop.request_close()
    desktop.expect_native_exit("Images suite closes naturally after rendering pictures")


PDF_DOCUMENT = b"""# Catatan PDF

Isi **tebal** dan `kode` serta caf\xc3\xa9.

| Kolom | Nilai |
| :--- | ---: |
| satu | 1 |
| dua | 2 |

```
baris kode
```

Emoji \xf0\x9f\x99\x82 ditolak.
"""

PDF_SCOPE = ("Headless PDF export: `hylite export-pdf` renders the same page the toolbar action "
             "would, a structurally valid PDF 1.4 with a resolvable xref and matching stream "
             "lengths, headings, bold, inline code, table cells and code lines in the content "
             "stream, non-WinAnsi text degraded to '?', the Markdown byte for byte untouched, and "
             "refusals that write nothing for a missing input, a self-overwrite or a malformed "
             "command line; the GUI still shows the untouched document and closes naturally; no "
             "save-dialog automation, font embedding, exact glyph positions, pixel oracle, dark "
             "theme PDF, compression, links, images or Wayland proof")


def pdf_fixture(root):
    """A document whose projection is visible in the PDF content stream."""
    root = Path(root)
    root.mkdir(parents=True, exist_ok=True)
    document = root / "catatan.md"
    document.write_bytes(PDF_DOCUMENT)
    return document


def assert_pdf_document(pdf):
    """Validate the PDF 1.4 skeleton the renderer promises, then return its text.

    latin-1 keeps every byte as one character, so string offsets are byte
    offsets and the xref numbers can be checked directly.
    """
    if not pdf.startswith(b"%PDF-1.4\n"):
        raise AssertionError(f"Missing PDF 1.4 header: {pdf[:16]!r}")
    if not pdf.endswith(b"%%EOF\n"):
        raise AssertionError(f"Missing EOF marker: {pdf[-16:]!r}")
    text = pdf.decode("latin-1")
    marker = text.rfind("startxref\n")
    if marker == -1:
        raise AssertionError("No startxref section")
    digits = ""
    for character in text[marker + len("startxref\n"):]:
        if not character.isdigit():
            break
        digits += character
    if not digits:
        raise AssertionError("startxref carries no offset")
    offset = int(digits)
    if not text[offset:].startswith("xref\n"):
        raise AssertionError(f"startxref {offset} does not point at the xref table")
    lines = text[offset:].split("\n")
    if lines[0] != "xref":
        raise AssertionError(f"First xref line is {lines[0]!r}")
    count = int(lines[1].split()[1])
    if lines[2] != "0000000000 65535 f ":
        raise AssertionError(f"Free entry is {lines[2]!r}")
    for number in range(1, count):
        entry = lines[2 + number]
        if len(entry) != 19:
            raise AssertionError(f"Object {number} xref entry is {len(entry)} bytes, not 19")
        at = int(entry[:10])
        if not text[at:].startswith(f"{number} 0 obj\n"):
            raise AssertionError(f"Object {number} offset {at} does not point at its header")
    # Every content stream must declare its real length.
    position = 0
    streams = 0
    while True:
        position = text.find("<< /Length ", position)
        if position == -1:
            break
        head = text[position + len("<< /Length "):]
        length = int(head.split()[0])
        body = head.find("stream\n") + len("stream\n")
        if not head[body + length:].startswith("endstream"):
            raise AssertionError(f"Stream at {position} declares {length} bytes")
        streams += 1
        position += len("<< /Length ")
    if streams == 0:
        raise AssertionError("No content stream found")
    return text


def run_export_cli(binary, arguments, env):
    """Run the headless export; no window, no save dialog, exact bytes back."""
    return subprocess.run([str(binary), "export-pdf", *[str(argument) for argument in arguments]],
                          env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=60)


def exercise_pdf(desktop, artifacts, binary, document):
    document = Path(document)
    artifacts = Path(artifacts)
    source = document.read_bytes()
    desktop.find_window()
    desktop.title_is_dirty(False)
    desktop.title_ends_with("catatan.md — Hylite\n")
    desktop.screenshot(artifacts / "before-export.png")

    output = artifacts / "catatan.pdf"
    result = run_export_cli(binary, [document, output], desktop.env)
    if result.returncode != 0:
        raise AssertionError(f"export-pdf exited {result.returncode}: {result.stderr[-2000:]!r}")
    text = assert_pdf_document(output.read_bytes())
    desktop.passed_check("CLI writes a PDF 1.4 whose xref resolves every object and whose stream lengths match")

    needles = ["(Catatan PDF) Tj", "(tebal) Tj", "(kode) Tj", "(Kolom) Tj", "(Nilai) Tj",
               "(satu) Tj", "(baris kode) Tj"]
    for needle in needles:
        if needle not in text:
            raise AssertionError(f"Missing {needle!r} in the content stream")
    desktop.passed_check("Headings, bold, inline code, table cells and code lines reach the content stream")

    if "Emoji ? ditolak." not in text:
        raise AssertionError("Non-WinAnsi text was not degraded to '?'")
    if "caf\u00e9" not in text:
        raise AssertionError("WinAnsi text (café) did not survive")
    desktop.passed_check("WinAnsi text survives and non-WinAnsi degrades to '?' instead of corrupting the page")

    if document.read_bytes() != source:
        raise AssertionError("Exporting modified the Markdown source")
    desktop.passed_check("Export leaves the Markdown byte for byte")

    refusal = run_export_cli(binary, [document, document], desktop.env)
    if refusal.returncode == 0 or document.read_bytes() != source:
        raise AssertionError(f"Overwriting the source was not refused (exit {refusal.returncode})")
    desktop.passed_check("Refuses to overwrite the Markdown source and writes nothing")

    missing = document.parent / "hilang.md"
    missing_pdf = document.parent / "hilang.pdf"
    absent = run_export_cli(binary, [missing, missing_pdf], desktop.env)
    if absent.returncode == 0 or missing_pdf.exists():
        raise AssertionError(f"A missing input was not refused (exit {absent.returncode})")
    desktop.passed_check("A missing input fails without creating an output file")

    usage = subprocess.run([str(binary), "export-pdf"], env=desktop.env,
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=60)
    if usage.returncode != 2 or b"usage: hylite export-pdf" not in usage.stderr:
        raise AssertionError(f"Malformed command line exited {usage.returncode}: {usage.stderr[-500:]!r}")
    desktop.passed_check("A malformed command line is a usage error with the documented text (exit 2)")

    # The window kept running the whole time and still owns the untouched file.
    desktop.expect_copy("The GUI document is still the exact Markdown after the exports", source,
                        refocus=False)
    desktop.screenshot(artifacts / "after-export.png")
    desktop.title_is_dirty(False)
    desktop.request_close()
    desktop.expect_native_exit("PDF suite closes the untouched document naturally")


def exercise_suite(desktop, artifacts, binary, suite="regression", workspace=None,
                   document=None):
    if suite == "regression":
        exercise(desktop, artifacts)
        exercise_navigation(desktop, artifacts)
        exercise_preview(desktop, artifacts)
        exercise_tables(desktop, artifacts)
        exercise_preview_cache(desktop, artifacts)
        exercise_preview_reveal(desktop, artifacts)
        exercise_cell_reveal(desktop, artifacts)
        exercise_task_toggle(desktop, artifacts)
        exercise_preview_task_history(desktop, artifacts)
        exercise_preview_heading(desktop, artifacts)
        exercise_preview_setext(desktop, artifacts)
        exercise_lifecycle(desktop, artifacts, binary)
    elif suite == "task-batch":
        exercise_task_batch(desktop, artifacts)
    elif suite == "table-alignment":
        exercise_table_alignment(desktop, artifacts)
    elif suite == "table-rows":
        exercise_table_rows(desktop, artifacts)
    elif suite == "table-cells":
        exercise_table_cells(desktop, artifacts)
    elif suite == "cell-navigation":
        exercise_cell_navigation(desktop, artifacts)
    elif suite == "table-insert":
        exercise_table_insert(desktop, artifacts)
    elif suite == "table-columns":
        exercise_table_columns(desktop, artifacts)
    elif suite == "table-column-delete":
        exercise_table_column_delete(desktop, artifacts)
    elif suite == "table-row-move":
        exercise_table_row_move(desktop, artifacts)
    elif suite == "table-column-move":
        exercise_table_column_move(desktop, artifacts)
    elif suite == "table-column-duplicate":
        exercise_table_column_duplicate(desktop, artifacts)
    elif suite == "cell-apply-previous":
        exercise_cell_apply_previous(desktop, artifacts)
    elif suite == "paragraph-draft":
        exercise_paragraph_draft(desktop, artifacts)
    elif suite == "paragraph-multiline":
        exercise_paragraph_multiline(desktop, artifacts)
    elif suite == "paragraph-draft-history":
        exercise_paragraph_draft_history(desktop, artifacts)
    elif suite == "paragraph-navigation":
        exercise_paragraph_navigation(desktop, artifacts)
    elif suite == "paragraph-apply-next":
        exercise_paragraph_apply_next(desktop, artifacts)
    elif suite == "paragraph-apply-previous":
        exercise_paragraph_apply_previous(desktop, artifacts)
    elif suite == "paragraph-draft-shortcuts":
        exercise_paragraph_draft_shortcuts(desktop, artifacts)
    elif suite == "paragraph-navigation-single":
        exercise_single_paragraph_navigation(desktop, artifacts)
    elif suite == "draft-history-single":
        exercise_draft_history_single(desktop, artifacts)
    elif suite == "draft-history-keys":
        exercise_draft_history_keys(desktop, artifacts)
    elif suite == "tabs":
        exercise_tabs(desktop, artifacts)
    elif suite == "workspace":
        if workspace is None:
            raise ValueError("The workspace suite needs a workspace folder")
        exercise_workspace(desktop, artifacts, workspace)
    elif suite == "images":
        if document is None:
            raise ValueError("The images suite needs a document path")
        exercise_images(desktop, artifacts, document)
    elif suite == "pdf":
        if document is None:
            raise ValueError("The pdf suite needs a document path")
        exercise_pdf(desktop, artifacts, binary, document)
    else:
        raise ValueError(f"Unknown GUI suite: {suite}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite", choices=("regression", "task-batch", "table-alignment", "table-rows", "table-cells", "cell-navigation", "table-insert", "table-columns", "table-column-delete", "table-row-move", "table-column-move", "table-column-duplicate", "cell-apply-previous", "paragraph-draft", "paragraph-multiline", "paragraph-draft-history", "paragraph-navigation", "paragraph-apply-next", "paragraph-apply-previous", "paragraph-draft-shortcuts", "paragraph-navigation-single", "draft-history-single", "draft-history-keys", "tabs", "workspace", "images", "pdf"),
                     default="regression")
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
        # The tree suite boots with a folder argument; every other suite keeps
        # the exact single-argument launch the older gates rely on.
        workspace = Path(temp) / "workspace" if args.suite == "workspace" else None
        arguments = [workspace] if workspace else []
        if workspace:
            workspace.mkdir()
        document = images_fixture(Path(temp) / "images") if args.suite == "images" else None
        if args.suite == "pdf":
            document = pdf_fixture(Path(temp) / "pdf")
        if document is not None:
            arguments = [document]
        with (artifacts / "application.log").open("wb") as log:
            try:
                app = subprocess.Popen([str(binary), *[str(a) for a in arguments]],
                                       env=env, stdout=log, stderr=log)
                desktop = Desktop(app, env, log, arguments)
                exercise_suite(desktop, artifacts, binary, args.suite, workspace, document)
                status = "passed"
                label = "smoke" if args.suite == "regression" else args.suite
                print(f"GUI X11/tiny-skia {label} passed: {len(desktop.passed)} checks (not a Hyprland/Wayland test).", flush=True)
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
                    "status": status, "suite": args.suite, "backend": "X11 / Xvfb / tiny-skia",
                    "checks": desktop.passed if desktop else [], "error": error_message,
                    "native_exits": desktop.native_exits if desktop else 0,
                    "scope": PDF_SCOPE if args.suite == "pdf" else "Read-only Preview images: a launch file opens a document with a real local PNG that renders in Preview and Split, a remote URL, a missing file and a file over the 4 MiB gate all stay inert markers instead of being fetched or decoded, a paste made while Preview is shown never edits the document, and the client closes naturally; screenshots are evidence without pixel assertions, so the decoded picture itself, HiDPI scaling, animated GIF, SVG, WebP and Wayland are not verified" if args.suite == "images" else "Workspace folder tree: a launch folder argument is scanned once without a watcher, Alt+J Alt+K move a clamped keyboard selection over the collapsed-aware rows, Alt+Enter opens the selected document byte for byte with an exact window title, a collapsed folder hides its children from navigation, expanding restores them, and F5 is the only way a file written after the scan appears; mouse rows exist but are not clicked here, dirty per-file markers are unit-tested only, and there is no drag, rename, delete or Wayland proof" if args.suite == "workspace" else "Multi-buffer tabs: Ctrl+T parks the active document and opens an empty tab, Alt+1/Alt+2 restore parked buffers byte for byte with their own text and undo history, an undo in one tab never reaches another, Ctrl+W closes a clean tab and is refused on the last one, and the client closes naturally; no tab drag reorder, split pane, session restore or Wayland proof" if args.suite == "tabs" else "Draft-local Ctrl+Z Ctrl+Y Ctrl+Shift+Z history for the single-line paragraph and table cell panels: the routed keys move only the local snapshot stack, an empty local stack is a no-op, Source bytes stay untouched until Apply, one Source Undo removes each committed edge, Esc cancels and the client closes naturally; no commit shortcut, grouping heuristic, cross-panel history or Wayland proof" if args.suite == "draft-history-keys" else "Draft-local Undo and Redo buttons for the single-line paragraph and table cell panels: staged snapshots, redo after undo, redo dropped by a new edit, restored value committed by Apply as exactly one Source edge, no Source mutation while the draft is open, and natural close; no keyboard shortcut, grouping heuristic, cross-panel history or Wayland proof" if args.suite == "draft-history-single" else "Previous Next unchanged move and atomic Apply and Previous Next over three single-line roots with unsupported block skipping, first and last boundary refusal, parse refusal, Source caret after Cancel, scoped Undo Redo and natural close; no cross-kind navigation or Wayland proof" if args.suite == "paragraph-navigation-single" else "Ctrl+Enter Apply, Ctrl+PageUp Ctrl+PageDown Apply and Previous Next through the same controller guards, boundary refusal without partial commit, exact bytes after each edge, scoped Source Undo and natural close; no implicit wrap, compositor binding or Wayland proof" if args.suite == "paragraph-draft-shortcuts" else "Explicit atomic multiline Apply and Previous over three roots with unchanged earlier offsets, fresh focused previous session, no-op transition, first boundary, parser refusal, Source caret after Cancel, scoped Undo Redo and natural close; no Apply shortcut, wrap or implicit creation" if args.suite == "paragraph-apply-previous" else "Explicit atomic multiline Apply and Next with full candidate and native buffer preflight, fresh root offsets and local history, no-op transition, last boundary and structural refusal no partial commit, Source caret after Cancel, scoped Undo Redo and natural close; no Apply Previous, implicit creation, pixel or Wayland proof" if args.suite == "paragraph-apply-next" else "Explicit unchanged-only Previous Next between eligible multiline roots, skip other blocks, dirty refusal without discard, no wrap or implicit Apply, fresh local history, exact Source atomic Apply and focus caret, natural clean close; no Apply and Next, cross-kind or Wayland proof" if args.suite == "paragraph-navigation" else "Bounded multiline draft-only Undo Redo keyboard and button focus, raw valid and invalid paste snapshots, local branching and session reset, no Source mutation before Apply, atomic Source history after Apply, natural clean close; no draft grouping, selection anchor, pixel or Wayland proof" if args.suite == "paragraph-draft-history" else "Explicit bounded multiline root paragraph native draft, Enter stages newline, exact soft/hard break bytes, full parser split refusal, atomic history, Split last applied line Source focus/caret and natural close; no inline typing, blank blocks, EOF or Wayland proof" if args.suite == "paragraph-multiline" else "Explicit single-line root paragraph raw draft Open Cancel Apply, full parser structural refusal, exact bytes and scoped history, Split applied-end Source caret/focus and natural close; no free Preview typing, multiline or Wayland proof" if args.suite == "paragraph-draft" else "Explicit atomic Apply and Previous cell draft transition, fresh backward/header provenance, invalid and first-cell refusal, unchanged navigation, exact bytes/history, Split Source focus/caret after Cancel and natural close; no wrap, implicit Apply or Wayland proof" if args.suite == "cell-apply-previous" else "Explicit atomic raw column duplication including header delimiter alignment and body, exact bytes, scoped history, Split new copied header Source caret/focus and natural close; no implicit creation, ragged mutation or Wayland proof" if args.suite == "table-column-duplicate" else "Explicit bounded raw column Move left/right with header delimiter alignment and body, exact bytes, scoped history, Split selected-header Source caret/focus and natural close; no wrap, ragged mutation or Wayland proof" if args.suite == "table-column-move" else "Explicit bounded raw body-row Move up/down, exact line bytes/header/suffix, scoped history, Split moved-row Source caret/focus and natural close; no header movement or Wayland proof" if args.suite == "table-row-move" else "Explicit atomic column deletion across header delimiter body, retained raw bytes/alignment, scoped history, Split surviving-header Source caret/focus and natural close; no last-column deletion, ragged tables or Wayland proof" if args.suite == "table-column-delete" else "Explicit atomic blank column insertion before/after across header delimiter and body, exact raw segments/alignment, scoped history, Split new-header Source caret/focus and natural close; no ragged tables or Wayland proof" if args.suite == "table-columns" else "Explicit blank table body row insertion before/after, byte-exact history, Split first-cell Source caret/focus and natural close; no implicit insertion or Wayland proof" if args.suite == "table-insert" else "Source clipboard, navigation focus, read-only Preview/Split and table cycles, warm Split edits/undo/redo, block/cell-source reveal, preview task edits/history, root ATX/Setext heading levels/history, New/Cancel/Discard and WM_DELETE_WINDOW lifecycle; screenshots without pixel assertions; no file portal or Wayland" if args.suite == "regression" else ("Supported task batch check/clear, exact Source clipboard, scoped history/no-op, Split focus and clean native close; no pixel or Wayland proof" if args.suite == "task-batch" else ("Root GFM column alignment, raw delimiter/cell preservation, scoped history, Split focus and clean native close; no pixel or Wayland proof" if args.suite == "table-alignment" else ("Root GFM body-row duplicate/delete, exact raw source, scoped history, Split focus/caret and clean native close; no pixel or Wayland proof" if args.suite == "table-rows" else ("Explicit root table cell draft input/apply/cancel, exact Unicode raw source, validation/no-op/history, Split focus/caret and natural close; no free typing in Preview or Wayland proof" if args.suite == "table-cells" else "Bounded cell draft Previous/Next, dirty refusal, explicit atomic Apply and Next, fresh offsets, scoped history and native Source focus/exit; not inline WYSIWYG or Wayland proof")))),
                }, ensure_ascii=False, indent=2) + "\n")
                if status != "passed":
                    log.flush()
                    print((artifacts / "application.log").read_bytes()[-16000:].decode("utf-8", errors="replace"), flush=True)


if __name__ == "__main__":
    main()
