#!/usr/bin/env python3
"""Black-box X11 smoke test. Invoke via test-gui-x11.sh, not on a live desktop.

No application test hooks: observe window title and clipboard through X11, send
real keyboard/mouse input, and compare exact UTF-8 bytes. This is not Wayland,
portal, pixel-layout, IME or window-close verification.
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

    def command(self, *args, **kwargs):
        return subprocess.run(args, env=self.env, check=True, timeout=3,
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
                                 stdout=subprocess.DEVNULL, stderr=self.log)
        self.owners.append(owner)
        owner.stdin.write(data)
        owner.stdin.close()
        self.wait("Clipboard fixture ownership", lambda: self.clipboard() == data)

    def expect_copy(self, description, expected):
        # Changing ownership BEFORE Ctrl+C prevents a false pass from simply
        # reading the bytes we staged for Ctrl+V. Never strip whitespace or LF.
        marker = f"hylite-copy-not-acknowledged-{uuid.uuid4()}".encode("ascii")
        self.put_clipboard(marker)
        actual = None

        def copied():
            nonlocal actual
            # The X window can be mapped before its first Iced frame. Retry a
            # harmless focus/Select All/Copy until the app acknowledges it.
            self.command("xdotool", "mousemove", "--window", self.window, "600", "300", "click", "1")
            self.key("ctrl+a")
            self.key("ctrl+c")
            actual = self.clipboard()
            return actual == expected

        try:
            self.wait(description, copied)
        except TimeoutError as error:
            raise AssertionError(f"{description}: expected {expected[:300]!r}, got {actual[:300] if actual is not None else None!r}") from error
        self.passed.append(description)
        print(f"PASS: {description}", flush=True)

    def paste_from_clean(self, data):
        self.title_is_dirty(False)
        self.put_clipboard(data)
        self.key("ctrl+a")
        self.key("ctrl+v")
        # A new dirty title acknowledges that Paste has CONSUMED the clipboard.
        # Only then may expect_copy replace the external clipboard owner.
        self.title_is_dirty(True)

    def screenshot(self, path):
        if self.window:
            self.command("import", "-window", self.window, str(path))

    def cleanup(self):
        # Deliberate process cleanup, NOT a successful native close-window test.
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
    desktop.command("xdotool", "type", "--clearmodifiers", "--delay", "30", "XYZ")
    desktop.title_is_dirty(True)
    desktop.expect_copy("Keyboard typing at document end", welcome + b"XYZ")
    desktop.key("ctrl+z")
    desktop.title_is_dirty(False)
    desktop.expect_copy("One undo removes the keyboard typing burst", welcome)


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
        raise TimeoutError("GUI smoke exceeded its 120 second global budget")

    signal.signal(signal.SIGALRM, timed_out)
    signal.alarm(120)
    with tempfile.TemporaryDirectory(prefix="hylite-gui-") as temp:
        env = isolated_environment(os.environ, Path(temp))
        with (artifacts / "application.log").open("wb") as log:
            try:
                app = subprocess.Popen([str(binary)], env=env, stdout=log, stderr=log)
                desktop = Desktop(app, env, log)
                exercise(desktop, artifacts)
                status = "passed"
                print("GUI X11/tiny-skia smoke passed (not a Hyprland/Wayland test).", flush=True)
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
                    "scope": "Source clipboard, typing and undo/redo; no portal, Wayland or visual assertions",
                }, ensure_ascii=False, indent=2) + "\n")
                if status != "passed":
                    log.flush()
                    print((artifacts / "application.log").read_bytes()[-16000:].decode("utf-8", errors="replace"), flush=True)


if __name__ == "__main__":
    main()
