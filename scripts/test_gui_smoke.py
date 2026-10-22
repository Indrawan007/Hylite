"""Unit tests for smoke-test safety/assertions; these do NOT exercise a GUI."""
import importlib.util
import io
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("gui_smoke", Path(__file__).with_name("gui-smoke-x11.py"))
gui = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gui)


class SmokeHelperTests(unittest.TestCase):
    def test_environment_isolates_user_paths_and_desktop_services(self):
        parent = {"DISPLAY": ":123", "XAUTHORITY": "/private/auth", "PATH": "/bin"}
        for key in ("HYPRLAND_INSTANCE_SIGNATURE", "WAYLAND_DISPLAY", "WAYLAND_SOCKET",
                    "DBUS_SESSION_BUS_ADDRESS", "DBUS_SESSION_BUS_PID"):
            parent[key] = "must-not-reach-application"
        with tempfile.TemporaryDirectory() as temp:
            env = gui.isolated_environment(parent, Path(temp))
            self.assertEqual(env["DISPLAY"], ":123")
            self.assertEqual(env["XAUTHORITY"], "/private/auth")
            for key in parent:
                if parent[key] == "must-not-reach-application":
                    self.assertNotIn(key, env)
            for key in ("XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_RUNTIME_DIR", "XDG_DATA_HOME", "XDG_STATE_HOME"):
                self.assertTrue(Path(env[key]).is_relative_to(temp))
                self.assertEqual(os.stat(env[key]).st_mode & 0o777, 0o700)
            self.assertEqual(env["ICED_BACKEND"], "tiny-skia")
            self.assertIn("WAYLAND_DISPLAY", parent)  # Caller environment unchanged.

    def test_dead_application_is_a_failure_not_a_success(self):
        app = SimpleNamespace(poll=lambda: 8, returncode=8)
        desktop = gui.Desktop(app, {}, None)
        with self.assertRaisesRegex(RuntimeError, "Application exited.*8"):
            desktop.wait("test", lambda: True)

    def test_wait_is_bounded(self):
        app = SimpleNamespace(poll=lambda: None)
        desktop = gui.Desktop(app, {}, None)
        with patch.object(gui.time, "monotonic", side_effect=[0, 0, 99]), patch.object(gui.time, "sleep"):
            with self.assertRaisesRegex(TimeoutError, "never ready"):
                desktop.wait("never ready", lambda: False)

    def copy_probe(self, copied):
        desktop = gui.Desktop(None, {}, None)
        clipboard = {"data": b"text\n\t"}  # Staged Paste data must not suffice.
        desktop.command = lambda *_args: None  # No X11 calls in helper tests.
        desktop.put_clipboard = lambda data: clipboard.update(data=data)
        desktop.clipboard = lambda: clipboard["data"]
        desktop.key = lambda key: clipboard.update(data=copied) if key == "ctrl+c" and copied is not None else None

        def bounded(_description, condition):
            if not condition():
                raise TimeoutError("copy not acknowledged")
        desktop.wait = bounded
        return desktop

    def test_copy_checks_exact_bytes_after_ownership_change(self):
        desktop = self.copy_probe(b"text\n\t")
        with patch("sys.stdout", new_callable=io.StringIO):
            desktop.expect_copy("exact", b"text\n\t")
        self.assertEqual(desktop.passed, ["exact"])

    def test_poll_does_not_queue_additional_copy_writes(self):
        desktop = self.copy_probe(b"text\n\t")
        keys = []
        desktop.key = keys.append
        reads = iter([b"not-ready", b"text\n\t"])
        desktop.clipboard = lambda: next(reads)

        def delayed(_description, condition):
            self.assertFalse(condition())
            self.assertTrue(condition())
        desktop.wait = delayed
        with patch("sys.stdout", new_callable=io.StringIO):
            desktop.expect_copy("delayed", b"text\n\t")
        self.assertEqual(keys, ["ctrl+a", "ctrl+c"])

    def test_keyboard_only_copy_never_clicks_or_refocuses(self):
        desktop = self.copy_probe(b"text\n\t")
        commands = []
        desktop.command = lambda *args: commands.append(args)
        with patch("sys.stdout", new_callable=io.StringIO):
            desktop.expect_copy("keyboard-only", b"text\n\t", refocus=False)
        self.assertEqual(commands, [])
        self.assertEqual(desktop.passed, ["keyboard-only"])

    def test_keyboard_text_is_ascii_and_does_not_fake_unicode_typing(self):
        desktop = gui.Desktop(None, {}, None)
        commands = []
        desktop.command = lambda *args: commands.append(args)
        with patch.object(gui.time, "sleep"):
            desktop.type_ascii("2:6")
        self.assertEqual(commands, [("xdotool", "type", "--clearmodifiers", "--delay", "30", "2:6")])
        with self.assertRaisesRegex(ValueError, "UTF-8"):
            desktop.type_ascii("café")
        self.assertEqual(len(commands), 1)

    def test_missing_copy_or_whitespace_loss_cannot_false_pass(self):
        for actual in (None, b"text", b"text\n", b"text\n\t\n"):
            with self.subTest(actual=actual):
                desktop = self.copy_probe(actual)
                with self.assertRaises(AssertionError):
                    desktop.expect_copy("must fail", b"text\n\t")
                self.assertEqual(desktop.passed, [])


if __name__ == "__main__":
    unittest.main()
