#!/usr/bin/env bash
set -euo pipefail

bundle_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
helper_dir="${HOME}/.local/libexec"
widget_id="com.github.brandedtamarasu.sagewatch"

command -v kpackagetool6 >/dev/null || {
  echo "Sagewatch requires KDE Plasma 6 and kpackagetool6." >&2
  exit 1
}

install -d -m 0755 "${helper_dir}"
install -m 0755 "${bundle_dir}/sagewatch-plasma-provider" "${helper_dir}/sagewatch-plasma-provider"
install -m 0755 "${bundle_dir}/sagewatch-plasma-provider" "${helper_dir}/sagewatch-plasma-refresh"

if kpackagetool6 --type Plasma/Applet --show "${widget_id}" >/dev/null 2>&1; then
  kpackagetool6 --type Plasma/Applet --upgrade "${bundle_dir}/package"
else
  kpackagetool6 --type Plasma/Applet --install "${bundle_dir}/package"
fi

echo "Sagewatch installed. Restart Plasma Shell or sign out and back in, then use Add Widgets > Sagewatch."
