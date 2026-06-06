<div align="center">

# 📸 SnapStation

**Die eigenständige Foto-Box für Events — Aufnahme, Live-Vorschau, Chroma-Key,
Druck und Teilen. Alles in einem Programm, komplett über den Browser bedienbar.**

[![CI](https://github.com/muckiseftle/FotoBox/actions/workflows/ci.yml/badge.svg)](https://github.com/muckiseftle/FotoBox/actions/workflows/ci.yml)
[![Release](https://github.com/muckiseftle/FotoBox/actions/workflows/release.yml/badge.svg)](https://github.com/muckiseftle/FotoBox/releases)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#lizenz)

</div>

---

SnapStation verwandelt einen Rechner mit Kamera und Fotodrucker in eine
bedienbare Foto-Station. Gäste lösen selbst aus, sehen ihr Bild sofort, drucken
es und laden es per QR-Code aufs Handy. Bedient wird **alles im Web-Interface** —
am Kiosk-Bildschirm genauso wie vom Handy aus.

## ✨ Funktionen

- **Aufnahme** – DSLR/Systemkamera (via gphoto2) oder Mock-Kamera; Countdown mit Ton, Auslösen per Touch/Tastatur/Hardware-Taster
- **Live-Vorschau** – flüssiger MJPEG-Stream; ein Kamera-Zugriff versorgt Kiosk **und** alle Handys gleichzeitig
- **Chroma-Key (Herzstück)** – hochwertiges Keying mit weichen Kanten & Despill, **1-Klick-Auto-Kalibrierung** und ehrlichem Licht-Feedback
- **Drucken** – Dye-Sub über CUPS, Auto-Druck, Druck-Limit, **klare Fehlermeldungen** („kein Papier", „Papierstau" …)
- **Teilen** – QR-Download + mobile Download-Seite, E-Mail-Versand (SMTP, mit Offline-Warteschlange)
- **Galerie & Diashow** – durchblätterbare Galerie, Bildschirmschoner-Diashow nach Leerlauf
- **Branding** – Primärfarbe, Logo, Event-Titel live anpassbar
- **Komfort** – Mehrsprachig (DE/EN), Backup/Export aller Fotos, PWA (auf dem Handy installierbar)
- **Admin** – passwortgeschütztes Dashboard mit Seitenmenü; Kamera-, Chroma-, Druck-, Design- & Komfort-Einstellungen

## 🚀 Schnellstart

1. Lade den Installer für dein System von der **[Releases-Seite](https://github.com/muckiseftle/FotoBox/releases)**:

   | System | Datei | Start |
   |--------|-------|-------|
   | **Windows** | `SnapStation-Setup.exe` (oder portables `.zip`) | Installieren bzw. `snapstation.exe` doppelklicken |
   | **macOS** | `SnapStation.dmg` | In „Programme" ziehen, öffnen |
   | **Linux** | `install.sh` (Station) oder portables `.tar.gz` | `sudo bash install.sh` bzw. `./snapstation` |

2. **Starten.** SnapStation öffnet automatisch den Browser auf `http://localhost:8080`.
3. **Ersteinrichtung:** Admin-Passwort, Event-Name und Sprache festlegen — fertig.

> Vom Handy aus erreichbar: Rechner und Handy im selben WLAN, dann `http://<IP-des-Rechners>:8080` öffnen (oder den QR-Code am Kiosk scannen).

## 🖥️ Bedienung

- **Kiosk** (`/`) – die Gäste-Vollbildansicht: auslösen, Countdown, Ergebnis mit Drucken & QR-Code.
- **Galerie** (`/#/gallery`) – alle Fotos, Großansicht, Download, QR.
- **Admin** – das **⚙-Zahnrad** oben rechts im Kiosk → Login → Dashboard mit linkem Menü
  (Kamera · Chroma-Key · Drucken · Design & Teilen · Komfort · Einrichtung).
- **Handy** – QR scannen → Foto herunterladen oder per E-Mail schicken.

## 🎛️ Hardware-Unterstützung

SnapStation läuft auf allen drei Systemen — der Funktionsumfang hängt aber von der Plattform ab:

| Plattform | Web-Interface, Chroma-Key, Galerie, Teilen, Branding | Echte DSLR (gphoto2) | Drucken (CUPS) |
|-----------|:--:|:--:|:--:|
| **Linux** (Ubuntu, Station) | ✅ | ✅ | ✅ |
| **macOS** | ✅ | ⏳ geplant | ✅ (CUPS) |
| **Windows** | ✅ | ❌ | ❌ |

> Auf Windows/macOS dient SnapStation zum **Ausprobieren, Einrichten und Vorführen** —
> mit einer **Mock-Kamera** (animiertes Testbild) und einem **Mock-Drucker**. Die
> vollwertige Foto-Box mit echter Spiegelreflexkamera und Dye-Sub-Drucker läuft auf
> **Ubuntu**. So lässt sich die ganze Oberfläche bequem am Schreibtisch vorbereiten.

## ⚙️ Konfiguration

Standardmäßig ist keine Konfiguration nötig — alles wird im Web-Interface eingestellt.
Für Sonderfälle gibt es Umgebungsvariablen:

| Variable | Standard | Zweck |
|----------|----------|-------|
| `SNAP_BIND` | `0.0.0.0:8080` | Adresse/Port des Webservers |
| `SNAP_DATA_DIR` | `./data` (Desktop) · `/var/lib/snapstation` (Linux-Dienst) | Datenbank + Fotos |
| `SNAP_NO_OPEN` | – | gesetzt = Browser **nicht** automatisch öffnen |
| `SNAP_CAMERA` | (auto) | `mock` erzwingt die Mock-Kamera |
| `SNAP_PRINTER` | (auto) | `mock` erzwingt den Mock-Drucker |

## 🔧 Aus dem Quellcode bauen

Voraussetzungen: **Rust** (stable) und **Node.js 20+**.

```bash
# 1. Frontend bauen (Output -> crates/snap-web/assets, wird ins Binary eingebettet)
cd frontend && npm install && npm run build && cd ..

# 2. App bauen & starten
cargo run -p snap-bin --release
# -> Browser öffnet http://localhost:8080
```

Tests & Linting:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## 🏗️ Architektur

Ein einzelner Rust-Prozess (Tokio + Axum). Ein **Kamera-Aktor** besitzt die Kamera
exklusiv, sodass Live-Vorschau und Auslösen ohne „Gerät belegt"-Konflikte ineinander
übergehen. Die Bildpipeline (Chroma-Key, Thumbnails) läuft auf einem Blocking-Pool.

```
frontend/            Vite + TypeScript + Tailwind + PWA  →  eingebettet via rust-embed
crates/
  snap-core/         Domänentypen, Config (in DB), SQLite/sqlx, Tokens
  snap-camera/       Kamera-Aktor: MockCamera (alle OS) + gphoto2 (Linux)
  snap-imaging/      Chroma-Key (Soft-Matte + Despill), Thumbnails, Komposition
  snap-print/        Druck: CUPS (Linux) + Mock
  snap-web/          Axum: REST + MJPEG-Live-View + Auth + eingebettete PWA
snap-bin/            Binary `snapstation`
```

## 🔒 Sicherheit

- **Admin-Authentifizierung:** Konfigurations- & Mutations-Endpunkte erfordern Login
  (Argon2-Passwort → HttpOnly/SameSite-Cookie). Gäste-Funktionen bleiben offen.
- **Foto-Privatsphäre:** nicht erratbare Share-Tokens statt hochzählbarer IDs.
- **Uploads** werden dekodiert & neu kodiert; SQL ist parametrisiert; Dateipfade kommen
  aus der DB, nicht aus URL-Eingaben.
- Für eine geschlossene Event-Station ausgelegt (LAN/Hotspot, HTTP). Bei exponierten
  Setups einen TLS-Reverse-Proxy davorsetzen.

## 📦 Releases

Bei jedem Versions-Tag (`v*`) baut GitHub Actions automatisch die Installer für
Windows, macOS und Linux und hängt sie an ein GitHub-Release.

```bash
git tag v0.1.0 && git push origin v0.1.0
```

## 📄 Lizenz

Wahlweise unter **MIT** ([LICENSE-MIT](LICENSE-MIT)) oder **Apache-2.0**
([LICENSE-APACHE](LICENSE-APACHE)).
