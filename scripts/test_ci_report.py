"""Tests use only temporary files/processes, never GitHub or user config."""
import contextlib
import importlib.util
import io
import pathlib
import subprocess
import unittest

ROOT = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("ci_report", ROOT / "ci-report.py")
ci_report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ci_report)


class DiagnosticsTests(unittest.TestCase):
    def rendered(self, text, status):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            ci_report.report(text, status)
        return output.getvalue()

    def test_log_commands_are_suspended_and_annotation_values_escaped(self):
        text = "prefix ::warning::untrusted\n::error::also untrusted\n100%\r\n"
        output = self.rendered(text, 7)
        token = output.splitlines()[0].removeprefix("::stop-commands::")
        self.assertEqual(len(token), 32)
        self.assertIn(f"::{token}::\n::error title=", output)
        self.assertIn("100%25", output)
        self.assertEqual(ci_report.escape("%\r\n"), "%25%0D%0A")

    def test_failure_tail_has_at_most_four_annotations_and_success_is_notice(self):
        output = self.rendered("x" * 100000, 1)
        self.assertEqual(output.count("::error title="), 4)
        notice = self.rendered("", 0)
        self.assertIn("::notice title=", notice)
        self.assertNotIn("::error title=", notice)

    def test_shell_wrapper_preserves_success_and_failure_exit_codes(self):
        for status in [0, 7, 101]:
            result = subprocess.run(
                ["bash", str(ROOT / "ci-cargo.sh"), "bash", "-c", f"echo fixture; exit {status}"],
                capture_output=True, text=True, check=False,
            )
            self.assertEqual(result.returncode, status)
            self.assertIn("fixture", result.stdout)


if __name__ == "__main__":
    unittest.main()
