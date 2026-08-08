#!/usr/bin/env bats

# shellcheck disable=SC2154 # Bats provides BATS_TEST_DIRNAME and BATS_TEST_TMPDIR.

bats_require_minimum_version 1.5.0

readonly SCRIPT="$BATS_TEST_DIRNAME/../../skills/yeet/scripts/get-macos-version.sh"

setup() {
  export MOCK_BIN="$BATS_TEST_TMPDIR/bin"
  export LICENSE_FILE="$BATS_TEST_TMPDIR/OSXSoftwareLicense.rtf"
  mkdir -p "$MOCK_BIN"
  cat > "$MOCK_BIN/uname" <<'EOF'
#!/bin/sh
printf '%s\n' Darwin
EOF
  cat > "$MOCK_BIN/sw_vers" <<'EOF'
#!/bin/sh
printf '%s\n' 26.0
EOF
  chmod +x "$MOCK_BIN/uname" "$MOCK_BIN/sw_vers"
  cat > "$LICENSE_FILE" <<'EOF'
SOFTWARE LICENSE AGREEMENT FOR macOS Big Sur 26.0
EOF
}

@test "preserves spaces in a multi-word macOS marketing name" {
  run env PATH="$MOCK_BIN:/usr/bin:/bin" MACOS_LICENSE_FILE="$LICENSE_FILE" "$SCRIPT"

  [ "$status" -eq 0 ]
  [ "$output" = "macOS Big Sur v26.0" ]
}
