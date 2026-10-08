# Code Review Prefixr

Stand: 2026-10-08, Commit `3cb1c99` (main)

## Kurzfassung

Prefixr ist ein überdurchschnittlich sauber gebautes Tauri-2/SvelteKit-Projekt. Die Architektur stimmt: kleine, einzeln verantwortliche Rust-Module, ein typisierter `AppError`, den das Frontend selbst übersetzt, konsequentes "Config-Mutex nur kurz halten, nie über ein `.await`", korrekt eingesetzte Svelte-5-Runes, und ein i18n-Test, der Rust, Svelte und TypeScript gleichzeitig gegen die Message-Dateien prüft. Die Wine/Proton-Domänenlogik (umu, `WINEPREFIX`/`PROTONPATH`, `steamuser`-Profil, `pfx/`-Erkennung, `shortcuts.vdf`, Non-Steam-AppID) habe ich durchgesehen und keine fachlichen Fehler gefunden.

Es gibt **keinen kritischen Fehler**. Die wichtigsten Punkte sind:

1. **Modal: Escape funktioniert nicht, sobald der Fokus im Dialog ist** (Frontend, hoch).
2. **`csp: null`** in der Produktionskonfiguration, kombiniert mit Backend-Commands, die ihre Eingaben nicht validieren (Sicherheit, mittel).
3. **umu-Update löscht die alte Installation, bevor die neue steht**; ein Fehler dazwischen lässt den Nutzer ohne umu zurück (mittel).
4. **DXVK/VKD3D werden ohne GitHub-Token geladen und danach nie aktualisiert** (mittel).
5. **Blockierende Arbeit (tar-Entpacken, `remove_dir_all`) läuft im Async-Runtime-Thread** auf dem Launch-Pfad (mittel).
6. Mehrere **Race Conditions bei schnellen Klicks** im Frontend (ArtworkPicker, Runner-Tabs) und **verschluckte Ladefehler** (mittel).
7. **Duplikate**: GameCard/GameListRow, drei SteamGridDB-Commands, drei Download-Pipelines (Qualität, mittel).

Werkzeuge: `bun run check` (svelte-check, 0 Fehler, 0 Warnungen) und `bun run test` laufen durch. `cargo clippy`/`cargo test` konnten in dieser Umgebung nicht laufen, weil die GTK/WebKit-Entwicklungspakete fehlen. Alle Rust-Befunde sind durch Lesen des Codes verifiziert, nicht durch Compiler-Läufe.

Legende: 🔴 hoch · 🟠 mittel · 🟡 niedrig · ⚪ Nit

---

## 1. Spielstart und Prozesskontrolle (`games.rs`, `lib.rs`, `runners.rs`)

Das ist das Herzstück und insgesamt sehr gut gemacht: Wrapper-Kette (gamescope → systemd-inhibit → powerprofilesctl → umu-run/wine) in sauberer Reihenfolge, `game_only_env` damit MangoHud/vkBasalt nicht doppelt in gamescope laden, `LD_PRELOAD`-Merge für das Steam-Overlay, `process_group(0)`, Lock-Datei pro Spiel über Prozessgrenzen hinweg, Kill über Prozessbaum plus `wineserver -k9` plus `pkill -f` mit korrekt escaptem Pattern. Das ist mehr Sorgfalt als in den meisten Launchern.

### 🟠 1.1 `--launch` einer zweiten Instanz kann verloren gehen
`src-tauri/src/lib.rs:97-100`

Beim Single-Instance-Handoff wird `--install <pfad>` sowohl in `PendingInstall` abgelegt **als auch** als Event gesendet. `--launch <id>` wird nur als Event gesendet. Startet der Nutzer die App über eine Desktop-Verknüpfung und klickt kurz darauf eine zweite, ist das Webview noch nicht bereit, der Listener in `+page.svelte` existiert noch nicht, und der Start geht verloren.

Vorschlag: analog zu `PendingInstall` auch in `PendingLaunch` schreiben, dann emittieren. Das Frontend liest ohnehin beides.

### 🟠 1.2 Vergiftete Mutexe blockieren die App dauerhaft
`games.rs`, `runners.rs`, `prefixes.rs` u. a., überall `lock().map_err(|_| "Configuration is locked")`

Ein Panic in einem Command, während der Config-Mutex gehalten wird, vergiftet ihn. Danach liefert **jeder** Command "Configuration is locked", bis die App neu gestartet wird, und die Meldung sagt dem Nutzer nichts. Da `panic = "abort"` im Release-Profil steht, tritt das dort praktisch nicht auf, aber im Dev-Build schon.

Vorschlag: einen Helper `fn config(state) -> MutexGuard` mit `lock().unwrap_or_else(PoisonError::into_inner)` oder `parking_lot::Mutex` (vergiftet nie). Ein Panic kann hier nichts Halbfertiges in der Config hinterlassen, weil `save_config` atomar schreibt.

### 🟠 1.3 Spiel entfernen prüft nicht, ob es gerade läuft
`games.rs:626-680` (`remove_game`)

`delete_prefix` und `delete_runner` lehnen ab, wenn ein Spiel läuft oder sie benutzt. `remove_game` löscht ein laufendes Spiel kommentarlos aus der Config und löscht dabei auch sein Log-Verzeichnis, in das der laufende Prozess noch schreibt. Der `RunningGames`-Eintrag bleibt, das Tray-Menü funktioniert noch, aber nach dem Exit landet `game-exited` für eine ID, die das Frontend nicht mehr kennt.

Vorschlag: `LaunchingGames` prüfen und `AppError::GameAlreadyRunning` (oder eine neue Variante) zurückgeben.

### 🟡 1.4 Log-Hinweis auf Deutsch, hart kodiert
`games.rs:1377-1385`

`"[prefixr] GameMode uebersprungen: ..."` steht fest im Code, während sonst alles über die Message-Dateien läuft. Entweder über `locale::text` lokalisieren oder einheitlich Englisch wie der Rest der Log-Ausgaben.

### 🟡 1.5 `steer_profile_to_steamuser` verlässt sich auf `$USER`
`games.rs:902`

Wine ermittelt den Profilnamen über `getpwuid`, nicht über `$USER`. In Containern, unter `sudo -u` oder bei gesetztem abweichendem `USER` zeigt der Symlink auf den falschen Namen und wineboot legt ein zweites Profil an. `libc::getpwuid_r` ist bereits als Abhängigkeit vorhanden (`libc`).

### 🟡 1.6 `competing_scheduler_active` startet bis zu fünf Prozesse nacheinander
`games.rs:59-73`

Bei jedem Start mit GameMode laufen bis zu fünf `systemctl is-active` hintereinander. Ein Aufruf `systemctl is-active <alle units>` plus Auswertung der Ausgabe reicht, oder `--quiet` mit allen Units und Exit-Code-Interpretation (0 nur wenn **alle** aktiv sind, deshalb Ausgabe parsen).

### 🟡 1.7 `find_runner` scannt bei jedem Aufruf das ganze Runner-Verzeichnis
`runners.rs:60-66`, aufgerufen in `run_game`, `run_installer`, `launch_wine_tool`, `delete_runner`

Für ein paar Runner harmlos. In `delete_runner` (`runners.rs:200-206`) passiert der Scan aber **innerhalb** des Config-Locks, sodass alle anderen Commands auf Platten-I/O warten. `runners_dir` und Spieleliste zuerst herausklonen, dann scannen, wie `list_runners` es schon tut.

### 🟡 1.8 `prefix_command` hat einen toten Wine-Zweig
`runners.rs:160-185`

Beide Aufrufer (`prepare_proton`, `install_winetricks_verbs` unter `uses_umu_winetricks`) erreichen nur den Proton-Arm. Den Wine-Arm samt Doku entfernen und `&Runner` durch den Proton-Pfad ersetzen, oder `direct_winetricks_command` darauf umstellen.

### ⚪ 1.9 Kleinkram
- `install_wine_mono` (`games.rs:841-848`) öffnet die Log-Datei selbst statt `log_stdio` zu benutzen.
- `Cargo.toml`: `description = "A Tauri App"`, `authors = ["you"]`, kein `rust-version`. `File::try_lock` braucht Rust ≥ 1.89, das sollte als `rust-version = "1.89"` festgehalten sein, sonst bricht ein älterer Toolchain-Build unverständlich ab.
- `process_descendants` nutzt `Vec::contains` in einer Schleife (O(n²)); bei Spielprozessbäumen irrelevant, `HashSet` wäre trotzdem die natürliche Wahl.

---

## 2. Downloads und Caches (`runner_downloads.rs`, `umu.rs`, `graphics_layers.rs`, `winetricks.rs`)

Drei Module implementieren jeweils "holen → prüfen → entpacken → einswappen" mit unterschiedlicher Sorgfalt. `runner_downloads.rs` ist die Referenz (Stream auf Platte, Checksumme vor dem Entpacken, Scratch-Verzeichnis, atomares Rename, `spawn_blocking`). Die anderen beiden sollten auf dasselbe Niveau gezogen oder auf eine gemeinsame Pipeline umgestellt werden.

### 🟠 2.1 umu-Update: alte Version wird vor dem Swap gelöscht
`umu.rs:329-334`

```rust
if dir.exists() { fs::remove_dir_all(&dir)?; }
fs::rename(&staging, &dir)?;
```

Schlägt das `rename` fehl oder stirbt der Prozess dazwischen, gibt es kein umu mehr; jeder Proton-Start scheitert, bis ein erneuter Download klappt. Außerdem ist `install_umu` nicht gegen laufende Starts koordiniert: `ensure_umu` gibt einen Pfad zurück, `install_umu` aus den Einstellungen kann ihn entfernen, bevor `spawn` läuft.

Vorschlag: `rename(dir, dir.old)` → `rename(staging, dir)` → `remove_dir_all(dir.old)`, bei Fehler im zweiten Schritt zurückbenennen. Gegen den Race den `INSTALL_LOCK` auch in `ensure_umu` bis nach dem Spawn halten oder die `LaunchingGames` prüfen.

### 🟠 2.2 DXVK/VKD3D: kein Token, kein Update-Pfad
`graphics_layers.rs:122-135, 345-385`

`ensure_directx_layer_cache` bekommt kein Token; beide `releases/latest`-Aufrufe sind anonym und zählen gegen das 60/h-Limit, das der Nutzer mit dem Token gerade umgehen wollte. `ensure_layer` kehrt bei `target.is_dir()` sofort zurück und speichert keine Version. Was beim ersten Wine-Start "latest" war, bleibt für immer, ohne dass die UI es anzeigen oder erneuern kann.

Vorschlag: `token: Option<&str>` durchreichen und `with_optional_auth` aus `runner_downloads` wiederverwenden; eine `version`-Datei neben das Verzeichnis legen wie `umu.rs` es tut; ein "Layer aktualisieren"-Command anbieten.

### 🟠 2.3 Blockierende Arbeit im Async-Kontext
`graphics_layers.rs:387-427` (`extract_archive`, `move_extracted_dir`), `umu.rs:320-335` (`unpack`, `remove_dir_all`)

Gzip/Zstd-Dekodierung von 10–20 MB und rekursives Löschen laufen direkt im Tokio-Worker, mitten im `launch_game`. Währenddessen stehen andere Commands. `runner_downloads.rs:553` macht es mit `spawn_blocking` richtig.

### 🟡 2.4 winetricks von `master` ohne Inhaltsprüfung
`winetricks.rs:71-101`

Ein bewegliches Branch-Ziel wird direkt ausführbar gemacht. `error_for_status` fängt Fehlerseiten ab, aber keine 200-Antwort mit falschem Inhalt (Captive Portal). Mindestens prüfen, dass die Datei mit `#!/bin/sh` beginnt und `WINETRICKS_VERSION=` enthält, besser einen Release-Tag pinnen.

### 🟡 2.5 Prüfsummen-Fehlermeldung übertreibt
`runner_downloads.rs:337-391`

Archiv und `.sha512sum` kommen aus demselben Release-Pfad; das prüft Integrität, nicht Authentizität. "manipuliert" sollte "beschädigt oder unvollständig" heißen. Dasselbe gilt für GitHubs `digest`-Feld in `umu.rs`.

### 🟡 2.6 `runner-download-error`-Event ist redundant und einmal vergessen
`runner_downloads.rs:463-594`

Das Frontend setzt den Fehlerzustand bereits aus dem abgelehnten `invoke` (`stores/runners.ts:112-117`). Das Event wird fünfmal dupliziert, und der `JoinError`-Pfad (Zeile 569) sendet es nicht und löscht das Archiv nicht. Entweder Event streichen (≈40 Zeilen weniger) oder alle Fehler durch eine `fail()`-Closure leiten.

### 🟡 2.7 `latest_mono_release` nimmt lexikalische Sortierung an
`graphics_layers.rs:523-541`

`rfind` auf die Apache-Indexseite liefert bei Namenssortierung `9.4.0/` nach `10.0.0/`. Hrefs in `(u32,u32,u32)` parsen und `max()` nehmen. Nur Fallback-Pfad, daher niedrig.

### ⚪ 2.8 `tag` ist nicht an `download_url` gebunden
`runner_downloads.rs:398-410`. Beide kommen vom Frontend; Tag aus dem `/download/<tag>/`-Segment der URL ableiten statt einem zweiten Parameter zu vertrauen.

---

## 3. Sicherheit und Eingabevalidierung

Das Bedrohungsmodell ist "lokale Desktop-App mit eigenem Frontend". Nichts davon ist heute ausnutzbar. Aber `csp: null` entfernt die Schicht, die einen Frontend-Fehler (z. B. ein per `{@html}` gerendeter Spielname aus SteamGridDB) daran hindern würde, zu den Commands durchzureichen. Die beiden Dinge gehören zusammen betrachtet.

### 🔴 3.1 `csp: null` in der Produktion
`src-tauri/tauri.conf.json:27`

Das Template-Default ist nie ersetzt worden. Das Webview lädt Remote-Bilder (SteamGridDB), `data:`-Icons und `asset:`-Cover. Tauri fügt seine IPC-Nonces selbst ein, sobald ein CSP-String gesetzt ist. Startpunkt:

```json
"csp": "default-src 'self'; img-src 'self' asset: http://asset.localhost data: https://*.steamgriddb.com; style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost"
```

Die genauen CDN-Hosts aus den `thumb`/`url`-Feldern der API verifizieren.

### 🟠 3.2 `prefix_path` und `verbs` werden ungeprüft verwendet
`winetricks.rs:199-261`, `wine_tools.rs:314-353`

`install_winetricks_verbs` und `launch_wine_tool` nehmen jeden `prefix_path`-String, legen dort `drive_c/users/steamuser` an, setzen Symlinks, starten wineboot und schreiben Logs. Nichts prüft gegen `config.prefixes`. `verbs` gehen direkt in `.args()`, ein Wert wie `--self-update` ist dann eine Option statt ein Verb.

Vorschlag: `prefix_path` gegen `config.prefixes`/`games[].prefix_path` auflösen, Verbs gegen `^[a-z0-9_-]+$` oder den geparsten Katalog prüfen.

### 🟡 3.3 `download_image` lädt beliebige URLs ohne Größenlimit
`steamgriddb.rs:391-408`

Host auf `steamgriddb.com` einschränken und `content_length()` deckeln.

### 🟡 3.4 Config-Werte ungeescaped in zeilenbasierte Configs
`mangohud.rs:382-384`, `graphics.rs:44`

Ein `position` oder `theme_color` mit Zeilenumbruch (per Hand editierte `config.json`) injiziert weitere MangoHud-/vkBasalt-Direktiven. Beim Speichern Steuerzeichen ablehnen.

### 🟡 3.5 Unbegrenzte Rekursion im VDF-Parser
`binary_vdf.rs:69-99`

Eine kaputte `shortcuts.vdf` aus `\x00\x00`-Paaren überläuft den Stack; mit `panic = "abort"` stirbt die App. Ein `depth`-Parameter mit Limit 32 kostet zwei Zeilen.

---

## 4. Frontend (Svelte 5 / TypeScript)

Alle 59 `invoke()`-Aufrufe, Argumentnamen (snake→camel), Event-Namen und Payload-Formen wurden gegen `generate_handler!` und die Rust-Signaturen geprüft: **keine Abweichung**. Runes sind korrekt eingesetzt, kein `any`, keine `$effect`-Schleifen, überall keyed `{#each}`.

### 🔴 4.1 Escape schließt Modals nicht, sobald der Fokus im Dialog ist
`src/lib/components/Modal.svelte:25, 37`

Der Escape-Handler hängt an `<svelte:window onkeydown>`, aber das Panel hat `onkeydown={(e) => e.stopPropagation()}`. Jedes Keydown aus einem Eingabefeld oder Button im Dialog wird gestoppt, bevor es `window` erreicht. Escape funktioniert nur, solange der Fokus noch außerhalb liegt. Das `stopPropagation` auf keydown hat keinen Zweck (das auf click schon, für den Backdrop).

Fix: die `onkeydown`-Zeile auf dem Panel entfernen.

### 🟠 4.2 Modal ohne Fokus-Management
`Modal.svelte:28-50`

`role="dialog" aria-modal="true"` ist gesetzt, aber der Fokus wandert nicht in den Dialog, es gibt keinen Focus-Trap (Tab läuft in die Sidebar dahinter), und der Fokus kehrt beim Schließen nicht zum Auslöser zurück. `tabindex="-1"` ist vorhanden, wird aber nicht benutzt. `aria-labelledby` auf die `<h2>` statt `aria-label` mit dem Titel.

### 🟠 4.3 Race im ArtworkPicker
`ArtworkPicker.svelte:99-123`

`loadAssets()` hat kein Abbruch-Token. Cover → Icon → Wide schnell geklickt: drei parallele Invokes, der letzte Rückläufer gewinnt `assetOptions`, auch wenn er zur falschen Art gehört. Der Nutzer wählt dann ein "Cover", das eine Icon-URL ist. Gleiches Muster zwischen `handleSearch` und `selectMatch`.

```ts
let req = 0;
async function loadAssets(id: number) {
  const my = ++req;
  const r = await ...;
  if (my !== req) return;
  assetOptions = r;
}
```

### 🟠 4.4 Race in den Runner-Download-Tabs
`RunnerDownloads.svelte:35-47`, `stores/runners.ts:32-41`

`refreshRunnerReleases(id)` setzt den Store unbedingt. Tab-Wechsel während ein langsamer GitHub-Request läuft: Tab B ist gewählt, Liste von A wird gezeigt. Store nach Quelle keyen oder vor dem Anzeigen `id === selectedSource` prüfen.

### 🟠 4.5 Install-Dialog während laufendem Installer schließbar
`InstallDialog.svelte:58-80`, `+page.svelte:212-220`

Backdrop-Klick oder Escape setzt `installingExePath = null`, der Dialog wird unmounted, der Installer läuft im Backend minutenlang weiter, und die erkannten Verknüpfungen kommen auf einer Komponente an, die es nicht mehr gibt. `onClose` während `busy` blockieren (`closable`-Prop) oder den Zustand in einen Store heben.

### 🟠 4.6 Ladefehler werden verschluckt
`GameList.svelte:76-80`, `GameForm.svelte:107-114`, `PrefixManager.svelte:45-48`, `RunnerList.svelte:44-60`, `+page.svelte:63-83`, `stores/runners.ts:85-102`

Fast jedes `onMount` ruft `refreshX()` ohne `.catch`. Schlägt `list_games` fehl (kaputte Config, Rechte), zeigt die Bibliothek den freundlichen "noch keine Spiele"-Leerzustand. In `+page.svelte` überspringt ein werfendes `take_pending_launch` das nachfolgende `takePendingInstall`. Pro Ansicht eine `loadError`-Fläche und "leer" von "fehlgeschlagen" trennen.

### 🟠 4.7 Exe-Icons weiterhin als Base64 über IPC
`types.ts:21`, `icons.rs:32`, `GameCard.svelte:102`, `GameListRow.svelte:100`

Cover wurden in PR #3 aus genau diesem Grund auf das Asset-Protokoll umgestellt; die Exe-Icons reiten aber noch in jedem `Game` mit, und `refreshGames()` läuft nach jedem Add/Update/Remove/Artwork-Wechsel. Icons liegen schon als Dateien vor (`exe_icon_path`); sie unter `$APPDATA/artwork` ablegen und wie `getGameCover` ausliefern, der Scope deckt das bereits ab.

### 🟠 4.8 GameForm lehnt stillschweigend ab
`GameForm.svelte:298-303`

Fehlender Name/Exe/Prefix/Runner wechselt nur zum Tab "Allgemein", ohne Meldung oder Markierung. `required` auf den Feldern oder `error` setzen und `aria-invalid`.

### 🟠 4.9 `role="menu"` ohne Tastaturverhalten
`GameCard.svelte:141-281`, `GameListRow.svelte:148-184`

Die Rolle verspricht Pfeiltasten, Escape und Fokusbewegung; nichts davon ist implementiert, und der unsichtbare Vollbild-Backdrop schluckt den ersten Klick woanders. Entweder das Muster umsetzen oder die Rollen weglassen und einfache Buttons nutzen (`<details>` oder Popover-API sparen Code).

### 🟠 4.10 Fokus-Indikator global entfernt
`src/app.css:141-144, 159-164`

`:focus { outline: none }` auf Eingabefeldern wird nur durch eine Rahmenfarbe ersetzt; normale `<button>`s haben gar keinen `:focus-visible`-Stil. Global `:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px }`.

### 🟠 4.11 GameCard und GameListRow duplizieren ≈100 Zeilen
`GameCard.svelte:39-92` vs `GameListRow.svelte:39-92`

Cover-Effekt, Shortcut-Zustandsmaschine, Menüpositionierung, Lösch-Bestätigung und 13 durchgereichte Callback-Props sind kopiert. Ein `useGameActions(game)`-Modul in `.svelte.ts` plus ein gemeinsames `GameCover.svelte`. Da alle Callbacks nur Store-Funktionen aufrufen, reichen `game`, `runState`, `inSteam`, `onEdit`, `onEditArtwork` als Props.

### 🟠 4.12 Handgepflegte TypeScript-Spiegel der Rust-Modelle
`src/lib/types.ts`, Payload-Interfaces in `stores/games.ts:46-71`, `stores/runners.ts:60-75`

Aktuell alles synchron, aber nichts erzwingt es; ein umbenanntes serde-Feld fällt erst zur Laufzeit als `undefined` auf. `ts-rs` (`#[derive(TS)]`) oder `tauri-specta` generieren lassen und in CI auf Diff prüfen.

### 🟠 4.13 Nur eine Testdatei im Frontend
`tests/i18n.test.ts`

Der i18n-Test ist ausgezeichnet, aber die kniffligste Logik ist ungetestet, obwohl sie pur ist: Override-Zusammenfalten in `GameForm` (`sameBlock`/`sameFields`), `backendError()`, `prettifyExeName`, `parseEnvVars`, der Run-State-Reducer in `stores/games.ts`. Mit vitest und gemocktem `@tauri-apps/api/core` günstig nachrüstbar.

### 🟡 4.14 Satzfragmente in den Message-Dateien
`prefixManager_explainerBefore/Drive/After`, `umuPicker_noneSetHintAfter: "."`, `winetricksInstaller_hintBefore/After`, `umuSettings_hintBefore/After`

Übersetzer können die Wortstellung nicht ändern. Eine Nachricht mit Platzhalter und im Component an einem Marker splitten, oder für die wenigen statischen Strings `{@html}` mit `<strong>` akzeptieren.

### 🟡 4.15 Manuelle Pluralisierung
`+page.svelte:157-158`, `PrefixManager.svelte:32-37`, `RunnerList.svelte:27-30`

`gameSingular/gamePlural` kodiert die 1-vs-viele-Regel im Code. Paraglide 2 unterstützt Plural-Varianten in der Message-Datei.

### 🟡 4.16 Timer und Listener überleben die Komponente
`GameList.svelte:89-97`, `GameCard.svelte:86-88`, `GameListRow.svelte:86-88`, `+page.svelte:63-70`, `stores/games.ts:73-120`, `stores/runners.ts:77-103`

`setTimeout`s werden nicht gecleart; `pending-*`-Listener werden pro Remount neu registriert; die `listen()`-Promises in `initRunnerDownloadEvents` werden nicht einmal gesammelt. Heute harmlos, bei HMR schon doppelte `launchGame`-Versuche. `UnlistenFn` zurückgeben und in `onDestroy`/`import.meta.hot.dispose` aufrufen.

### 🟡 4.17 Weitere Punkte
- Karten zeigen die rohe `runner_id` statt des Runner-Namens (`GameCard.svelte:288`, `GameListRow.svelte:120`); auch die Sortierung nach Runner sortiert dann nach etwas, das der Nutzer nicht sieht.
- Vier Kopien desselben Runner-`<select>` (`WinetricksInstaller`, `WineToolsLauncher`, `GameForm`, `InstallDialog`); beide Prefix-Werkzeuge starten mit leerem Runner, obwohl er aus `$games` oft eindeutig ist. `RunnerSelect.svelte` extrahieren und vorbelegen.
- Kein Formatter/Linter konfiguriert (sichtbar an `m.x( { … })`-Abständen in `GameList.svelte:105`). `prettier` + `prettier-plugin-svelte` + `eslint-plugin-svelte` mit `lint`-Script in CI.
- `app.html` hat `lang="de"` fest und `data-sveltekit-preload-data="hover"`, das in einer Ein-Routen-SPA nichts tut.
- `InfoIcon.svelte:25` registriert pro Instanz dauerhaft Window-Listener; nur bei `open` registrieren, wie `Modal` es tut.
- Locale→BCP-47-Mapping dreimal dupliziert; `Intl` nimmt `"de"`/`"en"` direkt.
- Tab-Leisten in `ArtworkPicker` und `RunnerDownloads` ohne `role="tablist"`, obwohl `GameForm` das Muster schon hat.

---

## 5. Weitere Backend-Module

### 🟠 5.1 SteamGridDB: Download vor Validierung, dreifach dupliziert
`steamgriddb.rs:440-469, 523-552, 601-629`

In `set_game_cover` passieren `download_image`, `remove_stale_asset_files` und `fs::write`, bevor `find_game` die ID prüft. Eine unbekannte ID erzeugt einen Netzwerk-Roundtrip und eine verwaiste Cache-Datei, die `remove_game_artwork_files` nie aufräumt. `set_game_icon` und `set_game_artwork` wiederholen das; die drei Bodies unterscheiden sich nur in Suffix, optionalem `ico_to_png` und den gesetzten Feldern. Ein Helper `store_asset(app, state, game_id, suffix, url, convert, apply)` der zuerst prüft, dann lädt.

### 🟠 5.2 Tests verändern Prozess-Umgebung ohne Wiederherstellung
`commands/umu/tests.rs:89-126`, `locale/tests.rs:670-697`

`remove_var("HOME")` / `remove_var("XDG_DATA_HOME")` werden nie zurückgesetzt, und `LANG`/`LC_*` werden unter einem **anderen** Lock mutiert. Cargo führt Tests parallel im selben Prozess aus; jeder Test, der `HOME`/`USER` liest, wird reihenfolgeabhängig. `set_var` ist in Edition 2024 aus genau diesem Grund `unsafe`. Außerdem räumen alle fs-Tests ihre `temp_dir()`-Verzeichnisse nur im Erfolgsfall auf.

Vorschlag: Umgebung als Parameter injizieren (`umu_local_dir_from(folders, xdg, home)`), `tempfile` als Dev-Dependency für RAII-Cleanup.

### 🟡 5.3 Synchrone Commands mit Datei-I/O auf dem UI-Thread
`proton_options.rs:222-240`, `steam.rs:423`, `prefixes.rs:37`

Nicht-`async` Commands laufen auf dem Main-Thread. `list_proton_options` liest das komplette `proton`-Skript (hunderte KB) und scannt das Runner-Verzeichnis bei jedem Öffnen der Einstellungen; `list_steam_games` parst `shortcuts.vdf`. `#[tauri::command(async)]` wie in `steamgriddb.rs:477`.

### 🟡 5.4 `max_map_count`: "ausreichend" ist erst das Maximum
`performance.rs:71`

`sufficient: current >= 2_147_483_642` markiert die 1 048 576, die Fedora, Arch und Ubuntu 24.04+ standardmäßig setzen, als unzureichend und drängt zu einem Sysctl-Drop-in, den die Nutzer nicht brauchen. `>= 1_048_576` als ausreichend, Steams Wert weiter als Empfehlung.

### 🟡 5.5 SteamGridDB-Fehlertext geht verloren
`steamgriddb.rs:144-149`

Die API liefert 401/404 **mit** JSON-Envelope (`{"success":false,"errors":["Invalid API key"]}`). Der frühe Return bei `!is_success()` zeigt "status 401 Unauthorized" statt der hilfreichen Meldung.

### 🟡 5.6 `gdbus` blockiert den Start bis zu 2 s
`tray.rs:411-431`

`tray_host_available` läuft synchron im Setup. Auf einem hängenden Session-Bus erscheint das Fenster entsprechend spät. `--timeout` auf 500 ms oder asynchron mit Default "verfügbar".

### 🟡 5.7 umu-Datenbank normalisiert bei jedem Tastendruck alle ≈1200 Titel
`umu_database.rs:417-425`

`normalize()` (drei Allokationen) pro Eintrag pro Query. Einmal in `load_database` vorberechnen.

### ⚪ 5.8 Kleinkram
- `http.rs:19-21`: `.build().unwrap_or_default()` würde stillschweigend User-Agent und Timeouts verlieren; GitHub antwortet dann mit 403 ohne Hinweis. `expect(...)` ist ehrlicher, der Builder kann mit diesen Optionen nicht scheitern.
- `shell_link.rs:148-166`: `resolve_windows_path` ist case-sensitiv, Wine nicht. Selten relevant, mindestens kommentieren.
- `locale.rs:33, 48`, `tray.rs:494`: Deutsch ist der harte Fallback ohne `LANG`. Für eine Linux-Desktop-App mit englischer UI ist Englisch der weniger überraschende Default.

---

## 6. Build, CI, Abhängigkeiten

- `release.yml:66` führt `bun run check`, aber nicht `bun run test` aus; `ci.yml` macht beides. Angleichen.
- `bump.yml:15`, `release.yml:116`: wiederverwendbare Workflows auf bewegliches `@v1` gepinnt. Auf SHA pinnen.
- `cargo clippy` läuft in CI nicht. `cargo clippy --all-targets -- -D warnings` und `cargo fmt --check` ergänzen; genauso `bun outdated`/`cargo audit` gelegentlich.
- `bun outdated`: nur Minor/Patch offen, Majors für `@sveltejs/kit` 3, `adapter-static` 4, `typescript` 7 nicht dringend.
- `Cargo.toml`: `rust-version` fehlt (siehe 1.9).

---

## 7. Was besonders gut ist

1. **Launch-Pipeline** mit korrekter Wrapper-Reihenfolge, getrenntem `game_only_env` für gamescope, `LD_PRELOAD`-Merge fürs Steam-Overlay, Prozessgruppen-Trennung und dreistufigem Kill (Baum, umu via `pkill -f` mit escaptem Pattern, `wineserver -k9`).
2. **Lock-Datei pro Spiel in `XDG_RUNTIME_DIR`**, sodass eine von Steam gestartete Headless-Instanz und die offene App sich nicht in die Quere kommen.
3. **Runner-Downloads**: Stream auf Platte neben dem Ziel, Checksumme vor dem Entpacken, Scratch-Verzeichnis, atomares Rename; `tar-rs` deckt Path-Traversal ab.
4. **`shortcuts.vdf`**: Backup + tmp + rename, case-insensitive Keys wie Steam, nur Prefixr-eigene Felder werden angefasst, Steam wird um das Schreiben herum sauber beendet.
5. **`AppError { code, …params }` + `backendError()`**: vollständig übersetzte Backend-Fehler, die auch nach einem Sprachwechsel korrekt bleiben; `every_app_error_has_a_text` hält Enum und Message-Dateien im Gleichschritt.
6. **`tests/i18n.test.ts`** prüft de/en-Paritäten, Platzhalter und "jeder Key benutzt / jeder benutzte Key vorhanden" über Svelte, TS und Rust hinweg.
7. **Atomares `save_config`** (tmp + fsync + rename) und ein Start, der eine unlesbare Config lieber verweigert als überschreibt.
8. **Kommentare erklären das Warum** (GE-Proton exit 1 nach `createprefix`, `config_info`-Marker, `distrobox-host-exec`-Erkennung über `/run/.containerenv`). Das ist selten und wertvoll.

---

## 8. Empfohlene Reihenfolge

| Prio | Aufwand | Punkt |
|---|---|---|
| 1 | 1 Zeile | 4.1 Modal-Escape (`onkeydown` entfernen) |
| 2 | klein | 3.1 CSP setzen |
| 3 | klein | 2.1 umu-Swap atomar machen |
| 4 | klein | 1.1 `--launch` auch in `PendingLaunch` ablegen |
| 5 | klein | 2.3 `spawn_blocking` für Entpacken/Löschen |
| 6 | klein | 4.3 / 4.4 Request-Counter gegen Races |
| 7 | klein | 4.5 Install-Dialog während `busy` nicht schließbar |
| 8 | klein | 5.4 `max_map_count`-Schwelle |
| 9 | mittel | 2.2 Token + Versionsdatei für DXVK/VKD3D |
| 10 | mittel | 3.2 `prefix_path`/`verbs` validieren |
| 11 | mittel | 4.6 Ladefehler sichtbar machen |
| 12 | mittel | 4.11 GameCard/GameListRow zusammenführen |
| 13 | mittel | 5.1 SteamGridDB-Commands zu einem Helper |
| 14 | mittel | 4.2 / 4.9 / 4.10 Fokus-Management und Tastatur |
| 15 | mittel | 1.2 Mutex-Poisoning-Helper |
| 16 | mittel | 5.2 Test-Umgebung injizieren, `tempfile` |
| 17 | größer | 4.12 Typen aus Rust generieren (`ts-rs`/`specta`) |
| 18 | größer | 4.13 Frontend-Unit-Tests |
| 19 | größer | 2.x Gemeinsame Download-Pipeline für umu/Layer |
