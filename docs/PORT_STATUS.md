# Create → Horizon: Stand des Ports

✅ fertig und getestet · 🟡 teilweise · 🔜 mit Horizons heutiger API machbar (kommt als Nächstes) ·
⛔ wartet auf Horizon (Nummer = Punkt in [HORIZON_API_WUENSCHE.md](HORIZON_API_WUENSCHE.md))

Der **Inhalt** (alle 641 Blöcke, 99 Items, Modelle, Texturen, Sprachen, 1005 Vanilla-Rezepte,
Loot, Tags, Zinkerz, Partikel, Sounds) ist für alles unten schon im Content-Mod. Diese Liste
betrifft das **Verhalten**, also Creates Java-Code.

## Kinetik (`content/kinetics`, 36.000 Zeilen Java)

| Funktion | Stand | Anmerkung |
|---|---|---|
| Welle, Zahnrad, großes Zahnrad | ✅ | Übersetzungen, Zahnversatz, drehendes Rendering |
| Ummantelte Welle, ummantelte Zahnräder | ✅ | einseitige Wellenanschlüsse |
| Getriebe | ✅ | Richtung je nach Antriebsseite |
| Kupplung, Schaltgetriebe | ✅ | mit Redstone |
| Drehzahl- und Stressmesser | 🟡 | Welle dreht; Zeiger ⛔ 3 |
| Kreativmotor | 🟡 | 16 RPM; Drehzahl per Mausrad einstellen 🔜 |
| Handkurbel | 🟡 | dreht beim Benutzen; Griff-Animation ⛔ 3, rückwärts ⛔ 7 |
| Stress, Überlastung, Konflikte, 256-RPM-Grenze | ✅ | |
| Ventilgriffe | 🔜 | |
| Kettenantrieb, verstellbarer Kettenantrieb | 🔜 | |
| Vertikales Getriebe (Item) | 🔜 | |
| Drehzahlregler | 🔜 | braucht auch ⛔ 4 |
| Riemen | 🔜 | großes Teilprojekt; Items darauf mit `draw_item` |
| Wasserrad, großes Wasserrad | ⛔ 10 | Strömung um das Rad |
| Windmühle | ⛔ 1 | ist eine Contraption |
| Dampfmaschine, Kessel | ⛔ 4 | Tank und Lohenbrenner als Nachbarn |

## Verarbeitung (`content/processing`, `content/kinetics`)

| Funktion | Stand | Anmerkung |
|---|---|---|
| Mühlstein | 🔜 | Container + `create:milling` (47 Rezepte) |
| Brechräder | 🔜 | Item-Entities (API 42) + `create:crushing` |
| Ablage (Depot) | 🔜 | ein Slot, Item obendrauf gezeichnet |
| Mechanische Presse | 🔜 | Logik über `insert_item`/`extract_item`; Pressenkopf ⛔ 3 |
| Ventilator: Waschen, Räuchern, Brennen, Spuken | 🔜 | Item-Entities, Spieler schieben |
| Lohenbrenner | 🔜 | füttern; Hitze an Becken ⛔ 4 |
| Mixer + Becken | ⛔ 4 | mit Flüssigkeiten auch ⛔ 2 |
| Mechanischer Handwerker | 🔜 | Menüs; Weitergabe ⛔ 4 |
| Deployer, sequenzielle Montage | ⛔ 6 | |
| Mechanischer Bohrer, Säge (stehend) | ⛔ 5 | |

## Logistik (`content/logistics`, 30.000 Zeilen)

| Funktion | Stand | Anmerkung |
|---|---|---|
| Trichter, Rutschen, Smart-Rutsche | 🔜 | `insert_item`, `extract_item`, `move_items` |
| Mechanischer Arm | 🔜 | Rendering ⛔ 3 |
| Item-Tresor | 🔜 | Mehrblock-Container |
| Redstone-Link, Pulsverlängerer, Verzögerer, Latch | 🔜 | `block_signal` (API 38) |
| Tunnel | 🔜 | nach den Riemen |
| Anzeigeverbindung, Anzeigetafel, Nixie-Röhre | ⛔ 11 | Text in der Welt |
| Pakete, Frogport, Lagerverwaltung | ⛔ 1 | Paket-Entities |

## Flüssigkeiten (`content/fluids`, 11.500 Zeilen)

| Funktion | Stand | Anmerkung |
|---|---|---|
| Rohre, Pumpe, Tanks mit Wasser und Lava | 🔜 | Tank-Helfer (API 54) |
| Ausguss (Spout), Item-Abfluss | 🟡 | mit Vanilla-Flüssigkeiten 🔜, sonst ⛔ 2 |
| Schokolade, Honig, Tränke als Flüssigkeit | ⛔ 2 | |

## Contraptions und Züge (`content/contraptions`, `content/trains`, 57.000 Zeilen)

| Funktion | Stand |
|---|---|
| Kolben, Lager, Seilzug, Portalkran, Lore-Montage, Aufzug | ⛔ 1 |
| Kleber, Chassis | ⛔ 1 |
| Züge, Gleise, Signale, Fahrpläne | ⛔ 1, 15 |

## Ausrüstung (`content/equipment`, 17.000 Zeilen)

| Funktion | Stand | Anmerkung |
|---|---|---|
| Schraubenschlüssel: drehen | ✅ | 153 Blöcke, Regeln aus `IWrenchable` |
| Schraubenschlüssel: aufheben | ⛔ 7 | |
| Goggles-Anzeige | ⛔ 8, 9 | |
| Rucksack, Tauchausrüstung | 🟡 | Luftversorgung 🔜 (`living_breathe`), Anzeige ⛔ 8 |
| Kartoffelkanone | ⛔ 1 | |
| Extendo Grip, Sandpapier, Werkzeugkasten | 🔜 | |
| Zauberstab der Symmetrie, Handheld Worldshaper | 🔜 | |

## Deko und Rest

| Funktion | Stand | Anmerkung |
|---|---|---|
| Steinpaletten, Fenster, Treppen, Türen, Leitern, Gerüste | ✅ | über Vanilla-Vorlagen |
| Ausrichtung beim Platzieren | ✅ | 99 Blöcke |
| Verbundene Texturen, Copycat-Blöcke | ⛔ 13 | |
| Sitze | ⛔ 1 | |
| Schematic-Tisch, Schematikanone | 🔜 | `edit_blocks` |
| Ponder | ⛔ 14 | |
