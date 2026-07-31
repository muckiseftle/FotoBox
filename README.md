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
   | **macOS** | `SnapStation-macos.dmg` | DMG öffnen → Doppelklick auf `Installieren.command` (siehe Hinweis unten) |
   | **Linux** | `install.sh` (Station) oder portables `.tar.gz` | `sudo bash install.sh` bzw. `./snapstation` |

2. **Starten.** SnapStation öffnet automatisch den Browser auf `http://localhost:8080`.
3. **Ersteinrichtung:** Admin-Passwort, Event-Name und Sprache festlegen — fertig.

> Vom Handy aus erreichbar: Rechner und Handy im selben WLAN, dann `http://<IP-des-Rechners>:8080` öffnen (oder den QR-Code am Kiosk scannen).

> **macOS: „SnapStation ist beschädigt und kann nicht geöffnet werden"?**
> Die App ist **nicht** defekt. SnapStation ist ein Open-Source-Projekt ohne bezahltes
> Apple-Entwicklerzertifikat; macOS markiert nicht notarisierte Downloads pauschal als
> „beschädigt" (Gatekeeper-Quarantäne). Lösung: Im DMG einfach `Installieren.command`
> doppelklicken — das Skript kopiert die App nach „Programme" und entfernt die Markierung.
> Alternativ von Hand: App nach „Programme" ziehen und im Terminal einmalig
> `xattr -cr /Applications/SnapStation.app` ausführen, danach startet die App normal.

> **Safari: „Die Steuerung ist fehlgeschlagen … Modus ‚HTTPS-Only'" (WebKitErrorDomain:305)?**
> SnapStation läuft lokal über HTTP (`http://localhost:8080`). Safaris Modus
> „Nur HTTPS" / „Vor dem Verbinden über HTTP warnen" blockiert das in manchen
> Safari-Versionen komplett — ein [bekannter Safari-Fehler](https://bugs.webkit.org/show_bug.cgi?id=284559),
> den Apple inzwischen behoben hat. Bis das Update ankommt:
> **Safari → Einstellungen → Erweitert** → die Option „Vor dem Verbinden mit einer
> Website über HTTP warnen" (bzw. „Nur HTTPS") **deaktivieren** — oder SnapStation
> einfach in einem anderen Browser (Chrome, Firefox) unter `http://localhost:8080` öffnen.

## 🖥️ Bedienung

- **Kiosk** (`/`) – die Gäste-Vollbildansicht: auslösen, Countdown, Ergebnis mit Drucken & QR-Code.
- **Galerie** (`/#/gallery`) – alle Fotos, Großansicht, Download, QR.
- **Admin** – das **⚙-Zahnrad** oben rechts im Kiosk → Login → Dashboard mit linkem Menü
  (Kamera · Chroma-Key · Drucken · Design & Teilen · Komfort · Einrichtung).
- **Handy** – QR scannen → Foto herunterladen oder per E-Mail schicken.

## 🎛️ Hardware-Unterstützung

SnapStation ist auf **allen drei Systemen voll funktionsfähig** — echte Aufnahme über
eine **Webcam** (USB oder eingebaut) und echter **Druck** auf den Standard-/gewählten
Drucker:

| Plattform | Web/Chroma/Teilen/Branding | Webcam (Aufnahme + Live-View) | DSLR via gphoto2 | Drucken |
|-----------|:--:|:--:|:--:|:--:|
| **Linux** (Ubuntu) | ✅ | ✅ (v4l2) | ✅ | ✅ CUPS |
| **macOS** | ✅ | ✅ (AVFoundation) | ⏳ optional | ✅ CUPS |
| **Windows** | ✅ | ✅ (Media Foundation) | – | ✅ (Standarddrucker) |

- **Kamera-Auswahl** (auto): tethered **DSLR** (Linux) → **Webcam** (alle Systeme) → Mock.
  Erzwingbar per `SNAP_CAMERA=webcam|gphoto|mock`.
- **Druck:** Linux/macOS über **CUPS** (Dye-Sub-Treiber), Windows über den Windows-Druckpfad.
- **macOS:** beim ersten Start die **Kamera-Freigabe** bestätigen (TCC-Abfrage).
- Ohne angeschlossene Kamera/Drucker läuft alles weiter mit Mock-Backends (zum Einrichten).

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
