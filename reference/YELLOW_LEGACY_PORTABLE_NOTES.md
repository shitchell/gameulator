# Pokémon Yellow LEGACY — Portable Notes (Player-Agnostic)

> Miscellaneous hard-won, **fully portable** knowledge about Yellow Legacy and
> emulating it — true no matter who's playing, and **not** already covered in the
> ROM/save parsing reference, the run tracker, or the player profile. Mechanics here
> are verified against the cRz-Shadows disassembly; emulation notes are practical.

---

## 1. Legacy has a DIFFICULTY setting that changes real mechanics ⚠️

This is the single most important portable thing that's easy to miss: **Legacy has a
"hard mode" (`wDifficulty`)**, and several mechanics branch on it. Tooling and advice
should ask/know which mode a save is in, because the same action behaves differently.

Confirmed difficulty-dependent behaviors:
- **The 1/256 miss bug is FIXED on normal mode, KEPT on hard mode.** In vanilla Gen 1,
  even a "100% accuracy" move has a ~1/256 chance to miss (an accuracy-check rounding
  bug). Legacy **removes this on normal mode** (100% truly means 100%) but
  **deliberately keeps it on hard mode** (`.DontRemoveMiss`). So "does a 100% move ever
  miss?" is **mode-dependent** — a real gotcha for a damage/reliability calculator.
- **Level caps (daycare & Rare Candy) only apply on hard mode.** The escalating
  story-progress level caps (12→…→65, see parsing ref §4.6) are gated by a
  `wDifficulty` check — on normal mode there's **no cap** (grind freely). On hard mode
  they bind until postgame.

There are likely other difficulty branches (encounter levels, trainer AI, item
availability) — worth a `grep -rn wDifficulty engine/ scripts/` when building tooling,
and worth surfacing the mode prominently in any UI.

---

## 2. Gym badges give in-battle STAT BOOSTS (and Pikachu gets a bonus) ⚠️

A vanilla Gen 1 mechanic that's easy to forget and that Legacy *tweaks*:

- Four badges each passively boost one stat of your active mon **in battle** (not
  reflected in the summary screen's stat numbers):
  - **Boulder Badge → Attack**
  - **Thunder Badge → Defense**
  - **Soul Badge → Speed**
  - **Volcano Badge → Special**
- The boost is **×1.125** per applicable badge (applied at battle start and after
  stat changes).
- ⚠️ **LEGACY tweak:** for **Pikachu specifically, the boost is ×1.25** instead of
  ×1.125 (a nod to Yellow's mascot). Confirmed in `ApplyBadgeStatBoosts`
  (`cp PIKACHU → extra shift`).

Implication for tooling: a mon's *effective* in-battle stats can be higher than the
stored/summary stats, depending on badges held and species. A damage calculator that
uses only the raw save stats will **under-estimate** damage for badge-boosted stats.
Model badge boosts (and the Pikachu special-case) for accuracy.

---

## 3. Sleep only ticks down while the mon is ACTIVE on the field

Portable battle-mechanics fact with real strategic weight: a sleeping Pokémon's sleep
counter **only decrements on turns it's the active battler**. Sleep does **not** tick
while the mon sits on the bench (switched out) — nor does it tick "in real time."
Consequence: you can sleep an enemy, switch to set up, and it stays asleep; but you
can't "wait out" your own mon's sleep by benching it. (Relevant to both offense —
maintaining enemy sleep — and the frustration of self-inflicted Rest sleep.)

---

## 4. Critical hits scale with BASE SPEED (fast mons crit a lot)

The Gen 1 crit formula (unchanged in Legacy) is **based on the user's base Speed**,
not a flat rate:
- Normal crit chance ≈ `base_speed / 512` (roughly; the code does `base_speed / 2`
  then compares against a random byte).
- **High-crit-ratio moves** (Slash, Razor Leaf, Crabhammer, Karate Chop) multiply
  this by ~8 (`sla b` twice), often capping near 255/256 (~99%).
- **Focus Energy** in Gen 1 is *bugged* — it **quarters** crit rate instead of
  boosting it. (Portable trap: don't recommend Focus Energy for more crits.)

Practical upshot: high-base-speed attackers (Mewtwo, Persian, Aerodactyl, Electrode,
Jolteon, etc.) crit *frequently* — a meaningful chunk of their hits. This explains a
lot of "sometimes it one-shots, sometimes it doesn't" behavior on threshold KOs, and
a crit ignores stat-stage modifiers (so it can punch through a Defense/Special boost).

---

## 5. The damage roll is 85–100%, top-heavy (not ±50%)

(Also in the parsing ref, but worth restating as a *portable strategy* fact:) Gen 1
damage variance is `× random(217–255) / 255` = **85% to 100% of base** — a tight,
one-sided band. This is why KO thresholds behave as near-binary switches:
- If **base damage ≥ target HP** → OHKO on essentially every roll (min roll is 85%,
  so once base ≥ HP/0.85 it's a *guaranteed* OHKO).
- If **base damage < target HP** → you leave a consistent sliver (never a wild
  under-roll to 50%).
So leveling/power creep converts "leaves 2% every time" into "always OHKO" over a
narrow range, cleanly. Useful mental model for "how much more do I need to one-shot X."

---

## 6. Emulation notes (portable, practical)

### Pizza Boy (recommended for tooling)
- Writes battery saves as accessible `.sav`/`.srm` files in a normal `ROMs/saves/`
  directory → reachable by scripts, file managers, sync tools. This is the big reason
  to prefer it over Lemuroid if you want to parse saves.
- **Fast-forward** is a toggle (turtle/rabbit icons on the on-screen control bar) with
  multiple speed tiers. Genuinely useful for grinding; see the FF caveat below.
- Save-state and battery-save are separate — for parsing you want the **battery save**
  (`.sav`), which is written when the game does an in-game SAVE, not the save-state.
  **Always do an in-game SAVE before reading the `.sav`**, or you'll parse stale data.

### Lemuroid (harder for tooling)
- Locks saves inside Android **scoped storage** (`Android/data/...`), which modern
  Android blocks from Termux, most file managers, and even Shizuku on Android 16.
  Workarounds (SAF grants via Material Files, copy-to-Download) are fiddle-prone.
  If you plan to script against saves, migrating to Pizza Boy sidesteps all of it.

### Fast-forward has a human-input floor (portable grinding insight)
- FF speeds up **game time**, not **your reaction time**. A battle ≈ ~12s of
  FF-scalable game-time (animations, text, walking) **+ ~3s of fixed human-time**
  (menu decisions, mashing). So effective speedup **saturates**: past ~4× FF, your
  thumbs dominate and cranking to 16× yields only ~4× real speedup, not 16×.
- **The daycare is the exception:** it's pure step-counting with **no human in the
  loop**, so it scales *linearly* with FF (walk a loop / hold a direction at max FF).
  This makes daycare + high FF competitive with — sometimes better than — active
  battling for grinding a single mon, and it avoids team attrition (no faints, no PP
  drain, no heal trips). Trade-off: a small ¥ cost and a level cap on hard mode.

---

## 7. Miscellaneous portable Legacy facts

- **Move reorder is done IN BATTLE**, not in the party/summary menu (vanilla Gen 1
  behavior Legacy keeps): at the FIGHT move list, press **Select** on a move (solid
  arrow appears), move the cursor, press **Select** again to swap. Persists after
  battle. Safe. (Other Select-in-menu tricks can corrupt memory — the FIGHT-menu
  reorder is the intended, safe one.)
- **Indigo Plateau lobby has an unlimited TM/item superstore** — a convenient spot to
  rebuy TMs and stock up between Elite Four runs (and to restore between rematches).
- **The Move Relearner is at Cinnabar Lab** (the Fossil/lab building) — re-teaches
  level-up moves a mon missed, for a fee. Useful when a mon skipped a move you want.
- **Bag capacity is 41 items** in Legacy (vs vanilla 20) — not just a save-offset
  concern (§ parsing ref), but a quality-of-life fact: you can carry far more.
- **Struggle** is move ID 165 and has its own entry; enemies with depleted PP will
  Struggle (self-recoil), which is exploitable via PP-stalling low-PP enemy movesets.
- **OHKO moves (Fissure/Horn Drill/Guillotine) fail against faster targets** in Gen 1
  (they check the speed relationship). So a fast enough mon is *immune* to an enemy's
  OHKO move — relevant for e.g. Nurse Joy's Fissure-Kangaskhan. Portable: lead fast
  vs OHKO-move users.
- **Type-chart specifics worth remembering** (Legacy = vanilla Gen 1 chart, with its
  known quirks): **Ghost is listed as "no effect" vs Psychic in the data ordering but
  Legacy treats Ghost as a special-category type that hits Psychic** — verify
  interactions in `type_matchups.asm` rather than trusting later-gen intuition. Bug/
  Poison, Ice/Fire, and Psychic's dominance are all the classic Gen 1 shapes.

---

## 8. Things to verify per-ROM-version (don't assume stability)

Legacy is actively maintained; between versions these can change, so
tooling/advice should re-derive them from the specific ROM the player uses:
- Exact move stats (power/PP/accuracy) — e.g. Thunder's PP was nerfed to 5.
- Evolution methods/levels (trade evos → level; the exact levels).
- Trainer rosters & special movesets (rematch teams, boss traps like Snorlax's
  Self-Destruct).
- Wild encounter tables and gift/event locations.
- Difficulty-mode branches.
- Level-cap thresholds.

The safe pattern: **treat the disassembly of the player's exact ROM version as ground
truth**, regenerate the data model from it, and don't hardcode values that a version
bump could move.
