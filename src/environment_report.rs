//! A deliberately small, privacy-conscious environment report for support.
//!
//! Only a fixed allowlist of locale/session hints is read. Display and D-Bus
//! endpoints are reported as set/unset; their values and all file paths are
//! intentionally omitted.

const LOCALE_HINTS: [(&str, &str); 4] = [
    ("LC_ALL", "LC_ALL"),
    ("LC_MESSAGES", "LC_MESSAGES"),
    ("LANG", "LANG"),
    ("LANGUAGE", "LANGUAGE"),
];

const SESSION_HINTS: [(&str, &str); 3] = [
    ("XDG_SESSION_TYPE", "XDG_SESSION_TYPE"),
    ("XDG_CURRENT_DESKTOP", "XDG_CURRENT_DESKTOP"),
    ("XDG_SESSION_DESKTOP", "XDG_SESSION_DESKTOP"),
];

#[derive(Debug, Default)]
struct EnvironmentHints {
    locale: Vec<(&'static str, Option<String>)>,
    session: Vec<(&'static str, Option<String>)>,
    wayland_display_set: bool,
    x11_display_set: bool,
    dbus_session_bus_set: bool,
}

impl EnvironmentHints {
    fn current() -> Self {
        let locale = LOCALE_HINTS
            .iter()
            .map(|(label, name)| (*label, safe_environment_hint(name)))
            .collect();
        let session = SESSION_HINTS
            .iter()
            .map(|(label, name)| (*label, safe_environment_hint(name)))
            .collect();

        Self {
            locale,
            session,
            wayland_display_set: environment_variable_is_set("WAYLAND_DISPLAY"),
            x11_display_set: environment_variable_is_set("DISPLAY"),
            dbus_session_bus_set: environment_variable_is_set("DBUS_SESSION_BUS_ADDRESS"),
        }
    }
}

/// Build the report from the active app configuration and the current process
/// environment. This is called only after an explicit user request.
pub(crate) fn generate(
    font_family: &str,
    font_size: f32,
    scale_factor: Option<f32>,
) -> String {
    render(
        font_family,
        font_size,
        scale_factor,
        std::any::type_name::<iced::Renderer>(),
        &EnvironmentHints::current(),
    )
}

fn render(
    font_family: &str,
    font_size: f32,
    scale_factor: Option<f32>,
    renderer_type: &str,
    hints: &EnvironmentHints,
) -> String {
    let mut report = format!(
        "Hylite environment report\n\
         Hylite version: {}\n\
         Build target: {} / {}\n\
         Iced dependency: 0.13.x\n\
         Iced renderer type (compile-time): {}\n\
         Window scale factor: {}\n\
         Configured editor font: {}\n\
         Configured font size: {} logical px\n\
         Locale hints:\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        safe_label(renderer_type, false),
        scale_factor
            .filter(|factor| factor.is_finite() && *factor > 0.0)
            .map(|factor| format!("{factor:.2}"))
            .unwrap_or_else(|| "not available yet".to_owned()),
        safe_label(font_family, true),
        if font_size.is_finite() && font_size > 0.0 {
            format!("{font_size:.1}")
        } else {
            "not available".to_owned()
        },
    );

    append_hints(&mut report, &hints.locale);
    report.push_str("Desktop session hints:\n");
    append_hints(&mut report, &hints.session);
    report.push_str(&format!(
        "Display endpoints: WAYLAND_DISPLAY={}, DISPLAY={} (presence only; values redacted)\n\
         D-Bus session bus configured: {} (address redacted)\n\
         File-dialog portal: active provider is not introspected by the app; run scripts/report-environment.sh for service-state hints.\n\
         Notes: renderer type is compile-time information, not the active GPU/window backend; portal service state is not proof of provider selection.\n\
         Privacy: document content/path, home/runtime paths, display values, and D-Bus address are not included.\n",
        set_or_unset(hints.wayland_display_set),
        set_or_unset(hints.x11_display_set),
        yes_or_no(hints.dbus_session_bus_set),
    ));

    report
}

fn append_hints(report: &mut String, hints: &[(&'static str, Option<String>)]) {
    for (label, value) in hints {
        let value = value
            .as_deref()
            .map(|value| safe_label(value, true))
            .unwrap_or_else(|| "unset".to_owned());
        report.push_str(&format!("  {label}: {value}\n"));
    }
}

fn safe_environment_hint(name: &str) -> Option<String> {
    let value = std::env::var_os(name)?;
    let value = value.to_string_lossy();
    (!value.is_empty()).then(|| safe_label(&value, true))
}

fn environment_variable_is_set(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| !value.as_os_str().is_empty())
}

/// Keep human-readable labels useful while filtering path separators, controls,
/// and other arbitrary content that could turn an environment hint into a path
/// or multiline log injection. Font names may contain spaces; type names may not.
fn safe_label(value: &str, allow_spaces: bool) -> String {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 128 {
        return "[filtered]".to_owned();
    }

    let safe = value.chars().all(|character| {
        character.is_ascii_alphanumeric()
            || "_-.@:+,".contains(character)
            || (allow_spaces && character == ' ')
    });

    if safe {
        value.to_owned()
    } else {
        "[filtered]".to_owned()
    }
}

fn set_or_unset(value: bool) -> &'static str {
    if value { "set" } else { "unset" }
}

fn yes_or_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_hints() -> EnvironmentHints {
        EnvironmentHints {
            locale: vec![
                ("LC_ALL", Some("C.UTF-8".to_owned())),
                ("LC_MESSAGES", None),
                ("LANG", Some("en_US.UTF-8".to_owned())),
                ("LANGUAGE", Some("en_US:en".to_owned())),
            ],
            session: vec![
                ("XDG_SESSION_TYPE", Some("wayland".to_owned())),
                ("XDG_CURRENT_DESKTOP", Some("Hyprland".to_owned())),
                ("XDG_SESSION_DESKTOP", Some("Hyprland".to_owned())),
            ],
            wayland_display_set: true,
            x11_display_set: false,
            dbus_session_bus_set: true,
        }
    }

    #[test]
    fn report_includes_target_renderer_scale_font_and_allowlisted_hints() {
        let report = render(
            "JetBrains Mono",
            17.5,
            Some(1.5),
            "iced_wgpu::Renderer",
            &sample_hints(),
        );

        for expected in [
            "Hylite environment report",
            "Iced dependency: 0.13.x",
            "Iced renderer type (compile-time): iced_wgpu::Renderer",
            "Window scale factor: 1.50",
            "Configured editor font: JetBrains Mono",
            "Configured font size: 17.5 logical px",
            "LC_ALL: C.UTF-8",
            "LC_MESSAGES: unset",
            "XDG_SESSION_TYPE: wayland",
            "XDG_CURRENT_DESKTOP: Hyprland",
            "WAYLAND_DISPLAY=set, DISPLAY=unset",
            "D-Bus session bus configured: yes",
        ] {
            assert!(report.contains(expected), "missing {expected:?} in report:\n{report}");
        }
    }

    #[test]
    fn sensitive_values_and_untrusted_labels_are_filtered() {
        let mut hints = sample_hints();
        hints.locale[0].1 = Some("/home/alice/private\nTOKEN=do-not-leak".to_owned());
        let report = render(
            "/home/alice/private-font",
            f32::NAN,
            Some(f32::INFINITY),
            "/opt/iced_wgpu::Renderer",
            &hints,
        );

        assert!(report.contains("Configured editor font: [filtered]"));
        assert!(report.contains("Configured font size: not available"));
        assert!(report.contains("Window scale factor: not available yet"));
        assert!(report.contains("Iced renderer type (compile-time): [filtered]"));
        assert!(report.contains("LC_ALL: [filtered]"));
        for secret in ["/home/alice", "private-font", "do-not-leak", "TOKEN="] {
            assert!(!report.contains(secret), "report leaked {secret:?}:\n{report}");
        }
    }

    #[test]
    fn only_present_or_absent_state_is_reported_for_display_and_bus_variables() {
        let hints = EnvironmentHints {
            wayland_display_set: true,
            x11_display_set: true,
            dbus_session_bus_set: true,
            ..sample_hints()
        };
        let report = render("monospace", 16.0, None, "iced_tiny_skia::Renderer", &hints);

        assert!(report.contains("WAYLAND_DISPLAY=set, DISPLAY=set (presence only; values redacted)"));
        assert!(report.contains("D-Bus session bus configured: yes (address redacted)"));
        assert!(!report.contains("wayland-1"));
        assert!(!report.contains("unix:path="));
    }
}
