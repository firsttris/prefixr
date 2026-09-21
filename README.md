# Tauri + SvelteKit + TypeScript

This template should help get you started developing with Tauri, SvelteKit and TypeScript in Vite.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Rust/Tauri Setup

Tauri benötigt Rust. Installation über rustup (Linux/macOS):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Unter Ubuntu/Debian werden zusätzlich folgende Systemabhängigkeiten benötigt:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Installation prüfen:

```bash
rustc --version
cargo --version
```

Weitere Plattformen (Windows, macOS) siehe [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/).

## Troubleshooting

### AppImage stürzt ab ("EGL_BAD_PARAMETER") oder zeigt nur ein weißes Fenster

**Symptom:** Beim Start der AppImage kommt sofort

```
Could not create default EGL display: EGL_BAD_PARAMETER. Aborting...
```

oder das Fenster öffnet sich, bleibt aber komplett weiß/leer. Betroffen war ein System mit AMD-GPU auf einer sehr aktuellen Distro (Bazzite/Fedora 44, Mesa 26.2.2) – auf einem anderen PC mit NVIDIA trat es nicht auf.

**Ursache:** Die AppImage bündelt ihre eigene, zum Build-Zeitpunkt aktuelle WebKitGTK-Version (aus dem Ubuntu-24.04-Build-Container). Diese gebündelte Version ist inkompatibel mit sehr neuen Mesa-/Grafiktreiber-Versionen auf dem Zielsystem. Weder `WEBKIT_DISABLE_DMABUF_RENDERER=1` noch `WEBKIT_DISABLE_COMPOSITING_MODE=1`, `GDK_BACKEND=x11` oder `LIBGL_ALWAYS_SOFTWARE=1` beheben das zuverlässig – bestenfalls verhindern sie den harten Absturz, das Fenster bleibt dann aber weiß, weil das eigentliche Rendering trotzdem nicht funktioniert.

Bestätigt per Test: die ganz normal kompilierte Binary (`src-tauri/target/release/prefixr`, **nicht** aus der AppImage) linkt automatisch gegen das System-WebKitGTK des Hosts statt der gebündelten Version und rendert auf demselben Rechner völlig normal, ohne Absturz.

**Workaround zum Testen:** Statt der AppImage die Binary direkt bauen und starten:

```bash
bun run tauri build   # NICHT "cargo build --release" – das bettet die
                       # Frontend-Assets nicht ein, die App versucht dann
                       # vergeblich, sich mit dem (nicht laufenden) Dev-Server
                       # zu verbinden ("Could not connect to localhost")
./src-tauri/target/release/prefixr
```

**Richtiger Fix (noch offen):** Das WebKitGTK-/GTK-Bundling aus der AppImage ausschließen, sodass sie immer die System-Bibliotheken des Hosts nutzt, statt eine eigene, potenziell veraltete Kopie mitzubringen. Ein Test, bei dem nur die gebündelte `libwebkit2gtk-4.1.so.0` gegen die System-Version getauscht wurde, ist an einer weiteren ABI-Inkompatibilität mit dem ebenfalls gebündelten `glib` gescheitert – das Bundle muss also als Ganzes überarbeitet werden, nicht nur einzelne Bibliotheken austauschen.

**Nebenbeobachtung:** Beim Abstürzen ("Aborting...") bleibt der Prozess teilweise als Zombie hängen und muss manuell (`kill -9`) beendet werden, statt sauber zu terminieren.
