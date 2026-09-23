# Roadmap: fehlende Funktionen

Ideen, was Prefixr im Vergleich zu Lutris, Bottles, PortProton und Heroic noch fehlt, sortiert nach **Nutzen im Verhältnis zum Aufwand**.

**Aufwand:** S = ein paar Stunden · M = ein bis zwei Tage · L = mehrere Tage
**Nutzen:** hoch / mittel / niedrig

Bereits erledigt: Start von Proton über umu-launcher und Performance-Overrides pro Spiel (MangoHud an/aus, GameMode, vkBasalt, Gamescope).

## Übersicht

| # | Funktion | Nutzen | Aufwand | Vorbild |
|---|---|---|---|---|
| 1 | Proton-Optionen als Schalter pro Spiel ✓ | hoch | S–M | Bottles, Lutris |
| 2 | Spielzeit und „zuletzt gespielt“ | mittel | S | Lutris, Heroic |
| 3 | Runner aus Steam/Lutris/umu mitbenutzen | mittel | S | ProtonUp-Qt, Lutris |
| 4 | Kleine Bugs und Kanten (siehe unten) | mittel | S | – |
| 5 | GAMEID-Zuordnung für protonfixes ✓ | hoch | M | Lutris, Heroic |
| 6 | Export nach Steam (Nicht-Steam-Spiel) ✓ | hoch | M | Lutris, Bottles, Heroic |
| 7 | Prefix-Snapshots, Backup und Restore | hoch | M–L | Bottles, PortProton |
| 8 | Log-Analyse mit Lösungsvorschlägen | mittel | M | Bottles (ansatzweise) |
| 9 | Spiel ohne Netzwerk starten | mittel | M | Bottles, PortProton |
| 10 | Import aus Lutris, Bottles und Heroic | mittel | M | – |
| 11 | Savegame-Backup über Ludusavi | mittel | M | Heroic |
| 12 | Pre-/Post-Launch-Skripte, eigener Wrapper | niedrig | S | Lutris |
| 13 | DXVK/VKD3D-Version wählen (Wine-Runner) | niedrig | S | Bottles, Lutris |

---

## Quick Wins: hoher Nutzen, wenig Aufwand

### 1. Proton-Optionen als Schalter pro Spiel
**Nutzen: hoch · Aufwand: S–M** · **erledigt**

Global unter „Proton“ und pro Spiel im Spieldialog ([proton_options.rs](src-tauri/src/commands/proton_options.rs)). Die Schalter werden aus den `check_environment("…")`-Aufrufen im `proton`-Skript des gewählten Runners gelesen. Jeder Runner bietet also nur an, was er kennt. Jeder Schalter hat drei Zustände: nicht gesetzt, an (`1`) oder aus (`0`). Die Werte werden vor den eigenen Env-Vars des Spiels gesetzt, manuelle Einträge gewinnen also weiter.

**Ursprünglicher Plan:**

Seit dem Umstieg auf umu wirken alle `PROTON_*`-Variablen. Man muss sie also nur noch in der Oberfläche anbieten, statt dass Nutzer die Namen kennen und von Hand als Env-Var eintragen.

Kandidaten (geprüft im `proton`-Skript von GE-Proton11-7):
- `PROTON_ENABLE_HDR=1`: HDR
- `PROTON_ENABLE_WAYLAND=1`: nativer Wayland-Treiber
- `PROTON_USE_WINED3D=1`: OpenGL statt DXVK, für alte Spiele
- `PROTON_DLSS_UPGRADE`, `PROTON_FSR4_UPGRADE`, `PROTON_XESS_UPGRADE`: Upscaler im Spiel auf die neueste Version heben
- `PROTON_DISABLE_NVAPI` / `PROTON_FORCE_NVAPI`: NVAPI (DLSS/Reflex) ist standardmäßig an; zum Abschalten oder Erzwingen
- `PROTON_NO_NTSYNC` / `PROTON_NO_FSYNC`: Sync abschalten, bei Problemen
- `PROTON_LOCAL_SHADER_CACHE=1`: Shader-Cache pro Spiel

**Umsetzung:** Neuer optionaler Block in `GameOverrides` ([models.rs](src-tauri/src/models.rs)), am besten mit derselben Logik „`None` = nicht gesetzt“. Die Werte kommen in `run_game` in `env`, *bevor* die eigenen Env-Vars des Spiels angehängt werden, damit manuelle Einträge weiter gewinnen. Anfangs nur für Proton-Runner. Für Wine-Runner gibt es nur einen Teil davon, jeweils mit anderen Variablennamen.

**Achtung:** Die Namen ändern sich zwischen Proton-Versionen (z. B. gibt es `PROTON_ENABLE_NVAPI` in GE-Proton 11 nicht mehr). Also nur Schalter anbieten, die das Skript des gewählten Runners tatsächlich kennt, etwa per Suche nach `check_environment("<NAME>"` im `proton`-Skript.

### 2. Spielzeit und „zuletzt gespielt“
**Nutzen: mittel · Aufwand: S**

`run_game` wartet ohnehin auf das Ende des Prozesses ([games.rs:1119](src-tauri/src/commands/games.rs#L1119)). Startzeit merken, Dauer auf `Game.playtime_secs` aufaddieren und `last_played` setzen. Danach kann die Bibliothek nach „zuletzt gespielt“ sortieren, und die GameCard zeigt die Spielzeit.

### 3. Runner aus Steam/Lutris/umu mitbenutzen
**Nutzen: mittel · Aufwand: S**

[`scan_runners`](src-tauri/src/commands/runners.rs#L27) liest nur den eigenen `runners_dir`. Wer schon GE-Proton über Steam oder ProtonUp-Qt installiert hat, lädt es doppelt herunter. Zusätzlich lesend durchsuchen:
- `~/.local/share/Steam/compatibilitytools.d` (bzw. `~/.steam/root/compatibilitytools.d`)
- `~/.local/share/umu/compatibilitytools`
- `~/.local/share/lutris/runners/wine` (nur Wine)

Die IDs müssen eindeutig bleiben, z. B. mit Quell-Präfix. Beim Löschen nie fremde Ordner anfassen.

### 4. Kleine Bugs und Kanten
**Nutzen: mittel · Aufwand: S (je Punkt)**

**Bugs** · **erledigt**

- Prefix löschen fragt nach, nennt die betroffenen Spiele und bietet „Nur aus Prefixr entfernen“ an. Solange ein Spiel darin läuft, wird der Prefix nicht gelöscht.
- Ein Spiel kann nicht mehr doppelt gestartet werden: Die UI zeigt „Startet…“ direkt nach dem Klick, und `launch_game` lehnt einen zweiten Start ab (`LaunchingGames`).
- `WINEDLLOVERRIDES` und `LD_PRELOAD` aus den Env-Vars eines Spiels werden an unsere Werte angehängt, statt sie zu ersetzen.
- Manuelles Beenden erscheint nicht mehr als Startfehler.
- Ein Runner-Download, der schon vor dem Start scheitert, zeigt den Fehler an und lässt sich erneut starten.
- winetricks, wine-mono und DXVK/VKD3D prüfen den HTTP-Status und werden erst nach dem vollständigen Schreiben unter ihrem endgültigen Namen abgelegt.
- Runner und DXVK/VKD3D werden in einen eigenen temporären Ordner entpackt und dann verschoben. Erstdownloads von umu und dem DirectX-Cache laufen nacheinander.
- `config.json` wird atomar geschrieben. Ist sie trotzdem kaputt, zeigt Prefixr eine Meldung mit dem Pfad und lässt die Datei unverändert.

**Kanten**

- **Import eines Proton-Prefixes (`pfx/`):** `add_prefix` akzeptiert einen Ordner mit `pfx/` darin ([prefixes.rs:37](src-tauri/src/commands/prefixes.rs#L37)). Beim Start wird aber der Ordner selbst als `WINEPREFIX` genutzt. Ein Wine-Runner legt daneben einen neuen, leeren Prefix an. Auch `prepare_proton` prüft auf `drive_c` direkt im Ordner. Besser: beim Hinzufügen auf `pfx/` umbiegen.
- **Setup und Wine-Werkzeuge ohne Vorbereitung:** `run_installer` und `launch_wine_tool` laufen direkt über `prefix_command` und nicht über `prepare_wine`. Bei einem Wine-Runner mit frischem Prefix fehlt dann wine-mono. Wine zeigt seinen eigenen Mono-Dialog, den Prefixr sonst vermeidet, und DXVK ist auch nicht eingerichtet. Die Ausgabe des Setups geht außerdem nach `/dev/null` ([games.rs:373](src-tauri/src/commands/games.rs#L373)), bei einem fehlgeschlagenen Setup gibt es also kein Log.
- **Entfernte Spiele hinterlassen Reste:** `remove_game` löscht nur die MangoHud- und vkBasalt-Konfiguration ([games.rs:481](src-tauri/src/commands/games.rs#L481)). Zurück bleiben das Artwork im Cache, das Verknüpfungs-Icon, `logs/<id>/` und angelegte `.desktop`-Verknüpfungen. Die Verknüpfungen starten danach ein Spiel, das es nicht mehr gibt.
- **Logs wachsen unbegrenzt:** Jeder Start legt eine neue Datei unter `logs/<id>/` an ([games.rs:514](src-tauri/src/commands/games.rs#L514)), bei Winetricks ebenso. Die letzten N pro Spiel behalten.
- **winetricks wird nie aktualisiert:** Das Skript wird einmal von `master` geladen und dann für immer benutzt ([winetricks.rs:37](src-tauri/src/commands/winetricks.rs#L37)). Ältere Versionen scheitern, sobald Microsoft Download-URLs oder Prüfsummen ändert. Wie bei umu eine Aktualisierung anbieten oder das Skript nach einer bestimmten Zeit neu laden.
- **„Beenden“ im Tray ohne Rückfrage:** Das Menü beendet Prefixr sofort, auch wenn noch Spiele laufen ([tray.rs:150](src-tauri/src/tray.rs#L150)). Die Spiele laufen weiter, weil sie in einer eigenen Prozessgruppe laufen. Nach einem Neustart weiß Prefixr aber nicht mehr, dass sie laufen, und kann sie nicht mehr beenden.
- **Gleichnamige Spiele überschreiben ihre Verknüpfungen:** Der Dateiname der `.desktop`-Datei kommt aus dem bereinigten Spielnamen ([games.rs:1302](src-tauri/src/commands/games.rs#L1302)). Zwei Spiele mit gleichem Namen, etwa „X: Y“ und „X_ Y“, landen in derselben Datei. Besser: die Spiel-ID in den Dateinamen aufnehmen.
- **Startargumente ohne Anführungszeichen:** `launch_args` wird nur an Leerzeichen getrennt ([games.rs:1060](src-tauri/src/commands/games.rs#L1060)). Argumente mit Leerzeichen, z. B. Pfade, gehen deshalb nicht. Besser: shell-artig parsen (`shlex`).
- **umu-Updates anzeigen:** Die Einstellungen zeigen nur die installierte Version. Eine Abfrage „neuere Version verfügbar“ fehlt, die neueste Version gibt es über `releases/latest`.
- **Runner-Verwendung anzeigen:** „Von N Spielen verwendet“ in der Runner-Liste. Unbenutzte Runner löschen können.

---

## Große Brocken: hoher Nutzen, mehr Aufwand

### 5. GAMEID-Zuordnung für protonfixes
**Nutzen: hoch · Aufwand: M** · **erledigt**

Neue Felder `Game.umu_id` und `Game.umu_store`, beim Start als `GAMEID` und `STORE` gesetzt (nur Proton-Runner). Eine reine Steam-App-ID wird zu `umu-<id>`. Im Spieldialog schlägt [umu_database.rs](src-tauri/src/commands/umu_database.rs) passende IDs vor. Die Vorschläge kommen aus der umu-database (die komplette Tabelle, einen Tag lang lokal zwischengespeichert) und aus der Steam-App-ID, die SteamGridDB kennt.

**Ursprünglicher Plan:**

Ohne `GAMEID` nutzt umu `umu-default`, dann greifen nur die allgemeinen protonfixes. Die eigentlichen Fixes pro Spiel werden über die umu-database zugeordnet (Titel/Store → UMU-ID, z. B. `umu-1091500`).

**Umsetzung:** Neues Feld `Game.umu_id: Option<String>`, beim Start als `GAMEID` (und ggf. `STORE`) setzen. Beim Hinzufügen eines Spiels in der umu-database nach dem Namen suchen und Treffer vorschlagen, ähnlich wie bei der SteamGridDB-Suche. Dabei auch die SteamGridDB-Zuordnung nutzen: Kennt SteamGridDB die Steam-App-ID, ergibt sich die UMU-ID direkt daraus.

**Vorher klären:** Wie die umu-database abgefragt wird (CSV im GitHub-Repo oder eine API) und welches Format sie hat.

### 6. Export nach Steam
**Nutzen: hoch · Aufwand: M** · **erledigt**

Pro Spiel über „Zu Steam hinzufügen“ ([steam.rs](src-tauri/src/commands/steam.rs)). Die Funktion trägt das Spiel in die `shortcuts.vdf` des zuletzt angemeldeten Kontos ein. Mit nach Steam geht genau das Artwork, das im Artwork-Dialog ausgewählt ist: Cover, Icon und die Steam-Bilder breites Cover, Hero und Logo. Für Bildarten ohne Auswahl behält Steam seine eigenen Bilder. Die App-ID leitet sich aus der Spiel-ID ab und bleibt deshalb auch nach einer Umbenennung gleich. „In Steam aktualisieren“ und „Aus Steam entfernen“ gibt es ebenfalls, und wer ein Spiel aus der Bibliothek entfernt, entfernt es auch aus Steam. Läuft Steam, fragt Prefixr nach und startet Steam für die Änderung neu.

Steam startet `prefixr --run <game-id>`: ohne Fenster, ohne Tray und neben einer offenen Prefixr-Instanz. Der Prozess beendet sich mit dem Spiel, sodass Steam das Spielende sieht. Desktop-Verknüpfungen (`--launch`) reichen den Start jetzt an eine laufende Instanz weiter.

**Offen:**
- Ein über Steam gestartetes Spiel erscheint in einer offenen Prefixr-Oberfläche nicht als „läuft“.
- Steam als Flatpak wird nicht unterstützt, weil es Prefixr außerhalb seiner Sandbox nicht starten kann.

### 7. Prefix-Snapshots, Backup und Restore
**Nutzen: hoch · Aufwand: M–L**

Vor einem Winetricks-Lauf, einem Runner-Wechsel oder einem Spiel-Update einen Snapshot anlegen und mit einem Klick zurückspringen können. Das schützt genau vor den Momenten, in denen ein Prefix typischerweise kaputtgeht.

**Umsetzung, erste Stufe:** Prefix ohne `drive_c/users/*/AppData/Local/Temp` und `shadercache` als `tar.zst` sichern (`tar` und `zstd` sind schon Abhängigkeiten). Auflisten, wiederherstellen, löschen. Optional automatisch vor Winetricks.
**Zweite Stufe:** Unterschiede statt Vollkopien, Bottles speichert z. B. nur geänderte Dateien. Auf Btrfs (Bazzite) wären Reflink-Kopien fast kostenlos.

---

## Nice to have

### 8. Log-Analyse mit Lösungsvorschlägen
**Nutzen: mittel · Aufwand: M**

Bei einem Startfehler das Log nach bekannten Mustern durchsuchen und konkrete Hilfe anbieten:
- `err:module:import_dll` mit `vcruntime`/`msvcp` → Winetricks-Verb `vcrun2022`
- fehlendes .NET oder `mscoree` → `dotnet48`
- `VK_ERROR_…` oder `vkCreateInstance failed` → Hinweis auf Vulkan-Treiber, WineD3D-Schalter anbieten
- `EAC`/`BattlEye` → Hinweis auf Anti-Cheat

Die Muster kämen in eine erweiterbare Tabelle. Die Vorschläge verlinken direkt in den Winetricks-Dialog.

### 9. Spiel ohne Netzwerk starten
**Nutzen: mittel · Aufwand: M**

Ein Schalter pro Spiel, der Telemetrie, Launcher-Updates und „Online-Pflicht“-Prüfungen blockiert. Bei Wine-Runnern reicht vermutlich `unshare --user --map-current-user --net` als Wrapper. **Bei Proton/umu vorher testen**, ob sich das mit dem bwrap-Container von pressure-vessel verträgt (verschachtelte User-Namespaces). Daher Aufwand M statt S.

### 10. Import aus Lutris, Bottles und Heroic
**Nutzen: mittel · Aufwand: M**

Fremde Prefixe lassen sich schon hinzufügen. Beim Import würden aber auch Name, Exe, Runner und Env-Vars direkt übernommen, das senkt die Hürde zum Umstieg erheblich.
- Lutris: `~/.local/share/lutris/pga.db` (SQLite) und die Spiel-YAMLs
- Bottles: `bottle.yml` pro Bottle, Programme darin
- Heroic: `~/.config/heroic/GamesConfig/*.json`

### 11. Savegame-Backup über Ludusavi
**Nutzen: mittel · Aufwand: M**

Ist `ludusavi` installiert, vor und nach jedem Start `ludusavi backup --wine-prefix <prefix> "<Spielname>"` ausführen. Das macht einen Runner- oder Prefix-Wechsel angstfrei. Braucht eine saubere Namenszuordnung (Ludusavi-Manifest), daher M.

---

## Für Power-User

### 12. Pre-/Post-Launch-Skripte und eigener Wrapper
**Nutzen: niedrig · Aufwand: S**

Zwei Felder pro Spiel: ein Befehl davor und einer danach, plus ein beliebiger Wrapper wie bei Steams `… %command%`. Das passt direkt in die bestehende Wrapper-Kette in `run_game`.

### 13. DXVK/VKD3D-Version wählen (nur Wine-Runner)
**Nutzen: niedrig · Aufwand: S**

[`ensure_layer`](src-tauri/src/commands/graphics_layers.rs#L92) lädt immer `releases/latest`, und zwar nur beim ersten Mal. Danach gibt es weder Updates noch eine Wahl. Eine Liste der Releases wie bei den Runnern, mit fester Version pro Spiel, würde reichen. Seit Proton über umu läuft, betrifft das nur noch Wine-Runner, daher die niedrige Priorität.

---

## Bewusst nicht geplant

- **Store-Integrationen** (GOG, Epic, EA, Ubisoft, Battle.net wie bei Lutris/Heroic): sehr viel Wartung für APIs und Logins, und nicht der Kern von Prefixr.
- **Lutris-artige Installer-Skripte:** Ihr Wert steckt in der Community-Pflege, nicht im Code. protonfixes (Punkt 5) und die Log-Analyse (Punkt 8) bringen einen großen Teil des Nutzens mit einem Bruchteil des Aufwands.
- **Eigene Fix-Datenbank pro Exe** (wie PortProtons `.ppdb`): Das deckt protonfixes über umu inzwischen ab.
