# API-Wünsche für Horizon (aus dem Create-Port)

Stand: 3. November 2026, geprüft gegen Horizon `main` `f42ef42` (Hook-API 359, Trait-Mods).

Jeder Punkt unten ist **geprüft**: Ich habe in der generierten Trait-API (`horizon_client::mod_api`,
`Hooks` und `Host`) und in `docs/native-mods.md` / `docs/mods.md` nachgesehen, dass es ihn noch
nicht gibt. Zu jedem Wunsch steht das NeoForge-Gegenstück, wie Horizon es in
`docs/native-hooks-neoforge.md` für neue Hooks vorschlägt.

Sortiert nach Priorität: wie viel von Create an dem Punkt hängt.

---

## Wunsch 0 (schnellster Gewinn): vorhandene Game-Helfer in die Trait-API

Viele Helfer gibt es schon als `Game::native_*`, aber `Host` hat sie nicht. Trait-Mods kommen
also nicht heran. Ein Abgleich von `crates/horizon-client/src/game/*.rs` gegen
`native-hooks.json` und den generierten `Host` findet 139 solche Funktionen, die meisten davon
intern. Für Create zählen diese:

| Helfer (gibt es schon) | Wofür in Create |
|---|---|
| `native_draw_model` (API 151) | alle bewegten Maschinenteile, siehe Wunsch 3 |
| `native_with_mod_block_entity` (API 18) | Maschinen, die zusammenarbeiten, siehe Wunsch 4 |
| `native_item_entities` (API 42) | Items, die auf Mühlstein, Brechrädern, Ablagen und Riemen landen; Ventilator-Verarbeitung; Trichter, die vom Boden saugen |
| `native_level_random` (API 43) | Zufallsergebnisse von Rezepten (Mühlstein, Brechräder) in Vanillas Reihenfolge |
| `native_play_sound`, `native_send_particles` | Geräusche und Partikel aus Block-Entity-Ticks (heute nur in Level-Hooks über `LevelWriteHost`) |
| `native_block_in_tag`, `native_item_in_tag` (API 28) | Creates Block- und Item-Tags (`create:fan_transparent`, `create:wrench_pickup` …) |
| `native_block_drops` | Drops beim Abbauen durch Maschinen, siehe Wunsch 5 |
| `native_set_component`, `native_remove_component` | Filter, Klemmbrett, Fahrpläne und Bauplan speichern ihren Inhalt als Item-Komponenten |
| `native_trigger_criterion` | Creates Fortschritte |

Das Skript, das die Liste erzeugt: `docs/tools/missing_helpers.js` in diesem Repo.

---

## Priorität 1: ohne das fehlen ganze Teile von Create

### 1. Eigene Entity-Typen mit eigenem Tick, Rendering und Kollision

- **Heute:** Ein Content-Mod kann Entity-Typen nur als Kopie von `minecraft:snowball` anlegen
  (`docs/mods.md`: „for now only `minecraft:snowball`“).
- **Create braucht es für:** Contraptions (Kolben, Lager, Seilzug, Portalkran, Lore-Konstruktionen,
  Aufzug, Windmühle). Das sind bewegte Blockgruppen als Entity. Außerdem Züge, Sitze,
  Pakete/Frogport, Kartoffelkanonen-Geschosse und Super-Glue-Entities.
  Das sind zusammen etwa 60.000 Zeilen Java (`contraptions/`, `trains/`).
- **Gewünscht:** Mod-Entity-Typen, deren Verhalten ein Native-Mod liefert:
  - Tick (Server), Position/Bewegung, gespeicherte Daten (NBT), Sync zum Client.
  - Zeichnen mit den vorhandenen Block-Entity-Zeichenhelfern (`draw_block`, `draw_cube`, `draw_item`)
    unter einer Entity-Pose.
  - Eigene Kollisionsboxen, auf denen Spieler stehen und gegen die sie laufen können (Contraptions
    tragen Spieler mit).
  - Mitfahren (`entity_mount` gibt es schon als Ereignis, fehlt aber für eigene Entities).
- **NeoForge:** `EntityType` registrieren, `EntityRenderer`, `Entity.tick`, `IEntityWithComplexSpawn`.

### 2. Eigene Flüssigkeiten

- **Heute:** Block-Entities können Tanks haben (`native_block_entity_fluids`, API 54), und
  `fluid_insert` / `fluid_extract` / `move_fluids` bewegen Flüssigkeiten. Das geht aber nur mit
  Vanilla-Flüssigkeiten. Ein Content-Mod kann keine Flüssigkeit deklarieren.
- **Create braucht es für:** Schokolade, Honig, Bauarbeiter-Tee, Tränke als Flüssigkeit (mit
  Komponenten), deren Quell- und Fließblöcke, Eimer, die Darstellung in Tanks, Rohren und Becken.
  Ohne das fallen auch die Rezepte mit diesen Flüssigkeiten weg (Abfüllen, Mischen, Leeren).
- **Gewünscht:** Data-driven Flüssigkeiten im Content-Mod (Texturen still/fließend, Farbe, Dichte,
  Viskosität, Temperatur, Eimer-Item) und Flüssigkeits-Stacks mit Komponenten in den Tank-Helfern.
- **NeoForge:** `FluidType`, `BaseFlowingFluid`, `FluidStack` mit Data Components.

### 3. `Host::draw_model` in der Trait-API (Teilmodelle zeichnen)

- **Heute:** `native_standalone_models` (Hook) und `Game::native_draw_model` gibt es (API 151),
  aber `Host` in der Trait-API hat kein `draw_model`. Trait-Mods können Teilmodelle also anmelden,
  aber nicht zeichnen.
- **Create braucht es für:** praktisch alle bewegten Maschinenteile, das sind rund 400 Teilmodelle
  in `AllPartialModels`: Pressenkopf, Mixer-Quirl, Zeiger von Drehzahl- und Stressmesser,
  Kurbelgriff, Ventilator-Rotor, Sägeblatt, Bohrkopf, Arm-Segmente, Wasserrad-Schaufeln, Riemen.
  Heute behelfe ich mir mit `draw_cube` für Wellenhälften. Das geht nur für einfache Quader.
- **Gewünscht:** `fn draw_model(&self, draw: &mut BlockEntityDraw, model: &str, tint: u32) -> bool`
  in `Host`, wie `native_draw_model`. Kleiner Aufwand, großer Nutzen.

### 4. Benachbarte Mod-Block-Entities lesen und ändern (Trait-API)

- **Heute:** `Game::native_with_mod_block_entity` gibt es (API 18), aber nicht in `Host`. Ein
  Trait-Mod kann im Tick eines Blocks die Daten eines anderen Mod-Blocks nicht lesen.
- **Create braucht es für:** Maschinen, die zusammenarbeiten: Presse über Becken oder Ablage,
  Mixer über Becken, Lohenbrenner unter Becken (Hitze), Mechanischer Arm zwischen Ablagen, Trichter
  an Riemen, Tunnel auf Riemen, Dampfmaschine an Tank (Kessel), Messgeräte am Netz.
- **Gewünscht:** `fn with_mod_block_entity(&mut self, pos, f: &mut dyn FnMut(&mut ModBlockEntity))`
  oder lesend `fn mod_block_entity(&self, pos) -> Option<ModBlockEntity>` in `Host`.

---

## Priorität 2: einzelne Maschinen und Werkzeuge

### 5. Blöcke von Maschinen abbauen lassen, mit Drops und Bruch-Fortschritt

- **Heute:** `set_block` ersetzt Blöcke, aber es gibt keinen Helfer „abbauen wie ein Spieler/Werkzeug“
  mit Loot-Drops, und keinen zum Setzen des Risse-Fortschritts (`block_break_progress_update` ist
  nur ein Ereignis).
- **Create braucht es für:** Mechanischer Bohrer, Mechanische Säge (Bäume fällen), Erntemaschine,
  Pflug, Deployer.
- **Gewünscht:** `fn destroy_block(&mut self, pos, drop: bool, tool: Option<&ItemStack>) -> Vec<ItemStack>`
  und `fn set_break_progress(&mut self, id: i32, pos, progress: i32)`.
- **NeoForge:** `Level.destroyBlock`, `Block.getDrops(state, level, pos, be, entity, tool)`,
  `Level.destroyBlockProgress`.

### 6. Item „als Spieler“ benutzen (Fake Player)

- **Create braucht es für:** den Deployer: Blöcke setzen, Items an Blöcken und Mobs benutzen,
  angreifen, ernten, Rezepte vom Typ `create:deploying` (167 Rezepte).
- **Gewünscht:** einen Server-Helfer, der ein Item an einer Position mit Blickrichtung benutzt
  (`use_item_on`, `use_item`, angreifen), als wäre es ein Spieler, ohne echten Spieler.
- **NeoForge:** `FakePlayer` / `FakePlayerFactory`.

### 7. Schleichen bei Item- und Block-Benutzung

- **Heute:** `PlacementContext.sneaking` gibt es beim Platzieren. `UseItemOnEvent`,
  `RightClickBlockEvent` und `acting_player()` sagen aber nicht, ob der Spieler schleicht.
  `player_toggle_sneak` meldet nur Wechsel bei Mitspielern.
- **Create braucht es für:** Schraubenschlüssel + Schleichen = Block aufheben, Handkurbel rückwärts,
  Trichter- und Filter-Bedienung, viele Werkzeuge.
- **Gewünscht:** `secondary_use: bool` in `UseItemOnEvent` und `RightClickBlockEvent`.
- **NeoForge:** `UseOnContext.isSecondaryUseActive()`, `Player.isShiftKeyDown()`.

### 8. Ausrüstung des eigenen Spielers auf dem Client

- **Heute:** Auf dem Client gibt es `held()` (Haupthand), aber keinen Weg, den Kopf-, Brust- oder
  Beinslot des eigenen Spielers zu lesen (`LivingRef::equipment` braucht eine Entity-ID, die der
  Client für sich selbst nicht bekommt).
- **Create braucht es für:** die Goggles-Anzeige (Stress und Drehzahl beim Anschauen), die
  Luftanzeige des Rucksacks, die Tauchhelm-Sicht.
- **Gewünscht:** `fn local_equipment(&self, slot: EquipmentSlot) -> Option<ItemStack>` oder die
  Entity-ID des eigenen Spielers.

### 9. Daten einer Mod-Block-Entity auf dem Client an einer Position lesen

- **Create braucht es für:** Goggles-Tooltips (Drehzahl, Stress, Kapazität des angeschauten Blocks)
  und Anzeigen, die Nachbarblöcke betreffen.
- **Gewünscht:** auf dem Client `fn block_entity_tag(&self, pos) -> Option<Compound>`, also die
  per `native_block_entity_update_tag` gesendeten Daten.

### 10. Fließrichtung von Flüssigkeiten

- **Create braucht es für:** Wasserrad und großes Wasserrad (Drehzahl aus der Strömung rund ums
  Rad), Waschen per Wasserströmung.
- **Gewünscht:** `fn fluid_flow(&self, pos) -> [f64; 3]`.
- **Vanilla/NeoForge:** `FluidState.getFlow(level, pos)`.

### 11. Text in der Welt zeichnen

- **Create braucht es für:** Anzeigetafeln, Nixie-Röhren, Klemmbrett, Bahnhofs- und Zugnamen.
- **Gewünscht:** `fn draw_text(&self, draw: &mut BlockEntityDraw, text: &Text, color: u32, glowing: bool)`
  in `block_entity_submit`, wie Schilder ihren Text zeichnen.
- **NeoForge:** `Font.drawInBatch` in einem `BlockEntityRenderer`.

---

## Priorität 3: Leistung, Optik, Komfort

### 12. Animierte Geometrie ohne Aufruf pro Bild (Ersatz für Flywheel)

- **Heute:** `draw.keep()` behält nur stillstehende Geometrie. Jede drehende Welle ruft jedes Bild
  `block_entity_submit` auf.
- **Create braucht es für:** große Fabriken mit Tausenden drehender Wellen und Zahnräder. Create
  nutzt dafür Flywheel (GPU-Instancing).
- **Gewünscht:** eine „drehende Instanz“, einmal angelegt: Modell, Drehachse, Drehpunkt, Drehzahl
  und Startwinkel. Den Winkel rechnet der Shader aus der Zeit. Erst bei Drehzahländerung wird sie
  aktualisiert.

### 13. Modelle abhängig von Position und Block-Entity-Daten (Connected Textures, Copycats)

- **Create braucht es für:** verbundene Texturen von Gehäusen, Glasfenstern und Rahmen
  (`CTSpriteShifter`), und Copycat-Blöcke, die das Aussehen eines anderen Blocks annehmen.
- **Gewünscht:** einen Hook, der beim Bauen des Chunk-Meshes für einen Mod-Block das Modell
  oder einzelne Flächen-Sprites ersetzen kann (abhängig von Nachbarn und Block-Entity-Daten).
- **NeoForge:** `ModelData`, `BakedModel.getQuads(state, side, rand, data, renderType)`.

### 14. 3D-Szenen in Bildschirmen (Ponder)

- **Create braucht es für:** Ponder, Creates eingebaute bebilderte Anleitungen. Das sind kleine
  animierte Welten in einem GUI.
- **Gewünscht:** in einem Mod-Screen eine Gruppe von Blockzuständen (plus Block-Entity-Zeichnung)
  mit Kamera-Transformation rendern.

### 16. Was ein Container annimmt, per Code entscheiden

- **Heute:** `ModContainer` sagt, welche Slots Trichter befüllen dürfen (`inputs`), aber nicht
  *welche Items*. `menu_slot_rules` gilt nur in Menüs.
- **Create braucht es für:** Mühlstein, Brechräder, Becken und Ablage nehmen nur Items an, für die
  es ein Rezept gibt (`isItemValid` → `canProcess`). Sonst verstopft ein falsches Item die Maschine.
- **Gewünscht:** ein Hook „darf dieses Item in diesen Slot von dieser Seite“, wie Horizons eigener
  Vorschlag `block_capability` in `docs/native-hooks-neoforge.md`.
- **NeoForge:** `IItemHandler.isItemValid`, `Capabilities.Item.BLOCK`.

### 15. Chunks geladen halten

- **Create braucht es für:** Züge, die durch ungeladene Chunks fahren, und lange Contraptions.
- **Gewünscht:** Chunk-Tickets für Mods.
- **NeoForge:** `TicketController`.

---

## Was schon reicht (wird im Port genutzt)

Damit das Team sieht, was gut funktioniert: Block-Entities auf beliebigen Vorlagen (API 129),
Tick, Update-Tag und Sync, `block_entity_submit` mit `draw_block` / `draw_cube`,
`state_for_placement`, `set_block_property`, `neighbor_signal`, `use_item_on`,
`right_click_block`, `block_changed`, Partikel- und Sound-Typen eigener Mods (API 103, 61),
`recipes_of_type` für Creates 391 eigene Rezepte (API 36), Container, Menüs und Tanks.
Die Trait-API mit `dispatch_*` ist sehr angenehm zu testen.
