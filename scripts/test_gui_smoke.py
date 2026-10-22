"""Unit tests for smoke-test safety/assertions; these do NOT exercise a GUI."""
import importlib.util
import io
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

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

    def test_preview_probe_uses_editor_bold_not_global_sidebar_shortcut(self):
        desktop = Mock(window="42")
        gui.exercise_preview(desktop, Path("/unused-evidence"))
        keys = [call.args[0] for call in desktop.key.call_args_list]
        self.assertIn("ctrl+shift+b", keys)
        self.assertNotIn("ctrl+b", keys)  # Ctrl+B hides Sidebar, changing Split geometry.
        self.assertEqual(keys.count("ctrl+shift+m"), 3)
        self.assertEqual(desktop.expect_copy.call_count, 4)
        self.assertEqual(desktop.screenshot.call_count, 3)
        self.assertEqual(desktop.expect_copy.call_args.args[1], (gui.ROOT / "src/welcome.md").read_bytes())

    def test_table_probe_preserves_exact_utf8_fixture_and_uses_horizontal_wheel(self):
        desktop = Mock(window="42")
        gui.exercise_tables(desktop, Path("/unused-evidence"))
        fixture = desktop.paste_from_clean.call_args.args[0]
        self.assertIn("café🙂".encode("utf-8"), fixture)
        self.assertTrue(fixture.endswith(b"\n\t"))
        self.assertIn(b"a\\|b &amp;", fixture)
        for call in desktop.expect_copy.call_args_list[:3]:
            self.assertEqual(call.args[1], fixture)
        self.assertEqual(desktop.expect_copy.call_count, 4)
        self.assertEqual(desktop.screenshot.call_count, 3)
        desktop.command.assert_called_once_with("xdotool", "mousemove", "--window", "42", "900", "300",
                                                "click", "--repeat", "6", "--delay", "70", "7")
        self.assertEqual([c.args[0] for c in desktop.key.call_args_list].count("ctrl+shift+m"), 3)

    def test_cache_probe_checks_warm_split_edits_undo_redo_without_refocusing(self):
        desktop = Mock(window="42")
        gui.exercise_preview_cache(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        edited = source.replace(b"| 3 |", b"| 93 |")
        self.assertNotEqual(source, edited)
        self.assertTrue(source.endswith(b"\n\t"))
        copies = desktop.expect_copy.call_args_list
        self.assertEqual([call.args[1] for call in copies[:4]], [source, edited, source, edited])
        self.assertEqual(len(copies), 5)
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        desktop.command.assert_not_called()  # No click/windowfocus workaround.
        self.assertIn("5:11", [c.args[0] for c in desktop.type_ascii.call_args_list])
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_reveal_probe_clicks_only_preview_action_and_never_refocuses_source(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_preview_reveal(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        self.assertTrue(source.startswith("# café🙂".encode("utf-8")))
        self.assertTrue(source.endswith(b"\n\t"))
        copies = desktop.expect_copy.call_args_list
        self.assertEqual([call.args[1] for call in copies[:3]], [source, b"X" + source, source])
        self.assertEqual(len(copies), 4)
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        desktop.command.assert_called_once_with("xdotool", "mousemove", "--window", "42", "600", "151", "click", "1")
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_cell_probe_requires_right_click_and_raw_markup_offset_without_source_click(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_cell_reveal(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        self.assertIn("|café🙂|**3**|".encode("utf-8"), source)
        self.assertTrue(source.endswith(b"\n\t"))
        copies = desktop.expect_copy.call_args_list
        self.assertEqual([call.args[1] for call in copies[:3]], [source, source.replace(b"|**3**|", b"|X**3**|"), source])
        self.assertEqual(len(copies), 4)
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        self.assertEqual([call.args for call in desktop.command.call_args_list], [
            ("xdotool", "mousemove", "--window", "42", "490", "316", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "490", "316", "click", "3"),
        ])
        self.assertEqual([call.args[0] for call in desktop.type_ascii.call_args_list], ["LEAK", "X"])
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_task_probe_uses_real_preview_checkboxes_and_atomic_undo_without_source_click(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_task_toggle(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        checked = source.replace(b"- [ ]", b"- [x]", 1)
        self.assertIn("café🙂".encode("utf-8"), source)
        self.assertIn(b"- [X] keep case", source)
        self.assertIn(b"```md\n- [ ] literal code", source)
        self.assertTrue(source.endswith(b"\n\t"))
        copies = desktop.expect_copy.call_args_list
        self.assertEqual([call.args[1] for call in copies[:7]], [source, checked, source, checked, source, checked, source])
        self.assertEqual(len(copies), 8)
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        self.assertEqual([call.args for call in desktop.command.call_args_list], [
            ("xdotool", "mousemove", "--window", "42", "272", "153", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "762", "153", "click", "1"),
        ])
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_preview_history_probe_covers_shortcuts_toolbar_and_source_boundary_without_source_click(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_preview_task_history(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        checked = source.replace(b"- [ ]", b"- [x]", 1)
        self.assertTrue(source.endswith(b"\n\t"))
        self.assertIn("café🙂".encode("utf-8"), source)
        copies = desktop.expect_copy.call_args_list
        self.assertEqual([call.args[1] for call in copies[:8]], [source, source, checked, source, checked, source, checked, source])
        self.assertEqual(len(copies), 9)
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        self.assertEqual([call.args for call in desktop.command.call_args_list], [
            ("xdotool", "mousemove", "--window", "42", "272", "153", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "40", "62", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "95", "62", "click", "1"),
        ])
        keys = [call.args[0] for call in desktop.key.call_args_list]
        self.assertIn("ctrl+shift+z", keys)
        self.assertIn("ctrl+y", keys)
        self.assertEqual(keys.count("ctrl+z"), 6)
        self.assertEqual(desktop.screenshot.call_count, 2)

    def xlib_probe(self, protocols):
        connection = Mock()
        connection.intern_atom.side_effect = lambda name: {"WM_DELETE_WINDOW": 17, "WM_PROTOCOLS": 19}[name]
        window = connection.create_resource_object.return_value
        window.get_wm_protocols.return_value = protocols
        xlib = SimpleNamespace(
            X=SimpleNamespace(CurrentTime=0),
            display=SimpleNamespace(Display=Mock(return_value=connection)),
            protocol=SimpleNamespace(event=SimpleNamespace(ClientMessage=Mock(return_value="client-event"))),
        )
        desktop = gui.Desktop(None, {"DISPLAY": ":123"}, None)
        desktop.window = "42"
        return desktop, connection, window, xlib

    def test_native_close_sends_protocol_not_destroy_or_kill(self):
        desktop, connection, window, xlib = self.xlib_probe([17])
        with patch.dict("sys.modules", {"Xlib": xlib}):
            desktop.request_close()
        xlib.display.Display.assert_called_once_with(":123")
        connection.create_resource_object.assert_called_once_with("window", 42)
        xlib.protocol.event.ClientMessage.assert_called_once_with(
            window=42, client_type=19, data=(32, [17, 0, 0, 0, 0]))
        window.send_event.assert_called_once_with("client-event", event_mask=0, propagate=False)
        connection.sync.assert_called_once()
        connection.close.assert_called_once()

    def test_missing_delete_protocol_fails_without_forcing_close(self):
        desktop, connection, window, xlib = self.xlib_probe([])
        with patch.dict("sys.modules", {"Xlib": xlib}):
            with self.assertRaisesRegex(RuntimeError, "WM_DELETE_WINDOW"):
                desktop.request_close()
        window.send_event.assert_not_called()
        connection.close.assert_called_once()

    def test_native_exit_requires_zero_not_signal_or_timeout(self):
        for code in [-15, 1, None]:
            desktop = gui.Desktop(SimpleNamespace(wait=Mock(return_value=code)), {}, None)
            with self.assertRaises(AssertionError):
                desktop.expect_native_exit("must not pass")
            self.assertEqual(desktop.native_exits, 0)
            self.assertEqual(desktop.passed, [])
        desktop = gui.Desktop(SimpleNamespace(wait=Mock(side_effect=gui.subprocess.TimeoutExpired("app", 12))), {}, None)
        with self.assertRaises(gui.subprocess.TimeoutExpired):
            desktop.expect_native_exit("must time out")
        self.assertEqual(desktop.passed, [])
        desktop = gui.Desktop(SimpleNamespace(wait=Mock(return_value=0)), {}, None)
        with patch("sys.stdout", new_callable=io.StringIO):
            desktop.expect_native_exit("natural close")
        self.assertEqual(desktop.native_exits, 1)
        self.assertEqual(desktop.passed, ["natural close"])

    def test_missing_copy_or_whitespace_loss_cannot_false_pass(self):
        for actual in (None, b"text", b"text\n", b"text\n\t\n"):
            with self.subTest(actual=actual):
                desktop = self.copy_probe(actual)
                with self.assertRaises(AssertionError):
                    desktop.expect_copy("must fail", b"text\n\t")
                self.assertEqual(desktop.passed, [])


if __name__ == "__main__":
    unittest.main()
