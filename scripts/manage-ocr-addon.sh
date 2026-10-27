#!/bin/bash
# User-invoked offline OCR Add-on installer; no sudo, network, or .app mutation.
set -euo pipefail
umask 077

usage() {
  echo "Usage:" >&2
  echo "  $0 install <PicPaste.app> <signed-PicPasteOCR>" >&2
  echo "  $0 uninstall" >&2
  echo "  $0 status <PicPaste.app>" >&2
  exit 2
}
die() { echo "PicPaste OCR Add-on: $*" >&2; exit 1; }

[[ "$(uname -s)" == "Darwin" ]] || die "macOS 14+ is required"
[[ "${HOME:-}" == /* && "$HOME" != "/" ]] || die "HOME must be an absolute user directory"
ROOT="$HOME/Library/Application Support/PicPaste/AddOns/OCR"
TARGET="$ROOT/PicPasteOCR"
CAPABILITY='PICPASTE-OCR-ADDON/1 SHOC/1'

reject_links() {
  local component
  for component in \
    "$HOME" \
    "$HOME/Library" \
    "$HOME/Library/Application Support" \
    "$HOME/Library/Application Support/PicPaste" \
    "$HOME/Library/Application Support/PicPaste/AddOns" \
    "$ROOT" "$TARGET"; do
    [[ ! -L "$component" ]] || die "symlink is not allowed: $component"
  done
}

team_id() {
  local details line candidate
  details=$(/usr/bin/codesign --display --verbose=4 "$1" 2>&1) || return 1
  [[ "$details" == *"Authority=Developer ID Application: "* ]] || return 1
  while IFS= read -r line; do
    case "$line" in
      TeamIdentifier=*)
        candidate="${line#TeamIdentifier=}"
        if [[ "$candidate" =~ ^[A-Z0-9]{10}$ ]]; then
          printf '%s' "$candidate"
          return 0
        fi
        return 1
        ;;
    esac
  done <<< "$details"
  return 1
}

verify_host() {
  local app="$1"
  [[ -d "$app" && -f "$app/Contents/MacOS/PicPaste" ]] ||
    die "expected the installed PicPaste.app path"
  /usr/bin/codesign --verify --strict --deep "$app" >/dev/null 2>&1 ||
    die "host app code signature is invalid"
  HOST_TEAM=$(team_id "$app") ||
    die "host app requires a non-ad-hoc Developer ID Team"
}

verify_addon() {
  local candidate="$1" expected_team="$2" actual cap_file
  [[ -f "$candidate" && ! -L "$candidate" ]] || die "helper is not a regular file"
  /usr/bin/codesign --verify --strict "$candidate" >/dev/null 2>&1 ||
    die "OCR helper signature verification failed"
  actual=$(team_id "$candidate") ||
    die "OCR helper must have a non-ad-hoc Apple Team ID"
  [[ "$actual" == "$expected_team" ]] ||
    die "helper Team ID does not match the PicPaste app"
  cap_file=$(/usr/bin/mktemp "$ROOT/.ocr-capability.XXXXXX")
  if ! "$candidate" --addon-info > "$cap_file"; then
    /bin/rm -f "$cap_file"
    die "helper capability handshake failed"
  fi
  if ! /usr/bin/cmp -s "$cap_file" <(printf '%s\n' "$CAPABILITY"); then
    /bin/rm -f "$cap_file"
    die "helper Add-on API / SHOC version is incompatible"
  fi
  /bin/rm -f "$cap_file"
}

reject_links
case "${1:-}" in
  install)
    [[ $# -eq 3 ]] || usage
    app="$2"
    source="$3"
    [[ -f "$source" && ! -L "$source" ]] || die "source helper missing or a symlink"
    verify_host "$app"

    /bin/mkdir -p "$ROOT"
    /bin/chmod 700 "$ROOT"
    staged=$(/usr/bin/mktemp "$ROOT/.ocr-staged.XXXXXX")
    backup=""
    published=0
    committed=0
    cleanup() {
      if [[ "$committed" -ne 1 ]]; then
        if [[ "$published" -eq 1 ]]; then
          /bin/rm -f "$TARGET"
        fi
        if [[ -n "$backup" && -f "$backup" ]]; then
          /bin/mv "$backup" "$TARGET" ||
            echo "Restore previous Add-on manually from $backup" >&2
        fi
      fi
      if [[ -n "$staged" ]]; then
        /bin/rm -f "$staged"
      fi
    }
    trap cleanup EXIT
    /bin/cp "$source" "$staged"
    /bin/chmod 700 "$staged"
    verify_addon "$staged" "$HOST_TEAM"
    if [[ -e "$TARGET" ]]; then
      backup=$(/usr/bin/mktemp "$ROOT/.ocr-backup.XXXXXX")
      /bin/mv "$TARGET" "$backup"
    fi
    if ! /bin/mv "$staged" "$TARGET"; then
      die "could not publish OCR helper; previous version will be restored"
    fi
    staged=""
    published=1
    verify_addon "$TARGET" "$HOST_TEAM"
    committed=1
    if [[ -n "$backup" ]]; then
      /bin/rm -f "$backup"
    fi
    echo "OCR Add-on installed: $TARGET"
    ;;
  uninstall)
    [[ $# -eq 1 ]] || usage
    if [[ -e "$TARGET" ]]; then
      [[ -f "$TARGET" ]] || die "refusing to remove non-file Add-on target"
      /bin/rm -f "$TARGET"
      echo "OCR Add-on uninstalled"
    else
      echo "OCR Add-on is not installed"
    fi
    ;;
  status)
    [[ $# -eq 2 ]] || usage
    if [[ ! -e "$TARGET" ]]; then
      echo "OCR Add-on is not installed"
      exit 0
    fi
    verify_host "$2"
    verify_addon "$TARGET" "$HOST_TEAM"
    echo "OCR Add-on installed and trusted: $TARGET"
    ;;
  *)
    usage
    ;;
esac
