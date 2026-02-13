#!/bin/bash
# Get macOS marketing name and version
name=$(awk -F 'macOS ' '/SOFTWARE LICENSE AGREEMENT FOR macOS/{gsub(/[0-9]+\.*/, "", $2); gsub(/\\.*/, "", $2); print $2; exit}' "/System/Library/CoreServices/Setup Assistant.app/Contents/Resources/en.lproj/OSXSoftwareLicense.rtf" 2>/dev/null | tr -d ' ')
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
