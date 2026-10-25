import os
import subprocess
import tempfile
import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
REPORT_SCRIPT = REPOSITORY_ROOT / "scripts" / "report-environment.sh"


class EnvironmentReportTests(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        fake_bin = Path(self.temp_dir.name)

        systemctl = fake_bin / "systemctl"
        systemctl.write_text("#!/bin/sh\nprintf 'active\\n'\n", encoding="utf-8")
        systemctl.chmod(0o755)

        fc_match = fake_bin / "fc-match"
        fc_match.write_text(
            "#!/bin/sh\nprintf 'Noto Sans Mono:style=Regular\\n'\n",
            encoding="utf-8",
        )
        fc_match.chmod(0o755)

        self.env = os.environ.copy()
        self.env.update(
            {
                "PATH": f"{fake_bin}{os.pathsep}{self.env.get('PATH', '')}",
                "HOME": "/home/diagnostic-user",
                "XDG_RUNTIME_DIR": "/run/user/1900/private-runtime",
                "XDG_SESSION_TYPE": "wayland",
                "XDG_CURRENT_DESKTOP": "Hyprland",
                "XDG_SESSION_DESKTOP": "Hyprland",
                "LC_ALL": "C.UTF-8",
                "LC_MESSAGES": "en_US.UTF-8",
                "LANG": "en_US.UTF-8",
                "LANGUAGE": "en_US:en",
                "WAYLAND_DISPLAY": "wayland-7-private",
                "DISPLAY": ":77-private",
                "DBUS_SESSION_BUS_ADDRESS": "unix:path=/run/user/1900/private-bus",
                "AWS_SECRET_ACCESS_KEY": "SECRET_TOKEN_MUST_NOT_APPEAR",
            }
        )

    def tearDown(self):
        self.temp_dir.cleanup()

    def run_report(self):
        return subprocess.run(
            [str(REPORT_SCRIPT)],
            cwd=REPOSITORY_ROOT,
            env=self.env,
            text=True,
            capture_output=True,
            check=True,
            timeout=10,
        ).stdout

    def test_report_contains_allowlisted_hints_and_portal_service_states(self):
        report = self.run_report()

        self.assertIn("LANG: en_US.UTF-8", report)
        self.assertIn("XDG_SESSION_TYPE: wayland", report)
        self.assertIn("XDG_CURRENT_DESKTOP: Hyprland", report)
        self.assertIn("WAYLAND_DISPLAY: set (value redacted)", report)
        self.assertIn("DISPLAY: set (value redacted)", report)
        self.assertIn("DBUS_SESSION_BUS_ADDRESS: set (value redacted)", report)
        self.assertIn("xdg-desktop-portal.service: active", report)
        self.assertIn("xdg-desktop-portal-hyprland.service: active", report)
        self.assertIn("Noto Sans Mono:style=Regular", report)

    def test_paths_endpoints_and_unallowlisted_secrets_are_not_printed(self):
        report = self.run_report()

        for secret in (
            "/home/diagnostic-user",
            "/run/user/1900",
            "wayland-7-private",
            ":77-private",
            "unix:path=",
            "SECRET_TOKEN_MUST_NOT_APPEAR",
            "AWS_SECRET_ACCESS_KEY",
        ):
            with self.subTest(secret=secret):
                self.assertNotIn(secret, report)

    def test_malformed_allowlisted_value_is_filtered_as_a_whole(self):
        self.env["LANG"] = "en_US.UTF-8\nTOKEN=SECRET_TOKEN_MUST_NOT_APPEAR"
        report = self.run_report()

        self.assertIn("LANG: [filtered]", report)
        self.assertNotIn("TOKEN=", report)
        self.assertNotIn("SECRET_TOKEN_MUST_NOT_APPEAR", report)


if __name__ == "__main__":
    unittest.main()
