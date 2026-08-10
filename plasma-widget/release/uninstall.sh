#!/usr/bin/env bash
set -euo pipefail

widget_id="com.github.brandedtamarasu.sagewatch"
if command -v kpackagetool6 >/dev/null && kpackagetool6 --type Plasma/Applet --show "${widget_id}" >/dev/null 2>&1; then
  kpackagetool6 --type Plasma/Applet --remove "${widget_id}"
fi
rm -f "${HOME}/.local/libexec/sagewatch-plasma-provider" "${HOME}/.local/libexec/sagewatch-plasma-refresh"
echo "Sagewatch Plasma widget removed."
