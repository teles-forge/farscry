

set -eu


FARSCRY_VERSION="${FARSCRY_VERSION:-0.1.0}"
FARSCRY_REPO="teles-forge/farscry"
FARSCRY_BASE="https://github.com/${FARSCRY_REPO}/releases/download/v${FARSCRY_VERSION}"

FARSCRY_PREFIX=""
while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) FARSCRY_PREFIX="$2"; shift 2 ;;
    *)        shift ;;
  esac
done

if [ -z "$FARSCRY_PREFIX" ]; then
  if echo ":${PATH}:" | grep -q ":${HOME}/.local/bin:"; then
    FARSCRY_PREFIX="${HOME}/.local/bin"
  elif [ -w /usr/local/bin ]; then
    FARSCRY_PREFIX="/usr/local/bin"
  else
    FARSCRY_PREFIX="${HOME}/.local/bin"
  fi
fi


detect_platform() {
  OS="$(uname -s)"
  ARCH="$(uname -m)"

  case "$OS" in
    Darwin)
      case "$ARCH" in
        arm64)  echo "farscry-aarch64-apple-darwin" ;;
        x86_64) echo "farscry-x86_64-apple-darwin" ;;
        *)      die "Unsupported macOS architecture: $ARCH" ;;
      esac
      ;;
    Linux)
      case "$ARCH" in
        x86_64) echo "farscry-x86_64-unknown-linux-gnu" ;;
        *)      die "Unsupported Linux architecture: $ARCH (x86_64 only in v${FARSCRY_VERSION})" ;;
      esac
      ;;
    *)
      die "Unsupported OS: $OS. farscry supports macOS and Linux. Use npm on Windows."
      ;;
  esac
}


die() {
  printf "\033[31m[farscry] ERROR: %s\033[0m\n" "$*" >&2
  exit 1
}

info() {
  printf "\033[32m[farscry]\033[0m %s\n" "$*"
}

warn() {
  printf "\033[33m[farscry] WARNING: %s\033[0m\n" "$*" >&2
}

need_cmd() {
  if ! command -v "$1" > /dev/null 2>&1; then
    die "Required command not found: $1"
  fi
}

sha256_file() {
  if command -v sha256sum > /dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  elif command -v shasum > /dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d' ' -f1
  else
    die "No SHA256 utility found. Install sha256sum or shasum."
  fi
}

download() {
  URL="$1"
  DEST="$2"
  if command -v curl > /dev/null 2>&1; then
    curl --fail --silent --show-error --location --output "$DEST" "$URL"
  elif command -v wget > /dev/null 2>&1; then
    wget --quiet --output-document="$DEST" "$URL"
  else
    die "Neither curl nor wget found. Install one to continue."
  fi
}


main() {
  info "Installing farscry v${FARSCRY_VERSION}…"

  ASSET="$(detect_platform)"
  ARCHIVE="${ASSET}.tar.gz"
  ARCHIVE_URL="${FARSCRY_BASE}/${ARCHIVE}"
  SHA256_URL="${FARSCRY_BASE}/${ASSET}.sha256"

  TMP_DIR="$(mktemp -d)"
  trap 'rm -rf "$TMP_DIR"' EXIT

  ARCHIVE_PATH="${TMP_DIR}/${ARCHIVE}"
  SHA256_PATH="${TMP_DIR}/${ASSET}.sha256"

  info "Downloading ${ARCHIVE_URL}"
  download "$ARCHIVE_URL" "$ARCHIVE_PATH" \
    || die "Download failed. Check your internet connection or visit:\n  https://github.com/${FARSCRY_REPO}/releases/v${FARSCRY_VERSION}"

  info "Downloading checksum…"
  download "$SHA256_URL" "$SHA256_PATH" \
    || die "Checksum download failed."

  EXPECTED_SHA256="$(cat "$SHA256_PATH" | tr '[:upper:]' '[:lower:]' | awk '{print $1}')"

  info "Extracting…"
  tar -xzf "$ARCHIVE_PATH" -C "$TMP_DIR"
  NESTED_DIR="${TMP_DIR}/${ASSET}"

  BINARY_PATH="${NESTED_DIR}/farscry"
  [ -f "$BINARY_PATH" ] || die "Binary not found inside archive: ${BINARY_PATH}"

  ACTUAL_SHA256="$(sha256_file "$BINARY_PATH")"

  if [ "$ACTUAL_SHA256" != "$EXPECTED_SHA256" ]; then
    rm -f "$BINARY_PATH"
    die "SHA256 MISMATCH - installation aborted.\n" \
        "  expected : ${EXPECTED_SHA256}\n" \
        "  actual   : ${ACTUAL_SHA256}\n" \
        "The binary has been removed. Please retry the installation.\n" \
        "If this persists, report it at https://github.com/${FARSCRY_REPO}/issues"
  fi

  info "SHA256 verified OK"

  mkdir -p "$FARSCRY_PREFIX"

  cp "$BINARY_PATH" "${FARSCRY_PREFIX}/farscry"
  chmod 755 "${FARSCRY_PREFIX}/farscry"

  for LIB in "${NESTED_DIR}/libonnxruntime"*; do
    [ -e "$LIB" ] || continue
    LIB_NAME="$(basename "$LIB")"
    cp "$LIB" "${FARSCRY_PREFIX}/${LIB_NAME}"
    chmod 755 "${FARSCRY_PREFIX}/${LIB_NAME}"
    info "Bundled ORT: ${LIB_NAME}"
  done

  info "OK farscry v${FARSCRY_VERSION} installed to ${FARSCRY_PREFIX}/farscry"

  if ! echo ":${PATH}:" | grep -q ":${FARSCRY_PREFIX}:"; then
    warn "${FARSCRY_PREFIX} is not on your PATH."
    warn "Add it with:"
    warn "  export PATH=\"${FARSCRY_PREFIX}:\$PATH\""
    warn "(Add this line to your ~/.bashrc, ~/.zshrc, or ~/.profile)"
  fi

  info ""
  info "Run: farscry setup"
  info "Docs: https://farscry.dev"
}

main "$@"
