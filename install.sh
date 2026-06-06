#!/usr/bin/env bash
# SnapStation one-command installer for Ubuntu.
#
#   curl -fsSL https://your-domain/install.sh | sudo bash
#
# Installs system libraries, the matching prebuilt binary for this machine's
# architecture, a dedicated service user and a systemd service, then reports
# detected hardware. No Rust toolchain is required on the target.
set -euo pipefail

REPO="${SNAP_REPO:-your-org/snapstation}"
VERSION="${SNAP_VERSION:-latest}"
BIN_DEST="/usr/local/bin/snapstation"
SERVICE_DEST="/etc/systemd/system/snapstation.service"
SERVICE_USER="snapstation"

log()  { printf '\033[1;36m[snapstation]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[snapstation]\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m[snapstation]\033[0m %s\n' "$*" >&2; exit 1; }

[ "$(id -u)" -eq 0 ] || die "Bitte mit sudo/root ausführen."
command -v apt-get >/dev/null || die "Dieser Installer ist für Ubuntu/Debian (apt)."

# --- 1. Map architecture to a release asset target triple --------------------
deb_arch="$(dpkg --print-architecture)"
case "$deb_arch" in
  amd64) target="x86_64-unknown-linux-gnu" ;;
  arm64) target="aarch64-unknown-linux-gnu" ;;
  armhf) target="armv7-unknown-linux-gnueabihf" ;;
  i386)  target="i686-unknown-linux-gnu" ;;
  *) die "Nicht unterstützte Architektur: $deb_arch" ;;
esac
log "Architektur: $deb_arch -> $target"

# --- 2. System libraries -----------------------------------------------------
# libgphoto2 (DSLR), CUPS + Gutenprint (dye-sub printing). The chroma-key core
# is pure Rust and needs no extra libraries on any architecture.
log "Installiere System-Bibliotheken ..."
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y --no-install-recommends \
  libgphoto2-6 cups cups-client printer-driver-gutenprint ca-certificates curl \
  || apt-get install -y --no-install-recommends \
       libgphoto2t64 cups cups-client printer-driver-gutenprint ca-certificates curl

# --- 3. Service user ---------------------------------------------------------
if ! id "$SERVICE_USER" >/dev/null 2>&1; then
  log "Lege Dienst-Benutzer '$SERVICE_USER' an ..."
  useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER"
fi
usermod -aG plugdev,video,lp "$SERVICE_USER" || true

# --- 4. Download the prebuilt binary -----------------------------------------
if [ "$VERSION" = "latest" ]; then
  url="https://github.com/$REPO/releases/latest/download/snapstation-$target"
else
  url="https://github.com/$REPO/releases/download/$VERSION/snapstation-$target"
fi
log "Lade Binary: $url"
tmp="$(mktemp)"
curl -fSL "$url" -o "$tmp" || die "Download fehlgeschlagen. Ist ein Release für $target veröffentlicht?"
install -m 0755 "$tmp" "$BIN_DEST"
rm -f "$tmp"

# --- 5. systemd service ------------------------------------------------------
log "Installiere systemd-Dienst ..."
curl -fsSL "https://raw.githubusercontent.com/$REPO/main/packaging/snapstation.service" \
  -o "$SERVICE_DEST" 2>/dev/null || cp packaging/snapstation.service "$SERVICE_DEST"
systemctl daemon-reload
systemctl enable --now snapstation.service

# --- 6. Hardware status ------------------------------------------------------
log "Status:"
systemctl --no-pager --lines=0 status snapstation.service || true
if command -v gphoto2 >/dev/null; then
  gphoto2 --auto-detect || warn "Keine Kamera erkannt — USB & Netzteil prüfen."
fi
lpstat -p 2>/dev/null || warn "Kein Drucker in CUPS konfiguriert."

log "Fertig. Oberfläche im Browser: http://<IP-dieses-Rechners>/"
