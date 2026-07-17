# Gameulator — Vision & Roadmap

> Living document. Captures where this is headed so we build *toward* it deliberately.
> Companion: `docs/plans/2026-07-17-gameulator-design.md` (architecture),
> `docs/plans/2026-07-17-gameulator-milestone1-plan.md` (M1, done).

## The north star (user's framing)

> "it should provide an API that allows a user to do basically anything imaginable.
> it doesn't have to expose functions that directly answer each of these, but it
> *should* expose API functions by which a user *can* answer (from the ROM / save
> files directly, esp in case any update to a romhack like Yellow Legacy changes
> something)"

The library's job is **structured, ground-truth access to everything in the ROM +
save**, so that *any* question can be computed — and stays correct across romhack
version bumps (regenerate, don't rewrite). Pre-baked answers are optional
conveniences on top; the raw access is the point.

## The two layers

**Layer 1 — Data.** Structured access to all ROM + save data, exposed through
**trait seams** (`pokegen1::core::data`) and filled by game overlays whose tables
are generated from the pinned disassembly (`xtask`). This is what makes it
version-proof. It grows by **adding new traits** (convention from the Task-11
review: *new capability = new trait, never new methods on an existing one*), each
fed by an `xtask` extractor. Today: save state (party/bag/PC/trainer/playtime) +
species/move/item name tables.

**Layer 2 — Analysis.** Computations built *on* Layer 1, as focused crates so they
never bloat the core (e.g. `pokebattle`, `pokemap`, `pokedex-analysis`). The
"anything imaginable" answers live here.

### How the north-star examples decompose (proof the framework is expandable)

| Question | Layer-1 data to extract (new traits + xtask) | Layer-2 analysis |
|---|---|---|
| What Pokémon have I seen? | save Pokédex seen/owned bitfields | diff |
| Min-level Mewtwo to beat E4 (+ Legacy rematch)? | base stats, full move data, type chart, learnsets, trainer rosters (+rematch), badge boosts | **battle simulator/solver** (search over level × movesets) |
| Longest contiguous straight path with no wild encounters? | map tiles/collision, map connections, wild-encounter zones | overworld **graph + path search** |
| Uncaught Pokémon + where to find them? | save owned flags + wild-encounter/gift/trade/evo tables | diff + location lookup |
| Render a PNG of Seafoam B1F | map blocks + tilesets + 2bpp tile graphics | tile compositor → `image` crate |
| Shortest Seafoam boulder/switch puzzle solution | map layout + object/script data | **state-space BFS solver** |
| Where can I get TM23 (incl. bag + PC)? | bag+PC (done) + mart/field/hidden/gift item-source tables | source query |

Every one is **additive**: extend Layer-1 extraction + add a Layer-2 computation.
None requires touching the core parse or the existing traits.

## Data sourcing (design area — schema TBD, seam decided)

**The seam is already in place:** consumers talk to traits; the provider resolves the
data. `app::game_data(GameId)` is the single construction chokepoint. The future
source-resolution precedence lives there (one place to change; all call sites
untouched):

```
1. config-override ROM   (user points at any ROM — always takes precedence)
2. cached data files     (content-addressed JSON, keyed by md5 — works without a ROM)
3. bundled ROM / disassembly   (source of truth)
```

Today the overlay resolves from bundled `include_str!` generated JSON — i.e. the
"cached data, works without the ROM" case already works. Adding ROM-direct parsing
and the config override is a *provider-internal* change later.

**Cache format — OPEN, to discuss before building.** Two candidates the user raised:
- **(A)** one md5-keyed JSON blob of everything unique to a byte-specific ROM version.
- **(B)** *(current lean)* content-addressed per-domain files, e.g.
  `data/tms/<md5>.json`, plus a manifest mapping each ROM (name + version string +
  ROM md5) → the domain files it uses. DRYer (versions sharing identical data point
  to the same file), diff-friendly, and shows at a glance what changes between
  versions — which serves the locality principle. User: *"a teensy bit dryer, reduce
  some data, and let us see at more of a glance which versions share which of the same
  infos and what changes between version."*

Storage location (crates in-repo vs a separate data repo) is likewise deferred —
"wherever," decide when we build it. What matters now: **the trait indirection keeps
all of this swappable without downstream refactoring.**

Cacheable data the user called out explicitly: TM list + effects, map info, learnsets,
and math functions (e.g. catch-probability formulas) — so *some or all* functionality
can exist without the ROM.

## Milestone sequencing (rough)

- **M1 — DONE.** Save parser (`pokegen1`) + overlay + `app` controller + `gameulator`
  CLI. Names resolved from disassembly-generated JSON.
- **M2 — Sync (current plan).** Syncthing transport + a Rust `sync` watcher
  (validate/snapshot/regression-alarm/status.json). See
  `docs/plans/2026-07-17-milestone2-sync-design.md`.
- **M3 — Web view.** Leptos/WASM dashboard over the `app` DTOs (user wants sooner).
- **Then — Library expansion toward the north star** (user: *"i'd love to start
  working on [it] as soon as we have our v1 / current plan finished... i will want to
  start working towards it straight away"*). Sequence Layer-1 data domains + Layer-2
  capabilities; formalize the data-source-resolution provider + config override; settle
  the cache schema (A vs B) when we get there. Candidate first steps: Pokédex
  seen/owned flags (small, high-value), full move/base-stats/type-chart/learnset
  extraction (unlocks the battle solver), then map extraction (unlocks path/puzzle/render).

## Governing principles (carry into everything)

- **Disassembly = ground truth**, regenerate on version bump — never hardcode a value
  a ROM update could move.
- **One conceptual change = one contiguous edit block** (compiled + modular).
- **New capability = new trait**, never new methods on existing traits — keeps
  name-only consumers from depending on stat/map resolution.
- **Expose access, not just answers** — the API should let a user compute things we
  never anticipated.
