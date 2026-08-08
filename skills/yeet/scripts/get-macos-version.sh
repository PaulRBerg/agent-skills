#!/bin/bash
# Get macOS marketing name and version
if [ "$(uname -s 2>/dev/null)" != "Darwin" ]; then
  echo "unsupported platform"
  exit 0
fi

license_file=${MACOS_LICENSE_FILE:-/System/Library/CoreServices/Setup Assistant.app/Contents/Resources/en.lproj/OSXSoftwareLicense.rtf}
name=$(awk -F 'macOS ' '/SOFTWARE LICENSE AGREEMENT FOR macOS/ {
  value = $2
  sub(/\\.*$/, "", value)
  sub(/[0-9]+[.].*$/, "", value)
  gsub(/[{}]/, "", value)
  sub(/^[[:space:]]+/, "", value)
  sub(/[[:space:]]+$/, "", value)
  print value
  exit
}' "$license_file" 2>/dev/null)
version=$(sw_vers -productVersion 2>/dev/null)

if [ -z "$version" ]; then
  echo "macOS unknown"
  exit 0
fi

if [ -n "$name" ]; then
  echo "macOS ${name} v${version}"
else
  echo "macOS v${version}"
fi
