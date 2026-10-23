"""Unit tests for smoke-test safety/assertions; these do NOT exercise a GUI."""
import contextlib
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

    def test_heading_probe_covers_both_controls_history_and_split_focus_without_source_click(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_preview_heading(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        demoted = b"#" + source
        self.assertTrue(source.startswith(b"##\t") and source.endswith(b"\n\t"))
        self.assertIn("café🙂".encode("utf-8"), source)
        copies = desktop.expect_copy.call_args_list
        self.assertEqual([call.args[1] for call in copies[:7]],
                         [source, demoted, source, demoted, source, demoted[:3] + b"X" + demoted[3:], source])
        self.assertEqual(len(copies), 8)
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        self.assertEqual([call.args for call in desktop.command.call_args_list], [
            ("xdotool", "mousemove", "--window", "42", "984", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "932", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1144", "151", "click", "1"),
        ])
        keys = [call.args[0] for call in desktop.key.call_args_list]
        self.assertEqual(keys.count("ctrl+z"), 6)
        self.assertEqual(keys.count("ctrl+y"), 1)
        desktop.type_ascii.assert_called_once_with("X")
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_setext_probe_preserves_multiline_body_and_source_focus_without_refocus_click(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_preview_setext(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        demoted = source.replace(b"=====", b"-----", 1)
        sentinel = source.replace(b"=====  \t", b"=====X  \t", 1)
        self.assertTrue(source.endswith(b"\n\t"))
        self.assertIn("café🙂".encode("utf-8"), source)
        self.assertIn(b"continued `code`\n   =====  \t\n", source)
        copies = desktop.expect_copy.call_args_list
        self.assertEqual([call.args[1] for call in copies[:6]], [source, demoted, source, demoted, sentinel, demoted])
        self.assertEqual(len(copies), 7)
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        self.assertEqual([call.args for call in desktop.command.call_args_list], [
            ("xdotool", "mousemove", "--window", "42", "984", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1092", "151", "click", "1"),
        ])
        keys = [call.args[0] for call in desktop.key.call_args_list]
        self.assertEqual(keys.count("ctrl+z"), 5)
        self.assertEqual(keys.count("ctrl+shift+z"), 1)
        desktop.type_ascii.assert_called_once_with("X")
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_task_batch_probe_checks_exact_bytes_history_noop_and_focus_without_refocus(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_task_batch(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        checked = source.replace(b"- [ ]", b"- [x]", 1).replace(b"> - [ ]", b"> - [x]", 1)
        cleared = checked.replace(b"- [x]", b"- [ ]", 1).replace(b"[X]", b"[ ]", 1).replace(b"> - [x]", b"> - [ ]", 1)
        sentinel = cleared.replace(b"> - [ ]", b"> - [ X]", 1)
        self.assertTrue(source.endswith(b"\n\t"))
        self.assertIn(b"[\t] unsupported", source)
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 8)
        self.assertEqual([call.args[1] for call in copies[1:7]], [source, checked, source, checked, sentinel, checked])
        self.assertEqual(copies[0].args[1], copies[-1].args[1])
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        self.assertEqual([call.args for call in desktop.command.call_args_list], [
            ("xdotool", "mousemove", "--window", "42", "185", "62", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "185", "62", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "305", "62", "click", "1"),
        ])
        self.assertEqual([call.args[0] for call in desktop.key.call_args_list].count("ctrl+z"), 5)
        desktop.type_ascii.assert_called_once_with("X")
        desktop.find_window.assert_called_once_with()
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_default_regression_dispatch_preserves_every_old_scenario_and_batch_is_separate(self):
        names = ["exercise", "exercise_navigation", "exercise_preview", "exercise_tables",
                 "exercise_preview_cache", "exercise_preview_reveal", "exercise_cell_reveal",
                 "exercise_task_toggle", "exercise_preview_task_history", "exercise_preview_heading",
                 "exercise_preview_setext", "exercise_lifecycle"]
        seen = []
        with contextlib.ExitStack() as stack:
            mocks = {name: stack.enter_context(patch.object(gui, name, side_effect=lambda *args, n=name: seen.append(n)))
                     for name in names + ["exercise_task_batch"]}
            desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
            gui.exercise_suite(desktop, artifacts, binary)
            self.assertEqual(seen, names)
            mocks["exercise_lifecycle"].assert_called_once_with(desktop, artifacts, binary)
            mocks["exercise_task_batch"].assert_not_called()
            seen.clear()
            gui.exercise_suite(desktop, artifacts, binary, "task-batch")
            self.assertEqual(seen, ["exercise_task_batch"])
            with self.assertRaises(ValueError):
                gui.exercise_suite(desktop, artifacts, binary, "unknown")

    def test_all_suites_refuse_nonprivate_displays_before_starting_a_process(self):
        for suite in ["regression", "task-batch", "table-alignment", "table-rows", "table-cells", "cell-navigation", "table-insert", "table-columns", "table-column-delete", "table-row-move", "table-column-move", "table-column-duplicate"]:
            with patch.object(gui.os, "environ", {"DISPLAY": ":0", "HYLITE_SMOKE_DISPLAY": ":999"}), \
                 patch("sys.argv", ["gui-smoke-x11.py", "--suite", suite]), \
                 patch("sys.stderr", new_callable=io.StringIO), \
                 patch.object(gui.subprocess, "Popen") as start:
                with self.assertRaises(SystemExit) as error:
                    gui.main()
                self.assertEqual(error.exception.code, 2)
                start.assert_not_called()

    def test_alignment_probe_uses_column_options_exact_source_and_no_refocus(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_table_alignment(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        centered = source.replace(b"---", b":---:", 1)
        sentinel = centered.replace(b"| --: |", b"| :--X |", 1)
        self.assertEqual(sentinel.splitlines()[1], b"  | :---:  | :--X |")
        self.assertTrue(source.endswith(b"\n\t"))
        self.assertIn(b"a\\|b &amp;", source)
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 8)
        self.assertEqual([call.args[1] for call in copies[1:7]], [source, centered, source, centered, sentinel, centered])
        self.assertEqual(copies[0].args[1], copies[-1].args[1])
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        self.assertEqual([call.args for call in desktop.command.call_args_list], [
            ("xdotool", "mousemove", "--window", "42", "984", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "900", "189", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1144", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1080", "244", "click", "1"),
        ])
        desktop.type_ascii.assert_called_once_with("X")
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_alignment_suite_dispatch_is_exclusive_and_does_not_substitute_old_gates(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        with patch.object(gui, "exercise_table_alignment") as alignment, \
             patch.object(gui, "exercise") as regression, \
             patch.object(gui, "exercise_task_batch") as batch:
            gui.exercise_suite(desktop, artifacts, binary, "table-alignment")
            alignment.assert_called_once_with(desktop, artifacts)
            regression.assert_not_called()
            batch.assert_not_called()

    def test_row_probe_has_independent_exact_lines_and_no_source_refocus(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_table_rows(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        row = "  | café🙂 **raw** | a\\|b &amp; |\n".encode("utf-8")
        duplicated = source.replace(row, row + row, 1)
        sentinel = source.replace(b"| short |\n", b"X| short |\n", 1)
        self.assertEqual(duplicated.splitlines()[2:5], [row[:-1], row[:-1], b"| short |"])
        self.assertEqual(sentinel.splitlines()[3], b"X| short |")
        self.assertTrue(source.endswith(b"\n\t"))
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 8)
        self.assertEqual([call.args[1] for call in copies[1:7]], [source, duplicated, source, duplicated, sentinel, duplicated])
        self.assertEqual(copies[0].args[1], copies[-1].args[1])
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        self.assertEqual([call.args for call in desktop.command.call_args_list], [
            ("xdotool", "mousemove", "--window", "42", "984", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "900", "281", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1144", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1080", "336", "click", "1"),
        ])
        desktop.type_ascii.assert_called_once_with("X")
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_row_dispatch_does_not_replace_any_previous_suite(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        with patch.object(gui, "exercise_table_rows") as rows, \
             patch.object(gui, "exercise") as regression, \
             patch.object(gui, "exercise_task_batch") as batch, \
             patch.object(gui, "exercise_table_alignment") as alignment:
            gui.exercise_suite(desktop, artifacts, binary, "table-rows")
            rows.assert_called_once_with(desktop, artifacts)
            regression.assert_not_called()
            batch.assert_not_called()
            alignment.assert_not_called()

    def test_cell_probe_checks_raw_draft_focus_apply_cancel_noop_and_invalid_input(self):
        desktop=Mock(window="42")
        with patch.object(gui.time,"sleep"):
            gui.exercise_table_cells(desktop,Path("/unused-evidence"))
        source=desktop.paste_from_clean.call_args.args[0]
        initial="café🙂 **raw**".encode("utf-8")
        value="new **bold** é🙂".encode("utf-8")
        edited=source.replace(initial,value,1)
        sentinel=edited.replace(b"| a\\|b &amp; |",b"| nextX |",1)
        self.assertEqual(sentinel.splitlines()[2]," | new **bold** é🙂 | nextX |".encode("utf-8"))
        copies=desktop.expect_copy.call_args_list
        self.assertEqual(len(copies),13)
        self.assertEqual([call.args[1] for call in copies[1:12]],[source,initial,source,value,edited,source,edited,edited,edited,sentinel,edited])
        self.assertEqual(copies[0].args[1],copies[-1].args[1])
        self.assertTrue(all(call.kwargs=={"refocus":False} for call in copies))
        self.assertEqual([call.args for call in desktop.type_ascii.call_args_list],[("discard me",),("bad|pipe",),("next",),("X",)])
        clicks=[call.args for call in desktop.command.call_args_list]
        self.assertEqual(len(clicks),10)
        self.assertTrue(all(call[:3]==("xdotool","mousemove","--window") and call[-2:]==("click","1") for call in clicks))
        self.assertEqual([call[5] for call in clicks[1::2]],["354","354","354","372","372"])
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count,3)

    def test_cell_dispatch_adds_a_fifth_independent_suite_without_running_old_probes(self):
        desktop,artifacts,binary=Mock(),Path("/unused"),Path("/unused/binary")
        with patch.object(gui,"exercise_table_cells") as cells, \
             patch.object(gui,"exercise") as regression, \
             patch.object(gui,"exercise_task_batch") as batch, \
             patch.object(gui,"exercise_table_alignment") as alignment, \
             patch.object(gui,"exercise_table_rows") as rows:
            gui.exercise_suite(desktop,artifacts,binary,"table-cells")
            cells.assert_called_once_with(desktop,artifacts)
            regression.assert_not_called(); batch.assert_not_called(); alignment.assert_not_called(); rows.assert_not_called()

    def test_navigation_probe_protects_dirty_drafts_bounds_offsets_and_source_focus(self):
        desktop=Mock(window="42")
        with patch.object(gui.time,"sleep"):
            gui.exercise_cell_navigation(desktop,Path("/unused-evidence"))
        source=desktop.paste_from_clean.call_args.args[0]
        initial="café🙂 **raw**".encode("utf-8")
        value="applied **é🙂**".encode("utf-8")
        second=b"a\\|b &amp;"
        edited=source.replace(initial,value,1)
        sentinel=source.replace(initial,b"finalX",1)
        self.assertEqual(sentinel.splitlines()[2],b" | finalX | a\\|b &amp; |")
        copies=desktop.expect_copy.call_args_list
        self.assertEqual(len(copies),15)
        self.assertEqual([c.args[1] for c in copies[1:14]],[source,b"Count",initial,b"pending",value,second,value,second,edited,source,edited,sentinel,edited])
        self.assertEqual(copies[0].args[1],copies[-1].args[1])
        self.assertTrue(all(c.kwargs=={"refocus":False} for c in copies))
        clicks=[c.args for c in desktop.command.call_args_list]
        self.assertEqual(len(clicks),13)
        self.assertEqual([c[4] for c in clicks if c[5]=="244"],["240","360","360","500","360","500","240","500","500"])
        self.assertEqual([c.args for c in desktop.type_ascii.call_args_list],[("pending",),("discard me",),("final",),("X",)])
        desktop.request_close.assert_called_once_with();desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count,2)

    def test_navigation_dispatch_does_not_replace_any_of_five_previous_suites(self):
        desktop,artifacts,binary=Mock(),Path("/unused"),Path("/unused/binary")
        with patch.object(gui,"exercise_cell_navigation") as navigation, \
             patch.object(gui,"exercise") as regression, \
             patch.object(gui,"exercise_task_batch") as batch, \
             patch.object(gui,"exercise_table_alignment") as alignment, \
             patch.object(gui,"exercise_table_rows") as rows, \
             patch.object(gui,"exercise_table_cells") as cells:
            gui.exercise_suite(desktop,artifacts,binary,"cell-navigation")
            navigation.assert_called_once_with(desktop,artifacts)
            regression.assert_not_called();batch.assert_not_called();alignment.assert_not_called();rows.assert_not_called();cells.assert_not_called()

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


    def test_insert_probe_locks_independent_raw_rows_and_first_cell_caret(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_table_insert(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        row = " | café🙂 **raw** | a\\|b &amp; |\n".encode("utf-8")
        before = source.replace(row, b" |  |  |\n" + row, 1)
        after = source.replace(row, row + b" |  |  |\n", 1)
        typed = source.replace(row, row + b" | X |  |\n", 1)
        self.assertEqual(before.splitlines()[2:4], [b" |  |  |", row[:-1]])
        self.assertEqual(after.splitlines()[2:4], [row[:-1], b" |  |  |"])
        self.assertEqual(typed.splitlines()[3], b" | X |  |")
        self.assertTrue(source.endswith(b"\n\t"))
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 10)
        self.assertEqual([call.args[1] for call in copies[1:-1]], [source, before, source, before, source, typed, after, source])
        self.assertEqual(copies[0].args[1], copies[-1].args[1])
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        self.assertEqual([call.args for call in desktop.command.call_args_list], [
            ("xdotool", "mousemove", "--window", "42", "984", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "900", "391", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1144", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1080", "409", "click", "1"),
        ])
        desktop.type_ascii.assert_called_once_with("X")
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_insert_suite_is_independent_and_preserves_all_previous_dispatch(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        names = ["exercise", "exercise_task_batch", "exercise_table_alignment", "exercise_table_rows", "exercise_table_cells", "exercise_cell_navigation"]
        from contextlib import ExitStack
        with ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_table_insert"))
            gui.exercise_suite(desktop, artifacts, binary, "table-insert")
            new.assert_called_once_with(desktop, artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_column_probe_uses_independent_full_raw_oracles_and_new_header_caret(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_table_columns(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 10)
        self.assertEqual(source.splitlines()[0], b"| Name | Count |")
        before = ("|  | Name | Count |\n  | --- | ---  | --: |\n |  | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
        after = ("| Name | Count |  |\n  | ---  | --: | --- |\n | café🙂 **raw** | a\\|b &amp; |  |\n\nafter\n\t").encode("utf-8")
        typed = ("| Name | Count | X |\n  | ---  | --: | --- |\n | café🙂 **raw** | a\\|b &amp; |  |\n\nafter\n\t").encode("utf-8")
        self.assertEqual([call.args[1] for call in copies[1:-1]], [source, before, source, before, source, typed, after, source])
        self.assertEqual(copies[0].args[1], copies[-1].args[1])
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        self.assertEqual([call.args for call in desktop.command.call_args_list], [
            ("xdotool", "mousemove", "--window", "42", "984", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "900", "428", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1144", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1080", "483", "click", "1"),
        ])
        desktop.type_ascii.assert_called_once_with("X")
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_column_suite_is_exclusive_and_not_a_substitute_for_old_gates(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        names = ["exercise", "exercise_task_batch", "exercise_table_alignment", "exercise_table_rows", "exercise_table_cells", "exercise_cell_navigation", "exercise_table_insert"]
        from contextlib import ExitStack
        with ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_table_columns"))
            gui.exercise_suite(desktop, artifacts, binary, "table-columns")
            new.assert_called_once_with(desktop, artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_column_delete_probe_checks_retained_raw_bytes_and_surviving_header_caret(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_table_column_delete(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 10)
        self.assertEqual(source.splitlines()[0], b"| Name | Count |")
        first = b"| Count |\n  | --: |\n | a\\|b &amp; |\n\nafter\n\t"
        last = ("| Name |\n  | ---  |\n | café🙂 **raw** |\n\nafter\n\t").encode("utf-8")
        typed = ("| XName |\n  | ---  |\n | café🙂 **raw** |\n\nafter\n\t").encode("utf-8")
        self.assertEqual([call.args[1] for call in copies[1:-1]], [source, first, source, first, source, typed, last, source])
        self.assertEqual(copies[0].args[1], copies[-1].args[1])
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        self.assertEqual([call.args for call in desktop.command.call_args_list], [
            ("xdotool", "mousemove", "--window", "42", "984", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "900", "501", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1144", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1080", "520", "click", "1"),
        ])
        desktop.type_ascii.assert_called_once_with("X")
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_column_delete_dispatch_is_exclusive_and_does_not_replace_old_suites(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        names = ["exercise", "exercise_task_batch", "exercise_table_alignment", "exercise_table_rows", "exercise_table_cells", "exercise_cell_navigation", "exercise_table_insert", "exercise_table_columns"]
        from contextlib import ExitStack
        with ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_table_column_delete"))
            gui.exercise_suite(desktop, artifacts, binary, "table-column-delete")
            new.assert_called_once_with(desktop, artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_row_move_probe_locks_independent_exact_raw_lines_and_selected_row_caret(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_table_row_move(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 10)
        moved = ("| Name | Count |\n  | ---  | --: |\n| short |\n  | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
        typed = ("| Name | Count |\n  | ---  | --: |\nX| short |\n  | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
        self.assertEqual(source.splitlines()[3], b"| short |")
        self.assertEqual([call.args[1] for call in copies[1:-1]], [source, moved, source, moved, source, typed, moved, source])
        self.assertEqual(copies[0].args[1], copies[-1].args[1])
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        self.assertEqual([call.args for call in desktop.command.call_args_list], [
            ("xdotool", "mousemove", "--window", "42", "984", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "900", "428", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1144", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1080", "446", "click", "1"),
        ])
        desktop.type_ascii.assert_called_once_with("X")
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_row_move_suite_is_exclusive_and_keeps_every_previous_gate(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        names = ["exercise", "exercise_task_batch", "exercise_table_alignment", "exercise_table_rows", "exercise_table_cells", "exercise_cell_navigation", "exercise_table_insert", "exercise_table_columns", "exercise_table_column_delete"]
        from contextlib import ExitStack
        with ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_table_row_move"))
            gui.exercise_suite(desktop, artifacts, binary, "table-row-move")
            new.assert_called_once_with(desktop, artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_column_move_probe_locks_raw_segments_alignment_and_selected_header_caret(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_table_column_move(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 10)
        moved = ("| Count | Name |\n  | --: | ---  |\n | a\\|b &amp; | café🙂 **raw** |\n\nafter\n\t").encode("utf-8")
        typed = ("| XCount | Name |\n  | --: | ---  |\n | a\\|b &amp; | café🙂 **raw** |\n\nafter\n\t").encode("utf-8")
        self.assertEqual(source.splitlines()[0], b"| Name | Count |")
        self.assertEqual([call.args[1] for call in copies[1:-1]], [source, moved, source, moved, source, typed, moved, source])
        self.assertEqual(copies[0].args[1], copies[-1].args[1])
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        self.assertEqual([call.args for call in desktop.command.call_args_list], [
            ("xdotool", "mousemove", "--window", "42", "984", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "900", "538", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1144", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1080", "556", "click", "1"),
        ])
        desktop.type_ascii.assert_called_once_with("X")
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_column_move_dispatch_is_exclusive_and_keeps_every_previous_suite(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        names = ["exercise", "exercise_task_batch", "exercise_table_alignment", "exercise_table_rows", "exercise_table_cells", "exercise_cell_navigation", "exercise_table_insert", "exercise_table_columns", "exercise_table_column_delete", "exercise_table_row_move"]
        from contextlib import ExitStack
        with ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_table_column_move"))
            gui.exercise_suite(desktop, artifacts, binary, "table-column-move")
            new.assert_called_once_with(desktop, artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_column_duplicate_probe_locks_copied_alignment_raw_segments_and_new_header_caret(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_table_column_duplicate(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 10)
        first = ("| Name | Name | Count |\n  | ---  | ---  | --: |\n | café🙂 **raw** | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
        last = ("| Name | Count | Count |\n  | ---  | --: | --: |\n | café🙂 **raw** | a\\|b &amp; | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
        typed = ("| Name | Count | XCount |\n  | ---  | --: | --: |\n | café🙂 **raw** | a\\|b &amp; | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
        self.assertEqual(source, ("| Name | Count |\n  | ---  | --: |\n | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8"))
        self.assertEqual([call.args[1] for call in copies[1:-1]], [source, first, source, first, source, typed, last, source])
        self.assertEqual(copies[0].args[1], copies[-1].args[1])
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        self.assertEqual([call.args for call in desktop.command.call_args_list], [
            ("xdotool", "mousemove", "--window", "42", "984", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "900", "575", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1144", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1080", "593", "click", "1"),
        ])
        keys = [call.args[0] for call in desktop.key.call_args_list]
        self.assertEqual(keys.count("ctrl+z"), 5)
        self.assertEqual(keys.count("ctrl+y"), 1)
        desktop.type_ascii.assert_called_once_with("X")
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_column_duplicate_dispatch_is_exclusive_and_keeps_all_eleven_previous_suites(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        names = ["exercise", "exercise_task_batch", "exercise_table_alignment", "exercise_table_rows", "exercise_table_cells", "exercise_cell_navigation", "exercise_table_insert", "exercise_table_columns", "exercise_table_column_delete", "exercise_table_row_move", "exercise_table_column_move"]
        with contextlib.ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_table_column_duplicate"))
            gui.exercise_suite(desktop, artifacts, binary, "table-column-duplicate")
            new.assert_called_once_with(desktop, artifacts)
            for probe in old:
                probe.assert_not_called()

if __name__ == "__main__":
    unittest.main()
