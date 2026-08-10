#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="$(node -p "require('${repo_root}/package.json').version")"
architecture="$(uname -m)"
case "${architecture}" in
  x86_64) asset_arch="amd64" ;;
  *) echo "Unsupported release architecture: ${architecture}" >&2; exit 1 ;;
esac

cargo build --release --manifest-path "${repo_root}/src-tauri/Cargo.toml" --bin sagewatch-plasma-provider

staging_root="$(mktemp -d)"
trap 'rm -rf "${staging_root}"' EXIT
bundle_name="Sagewatch_Plasma_${version}_${asset_arch}"
bundle_dir="${staging_root}/${bundle_name}"
mkdir -p "${bundle_dir}"
cp -R "${repo_root}/plasma-widget/package" "${bundle_dir}/package"
cp "${repo_root}/src-tauri/target/release/sagewatch-plasma-provider" "${bundle_dir}/sagewatch-plasma-provider"
cp "${repo_root}/plasma-widget/release/install.sh" "${bundle_dir}/install.sh"
cp "${repo_root}/plasma-widget/release/uninstall.sh" "${bundle_dir}/uninstall.sh"
chmod 0755 "${bundle_dir}/sagewatch-plasma-provider" "${bundle_dir}/install.sh" "${bundle_dir}/uninstall.sh"

mkdir -p "${repo_root}/dist-release"
tar -C "${staging_root}" -czf "${repo_root}/dist-release/${bundle_name}.tar.gz" "${bundle_name}"
echo "${repo_root}/dist-release/${bundle_name}.tar.gz"
