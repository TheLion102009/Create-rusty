# Create für Horizon

Ein Port der Create-Mod (NeoForge, Minecraft 1.21.1) auf [Horizon](https://playhorizon.dev),
den Rust-Nachbau von Minecraft 26.3. Java-Code läuft in Horizon nicht, deshalb besteht der Port
aus zwei Hälften:

| Teil | Ordner | Was er macht |
|---|---|---|
| **Konverter** | `converter/` | Liest deinen Create-Quellcode und schreibt daraus ein Horizon-**Content-Mod** (`create.hmod`): alle Blöcke, Items, Modelle, Texturen, Sprachdateien, Rezepte, Loot-Tabellen, Tags und das Zinkerz, aufgerüstet auf die Formate von 26.3. |
| **Native-Mod** | `native/create/` | Rust-Code, den der Horizon-Launcher ins Spiel kompiliert: Creates Kinetik (Wellen, Zahnräder, Motor, Handkurbel), drehendes Rendering und Platzierung. |

## Stand

**Funktioniert (getestet):**

- Konvertierung: 641 Blöcke, 99 Items, 1005 craftbare Rezepte (Werkbank, Steinsäge, Ofen,
  Schmiedetisch), 640 Loot-Tabellen, 183 Tags, Zinkerz-Weltgenerierung, zwei Kreativ-Tabs.
  Das `.hmod` besteht `cargo horizon-mod check`; alle Modell-, Rezept- und Loot-Verweise
  zeigen auf existierende Dateien und Items.
- Jeder Block bekommt eine passende Vanilla-Vorlage: Treppen, Stufen, Mauern, Türen, Leitern
  und Gerüste verhalten sich wie in Vanilla. Wellen und Zahnräder haben Creates Kollisionsformen
  und verdecken keine Nachbarflächen. Fehlende Blockzustände (z. B. `facing` einer Presse)
  werden deklariert.
- Native-Mod (kompiliert gegen Horizon-Nightly `f42ef42`, 33 Tests inkl. Hook-Tests über
  Horizons Dispatch): Drehzahl-Ausbreitung mit Creates Regeln (Welle 1:1, kleine Zahnräder
  gegenläufig, groß→klein ×2, große Zahnräder im rechten Winkel), Getriebe, Kupplung und
  Schaltgetriebe mit Redstone, ummantelte Wellen und Zahnräder, Drehzahl- und Stressmesser,
  Konflikte, 256-RPM-Grenze, Stress und Überlastung, Kreativmotor (16 RPM), Handkurbel (32 RPM,
  solange benutzt), drehendes Rendering mit ineinandergreifenden Zähnen, der Schraubenschlüssel
  (dreht die 153 Blöcke, die Create drehen lässt), Ausrichtung von 99 Blöcken beim Setzen.

**Wo der Port steht und worauf er wartet:** [docs/PORT_STATUS.md](docs/PORT_STATUS.md), jeder
Create-Bereich mit Status. **Was Horizon dafür noch braucht:** [docs/HORIZON_API_WUENSCHE.md](docs/HORIZON_API_WUENSCHE.md),
16 geprüfte API-Wünsche (plus eine Liste vorhandener, aber für Trait-Mods unerreichbarer Helfer) fürs Dev-Team, priorisiert und mit NeoForge-Gegenstück.

**Noch nicht im Spiel gesehen:** Ich konnte Horizon hier nicht starten, weil dafür die
vorbereiteten Spieldaten aus deinem Minecraft-Konto nötig sind. Die Prüfungen oben sind
statisch und per Compiler bzw. Tests. Den ersten echten Test im Spiel machst du.

**Fehlt (Java-Code, der noch portiert werden muss):** alle Maschinen außer Motor und Kurbel
(Presse, Mixer, Mühle, Brecher, Bohrer, Säge, Deployer …), Riemen, Getriebe, Kupplung,
Contraptions, Züge, Flüssigkeiten und Rohre, Logistik, Ponder, Goggles-Anzeige. Die
391 Rezepte von Creates eigenen Typen (`create:pressing`, `create:milling` …) liegen schon als
Daten im Mod. Horizon reicht sie über `recipes_of_type` an den Native-Mod weiter, sobald eine
Maschine sie verarbeitet.

## Benutzung

### 1. Content-Mod erzeugen

```sh
cargo run --release -- --create "C:/Users/the/Desktop/Create-mc1.21.1-dev" --out out --animated --native-table native/create/api/src/tables.rs
```

- `--animated`: Wellen und Zahnräder werden vom Native-Mod gezeichnet. Ohne Native-Mod wären
  sie unsichtbar, dann diesen Schalter weglassen.
- `--native-table`: schreibt Creates Stresswerte und die Platzierungsregeln für den Native-Mod.
- Ergebnis: `out/create/` (Mod-Ordner) und `out/create-report.md` (was übernommen wurde und
  was nicht).

Packen und prüfen mit Horizons Werkzeug:

```sh
cargo install --git https://github.com/horizon-rust/horizon cargo-horizon-mod
cd out/create
cargo horizon-mod build        # -> target/horizon-mod/create.hmod
cargo horizon-mod check target/horizon-mod/create.hmod
```

### 2. Im Spiel testen

1. Im Horizon-Launcher einmal **Play** drücken, damit das Spiel aus deinem Minecraft vorbereitet wird.
2. Entweder unter *Settings > Game arguments* `--dev-mod "C:/Users/the/RustroverProjects/untitled/out/create"`
   eintragen, oder `create.hmod` in den Ordner `horizon-mods` des Spiels kopieren und unter
   *Options > Mods* einschalten. Danach das Spiel neu starten.
3. Fehler stehen in `logs/latest.log`.

### 3. Native-Mod bauen

Der Native-Mod nutzt Horizons Trait-API und ist auf die Nightly vom 3. November gepinnt
(`f42ef42`). Deine installierte Nightly (`c9d474d`, 1. November) ist dafür zu alt:
Aktualisiere Horizon im Launcher auf `nightly-2026-11-03-f42ef42` oder neuer.

```sh
horizon-launcher native-build native/create --trust-native-code
```

Das erzeugt die Instanz **Native mods**. Dort brauchst du auch das Content-Mod, mit `--animated`
erzeugt. Der erste Build dauert 20 bis 30 Minuten.

Lokal prüfen, ohne Launcher (braucht einen Horizon-Klon auf demselben Commit):

```sh
git clone https://github.com/horizon-rust/horizon && git -C horizon checkout f42ef42ef6ceccb3c6a20bea2481d87949a51c81
HORIZON_SRC=$PWD/horizon native/check.sh          # cargo test
HORIZON_SRC=$PWD/horizon native/check.sh clippy
```

## Aufbau

```
converter/src/
  main.rs      Ablauf, Ressourcen einlesen, Report, Kreativ-Tabs, Native-Tabellen
  blocks.rs    Vorlagenwahl, deklarierte Zustände, Formen aus AllShapes.java
  items.rs     Item-Vorlagen, Stapelgrößen, Essen, Haltbarkeit
  java.rs      liest Material, Sounds, Kartenfarbe, Licht, Stress, Klasse aus AllBlocks/AllItems.java
  classes.rs   Creates Klassenhierarchie (welche Blöcke IWrenchable sind)
  data.rs      Rezepte (Bedingungen, 26.3-Zutaten), Loot, Tags, Weltgenerierung
  upgrade.rs   1.21.1 -> 26.3 Datenupgrade, übernommen aus Horizons horizon-port-pack
native/create/
  horizon-mixin.toml   Bundle-Manifest (Trait-Mod, gepinnt auf Horizon f42ef42)
  api/src/lib.rs       Horizon-Hooks: Block-Entities, Netzlösung, Redstone, Kurbel, Schraubenschlüssel,
                       Sync, Zeichnen, Platzierung
  api/src/kinetics.rs  Creates RotationPropagator als reiner, getesteter Löser
  api/src/parts.rs     welcher Create-Block welches Kinetik-Teil ist, Zeichenart, Schraubenschlüssel
  api/src/placement.rs Creates Platzierungsregeln
  api/src/tables.rs    generiert vom Konverter
```

## Nächste Schritte

1. Im Spiel testen und Fehler aus `logs/latest.log` beheben.
2. Die ersten Maschinen mit Rezepten: Mühlstein (`create:milling`), Ablage, Mechanische Presse
   (`create:pressing`), Brechräder, Ventilator-Verarbeitung.
3. Kettenantriebe, Ventilgriffe, einstellbarer Kreativmotor.
4. Logistik: Trichter, Rutschen, Redstone-Bauteile.
5. Alles mit ⛔ in [docs/PORT_STATUS.md](docs/PORT_STATUS.md), sobald Horizon die passende API hat.

## Lizenz

Der Code dieses Projekts ist MIT-lizenziert. Er enthält portierten Code aus Create (MIT) und
Horizon (MIT), siehe [THIRD_PARTY.md](THIRD_PARTY.md). Creates **Assets** (Texturen, Modelle,
Sounds) sind *All Rights Reserved*. Dieses Repository enthält keine davon. Der Konverter liest
sie aus deiner lokalen Create-Kopie in `out/`, und `out/` wird von git ignoriert. Ein
konvertiertes `create.hmod` daher nicht ohne Erlaubnis des Create-Teams veröffentlichen.
