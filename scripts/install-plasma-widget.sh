#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper_dir="${HOME}/.local/libexec"
package_dir="${repo_root}/plasma-widget/package"
widget_id="com.github.brandedtamarasu.sagewatch"
cargo build --release --manifest-path "${repo_root}/src-tauri/Cargo.toml" --bin sagewatch-plasma-provider
install -d -m 0755 "${helper_dir}"
install -m 0755 "${repo_root}/src-tauri/target/release/sagewatch-plasma-provider" "${helper_dir}/sagewatch-plasma-provider"
install -m 0755 "${repo_root}/src-tauri/target/release/sagewatch-plasma-provider" "${helper_dir}/sagewatch-plasma-refresh"
if kpackagetool6 --type Plasma/Applet --show "${widget_id}" >/dev/null 2>&1; then
  kpackagetool6 --type Plasma/Applet --upgrade "${package_dir}"
else
  kpackagetool6 --type Plasma/Applet --install "${package_dir}"
fi
echo "Installed Sagewatch. Open Plasma Edit Mode and choose Add Widgets > Sagewatch."
