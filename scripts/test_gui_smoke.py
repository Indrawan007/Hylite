"""Unit tests for smoke-test safety/assertions; these do NOT exercise a GUI."""
import contextlib
import importlib.util
import io
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, call, patch

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
        for suite in ["regression", "task-batch", "table-alignment", "table-rows", "table-cells", "cell-navigation", "table-insert", "table-columns", "table-column-delete", "table-row-move", "table-column-move", "table-column-duplicate", "cell-apply-previous", "paragraph-draft", "paragraph-multiline", "paragraph-draft-history", "paragraph-navigation", "paragraph-apply-next", "paragraph-apply-previous", "paragraph-draft-shortcuts", "paragraph-navigation-single", "draft-history-single"]:
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

    def test_apply_previous_probe_locks_backward_raw_payloads_boundaries_history_and_focus(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_cell_apply_previous(desktop, Path("/unused-evidence"))
        source = desktop.paste_from_clean.call_args.args[0]
        expected = ("| Name | Count |\n  | ---  | --: |\n | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
        self.assertEqual(source, expected)
        edited = ("| Name | Count |\n  | ---  | --: |\n | café🙂 **raw** | applied **é🙂** |\n\nafter\n\t").encode("utf-8")
        sentinel = ("| Name | Count |\n  | ---  | --: |\n | café🙂 **raw** | finalX |\n\nafter\n\t").encode("utf-8")
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 15)
        self.assertEqual([call.args[1] for call in copies[1:-1]], [source, b"a\\|b &amp;", b"bad|pipe", source, "café🙂 **raw**".encode("utf-8"), b"Count", b"Name", b"first pending", edited, source, edited, sentinel, edited])
        self.assertEqual(copies[0].args[1], copies[-1].args[1])
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        clicks = [call.args for call in desktop.command.call_args_list if call.args[-2:] == ("click", "1")]
        previous = ("xdotool", "mousemove", "--window", "42", "650", "244", "click", "1")
        self.assertEqual(clicks, [
            ("xdotool", "mousemove", "--window", "42", "984", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "900", "373", "click", "1"),
            previous,
            ("xdotool", "mousemove", "--window", "42", "984", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "900", "373", "click", "1"),
            previous, previous, previous, previous,
            ("xdotool", "mousemove", "--window", "42", "1144", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1080", "373", "click", "1"), previous,
        ])
        self.assertEqual([call.args[0] for call in desktop.type_ascii.call_args_list], ["bad|pipe", "first pending", "final", "X"])
        keys = [call.args[0] for call in desktop.key.call_args_list]
        self.assertEqual(keys.count("ctrl+z"), 5)
        self.assertEqual(keys.count("ctrl+y"), 1)
        self.assertEqual(keys.count("Escape"), 3)
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_apply_previous_dispatch_is_exclusive_and_keeps_all_twelve_old_suites(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        names = ["exercise", "exercise_task_batch", "exercise_table_alignment", "exercise_table_rows", "exercise_table_cells", "exercise_cell_navigation", "exercise_table_insert", "exercise_table_columns", "exercise_table_column_delete", "exercise_table_row_move", "exercise_table_column_move", "exercise_table_column_duplicate"]
        with contextlib.ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_cell_apply_previous"))
            gui.exercise_suite(desktop, artifacts, binary, "cell-apply-previous")
            new.assert_called_once_with(desktop, artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_paragraph_probe_locks_raw_draft_source_structure_refusal_caret_and_history(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_paragraph_draft(desktop, Path("/unused-evidence"))
        source = ("café🙂 **raw** &amp; a\\|b  \n\n# untouched\n\nterminal\n\t").encode("utf-8")
        value = "edited **é🙂** &amp; a\\|b".encode("utf-8")
        edited = ("edited **é🙂** &amp; a\\|b\n\n# untouched\n\nterminal\n\t").encode("utf-8")
        typed = b"finalX\n\n# untouched\n\nterminal\n\t"
        self.assertEqual(desktop.paste_from_clean.call_args.args[0], source)
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 14)
        self.assertEqual([call.args[1] for call in copies[1:-1]], [source, "café🙂 **raw** &amp; a\\|b  ".encode("utf-8"), source, value, edited, source, edited, edited, b"# heading", edited, typed, edited])
        self.assertEqual(copies[0].args[1], copies[-1].args[1])
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        clicks = [call.args for call in desktop.command.call_args_list if call.args[-2:] == ("click", "1")]
        preview = ("xdotool", "mousemove", "--window", "42", "984", "151", "click", "1")
        split = ("xdotool", "mousemove", "--window", "42", "1144", "151", "click", "1")
        self.assertEqual(clicks, [preview, preview, split, split, split])
        self.assertEqual([call.args[0] for call in desktop.type_ascii.call_args_list], ["discard me", "# heading", "final", "X"])
        keys = [call.args[0] for call in desktop.key.call_args_list]
        self.assertEqual(keys.count("ctrl+z"), 5)
        self.assertEqual(keys.count("ctrl+y"), 1)
        self.assertEqual(keys.count("Escape"), 2)
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count, 3)

    def test_paragraph_dispatch_is_exclusive_and_keeps_all_thirteen_old_suites(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        names = ["exercise", "exercise_task_batch", "exercise_table_alignment", "exercise_table_rows", "exercise_table_cells", "exercise_cell_navigation", "exercise_table_insert", "exercise_table_columns", "exercise_table_column_delete", "exercise_table_row_move", "exercise_table_column_move", "exercise_table_column_duplicate", "exercise_cell_apply_previous"]
        with contextlib.ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_paragraph_draft"))
            gui.exercise_suite(desktop, artifacts, binary, "paragraph-draft")
            new.assert_called_once_with(desktop, artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_multiline_probe_locks_raw_breaks_native_enter_structural_refusal_focus_and_history(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_paragraph_multiline(desktop, Path("/unused-evidence"))
        source = ("café🙂 **raw**  \na\\|b &amp;\n\n# untouched\n\nterminal\n\t").encode("utf-8")
        value = "edited **é🙂**\nsecond  \nthird &amp;".encode("utf-8")
        edited = ("edited **é🙂**\nsecond  \nthird &amp;\n\n# untouched\n\nterminal\n\t").encode("utf-8")
        typed = b"final\nnextX\n\n# untouched\n\nterminal\n\t"
        self.assertEqual(desktop.paste_from_clean.call_args.args[0], source)
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 15)
        self.assertEqual([call.args[1] for call in copies[1:-1]], [source, "café🙂 **raw**  \na\\|b &amp;".encode("utf-8"), b"discard\npending", source, value, edited, source, edited, edited, b"first\n# heading", edited, typed, edited])
        self.assertEqual(copies[0].args[1], copies[-1].args[1])
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        clicks = [call.args for call in desktop.command.call_args_list if call.args[-2:] == ("click", "1")]
        preview = ("xdotool", "mousemove", "--window", "42", "984", "151", "click", "1")
        split = ("xdotool", "mousemove", "--window", "42", "1144", "151", "click", "1")
        apply = ("xdotool", "mousemove", "--window", "42", "225", "364", "click", "1")
        self.assertEqual(clicks, [preview, preview, apply, split, apply, split, apply, split, apply])
        self.assertEqual([call.args[0] for call in desktop.type_ascii.call_args_list], ["discard", "pending", "X"])
        keys = [call.args[0] for call in desktop.key.call_args_list]
        self.assertEqual(keys.count("Return"), 1)
        self.assertEqual(keys.count("Escape"), 2)
        self.assertEqual(keys.count("ctrl+z"), 5)
        self.assertEqual(keys.count("ctrl+y"), 1)
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count, 3)

    def test_multiline_dispatch_is_exclusive_and_keeps_all_fourteen_previous_suites(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        names = ["exercise", "exercise_task_batch", "exercise_table_alignment", "exercise_table_rows", "exercise_table_cells", "exercise_cell_navigation", "exercise_table_insert", "exercise_table_columns", "exercise_table_column_delete", "exercise_table_row_move", "exercise_table_column_move", "exercise_table_column_duplicate", "exercise_cell_apply_previous", "exercise_paragraph_draft"]
        with contextlib.ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_paragraph_multiline"))
            gui.exercise_suite(desktop, artifacts, binary, "paragraph-multiline")
            new.assert_called_once_with(desktop, artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_draft_history_probe_locks_local_raw_branch_reset_focus_and_source_atomicity(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_paragraph_draft_history(desktop, Path("/unused-evidence"))
        initial = "café🙂 **raw**  \na\\|b &amp;".encode("utf-8")
        alpha = "alpha é🙂  \nbeta\nlast &amp;".encode("utf-8")
        invalid, branch = b"bad\n\ntrailing\t", b"branch\nkept"
        final = "final é🙂\nnext  ".encode("utf-8")
        suffix = b"\n\n# untouched\n\nterminal\n\t"
        source, applied, typed = initial + suffix, final + suffix, final + b"X" + suffix
        self.assertEqual(desktop.paste_from_clean.call_args.args[0], source)
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 23)
        self.assertEqual([call.args[1] for call in copies[1:-1]], [source, initial, alpha, initial, alpha, alpha, invalid, alpha, invalid, branch, branch, alpha, source, initial, final, initial, final, typed, applied, source, applied])
        self.assertEqual(copies[0].args[1], copies[-1].args[1])
        self.assertTrue(all(call.kwargs == {"refocus": False} for call in copies))
        clicks = [call.args for call in desktop.command.call_args_list if call.args[-2:] == ("click", "1")]
        self.assertEqual(clicks, [
            ("xdotool", "mousemove", "--window", "42", "984", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "425", "364", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "1144", "151", "click", "1"),
            ("xdotool", "mousemove", "--window", "42", "225", "364", "click", "1"),
        ])
        keys = [call.args[0] for call in desktop.key.call_args_list]
        self.assertEqual(keys.count("ctrl+z"), 10)
        self.assertEqual(keys.count("ctrl+y"), 6)
        self.assertEqual(keys.count("ctrl+shift+z"), 1)
        self.assertEqual(keys.count("ctrl+shift+m"), 5)
        self.assertEqual(keys[-8:], ["ctrl+z", "ctrl+z", "ctrl+shift+m", "ctrl+shift+m", "ctrl+y", "ctrl+shift+m", "ctrl+z", "ctrl+z"])
        self.assertEqual([call.args[0] for call in desktop.type_ascii.call_args_list], ["X"])
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_draft_history_dispatch_is_exclusive_and_keeps_all_fifteen_previous_suites(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        names = ["exercise", "exercise_task_batch", "exercise_table_alignment", "exercise_table_rows", "exercise_table_cells", "exercise_cell_navigation", "exercise_table_insert", "exercise_table_columns", "exercise_table_column_delete", "exercise_table_row_move", "exercise_table_column_move", "exercise_table_column_duplicate", "exercise_cell_apply_previous", "exercise_paragraph_draft", "exercise_paragraph_multiline"]
        with contextlib.ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_paragraph_draft_history"))
            gui.exercise_suite(desktop, artifacts, binary, "paragraph-draft-history")
            new.assert_called_once_with(desktop, artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_paragraph_navigation_probe_locks_dirty_refusal_skip_bounds_fresh_history_and_apply(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_paragraph_navigation(desktop, Path("/unused-evidence"))
        first = "café🙂 **first**  \na\\|b &amp;".encode("utf-8")
        middle = b"\n\n# gap\n\nsingle line\n\n> nested\n> paragraph\n\n```text\nliteral\nparagraph\n```\n\n"
        second = "second é🙂\nraw **tail**  ".encode("utf-8")
        value = "edited é🙂\nlast  ".encode("utf-8")
        suffix = b"\n\nterminal\n\t"
        source, applied, typed = first+middle+second+suffix, first+middle+value+suffix, first+middle+value+b"X"+suffix
        self.assertEqual(desktop.paste_from_clean.call_args.args[0], source)
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 18)
        self.assertEqual([c.args[1] for c in copies[1:-1]], [source, first, b"changed\npending", b"changed\npending", first, second, second, first, source, second, source, value, value, typed, applied, source])
        self.assertEqual(copies[0].args[1], copies[-1].args[1])
        self.assertTrue(all(c.kwargs == {"refocus": False} for c in copies))
        clicks = [c.args for c in desktop.command.call_args_list if c.args[-2:] == ("click", "1")]
        def at(x, y):
            return ("xdotool", "mousemove", "--window", "42", str(x), str(y), "click", "1")
        self.assertEqual(clicks, [at(984,151), at(450,408), at(450,408), at(280,408), at(280,408), at(1144,151), at(450,408), at(450,408), at(1144,151), at(450,408), at(280,408), at(225,364)])
        keys = [c.args[0] for c in desktop.key.call_args_list]
        self.assertEqual(keys.count("ctrl+z"), 4)
        self.assertEqual(keys.count("ctrl+y"), 1)
        self.assertEqual(keys.count("ctrl+shift+m"), 2)
        self.assertEqual(keys.count("Escape"), 2)
        self.assertEqual([c.args[0] for c in desktop.type_ascii.call_args_list], ["X"])
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count, 2)

    def test_paragraph_navigation_dispatch_is_exclusive_and_keeps_all_sixteen_previous_suites(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        names = ["exercise", "exercise_task_batch", "exercise_table_alignment", "exercise_table_rows", "exercise_table_cells", "exercise_cell_navigation", "exercise_table_insert", "exercise_table_columns", "exercise_table_column_delete", "exercise_table_row_move", "exercise_table_column_move", "exercise_table_column_duplicate", "exercise_cell_apply_previous", "exercise_paragraph_draft", "exercise_paragraph_multiline", "exercise_paragraph_draft_history"]
        with contextlib.ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_paragraph_navigation"))
            gui.exercise_suite(desktop, artifacts, binary, "paragraph-navigation")
            new.assert_called_once_with(desktop, artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_paragraph_apply_next_probe_locks_fresh_raw_target_noop_boundary_refusal_focus_and_history(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_paragraph_apply_next(desktop, Path("/unused-evidence"))
        first = "café🙂 **first**  \na\\|b &amp;".encode("utf-8")
        middle = b"\n\n# gap\n\nsingle line\n\n> nested\n> paragraph\n\n```text\nliteral\nparagraph\n```\n\n"
        second = "second é🙂\nraw **tail**  ".encode("utf-8")
        suffix = b"\n\nterminal\n\t"
        value = "grown **é🙂**\nline two  \nthird &amp;".encode("utf-8")
        final = "final é🙂\nnext  ".encode("utf-8")
        source, applied = first+middle+second+suffix, value+middle+second+suffix
        final_source, typed = final+middle+second+suffix, final+b"X"+middle+second+suffix
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 22)
        self.assertEqual([c.args[1] for c in copies[1:-1]], [source,first,value,second,second,value,second,b"last\npending",applied,source,applied,b"one\n# heading",b"one\n# heading",applied,final,second,typed,final_source,applied,source])
        self.assertEqual(copies[0].args[1],copies[-1].args[1])
        self.assertEqual(desktop.paste_from_clean.call_args.args[0],source)
        self.assertTrue(all(c.kwargs=={"refocus":False} for c in copies))
        def at(x,y):
            return ("xdotool","mousemove","--window","42",str(x),str(y),"click","1")
        clicks=[c.args for c in desktop.command.call_args_list if c.args[-2:]==("click","1")]
        self.assertEqual(clicks,[at(984,151),at(625,408),at(280,408),at(625,408),at(625,408),at(1144,151),at(625,408),at(1144,151),at(625,408)])
        keys=[c.args[0] for c in desktop.key.call_args_list]
        self.assertEqual(keys.count("ctrl+z"),6)
        self.assertEqual(keys.count("ctrl+y"),1)
        self.assertEqual(keys.count("ctrl+shift+m"),8)
        self.assertEqual(keys.count("Escape"),3)
        self.assertEqual([c.args[0] for c in desktop.type_ascii.call_args_list],["X"])
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count,2)

    def test_paragraph_apply_next_dispatch_is_exclusive_and_keeps_all_seventeen_previous_suites(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        names=["exercise","exercise_task_batch","exercise_table_alignment","exercise_table_rows","exercise_table_cells","exercise_cell_navigation","exercise_table_insert","exercise_table_columns","exercise_table_column_delete","exercise_table_row_move","exercise_table_column_move","exercise_table_column_duplicate","exercise_cell_apply_previous","exercise_paragraph_draft","exercise_paragraph_multiline","exercise_paragraph_draft_history","exercise_paragraph_navigation"]
        with contextlib.ExitStack() as stack:
            old=[stack.enter_context(patch.object(gui,name)) for name in names]
            new=stack.enter_context(patch.object(gui,"exercise_paragraph_apply_next"))
            gui.exercise_suite(desktop,artifacts,binary,"paragraph-apply-next")
            new.assert_called_once_with(desktop,artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_paragraph_apply_previous_probe_locks_three_roots_noop_boundary_refusal_focus_and_history(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_paragraph_apply_previous(desktop, Path("/unused-evidence"))
        first = "café🙂 **first**  \na\\|b &amp;".encode("utf-8")
        middle = b"\n\n# gap\n\nsingle line\n\n> nested\n> paragraph\n\n```text\nliteral\nparagraph\n```\n\n"
        second = "second é🙂\nraw **tail**  ".encode("utf-8")
        between = b"\n\n## second gap\n\n- item\n  - nested item\n\n"
        third = "third 🙂\nraw final  ".encode("utf-8")
        suffix = b"\n\nterminal\n\t"
        third2 = "growth **é🙂**\nfinal two  \nthird  ".encode("utf-8")
        first2 = "first edited é🙂\nsecond line  ".encode("utf-8")
        final = "final é🙂\nsecond two  ".encode("utf-8")
        source = first+middle+second+between+third+suffix
        applied = first+middle+second+between+third2+suffix
        applied2 = first+middle+final+between+third2+suffix
        typed = first+middle+final+b"X"+between+third2+suffix
        copies = desktop.expect_copy.call_args_list
        self.assertEqual(len(copies), 25)
        self.assertEqual([c.args[1] for c in copies[1:-1]], [source,first,second,third,third2,second,second,first,first2,applied,source,applied,second,b"one\n# heading",b"one\n# heading",applied,second,final,first,typed,applied2,applied,source])
        self.assertEqual(copies[0].args[1],copies[-1].args[1])
        self.assertEqual(desktop.paste_from_clean.call_args.args[0],source)
        self.assertTrue(all(c.kwargs=={"refocus":False} for c in copies))
        def at(x,y):
            return ("xdotool","mousemove","--window","42",str(x),str(y),"click","1")
        clicks=[c.args for c in desktop.command.call_args_list if c.args[-2:]==("click","1")]
        self.assertEqual(clicks,[at(984,151),at(450,408),at(450,408),at(302,452),at(302,452),at(302,452),at(1144,151),at(450,408),at(302,452),at(1144,151),at(450,408),at(302,452)])
        keys=[c.args[0] for c in desktop.key.call_args_list]
        self.assertEqual(keys.count("ctrl+z"),6)
        self.assertEqual(keys.count("ctrl+y"),1)
        self.assertEqual(keys.count("ctrl+shift+m"),2)
        self.assertEqual(keys.count("Escape"),3)
        self.assertEqual(keys.count("ctrl+a"),4)
        self.assertEqual(keys.count("ctrl+v"),4)
        self.assertEqual([c.args[0] for c in desktop.type_ascii.call_args_list],["X"])
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count,2)

    def test_paragraph_apply_previous_dispatch_is_exclusive_and_keeps_all_eighteen_previous_suites(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        names=["exercise","exercise_task_batch","exercise_table_alignment","exercise_table_rows","exercise_table_cells","exercise_cell_navigation","exercise_table_insert","exercise_table_columns","exercise_table_column_delete","exercise_table_row_move","exercise_table_column_move","exercise_table_column_duplicate","exercise_cell_apply_previous","exercise_paragraph_draft","exercise_paragraph_multiline","exercise_paragraph_draft_history","exercise_paragraph_navigation","exercise_paragraph_apply_next"]
        with contextlib.ExitStack() as stack:
            old=[stack.enter_context(patch.object(gui,name)) for name in names]
            new=stack.enter_context(patch.object(gui,"exercise_paragraph_apply_previous"))
            gui.exercise_suite(desktop,artifacts,binary,"paragraph-apply-previous")
            new.assert_called_once_with(desktop,artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_paragraph_draft_shortcuts_probe_locks_ctrl_commit_noop_boundary_focus_and_history(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_paragraph_draft_shortcuts(desktop, Path("/unused-evidence"))
        first = "café🙂 **first**  \na\\|b &amp;".encode("utf-8")
        middle = b"\n\n# gap\n\nsingle line\n\n> nested\n> paragraph\n\n```text\nliteral\nparagraph\n```\n\n"
        second = "second é🙂\nraw **tail**  ".encode("utf-8")
        suffix = b"\n\nterminal\n\t"
        edited = "edited **é🙂**\nsecond  \nthird &amp;".encode("utf-8")
        changed = "changed 🙂\nlast draft  ".encode("utf-8")
        source, applied = first+middle+second+suffix, edited+middle+second+suffix
        copies = [c.args[1] for c in desktop.expect_copy.call_args_list]
        welcome = (gui.ROOT / "src" / "welcome.md").read_bytes()
        self.assertEqual(copies, [welcome, source, first, edited, applied, edited, second, edited, edited, second, changed, changed, applied, source, welcome])
        self.assertEqual(desktop.paste_from_clean.call_args.args[0], source)
        self.assertTrue(all(c.kwargs=={"refocus":False} for c in desktop.expect_copy.call_args_list))
        def at(x,y):
            return ("xdotool","mousemove","--window","42",str(x),str(y),"click","1")
        clicks=[c.args for c in desktop.command.call_args_list if c.args[-2:]==("click","1")]
        self.assertEqual(clicks,[at(984,151),at(1144,151)])
        keys=[c.args[0] for c in desktop.key.call_args_list]
        for key, count in [("ctrl+Return",1),("ctrl+Next",3),("ctrl+Prior",2),("ctrl+shift+m",2),("Escape",1),("ctrl+z",2),("ctrl+a",2),("ctrl+v",2)]:
            self.assertEqual(keys.count(key), count, key)
        self.assertEqual(desktop.type_ascii.call_count,0)
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count,0)

    def test_paragraph_draft_shortcut_dispatch_is_exclusive_and_keeps_all_nineteen_previous_suites(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        names=["exercise","exercise_task_batch","exercise_table_alignment","exercise_table_rows","exercise_table_cells","exercise_cell_navigation","exercise_table_insert","exercise_table_columns","exercise_table_column_delete","exercise_table_row_move","exercise_table_column_move","exercise_table_column_duplicate","exercise_cell_apply_previous","exercise_paragraph_draft","exercise_paragraph_multiline","exercise_paragraph_draft_history","exercise_paragraph_navigation","exercise_paragraph_apply_next","exercise_paragraph_apply_previous"]
        with contextlib.ExitStack() as stack:
            old=[stack.enter_context(patch.object(gui,name)) for name in names]
            new=stack.enter_context(patch.object(gui,"exercise_paragraph_draft_shortcuts"))
            gui.exercise_suite(desktop,artifacts,binary,"paragraph-draft-shortcuts")
            new.assert_called_once_with(desktop,artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_single_paragraph_navigation_probe_locks_skips_boundaries_refusal_focus_and_history(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_single_paragraph_navigation(desktop, Path("/unused-evidence"))
        first = "café🙂 **first**  ".encode("utf-8")
        gap = b"\n\n# gap\n\n> nested\n> quote\n\n```text\ncode\nblock\n```\n\n"
        second = "second é🙂 &amp;".encode("utf-8")
        between = b"\n\n## second gap\n\n- item\n  - nested item\n\n"
        third = "third 🙂  ".encode("utf-8")
        suffix = b"\n\nterminal\n\t"
        third2 = "growth **é🙂**  ".encode("utf-8")
        first2 = "first edited 🙂".encode("utf-8")
        final = "final é🙂".encode("utf-8")
        source = first+gap+second+between+third+suffix
        applied = first+gap+second+between+third2+suffix
        final_source = final+gap+second+between+third2+suffix
        typed = final+b"X"+gap+second+between+third2+suffix
        welcome = (gui.ROOT / "src" / "welcome.md").read_bytes()
        copies = [c.args[1] for c in desktop.expect_copy.call_args_list]
        self.assertEqual(copies, [welcome, source, first, second, third, third2, second, first, first, first2, applied, source, applied,
                                  second, b"# heading", b"# heading", applied, final, second, typed, final_source, applied, source, welcome])
        self.assertEqual(desktop.paste_from_clean.call_args.args[0], source)
        self.assertTrue(all(c.kwargs=={"refocus":False} for c in desktop.expect_copy.call_args_list))
        def at(x,y):
            return ("xdotool","mousemove","--window","42",str(x),str(y),"click","1")
        clicks=[c.args for c in desktop.command.call_args_list if c.args[-2:]==("click","1")]
        self.assertEqual(clicks,[at(984,151),at(430,244),at(430,244),at(270,288),at(270,288),at(270,288),
                                 at(1144,151),at(430,244),at(428,288),at(1144,151),at(428,288)])
        keys=[c.args[0] for c in desktop.key.call_args_list]
        self.assertEqual(keys.count("ctrl+shift+m"),2)
        self.assertEqual(keys.count("Escape"),3)
        self.assertEqual(keys.count("ctrl+z"),5)
        self.assertEqual(keys.count("ctrl+y"),1)
        self.assertEqual(keys.count("ctrl+Prior"),1)
        self.assertEqual(keys.count("ctrl+a"),4)
        self.assertEqual(keys.count("ctrl+v"),4)
        self.assertEqual([c.args[0] for c in desktop.type_ascii.call_args_list],["X"])
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()
        self.assertEqual(desktop.screenshot.call_count,2)

    def test_single_paragraph_navigation_dispatch_is_exclusive_and_keeps_all_twenty_previous_suites(self):
        desktop, artifacts, binary = Mock(), Path("/unused"), Path("/unused/binary")
        names=["exercise","exercise_task_batch","exercise_table_alignment","exercise_table_rows","exercise_table_cells","exercise_cell_navigation","exercise_table_insert","exercise_table_columns","exercise_table_column_delete","exercise_table_row_move","exercise_table_column_move","exercise_table_column_duplicate","exercise_cell_apply_previous","exercise_paragraph_draft","exercise_paragraph_multiline","exercise_paragraph_draft_history","exercise_paragraph_navigation","exercise_paragraph_apply_next","exercise_paragraph_apply_previous","exercise_paragraph_draft_shortcuts"]
        with contextlib.ExitStack() as stack:
            old=[stack.enter_context(patch.object(gui,name)) for name in names]
            new=stack.enter_context(patch.object(gui,"exercise_single_paragraph_navigation"))
            gui.exercise_suite(desktop,artifacts,binary,"paragraph-navigation-single")
            new.assert_called_once_with(desktop,artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_draft_history_single_probe_locks_buttons_undo_redo_and_source_isolation(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_draft_history_single(desktop, Path("/unused-evidence"))
        welcome = (gui.ROOT / "src" / "welcome.md").read_bytes()
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
        copies = [c.args[1] for c in desktop.expect_copy.call_args_list]
        self.assertEqual(copies, [welcome, para_source, payload, first, payload, first, payload, para_source,
                                  kept2, kept, applied, para_source, table, cell_initial, cell_a, cell_initial,
                                  cell_a, cell_initial, table, cell_kept, cell_applied, table, para_source,
                                  welcome])
        self.assertTrue(all(c.kwargs == {"refocus": False} for c in desktop.expect_copy.call_args_list))
        clicks = [c.args for c in desktop.command.call_args_list if c.args[-2:] == ("click", "1")]
        def at(x, y):
            return ("xdotool", "mousemove", "--window", "42", str(x), str(y), "click", "1")
        self.assertEqual(clicks, [at(984,151),at(566,288),at(694,288),at(566,288),
                                  at(984,151),at(566,288),
                                  at(984,151),at(900,354),at(486,200),at(614,200),at(486,200),
                                  at(984,151),at(900,354),at(486,200)])
        keys = [c.args[0] for c in desktop.key.call_args_list]
        self.assertEqual(keys.count("ctrl+shift+m"), 11)
        self.assertEqual(keys.count("Escape"), 2)
        self.assertEqual(keys.count("Return"), 2)
        self.assertEqual(keys.count("ctrl+z"), 4)
        self.assertEqual(keys.count("ctrl+a"), 7)
        self.assertEqual(keys.count("ctrl+v"), 7)
        self.assertEqual(keys.count("ctrl+y"), 0)
        self.assertEqual([c.args[0] for c in desktop.paste_from_clean.call_args_list], [para_source])
        self.assertEqual(desktop.title_is_dirty.call_count, 2)
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()

    def test_draft_history_single_dispatch_is_exclusive_and_keeps_every_previous_suite(self):
        desktop, artifacts = Mock(), Path("/unused")
        names = ["exercise","exercise_task_batch","exercise_table_alignment","exercise_table_rows","exercise_table_cells","exercise_cell_navigation","exercise_table_insert","exercise_table_columns","exercise_table_column_delete","exercise_table_row_move","exercise_table_column_move","exercise_table_column_duplicate","exercise_cell_apply_previous","exercise_paragraph_draft","exercise_paragraph_multiline","exercise_paragraph_draft_history","exercise_paragraph_navigation","exercise_paragraph_apply_next","exercise_paragraph_apply_previous","exercise_paragraph_draft_shortcuts","exercise_single_paragraph_navigation"]
        with contextlib.ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_draft_history_single"))
            gui.exercise_suite(desktop, artifacts, Path("/unused/binary"), "draft-history-single")
            new.assert_called_once_with(desktop, artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_draft_history_keys_probe_drives_only_the_local_stacks(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_draft_history_keys(desktop, Path("/unused-evidence"))
        welcome = (gui.ROOT / "src" / "welcome.md").read_bytes()
        payload = "café🙂 **raw** &amp;  ".encode("utf-8")
        para_source = payload + b"\n\n# untouched\n\ntail \xf0\x9f\x99\x82\n\t"
        first = "first edited 🙂".encode("utf-8")
        table = ("| Name | Count |\n  | ---  | --: |\n | café🙂 **raw** | a\\|b &amp; |\n\nafter\n\t").encode("utf-8")
        cell_initial = "café🙂 **raw**".encode("utf-8")
        cell_a = "café🙂 **raw** A".encode("utf-8")
        copies = [c.args[1] for c in desktop.expect_copy.call_args_list]
        self.assertEqual(copies, [welcome, para_source, payload, first, payload, first, first, payload,
                                  para_source, table, cell_initial, cell_a, cell_initial, cell_a,
                                  table, para_source, welcome])
        self.assertTrue(all(c.kwargs == {"refocus": False} for c in desktop.expect_copy.call_args_list))
        clicks = [c.args for c in desktop.command.call_args_list if c.args[-2:] == ("click", "1")]
        def at(x, y):
            return ("xdotool", "mousemove", "--window", "42", str(x), str(y), "click", "1")
        self.assertEqual(clicks, [at(984,151), at(984,151), at(900,354)])
        keys = [c.args[0] for c in desktop.key.call_args_list]
        self.assertEqual(keys.count("ctrl+z"), 7)
        self.assertEqual(keys.count("ctrl+y"), 1)
        self.assertEqual(keys.count("ctrl+shift+z"), 2)
        self.assertEqual(keys.count("ctrl+shift+m"), 5)
        self.assertEqual(keys.count("Escape"), 2)
        self.assertEqual(keys.count("Return"), 0)
        self.assertEqual(keys.count("ctrl+a"), 3)
        self.assertEqual(keys.count("ctrl+v"), 3)
        self.assertEqual([c.args[0] for c in desktop.paste_from_clean.call_args_list], [para_source])
        self.assertEqual(desktop.title_is_dirty.call_count, 2)
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()

    def test_draft_history_keys_dispatch_is_exclusive_and_keeps_every_previous_suite(self):
        desktop, artifacts = Mock(), Path("/unused")
        names = ["exercise","exercise_task_batch","exercise_table_alignment","exercise_table_rows","exercise_table_cells","exercise_cell_navigation","exercise_table_insert","exercise_table_columns","exercise_table_column_delete","exercise_table_row_move","exercise_table_column_move","exercise_table_column_duplicate","exercise_cell_apply_previous","exercise_paragraph_draft","exercise_paragraph_multiline","exercise_paragraph_draft_history","exercise_paragraph_navigation","exercise_paragraph_apply_next","exercise_paragraph_apply_previous","exercise_paragraph_draft_shortcuts","exercise_single_paragraph_navigation","exercise_draft_history_single"]
        with contextlib.ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_draft_history_keys"))
            gui.exercise_suite(desktop, artifacts, Path("/unused/binary"), "draft-history-keys")
            new.assert_called_once_with(desktop, artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_tabs_probe_keeps_every_buffer_independent(self):
        desktop = Mock(window="42")
        with patch.object(gui.time, "sleep"):
            gui.exercise_tabs(desktop, Path("/unused-evidence"))
        welcome = (gui.ROOT / "src" / "welcome.md").read_bytes()
        one = "tab one 🙂 **raw**".encode("utf-8")
        two = "tab two é &amp;".encode("utf-8")
        copies = [c.args[1] for c in desktop.expect_copy.call_args_list]
        self.assertEqual(copies, [welcome, one, welcome, one, two, welcome, two, two])
        self.assertTrue(all(c.kwargs == {"refocus": False} for c in desktop.expect_copy.call_args_list))
        self.assertEqual(desktop.title_is_dirty.call_count, 3)
        keys = [c.args[0] for c in desktop.key.call_args_list]
        self.assertEqual(keys.count("ctrl+t"), 1)
        self.assertEqual(keys.count("ctrl+w"), 2)
        self.assertEqual(keys.count("alt+1"), 2)
        self.assertEqual(keys.count("alt+2"), 1)
        self.assertEqual(keys.count("ctrl+z"), 2)
        self.assertEqual(keys.count("ctrl+a"), 2)
        self.assertEqual(keys.count("ctrl+v"), 2)
        # The tab suite must never click: every tab move is keyboard-only so no
        # toolbar coordinate can silently drift.
        self.assertEqual([c.args for c in desktop.command.call_args_list], [])
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()

    def test_tabs_dispatch_is_exclusive_and_keeps_every_previous_suite(self):
        desktop, artifacts = Mock(), Path("/unused")
        names = ["exercise","exercise_task_batch","exercise_table_alignment","exercise_table_rows","exercise_table_cells","exercise_cell_navigation","exercise_table_insert","exercise_table_columns","exercise_table_column_delete","exercise_table_row_move","exercise_table_column_move","exercise_table_column_duplicate","exercise_cell_apply_previous","exercise_paragraph_draft","exercise_paragraph_multiline","exercise_paragraph_draft_history","exercise_paragraph_navigation","exercise_paragraph_apply_next","exercise_paragraph_apply_previous","exercise_paragraph_draft_shortcuts","exercise_single_paragraph_navigation","exercise_draft_history_single","exercise_draft_history_keys"]
        with contextlib.ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_tabs"))
            gui.exercise_suite(desktop, artifacts, Path("/unused/binary"), "tabs")
            new.assert_called_once_with(desktop, artifacts)
            for probe in old:
                probe.assert_not_called()

    def test_workspace_probe_drives_the_tree_from_the_keyboard_only(self):
        desktop = Mock(window="42")
        with tempfile.TemporaryDirectory() as temp, patch.object(gui.time, "sleep"):
            workspace = Path(temp) / "workspace"
            gui.exercise_workspace(desktop, Path("/unused-evidence"), workspace)
        welcome = (gui.ROOT / "src" / "welcome.md").read_bytes()
        copies = [call.args[1] for call in desktop.expect_copy.call_args_list]
        self.assertEqual(copies, [
            welcome,
            welcome,
            "beta tree é".encode("utf-8"),
            "beta tree é".encode("utf-8"),
            "alpha tree 🙂".encode("utf-8"),
            "nested tree ✓".encode("utf-8"),
            "gamma tree ✓".encode("utf-8"),
        ])
        self.assertTrue(all(call.kwargs == {"refocus": False}
                            for call in desktop.expect_copy.call_args_list))
        keys = [call.args[0] for call in desktop.key.call_args_list]
        self.assertEqual(keys.count("alt+Return"), 6)
        self.assertEqual(keys.count("F5"), 1)
        self.assertEqual(keys.count("alt+j"), 10 + 1 + 1 + 10)
        self.assertEqual(keys.count("alt+k"), 3 + 1)
        titles = [call.args[0] for call in desktop.title_ends_with.call_args_list]
        self.assertEqual(titles, ["beta.md — Hylite\n", "alpha.md — Hylite\n",
                                  "nested.md — Hylite\n", "gamma.md — Hylite\n"])
        self.assertEqual(desktop.title_is_dirty.call_count, 2)
        # No mouse: the whole tree walk is keyboard-only, so no row or sidebar
        # coordinate can silently drift.
        self.assertEqual([call.args for call in desktop.command.call_args_list], [])
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()

    def test_workspace_fixture_is_exact_utf8_and_creates_the_nested_folder(self):
        with tempfile.TemporaryDirectory() as temp:
            documents = gui.workspace_fixture(Path(temp) / "workspace")
            for name, data in documents.items():
                self.assertEqual((Path(temp) / "workspace" / name).read_bytes(), data)
            self.assertEqual(documents[Path("beta.md")], "beta tree é".encode("utf-8"))
            self.assertIn(Path("notes") / "nested.md", documents)

    def test_workspace_dispatch_is_exclusive_and_keeps_every_previous_suite(self):
        desktop, artifacts = Mock(), Path("/unused")
        names = ["exercise","exercise_task_batch","exercise_table_alignment","exercise_table_rows","exercise_table_cells","exercise_cell_navigation","exercise_table_insert","exercise_table_columns","exercise_table_column_delete","exercise_table_row_move","exercise_table_column_move","exercise_table_column_duplicate","exercise_cell_apply_previous","exercise_paragraph_draft","exercise_paragraph_multiline","exercise_paragraph_draft_history","exercise_paragraph_navigation","exercise_paragraph_apply_next","exercise_paragraph_apply_previous","exercise_paragraph_draft_shortcuts","exercise_single_paragraph_navigation","exercise_draft_history_single","exercise_draft_history_keys","exercise_tabs"]
        with contextlib.ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_workspace"))
            gui.exercise_suite(desktop, artifacts, Path("/unused/binary"), "workspace", Path("/unused/folder"))
            new.assert_called_once_with(desktop, artifacts, Path("/unused/folder"))
            for probe in old:
                probe.assert_not_called()

    def test_workspace_suite_refuses_to_run_without_a_folder(self):
        desktop, artifacts = Mock(), Path("/unused")
        with self.assertRaisesRegex(ValueError, "needs a workspace folder"):
            gui.exercise_suite(desktop, artifacts, Path("/unused/binary"), "workspace")


    def test_images_probe_renders_local_pictures_and_keeps_rejected_markers_inert(self):
        desktop = Mock(window="42")
        with tempfile.TemporaryDirectory() as temp:
            document = gui.images_fixture(Path(temp) / "images")
            source = document.read_bytes()
            with patch.object(gui.time, "sleep"):
                gui.exercise_images(desktop, Path("/unused-evidence"), document)
        self.assertEqual(source, gui.IMAGE_DOCUMENT)
        copies = [call.args[1] for call in desktop.expect_copy.call_args_list]
        self.assertEqual(copies, [source, source, source])
        self.assertTrue(all(call.kwargs == {"refocus": False}
                            for call in desktop.expect_copy.call_args_list))
        keys = [call.args[0] for call in desktop.key.call_args_list]
        self.assertEqual(keys.count("ctrl+shift+m"), 6)
        self.assertEqual(keys.count("ctrl+v"), 1)
        self.assertEqual(desktop.title_ends_with.call_count, 1)
        self.assertEqual(desktop.title_is_dirty.call_count, 2)
        self.assertEqual(desktop.screenshot.call_count, 2)
        # Keyboard-only apart from the initial window focus in find_window.
        self.assertEqual([call.args for call in desktop.command.call_args_list], [])
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()

    def test_images_fixture_writes_a_decodable_png_and_an_oversized_file(self):
        with tempfile.TemporaryDirectory() as temp:
            document = gui.images_fixture(Path(temp) / "images")
            png = (document.parent / "pic.png").read_bytes()
            self.assertTrue(png.startswith(b"\x89PNG\r\n\x1a\n"))
            self.assertEqual(gui.tiny_png()[1:4], b"PNG")
            self.assertGreater((document.parent / "big.png").stat().st_size,
                               4 * 1024 * 1024)
            self.assertFalse((document.parent / "gone.png").exists())
            self.assertIn(b"https://example.invalid/x.png", gui.IMAGE_DOCUMENT)
            self.assertIn("before ![inline](pic.png) after".encode("utf-8"),
                          gui.IMAGE_DOCUMENT)

    def test_images_dispatch_is_exclusive_and_keeps_every_previous_suite(self):
        desktop, artifacts = Mock(), Path("/unused")
        names = ["exercise","exercise_task_batch","exercise_table_alignment","exercise_table_rows","exercise_table_cells","exercise_cell_navigation","exercise_table_insert","exercise_table_columns","exercise_table_column_delete","exercise_table_row_move","exercise_table_column_move","exercise_table_column_duplicate","exercise_cell_apply_previous","exercise_paragraph_draft","exercise_paragraph_multiline","exercise_paragraph_draft_history","exercise_paragraph_navigation","exercise_paragraph_apply_next","exercise_paragraph_apply_previous","exercise_paragraph_draft_shortcuts","exercise_single_paragraph_navigation","exercise_draft_history_single","exercise_draft_history_keys","exercise_tabs","exercise_workspace"]
        with contextlib.ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_images"))
            gui.exercise_suite(desktop, artifacts, Path("/unused/binary"), "images",
                               None, Path("/unused/doc.md"))
            new.assert_called_once_with(desktop, artifacts, Path("/unused/doc.md"))
            for probe in old:
                probe.assert_not_called()

    def test_images_suite_refuses_to_run_without_a_document(self):
        desktop, artifacts = Mock(), Path("/unused")
        with self.assertRaisesRegex(ValueError, "needs a document path"):
            gui.exercise_suite(desktop, artifacts, Path("/unused/binary"), "images")


    def sample_pdf(self, content=None, with_stream=True):
        """A minimal, structurally valid PDF 1.4 the probe must accept."""
        if content is None:
            content = ("BT /F1 10 Tf 56.00 786.00 Td (Catatan PDF) Tj ET\n"
                       "BT /F2 10 Tf 56.00 770.00 Td (tebal) Tj ET\n"
                       "BT /F4 9 Tf 56.00 754.00 Td (kode) Tj ET\n"
                       "BT /F1 10 Tf 56.00 738.00 Td (Kolom) Tj ET\n"
                       "BT /F1 10 Tf 120.00 738.00 Td (Nilai) Tj ET\n"
                       "BT /F1 10 Tf 56.00 722.00 Td (satu) Tj ET\n"
                       "BT /F4 9 Tf 56.00 706.00 Td (baris kode) Tj ET\n"
                       "BT /F1 10 Tf 56.00 690.00 Td (Emoji ? ditolak.) Tj ET\n"
                       "BT /F1 10 Tf 56.00 674.00 Td (caf\\351) Tj ET\n")
        body = content.encode("latin-1")
        objects = [b"<< /Type /Catalog /Pages 2 0 R >>",
                   b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                   b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595.0 842.0] >>"]
        if with_stream:
            objects.append(b"<< /Length " + str(len(body)).encode("ascii")
                           + b" >>\nstream\n" + body + b"endstream")
        pdf = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n"
        offsets = []
        for number, item in enumerate(objects, start=1):
            offsets.append(len(pdf))
            pdf += f"{number} 0 obj\n".encode("ascii") + item + b"\nendobj\n"
        xref = len(pdf)
        pdf += f"xref\n0 {len(objects) + 1}\n".encode("ascii") + b"0000000000 65535 f \n"
        for offset in offsets:
            pdf += f"{offset:010d} 00000 n \n".encode("ascii")
        pdf += f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode("ascii")
        return pdf

    def test_pdf_document_validator_accepts_the_renderer_skeleton(self):
        text = gui.assert_pdf_document(self.sample_pdf())
        self.assertIn("(Catatan PDF) Tj", text)

    def test_pdf_document_validator_rejects_a_broken_structure(self):
        valid = self.sample_pdf()
        with self.assertRaisesRegex(AssertionError, "Missing PDF 1.4 header"):
            gui.assert_pdf_document(b"not a pdf")
        with self.assertRaisesRegex(AssertionError, "Missing EOF marker"):
            gui.assert_pdf_document(valid[:-6])
        with self.assertRaisesRegex(AssertionError, "does not point at the xref table"):
            gui.assert_pdf_document(valid.replace(b"startxref\n", b"startxref\n9", 1))
        length = int(valid.split(b"<< /Length ")[1].split(b" ")[0])
        with self.assertRaisesRegex(AssertionError, "declares"):
            gui.assert_pdf_document(valid.replace(b"<< /Length %d" % length,
                                                  b"<< /Length %d" % (length + 7), 1))
        first = len(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
        with self.assertRaisesRegex(AssertionError, "does not point at its header"):
            gui.assert_pdf_document(valid.replace(b"%010d 00000 n" % first,
                                                  b"0000000009 00000 n", 1))
        with self.assertRaisesRegex(AssertionError, "No content stream found"):
            gui.assert_pdf_document(self.sample_pdf(with_stream=False))

    def test_pdf_fixture_is_exact_utf8_with_a_table_a_code_block_and_an_emoji(self):
        with tempfile.TemporaryDirectory() as temp:
            document = gui.pdf_fixture(Path(temp) / "pdf")
            self.assertEqual(document.read_bytes(), gui.PDF_DOCUMENT)
        self.assertIn("| :--- | ---: |".encode("utf-8"), gui.PDF_DOCUMENT)
        self.assertIn("baris kode".encode("utf-8"), gui.PDF_DOCUMENT)
        self.assertIn("caf\xc3\xa9".encode("latin-1"), gui.PDF_DOCUMENT)
        self.assertIn("\xf0\x9f\x99\x82".encode("latin-1"), gui.PDF_DOCUMENT)

    def test_pdf_probe_exports_headlessly_and_refuses_destructive_inputs(self):
        with tempfile.TemporaryDirectory() as temp:
            document = gui.pdf_fixture(Path(temp) / "pdf")
            source = document.read_bytes()
            artifacts = Path(temp) / "evidence"
            artifacts.mkdir()
            sample = self.sample_pdf()

            def fake_export(binary, arguments, env):
                if len(arguments) != 2:
                    return gui.subprocess.CompletedProcess([], 2, b"", b"usage")
                head, tail = Path(arguments[0]), Path(arguments[1])
                if not head.exists() or head == tail:
                    return gui.subprocess.CompletedProcess([], 1, b"", b"refused")
                tail.write_bytes(sample)
                return gui.subprocess.CompletedProcess([], 0, b"", b"")

            usage = gui.subprocess.CompletedProcess(
                [], 2, b"", b"usage: hylite export-pdf <input.md> <output.pdf>\n")
            desktop = Mock(window="42", env={})
            with patch.object(gui, "run_export_cli", side_effect=fake_export), \
                 patch.object(gui.subprocess, "run", return_value=usage):
                gui.exercise_pdf(desktop, artifacts, Path("/unused/binary"), document)

            self.assertEqual(document.read_bytes(), source)
            self.assertEqual((artifacts / "catatan.pdf").read_bytes(), sample)
            self.assertEqual(desktop.passed_check.call_count, 7)
            self.assertEqual([call.args for call in desktop.screenshot.call_args_list],
                             [(artifacts / "before-export.png",), (artifacts / "after-export.png",)])
            desktop.expect_copy.assert_called_once_with(
                "The GUI document is still the exact Markdown after the exports", source, refocus=False)
            desktop.request_close.assert_called_once_with()
            desktop.expect_native_exit.assert_called_once()

    def test_pdf_dispatch_is_exclusive_and_keeps_every_previous_suite(self):
        desktop, artifacts = Mock(), Path("/unused")
        names = ["exercise", "exercise_navigation", "exercise_preview", "exercise_tables",
                 "exercise_preview_cache", "exercise_preview_reveal", "exercise_cell_reveal",
                 "exercise_task_toggle", "exercise_preview_task_history", "exercise_preview_heading",
                 "exercise_preview_setext", "exercise_lifecycle", "exercise_task_batch",
                 "exercise_table_alignment", "exercise_table_rows", "exercise_table_cells",
                 "exercise_cell_navigation", "exercise_table_insert", "exercise_table_columns",
                 "exercise_table_column_delete", "exercise_table_row_move",
                 "exercise_table_column_move", "exercise_table_column_duplicate",
                 "exercise_cell_apply_previous", "exercise_paragraph_draft",
                 "exercise_paragraph_multiline", "exercise_paragraph_draft_history",
                 "exercise_paragraph_navigation", "exercise_paragraph_apply_next",
                 "exercise_paragraph_apply_previous", "exercise_paragraph_draft_shortcuts",
                 "exercise_single_paragraph_navigation", "exercise_draft_history_single",
                 "exercise_draft_history_keys", "exercise_tabs", "exercise_workspace",
                 "exercise_images"]
        with contextlib.ExitStack() as stack:
            old = [stack.enter_context(patch.object(gui, name)) for name in names]
            new = stack.enter_context(patch.object(gui, "exercise_pdf"))
            gui.exercise_suite(desktop, artifacts, Path("/unused/binary"), "pdf",
                               None, Path("/unused/doc.md"))
            new.assert_called_once_with(desktop, artifacts, Path("/unused/binary"),
                                        Path("/unused/doc.md"))
            for probe in old:
                probe.assert_not_called()

    def test_pdf_suite_refuses_to_run_without_a_document(self):
        desktop, artifacts = Mock(), Path("/unused")
        with self.assertRaisesRegex(ValueError, "needs a document path"):
            gui.exercise_suite(desktop, artifacts, Path("/unused/binary"), "pdf")

    def test_math_fixture_contains_exact_inline_display_and_unsupported_forms(self):
        with tempfile.TemporaryDirectory() as temp:
            document = gui.math_fixture(Path(temp) / "math")
            self.assertEqual(document.name, "math.md")
            self.assertEqual(document.read_bytes(), gui.MATH_DOCUMENT)
        self.assertIn(b"$x^2", gui.MATH_DOCUMENT)
        self.assertIn(b"$$\n", gui.MATH_DOCUMENT)
        self.assertIn(b"\\begin{matrix}", gui.MATH_DOCUMENT)

    def test_math_probe_keeps_document_bytes_through_preview_and_split(self):
        with tempfile.TemporaryDirectory() as temp:
            document = gui.math_fixture(Path(temp) / "math")
            source = document.read_bytes()
            desktop = Mock(window="42")
            gui.exercise_math(desktop, Path(temp) / "evidence", document)
            self.assertEqual(document.read_bytes(), source)

        self.assertEqual(desktop.passed_check.call_count, 3)
        self.assertEqual(
            [call.args[0] for call in desktop.key.call_args_list],
            ["ctrl+shift+m", "ctrl+v", "ctrl+shift+m", "ctrl+shift+m"],
        )
        self.assertEqual(
            [call.args[0] for call in desktop.screenshot.call_args_list],
            [Path(temp) / "evidence" / "before-math.png",
             Path(temp) / "evidence" / "math-preview.png",
             Path(temp) / "evidence" / "math-split.png"],
        )
        self.assertEqual(desktop.expect_copy.call_count, 2)
        self.assertEqual(desktop.expect_copy.call_args.kwargs, {"refocus": False})
        desktop.title_is_dirty.assert_any_call(False)
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()

    def test_math_dispatch_is_exclusive_and_requires_a_document(self):
        desktop, artifacts = Mock(), Path("/unused")
        document = Path("/unused/math.md")
        with patch.object(gui, "exercise_math") as math, \
             patch.object(gui, "exercise_pdf") as pdf:
            gui.exercise_suite(desktop, artifacts, Path("/unused/binary"), "math",
                               None, document)
        math.assert_called_once_with(desktop, artifacts, document)
        pdf.assert_not_called()
        with self.assertRaisesRegex(ValueError, "needs a document path"):
            gui.exercise_suite(desktop, artifacts, Path("/unused/binary"), "math")


    def test_inline_spike_fixture_and_probe_use_an_exact_bold_source_delta(self):
        with tempfile.TemporaryDirectory() as temp:
            document = gui.inline_spike_fixture(Path(temp) / "inline-spike")
            self.assertEqual(document.name, "inline-spike.md")
            self.assertEqual(document.read_bytes(), b"left **bold** right\n")
            desktop = Mock(window="42")
            artifacts = Path(temp) / "evidence"
            gui.exercise_inline_spike(desktop, artifacts, document)
            self.assertEqual(document.read_bytes(), b"left **bold** right\n")

        self.assertEqual(desktop.title_is_dirty.call_args_list,
                         [call(False), call(True), call(False), call(True), call(False)])
        self.assertEqual(
            [call.args for call in desktop.expect_copy.call_args_list],
            [
                ("Visible caret inside bold inserts exactly one Source character",
                 b"left **boXld** right\n"),
                ("One Source Undo restores the byte-identical inline spike fixture",
                 b"left **bold** right\n"),
                ("Redo restores the exact bold-run insertion", b"left **boXld** right\n"),
            ],
        )
        keys = [item.args[0] for item in desktop.key.call_args_list]
        self.assertEqual(keys[:8], ["Home", *["Right"] * 7])
        self.assertEqual(keys[8:], ["Escape", "ctrl+shift+m", "ctrl+z",
                                    "ctrl+shift+z", "ctrl+z"])
        desktop.command.assert_any_call(
            "xdotool", "mousemove", "--window", "42", "400", "150", "click", "1"
        )
        self.assertEqual(desktop.screenshot.call_count, 4)
        desktop.find_window.assert_called_once_with(initial_click=False)
        self.assertTrue(all(item.kwargs == {"refocus": False}
                            for item in desktop.expect_copy.call_args_list))
        desktop.request_close.assert_called_once_with()
        desktop.expect_native_exit.assert_called_once()

    def test_inline_spike_dispatch_is_exclusive_and_requires_a_document(self):
        desktop, artifacts = Mock(), Path("/unused")
        document = Path("/unused/inline-spike.md")
        with patch.object(gui, "exercise_inline_spike") as spike, \
             patch.object(gui, "exercise_math") as math:
            gui.exercise_suite(desktop, artifacts, Path("/unused/binary"), "inline-spike",
                               None, document)
        spike.assert_called_once_with(desktop, artifacts, document)
        math.assert_not_called()
        with self.assertRaisesRegex(ValueError, "needs a document path"):
            gui.exercise_suite(desktop, artifacts, Path("/unused/binary"), "inline-spike")


if __name__ == "__main__":
    unittest.main()
