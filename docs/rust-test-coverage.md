# Rust Test Coverage Findings

Stand: 2026-09-24

## Snapshot

- Rust test suite: 125 Tests, alle gruen
- Coverage command: `cd src-tauri && cargo llvm-cov --summary-only`
- Region coverage: 48.76%
- Function coverage: 39.92%
- Line coverage: 45.82%

## Was in dieser Runde dazu kam

Neue Unit-Tests wurden fuer diese Module hinzugefuegt:

- `src-tauri/src/commands/github.rs`
- `src-tauri/src/commands/graphics.rs`
- `src-tauri/src/commands/mangohud.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/winetricks.rs`
- `src-tauri/src/commands/shell_link.rs`
- `src-tauri/src/commands/icons.rs`
- `src-tauri/src/commands/performance.rs`
- `src-tauri/src/commands/steamgriddb.rs`
- `src-tauri/src/commands/games.rs`
- `src-tauri/src/tray.rs`
- `src-tauri/src/locale.rs`
- `src-tauri/src/http.rs`
- `src-tauri/src/commands/umu.rs`
- `src-tauri/src/commands/wine_tools.rs`
- `src-tauri/src/commands/steamgriddb.rs`

Die neuen Tests decken vor allem reine Parser-, String- und Konfigurations-Helfer ab. Das ist der guenstigste Weg, schnell mehr Sicherheit zu gewinnen, ohne Tauri- oder Desktop-Integration mocken zu muessen.

## Starke Bereiche

Diese Module sind bereits vergleichsweise gut abgedeckt:

| Modul | Regions | Functions | Lines |
| --- | ---: | ---: | ---: |
| `models.rs` | 91.38% | 91.67% | 93.66% |
| `commands/binary_vdf.rs` | 88.08% | 82.61% | 84.44% |
| `commands/logs.rs` | 71.76% | 46.15% | 74.32% |
| `commands/proton_options.rs` | 65.71% | 52.94% | 66.12% |
| `commands/umu.rs` | 62.30% | 53.33% | 59.59% |
| `commands/shell_link.rs` | 65.93% | 63.16% | 58.90% |
| `commands/umu_database.rs` | 63.35% | 59.46% | 58.41% |
| `commands/performance.rs` | 60.25% | 59.09% | 65.32% |

## Mittlere Bereiche

Diese Module sind brauchbar gestartet, aber noch nicht tief genug abgesichert:

| Modul | Regions | Functions | Lines |
| --- | ---: | ---: | ---: |
| `config.rs` | 58.78% | 30.00% | 55.87% |
| `commands/mangohud.rs` | 56.91% | 25.00% | 61.65% |
| `commands/icons.rs` | 51.95% | 50.00% | 50.51% |
| `commands/graphics.rs` | 50.83% | 25.00% | 48.28% |
| `commands/games.rs` | 49.70% | 44.50% | 44.88% |
| `commands/runner_downloads.rs` | 41.15% | 38.03% | 34.54% |
| `commands/steam.rs` | 39.15% | 22.08% | 31.94% |
| `commands/github.rs` | 39.29% | 40.00% | 32.50% |
| `commands/winetricks.rs` | 38.50% | 24.44% | 43.10% |
| `commands/steamgriddb.rs` | 35.97% | 28.07% | 36.76% |
| `tray.rs` | 36.32% | 38.46% | 36.26% |

## Schwache oder ungetestete Bereiche

Diese Module haben aktuell kaum oder gar keine Abdeckung:

| Modul | Regions | Functions | Lines |
| --- | ---: | ---: | ---: |
| `commands/wine_tools.rs` | 24.24% | 41.67% | 29.51% |
| `commands/locale.rs` | 0.00% | 0.00% | 0.00% |
| `error.rs` | 2.59% | 50.00% | 3.61% |
| `main.rs` | 0.00% | 0.00% | 0.00% |

Fast vollstaendig oder vollstaendig abgedeckt in dieser Runde:

| Modul | Regions | Functions | Lines |
| --- | ---: | ---: | ---: |
| `http.rs` | 100.00% | 100.00% | 100.00% |
| `locale.rs` | 90.48% | 100.00% | 92.19% |

`commands/games.rs` ist weiterhin ein wichtiger Sonderfall: trotz des Sprungs auf 49.70% Regions und 44.88% Lines bleibt das Modul sehr gross und fachlich zentral. Weitere gezielte Tests oder kleine Extraktionen lohnen sich dort weiterhin ueberproportional.

`commands/umu.rs` ist mit den neuen Helfer-Tests jetzt in einem deutlich besseren Bereich. Die billigen Gewinne dort sind weitgehend mitgenommen; weitere Spruenge wuerden eher ueber Download-/Installationspfade mit mehr Dateisystem- oder Netzwerkaufwand kommen.

`commands/wine_tools.rs` ist nicht mehr ungetestet. Der billige Teil dort war fast komplett auf die Tool-Auswahl konzentriert; weitere Abdeckung wuerde schnell in Prefix-Preparation und Prozess-Spawn-Pfade laufen und ist deshalb weniger attraktiv als weitere Arbeit in `games.rs` oder `steamgriddb.rs`.

`commands/steamgriddb.rs` ist mit den neuen Lookup-, Data-URL- und Cache-Tests deutlich vorangekommen. Die naechsten Spruenge dort wuerden eher ueber die Set/Get/Remove-Pfade oder API-Envelopes kommen, also ueber etwas schwerere State- und I/O-Wege.

`tray.rs` hat mit den zuletzt extrahierten Label-, Dialog-, ID-, GDBus- und Sortier-Helfern einen brauchbaren Sockel erreicht. Der naechste sinnvolle Schritt dort waere nur noch weitere kleine pure Entscheidungslogik; fuer groessere Spruenge sind andere Module aktuell ergiebiger.

## Priorisierte naechste Schritte

1. `src-tauri/src/commands/games.rs`
   Grund: grosses Kernmodul, inzwischen fast bei 50% Regions, aber weiterhin mit vielen unberuehrten Pfaden; hohes Risiko bei Regressionen und weiter der beste Hebel fuer die Gesamtquote.
2. `src-tauri/src/tray.rs`
   Grund: deutlich besser als zuvor, aber weiterhin UI-/Tauri-lastig. Weitere Gewinne sind moeglich, aber nicht mehr ganz so billig wie in `games.rs` oder `umu.rs`.
3. `src-tauri/src/commands/runners.rs`
   Grund: relativ klein, aber funktional noch schwach abgedeckt; koennte mit ein paar gezielten reinen Helper-Tests einen ordentlichen Quotensprung liefern.
4. `src-tauri/src/commands/locale.rs`
   Grund: 0%, klein, aber nur begrenzter Nutzen; kann man nebenbei mitnehmen.
5. `src-tauri/src/commands/steamgriddb.rs`
   Grund: nicht mehr im roten Bereich, aber weitere Zugewinne sind jetzt teurer und eher in stateful Pfaden versteckt.
6. `src-tauri/src/commands/wine_tools.rs`
   Grund: nicht mehr bei 0%, aber weitere Zugewinne waeren deutlich teurer als bei den verbleibenden reinen Logikmodulen.

## Praktische Interpretation

- Die Kernlogik fuer Modelle und einige Parser ist solide abgesichert.
- Die Gesamt-Coverage ist jetzt messbar, aber fuer ein Backend noch eher mittel.
- Die groessten Risiken liegen weniger bei den kleinen Datenmodellen als bei grossen System- und Integrationsmodulen.
- Der schnellste weitere Gewinn kommt ueber kleine Refactorings, die I/O von Entscheidungslogik trennen.

## Reproduktion

Coverage neu berechnen:

```bash
cd src-tauri
cargo llvm-cov --summary-only
```

Normale Tests ausfuehren:

```bash
cd src-tauri
cargo test
```