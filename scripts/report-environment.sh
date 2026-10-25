#!/usr/bin/env bash
# Print a small support report without dumping the process environment.
# Only named locale/session hints are shown; all endpoint values and paths are
# intentionally omitted. Portal unit states are hints, not provider detection.
set -euo pipefail

safe_report_value() {
    local value="$1"
    if [[ -z "$value" || "${#value}" -gt 160 || "$value" == *$'\n'* || "$value" == *$'\r'* || "$value" == *$'\t'* ]]; then
        printf '[filtered]'
        return
    fi
    if printf '%s\n' "$value" | LC_ALL=C grep -Eq '^[A-Za-z0-9_.:@,+ =-]{1,160}$'; then
        printf '%s' "$value"
    else
        printf '[filtered]'
    fi
}

print_hint() {
    local name="$1"
    local value="${!name:-}"
    if [[ -z "$value" ]]; then
        printf '  %s: unset\n' "$name"
    else
        printf '  %s: %s\n' "$name" "$(safe_report_value "$value")"
    fi
}

print_presence() {
    local name="$1"
    if [[ -n "${!name:-}" ]]; then
        printf '  %s: set (value redacted)\n' "$name"
    else
        printf '  %s: unset\n' "$name"
    fi
}

os_release_value() {
    local key="$1"
    local value=''
    if [[ -r /etc/os-release ]]; then
        value="$(awk -F= -v key="$key" '$1 == key { value = $2; sub(/^\"/, "", value); sub(/\"$/, "", value); print value; exit }' /etc/os-release 2>/dev/null || true)"
    fi
    if [[ -n "$value" ]]; then
        safe_report_value "$value"
    else
        printf 'unknown'
    fi
}

command_version() {
    local command_name="$1"
    local version=''
    if command -v "$command_name" >/dev/null 2>&1; then
        version="$("$command_name" --version 2>/dev/null | awk 'NR == 1 { print $1, $2 }' || true)"
    fi
    if [[ -n "$version" ]]; then
        safe_report_value "$version"
    else
        printf 'not installed'
    fi
}

portal_service_state() {
    local service="$1"
    local state=''
    if command -v systemctl >/dev/null 2>&1 && command -v timeout >/dev/null 2>&1; then
        state="$(timeout 3s systemctl --user is-active "$service" 2>/dev/null || true)"
    fi
    case "$state" in
        active|inactive|failed|activating|deactivating|reloading|maintenance)
            printf '%s' "$state"
            ;;
        *)
            printf 'unknown'
            ;;
    esac
}

printf 'Hylite safe environment report (schema 1)\n'
printf 'Operating system: %s %s\n' \
    "$(os_release_value ID)" "$(os_release_value VERSION_ID)"
kernel_info="$(uname -srm 2>/dev/null || true)"
if [[ -n "$kernel_info" ]]; then
    printf 'Kernel / architecture: %s\n' "$(safe_report_value "$kernel_info")"
else
    printf 'Kernel / architecture: unknown\n'
fi
printf 'Toolchain: rustc %s; cargo %s\n' "$(command_version rustc)" "$(command_version cargo)"
printf 'Iced renderer: not inspectable by this shell script; use the app Copy env report\n'

printf '\nLocale hints (allowlist):\n'
for name in LC_ALL LC_MESSAGES LANG LANGUAGE; do
    print_hint "$name"
done

printf '\nDesktop session hints (allowlist):\n'
for name in XDG_SESSION_TYPE XDG_CURRENT_DESKTOP XDG_SESSION_DESKTOP; do
    print_hint "$name"
done

printf '\nDisplay and bus endpoint presence (values redacted):\n'
for name in WAYLAND_DISPLAY DISPLAY DBUS_SESSION_BUS_ADDRESS; do
    print_presence "$name"
done

printf '\nFontconfig hint:\n'
if command -v fc-match >/dev/null 2>&1; then
    font_match="$(fc-match -f '%{family}:style=%{style}' monospace 2>/dev/null || true)"
    if [[ -n "$font_match" ]]; then
        printf '  monospace family/style: %s\n' "$(safe_report_value "$font_match")"
    else
        printf '  monospace family/style: unavailable\n'
    fi
else
    printf '  fc-match: not installed\n'
fi

printf '\nPortal service-state hints (not provider detection):\n'
for service in \
    xdg-desktop-portal.service \
    xdg-desktop-portal-hyprland.service \
    xdg-desktop-portal-gtk.service; do
    printf '  %s: %s\n' "$service" "$(portal_service_state "$service")"
done
printf '\nNotes: service state does not prove which file-dialog provider an application selected.\n'
printf 'Privacy: this script does not print HOME, runtime paths, file paths, document content, display values, or the D-Bus address.\n'
