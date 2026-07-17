# Pokémon Yellow LEGACY — ROM & Save Parsing Reference

> **Purpose:** A consolidated braindump of everything learned reverse-engineering
> Pokémon Yellow **Legacy** (the cRz-Shadows/Pokemon_Yellow_Legacy disassembly) and
> its `.sav`/`.srm` save files. Intended as the foundation for a parsing library +
> MVC CLI/webapp for exploring game state, the ROM itself, and progression tooling
> — including future from-scratch runs.
>
> **Scope note:** This targets *Legacy* specifically. It's built on the Gen 1
> (R/B/Y) engine, so most vanilla knowledge transfers, but Legacy makes several
> **deliberate changes** (flagged ⚠️ **LEGACY** throughout) that vanilla references
> get wrong. When in doubt, the disassembly is ground truth — clone it and grep.

---

## 0. Two data sources, two parsers

There are **two distinct artifacts** to parse, and they need different approaches:

| Artifact | What it is | How to parse |
|----------|-----------|--------------|
| **The ROM** (`.gbc`) | Static game data: base stats, movesets, type chart, trainer rosters, wild encounters, evolutions, TM/HM lists, item/move/species tables | Parse the **disassembly source** (`.asm` files), not the ROM binary. Legacy ships as a buildable disassembly; the `.asm` data files are human-readable and authoritative. |
| **The save** (`.sav`/`.srm`) | Dynamic player state: party, stats, bag, PC, playtime, badges, dex flags, money, events | Read raw bytes at fixed **SRAM offsets**. 32768 bytes (32KB). |

The ROM data rarely changes (only on Legacy version bumps); the save changes every
time you play. A good architecture parses the ROM data **once** into a static data
model (JSON), then the save parser resolves its numeric IDs against that model.

---

## 1. The Disassembly (ROM data extraction)

### 1.1 Getting it

```bash
git clone https://github.com/cRz-Shadows/Pokemon_Yellow_Legacy
# The data lives in data/, constants/, engine/, scripts/, maps/
```

Key directories:
- `data/pokemon/base_stats/*.asm` — one file per species (stats, types, catch rate, growth, TM/HM compat)
- `data/pokemon/evos_moves.asm` — evolutions + level-up learnsets (all species)
- `data/moves/moves.asm` — move power/type/accuracy/PP/effect
- `data/types/type_matchups.asm` — the type effectiveness chart
- `constants/type_constants.asm` — type ordering + the **physical/special split point**
- `constants/item_constants.asm` — item IDs, TM/HM ordering (`add_tm`/`add_hm`)
- `data/trainers/parties.asm` — every trainer's roster (incl. bosses, E4, Oak, Joy, Jenny)
- `data/trainers/special_moves.asm` — specific movesets assigned to trainer mons
- `data/wild/maps/*.asm` — wild encounter tables per map
- `data/maps/objects/*.asm` — sprite/object placement (incl. static legendaries)
- `data/maps/hide_show_data.asm` — which objects are hidden until an event fires
- `scripts/*.asm` — event logic (gift Pokémon, boss triggers, unlock chains)
- `engine/**/*.asm` — game mechanics (catch formula, damage, XP, blackout, daycare)

### 1.2 Extraction approach (regex over .asm)

The proven approach (see `parse_legacy.py`) is targeted regex extraction. Examples:

**Base stats** (`data/pokemon/base_stats/exeggutor.asm`):
```asm
	db DEX_EXEGGUTOR ; pokedex id
	db 95, 95, 85, 55, 125    ; hp atk def spd spc
	db GRASS, PSYCHIC_TYPE     ; type
	db 45 ; catch rate
	db GROWTH_SLOW ; growth rate
	tmhm TOXIC, TAKE_DOWN, ... ; end
```
Regex: `db DEX_(\w+)`, then `db\s+(\d+),\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+)` for the
stat line (order: **HP, ATK, DEF, SPD, SPC** — note SPD before SPC), then
`db (\w+), (\w+) ; type`, `db (\d+) ; catch rate`, `db GROWTH_(\w+)`, and the
`tmhm ... ; end` block (comma-separated, may span lines with `\` continuations).

**Level-up moves & evolutions** (`data/pokemon/evos_moves.asm`):
```asm
ExeggcuteEvosMoves:
	db EVOLVE_ITEM, LEAF_STONE, 1, EXEGGUTOR
	db 0
	db 10, POISONPOWDER
	db 13, LEECH_SEED
	...
	db 0
```
Each species block is `^(\w+)EvosMoves:\n(evos)\n\tdb 0\n(learnset)\n\tdb 0`.
Learnset entries: `db (\d+), (\w+)` = (level, move). Evolutions start with
`EVOLVE_ITEM` / `EVOLVE_LEVEL` / `EVOLVE_TRADE`.

**Moves** (`data/moves/moves.asm`):
```asm
	move ICE_BEAM, FREEZE_SIDE_EFFECT, 95, ICE, 100, 15
	;    name,     effect,             power, type, acc, pp
```
Regex: `move\s+(\w+),\s*(\w+),\s*(\d+),\s*(\w+),\s*(\d+),\s*(\d+)`.

**Type chart** (`data/types/type_matchups.asm`):
```asm
	db ICE, DRAGON, SUPER_EFFECTIVE
	db WATER, FIRE, SUPER_EFFECTIVE
```
Regex: `db (\w+),\s*(\w+),\s*(\w+)` → (attacker, defender, multiplier). Multipliers:
`SUPER_EFFECTIVE`=2.0, `NOT_VERY_EFFECTIVE`=0.5, `NO_EFFECT`=0.0. Absent pairs = 1.0.

**TM/HM numbering** (`constants/item_constants.asm`):
```asm
	add_tm MEGA_PUNCH   ; TM01
	add_tm RAZOR_WIND   ; TM02
	...
	add_hm CUT          ; HM01
```
`add_tm` entries in order → TM01..TM55; `add_hm` → HM01..HM05. Map move name → TMxx.

**Trainer parties** (`data/trainers/parties.asm`):
```asm
LanceData:
	db $FF, 61, DRAGONITE, 60, GYARADOS, ...   ; first fight (per-mon levels, $FF prefix)
	db $FF, 73, ARCANINE, 73, ELECTABUZZ, ...  ; rematch
```
Two formats: `db LEVEL, MON, MON, ...` (shared level) or `db $FF, LVL, MON, LVL, MON, ...`
(per-mon levels, `$FF` prefix). Multiple lines = multiple rosters (e.g. first vs rematch).

**Trainer-specific movesets** (`data/trainers/special_moves.asm`):
```asm
	db LANCE, 2 ; which trainer, which roster
	db 3, 1, SELFDESTRUCT   ; party slot 3, move slot 1 = Selfdestruct
	db 3, 2, EARTHQUAKE
```
Format: `db PARTY_POS, MOVE_SLOT, MOVE`. This is how you find e.g. Lance's Snorlax
having Self-Destruct (see §5).

### 1.3 The static data model (parsed output)

`parse_legacy.py` emits `legacy.json` with this schema:
```json
{
  "poke":  { "EXEGGUTOR": { "key","hp","atk","defense","spd","spc",
                            "t1","t2","tmhm":[...],"levelup":[...],"evos":[...] }, ... },
  "moves": { "ICE_BEAM": { "name","effect","power","type","acc","pp","tm","cat" }, ... },
  "chart": { "Ice": { "Dragon":2.0,"Water":0.5,... }, ... },   // attacker → defender → mult
  "phys":  ["Bug","Fighting","Flying","Ground","Normal","Poison","Rock"],
  "special":["Dragon","Electric","Fire","Ghost","Grass","Ice","Psychic","Water"],
  "tms":   ["MEGA_PUNCH", ...],   // in TM order
  "hms":   ["CUT","FLY","SURF","STRENGTH","FLASH"],
  "bosses":[ ["Brock","Gym 1","GEODUDE",10,"Rock","Ground"], ... ]  // flattened rosters
}
```
151 species, 165 moves (incl. Struggle), full chart. This is the "source of truth"
the save parser resolves IDs against.

---

## 2. The Save File (`.sav` / `.srm`)

### 2.1 Basics
- **32768 bytes** (32KB SRAM). Gen 1 format; Legacy shares vanilla Yellow's layout
  **except** where the 41-slot bag shifts later offsets (see ⚠️ below).
- **Emulator note:** Pizza Boy writes `.sav` to an accessible `ROMs/saves/` dir.
  Lemuroid locks saves in Android scoped storage (`Android/data`) — painful to reach.
  Recommend Pizza Boy for tooling access.
- Multibyte integers are **big-endian** (HP = `d[b+1]<<8 | d[b+2]`).

### 2.2 Verified offsets (Legacy, confirmed against live saves)

| Field | Offset | Notes |
|-------|--------|-------|
| Trainer name | `0x2598` | 11 bytes, GB charmap (see §2.4), 0x50-terminated |
| Bag item count | `0x25C9` | ⚠️ **LEGACY: up to 41 items** (vanilla is 20) |
| Bag data | `0x25CA` | (item, qty) byte pairs, `0xFF` terminator |
| PC item count | `0x2834` | count byte |
| PC item data | `0x2835` | (item, qty) pairs, `0xFF` terminator |
| Playtime hours | `0x2CED` | 1 byte |
| Playtime minutes | `0x2CEF` | 1 byte (0x2CEE is frame/second-ish; hrs:min is what UI shows) |
| Party count | `0x2F2C` | 1 byte (1–6) |
| Party species list | `0x2F2D` | count+1 bytes, species IDs, `0xFF` terminator (redundant w/ structs) |
| Party mon structs | `0x2F34` | 6 × **44-byte** structs |
| Party nicknames | `0x307E` | 6 × 11 bytes |

⚠️ **CRITICAL LEGACY QUIRK:** The bag holds **41 items** instead of vanilla's 20.
This **shifts every offset after the bag** (PC, playtime, party, etc.) relative to
vanilla Yellow maps. The offsets in the table above are the **Legacy-correct** ones.
Do **not** trust a vanilla R/B/Y save map for anything past the bag. Money, badges,
and the map/position block also shift and were **left unparsed** in the current tool
(low value, unstable to locate) — **a from-scratch tooling effort should nail these
down by diffing saves** (e.g. spend money, diff; earn badge, diff).

### 2.3 Party mon struct (44 bytes, offset from struct base `b`)

| Field | Offset | Size | Notes |
|-------|--------|------|-------|
| Species ID | `b+0x00` | 1 | **Internal index** (see §2.5), NOT dex number |
| Current HP | `b+0x01` | 2 | big-endian; **0 = fainted** |
| "Level" (stale) | `b+0x03` | 1 | ⚠️ often **stale/0** — do NOT use |
| Status | `b+0x04` | 1 | bitfield (see below) |
| Type 1 | `b+0x05` | 1 | |
| Type 2 | `b+0x06` | 1 | |
| Catch rate/held | `b+0x07` | 1 | |
| Moves | `b+0x08` | 4 | move IDs (0 = empty slot) |
| OT ID | `b+0x0C` | 2 | |
| Experience | `b+0x0E` | 3 | 3-byte big-endian |
| HP EV | `b+0x11` | 2 | stat experience |
| Atk/Def/Spd/Spc EV | `b+0x13..0x1A` | 2 each | |
| IV/DV | `b+0x1B` | 2 | packed nibbles |
| **PP** | `b+0x1D` | 4 | one byte per move: **low 6 bits = current PP, top 2 bits = PP Ups** |
| **Level** | `b+0x21` | 1 | ✅ **the live level — use THIS**, not b+0x03 |
| Max HP | `b+0x22` | 2 | |
| Attack | `b+0x24` | 2 | |
| Defense | `b+0x26` | 2 | |
| Speed | `b+0x28` | 2 | |
| Special | `b+0x2A` | 2 | (Gen 1 has one Special stat, not Sp.Atk/Sp.Def) |

**Status bitfield** (`b+0x04`):
- bits 0–2 (`& 0x07`): sleep turn counter (nonzero = asleep, value = turns left)
- bit 3 (`0x08`): poison
- bit 4 (`0x10`): burn
- bit 5 (`0x20`): freeze
- bit 6 (`0x40`): paralyze

**PP max calculation:** `base_pp × (5 + pp_ups) / 5`, where `base_pp` comes from the
move's PP in the ROM data (see BASEPP table in `read_save.py`, indexed by move ID),
and `pp_ups` = top 2 bits of the PP byte. Current PP = low 6 bits.

### 2.4 GB text charmap (for names)
```
0x50 → terminator ("")
0x7F → space
0x80–0x99 → A–Z
0xE8 → "."   0x9A → "("   0x9B → ")"   0x9C → ":"
0xE3 → "-"   0xF3 → "/"   0xF4 → ","   0x9D → ";"
```
Read bytes until 0x50 or 0x00. (Lowercase, digits, and other glyphs exist at other
codepoints — extend the map if you need them; names are usually uppercase.)

### 2.5 Species ID = internal index, NOT dex number ⚠️

Gen 1 stores an **internal index number** in the save, which is **different** from
the Pokédex number. E.g. index 131 = Mewtwo, 133 = Snorlax, 151 = Victreebel.
There's a full 190-entry index table (many are `MISSINGNO.` gaps). The `MON` dict in
`read_save.py` is the complete index→name map. **Never** assume save species byte =
dex number. (`savemap.json` has the dex-order name list separately.)

---

## 3. The Parser Architecture (current `read_save.py`)

Clean pattern worth keeping in the new lib:

1. **`extract(bytes) → dict`** — pure parse into a structured dict (the source of
   truth). No formatting. Reads party (species/nick/level/HP/stats/moves/PP/status),
   bag, PC, trainer, playtime.
2. **Renderers** consume that dict:
   - `render_full(s, full)` — human-readable, with a `--full` toggle for all items
   - `render_compact(s)` — one line per mon, **NAME-prefixed** (stable ordering for
     clean diffs), `·`-separated moves
   - `--json` — dumps the raw dict
3. **Embedded reference tables** (MON, MV, BASEPP, ITEMS) so the parser is
   self-contained. In the new lib these should come from the parsed `legacy.json`
   instead of being hardcoded.

### 3.1 The telemetry pipeline (`pokedump.sh`)
One-command workflow that proved extremely valuable:
```
save in-game → ./pokedump.sh
  → reads Pizza Boy .sav
  → writes timestamped dump to ROMs/pokedumps/YYYY-MM-DD_HHMMSS.txt (--compact)
  → git diff --no-index --word-diff --word-diff-regex='[A-Za-z0-9]+' vs previous dump
  → copies diff to clipboard
```
The **word-diff with alphanumeric regex** gives inline char-level highlighting
(`L79→L80`, `PSYCHIC 15/15→0/15`, `[FAINTED]` appearing) — makes battle-by-battle
state changes instantly legible. This is the killer feature: you save after a fight,
run one command, and see exactly what changed (HP, PP, levels, status, faints).

The **`--compact` NAME-prefix format** is what makes diffs clean — because the line
starts with the stable species name, git aligns lines correctly even when party
order shifts or stats change.

---

## 4. Legacy-specific MECHANICS (⚠️ differs from vanilla)

These are verified from the disassembly and matter for accurate tooling / advice.

### 4.1 Type system
- ⚠️ **Ghost is a SPECIAL type in Legacy** (moved into the special block in
  `type_constants.asm`). Damage category in Gen 1 is determined by **type**, not
  per-move: types after the `SPECIAL` split point are special.
  - **Special types:** Dragon, Electric, Fire, Ghost, Grass, Ice, Psychic, Water
  - **Physical types:** Bug, Fighting, Flying, Ground, Normal, Poison, Rock
- Ghost hits Psychic for 2× (and being special, keys off Special stat).

### 4.2 Trade evolutions REMOVED ⚠️
Alakazam, Gengar, Golem, Machamp all evolve by **level-up** in Legacy (not trade):
- Kadabra → Alakazam @ L42 (level)
- Haunter → Gengar @ L42 (level)
- Graveler → Golem @ L38 (level)
- Machoke → Machamp @ L38 (level)

(Confirm exact levels in `evos_moves.asm`.) This is part of Legacy's "all 151
obtainable solo" design — no trading required for the dex.

### 4.3 All 151 obtainable in one playthrough ⚠️
Legacy's README states all 151 are catchable/obtainable solo. Mechanisms include the
trade-evo removal above, plus **starter gifts** (see §6) and **Mew being catchable**
(see §5). This is central to any dex-completion tooling.

### 4.4 HMs are deletable ⚠️
Unlike vanilla, HM moves can be deleted/overwritten freely. Relevant for movepool
planning tools.

### 4.5 Thunder PP nerf ⚠️
Thunder has **5 base PP** in Legacy (vanilla 10). Check `moves.asm` for other PP/power
tweaks — don't assume vanilla move stats.

### 4.6 Daycare level cap + XP ⚠️
- Daycare (Route 5): **+1 XP per step** while deposited (`IncrementDayCareMonExp`).
- Cost on pickup: **¥100 × (levels gained + 1)**.
- ⚠️ **LEGACY level cap** tied to story progress (hard mode): the cap escalates with
  badges/game-completion: 12→21→24→35→43→50→53→55→**65** (postgame). Caps are keyed
  in `scripts/Daycare.asm` and the Rare Candy / XP code. Deposited mons won't level
  past the current cap.

### 4.7 Freeze is a run-ender
No natural thaw in Gen 1 — a frozen mon stays frozen until (a) hit by a Fire move, or
(b) healed at a Center. `CheckDefrost` in `engine/battle/effects.asm`. Relevant for
risk modeling.

### 4.8 Rare Candy is finite & non-farmable
- **Not sold** in any mart (price field is 0/unbuyable).
- **Not a Game Corner prize** (those are Pokémon + TMs).
- Only ~**22 findable** in-world + 3 hidden. The "fishing hack" people mention is the
  **Missingno item-duplication glitch** (avoid).
- At L100, Rare Candy **does nothing** (hard level-cap check → `.vitaminNoEffect`;
  no "255 rollover" — that myth is false in Legacy, the cap is guarded).

### 4.9 XP at level 100
- A L100 mon can't level further (cap check `cp MAX_LEVEL`). Excess XP is effectively
  capped (not a void-overflow, not corruption — cleanly handled).
- ⚠️ **Exp All still distributes its share to the rest of the party** normally, even
  when the killer is L100. So a maxed lead + Exp All is an efficient team-uplift farm.

### 4.10 Blackout money penalty
- Blacking out (all mons faint) **halves your money** (`ResetStatusAndHalveMoney`
  `OnBlackout` in `engine/events/black_out.asm`; divisor = 2). Then heals party,
  respawns at last Center.
- ⚠️ Practical trap: "faint everyone while XP farming" silently halves money each
  time. Fleeing avoids it. (Real example: ~4 blackouts took ¥241k → ¥20k.)

### 4.11 Catch mechanics
- Formula uses RNG + catch rate + current/max HP + status + ball type. **No joypad
  reads** in `ItemUseBall` — the **A/B-during-shake trick is a myth** (the routine
  even clears held buttons). Catch is decided at throw.
- Gen 1 sleep/freeze catch branch uses a **fixed threshold (25)** rather than adding
  to catch rate, so for a **moderate catch-rate** mon (e.g. Mew, rate 45),
  **paralysis actually beats sleep** for catch odds. (For low catch-rate mons the
  difference is smaller; Master Ball for the rate-3 ones like Mewtwo.)

### 4.12 Damage roll (Gen 1, unchanged but widely misunderstood)
`RandomizeDamage`: damage × random(217–255) / 255. So the roll is **85%–100% of base**
(a ~15% band, top-heavy) — **not** ±50% symmetric. Consequence: a target sits either
"always OHKO'd" or "never OHKO'd (leaves a sliver)" depending on whether base ≥ HP,
with a narrow transition band. Crits (~roughly Speed/512 chance, high for fast mons)
roughly double and ignore stat stages.

### 4.13 Softboiled / Milk Drink field heal
- **In battle:** heals user 50% max HP (costs PP).
- **Out of battle:** transfers ~**1/5 of the user's MAX HP** to *another* party member
  (forced to pick a different mon — can't self-target in field). No PP/item cost.
  User must have >1/5 max HP to use it. Effectively a free party-heal engine.

---

## 5. Legendaries, Mew & the Oak/Dex endgame chain

### 5.1 Static legendary encounters (sprites in `data/maps/objects/`)
- **Articuno** — Seafoam Islands, L50
- **Zapdos** — Power Plant, L50
- **Moltres** — Victory Road, L50
- **Mewtwo** — Cerulean Cave B1F (sprite at 27,13), **L70, catch rate 3** (Master Ball it)
- **Mew** — Pokémon Mansion **B1F** (sprite at 4,12), **L70, catch rate 45**

### 5.2 The Mew unlock chain (⚠️ Legacy-specific, gated)
Mew is hidden (`HIDE` in `hide_show_data.asm`) until a chain completes:
```
1. Catch 150 Pokémon (OWNED, not seen — Oak checks wPokedexOwned count vs NUM_POKEMON-1)
2. Show Oak in his Lab (Pallet) → sets HS_POKEMON_MANSION_2F_OAK (Oak appears on Mansion 2F)
3. Beat Oak (Mansion 2F superboss) → ShowObject reveals Mew on Mansion B1F
4. Catch Mew (paralysis + Ultra Balls, or Master Ball) → 151
```
- ⚠️ **Oak's dex check counts OWNED** (`wPokedexOwned` bits via `CountSetBits`), not
  seen. Evolving fills a line's entries, so it's ~fewer catches than 150.
- ⚠️ **Mew requires beating Oak first** — walking to Mansion B1F before that shows
  nothing.

### 5.3 Oak superboss (`ProfOakData` in parties.asm)
Two rosters:
- Postgame: `$FF, 69 TAUROS, 70 CHARIZARD, 70 VENUSAUR, 70 BLASTOISE, 69 SNORLAX, 70 NIDOKING`
- Harder variant: `$FF, 78 TAUROS, 77 ZAPDOS, 77 ARTICUNO, 77 MOLTRES, 78 SNORLAX, 81 NIDOKING`

### 5.4 Cerulean Cave contents (great dex/XP spot)
- **1F:** Rhydon, Golem, Electrode, Lickitung, Chansey, Ditto, Vileplume, Victreebel, Raichu
- **2F:** Golem, Sandslash, Wigglytuff, Marowak, Dodrio, Ditto, Magneton, Chansey, Raichu
- **B1F:** Hypno, Golduck, Golbat, Sandslash, Parasect, Slowbro, Kadabra, Ditto, Alakazam + Mewtwo
- All L60-65. Chansey = big XP. 2 findable Rare Candies (1F @29,16 & 2F).

---

## 6. Starter gifts & hidden bosses (⚠️ Legacy additions)

### 6.1 All three Kanto starters are gift-able (dex-completion enablers)
- **Squirtle** — Officer Jenny, Vermilion City (after beating her; L15).
  `scripts/VermilionCity_2.asm`: `lb bc, SQUIRTLE, 15; call GivePokemon`, gated by
  `EVENT_GOT_SQUIRTLE_FROM_OFFICER_JENNY`.
- **Bulbasaur** — Cerulean City, Melanie's House (`scripts/CeruleanMelaniesHouse.asm`).
- **Charmander** — Route 24 (`scripts/Route24.asm`).

Each = 3 dex entries via evolution. These were trade-locked in vanilla.

### 6.2 Repeatable hidden postgame bosses (money/XP farms)
Both require having beaten the game (`wGameStage` check), then talking to the NPC.
The `EVENT_BEAT_*` flag only gates first-time vs rematch **dialogue** — the **battle
re-triggers every time** (repeatable farm).

- **Nurse Joy** — Fuchsia City Pokémon Center. Team (all L65): Kangaskhan, Snorlax,
  Starmie, Porygon, Exeggutor, Chansey. Stall team (Double Team, Recover, Rest,
  Amnesia, Substitute) + ⚠️ **Kangaskhan has FISSURE** (OHKO move — fails vs faster
  mons, so a fast lead is immune). ~¥6.5k + full XP per ~30s clear. **You heal at the
  counter right after** (and must, to re-trigger) — self-contained loop.
- **Officer Jenny** — Vermilion City, overworld sprite at (19,15). Team (L65):
  Pidgeot, Blastoise, Tangela, Gengar, Parasect, Arcanine. Also gives the Squirtle
  (§6.1) on first win. No auto-heal (overworld, not a Center).

### 6.3 The Snorlax Self-Destruct trap ⚠️
Lance's **rematch** Snorlax (position 3, L74) has **Self-Destruct** as move 1
(`special_moves.asm`: `db 3, 1, SELFDESTRUCT`, also Earthquake/Reflect). It's
anti-sweeper tech — a 200 BP explosion that can OHKO a healthy L79 Mewtwo. Counter:
swap your sweeper out, or OHKO it first (a high-enough level nuke kills before it
detonates). This is the kind of trainer-specific moveset only visible via
`special_moves.asm`, not the party roster.

---

## 7. Reference data snapshots (from `legacy.json`)

### 7.1 Growth rates (matters for XP/leveling tools)
Gen 1 curves; per-species in base_stats (`GROWTH_*`). Total XP to level n:
- `GROWTH_FAST`: `0.8·n³`
- `GROWTH_MEDIUM_FAST`: `n³`
- `GROWTH_MEDIUM_SLOW`: `1.2·n³ − 15·n² + 100·n − 140`
- `GROWTH_SLOW`: `1.25·n³`

(Slow needs ~25% more XP than medium-fast. Mewtwo, Exeggutor, Gyarados, and the
starters are all Slow — relevant when estimating grind times.)

### 7.2 Stat formulas (Gen 1)
```
HP  = floor((base + IV) · 2 · level / 100) + level + 10
stat= floor((base + IV) · 2 · level / 100) + 5     (for Atk/Def/Spd/Spc)
```
(Plus a stat-experience/EV term for fully-trained mons: `+ floor(sqrt(statEV)/4)`
scaled — see the full Gen 1 formula if you need EV-exact values.)

### 7.3 Damage formula (Gen 1)
```
base = (((2·level·crit/5 + 2) · power · A/D) / 50 + 2) · STAB · type_eff · random
```
where A/D = attacker's relevant attack stat / defender's relevant defense stat
(physical → Atk/Def, special → Spc/Spc), STAB = 1.5 if move type matches user,
type_eff = product of chart multipliers, random = (217–255)/255, crit = 2 if crit
else 1 (crit also ignores stat-stage modifiers).

### 7.4 Catch formula (Gen 1, simplified)
Two RNG checks: a status/rate gate, then an HP-based shake check scaled by ball type
(Ultra/Great use divisor 8, Poké 12). Status bonus: sleep/freeze=25 (fixed threshold),
para/burn/poison=12. Lower HP + status + better ball = higher odds. See §4.11 for the
sleep-vs-paralysis nuance on moderate-catch-rate mons.

### 7.5 Boss rosters
`legacy.json["bosses"]` has flattened `[trainer, label, MON, level, type1, type2]`
rows for gyms and E4. For rematch/Oak/Joy/Jenny, parse the specific `*Data:` labels
in `parties.asm` + `special_moves.asm` (rematches are the 2nd `db` line under a label).

---

## 8. Suggested architecture for the new tooling

A clean MVC split that fits what's been built:

**Model (data layer)**
- `rom_data`: parse the disassembly `.asm` → `legacy.json`-style static model
  (species, moves, chart, TMs, learnsets, evolutions, trainers, wild tables,
  encounter maps, gift/event locations). Cache to JSON; re-run on ROM version bump.
- `save_data`: the `extract()` parser → structured save dict. Resolve IDs against
  `rom_data`. **Nail down the post-bag offsets** (money, badges, dex flags, map/pos)
  by save-diffing — these are the current gaps.
- `dex`: read `wPokedexOwned`/`wPokedexSeen` bitfields from the save (find offsets by
  diffing a catch) → completion tracking, "what's missing", "where to get X".

**View**
- CLI: the existing `--full`/`--compact`/`--json` renderers, plus dex/progress views.
- Web: interactive party/box/bag explorer, dex-completion map, damage/catch
  calculators, "route to next goal" planner. The static model powers calculators
  entirely client-side.

**Controller / features**
- **Diff engine** (generalize `pokedump.sh`): snapshot saves, word-diff state.
- **Damage calc**: §7.3 + chart + live party stats → "what OHKOs what".
- **Catch calc**: §7.4 → optimal ball/status/HP for any encounter.
- **Dex planner**: from owned set → remaining species → obtainment routes (wild
  locations from `data/wild`, gifts from `scripts`, evolutions, legendaries, Mew chain).
- **XP/grind estimator**: growth curve + encounter XP → time-to-level (battle vs
  daycare, factoring the human-input floor: a battle ≈ 12s game-time (FF-scalable) +
  ~3s human-time (fixed), daycare = pure steps (scales linearly with FF, capped at ~65).
- **Boss prep**: pull a trainer's roster + special moves → recommend leads/answers,
  flag traps (Fissure, Self-Destruct, Double-Team/Recover stall on Alakazam).

**Watch out for (lessons already learned):**
- Post-bag offset shift (§2.2) — the #1 gotcha vs vanilla maps.
- Internal index ≠ dex number (§2.5).
- Use level at `b+0x21`, not `b+0x03` (§2.3).
- Ghost = special, trade evos removed, Thunder PP=5, daycare cap (§4).
- Dex check is OWNED not seen; Mew gated behind Oak gated behind 150 (§5.2).
- Trainer *special movesets* live in a separate file from rosters (§6.3).

---

## 9. Files carried over from the exploratory session

Reference implementations (in the working dir during R&D — reimplement cleanly):
- `read_save.py` — working save parser, 4 output modes, embedded ref tables.
- `parse_legacy.py` — disassembly → `legacy.json` extractor.
- `legacy.json` — parsed static model (151 poke, 165 moves, chart, bosses, TMs).
- `savemap.json` — index→name + dex-order name list.
- `gen1_basepp.json`, `gen1_names.json`, `gen1_extra.json` — supporting maps.
- `pokedump.sh` — the save→diff→clipboard telemetry pipeline (git word-diff).

The new lib should regenerate the data model from the disassembly (don't hardcode the
big tables) and treat the save `extract()` as the stable core, with the offset gaps
(money/badges/dex/position) filled in via save-diffing as the first order of business.
