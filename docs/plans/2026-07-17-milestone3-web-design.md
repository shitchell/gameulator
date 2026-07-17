# Gameulator Milestone 3 — Web Dashboard (Design)

> Status: **validated in brainstorming (2026-07-17), ready for an implementation plan**
> Companion: `docs/plans/2026-07-17-gameulator-design.md` §3 (WASM-first web view).

**Goal:** A browser dashboard that shows the current Yellow Legacy save (trainer,
playtime, party, bag/PC) live — "save on your phone, watch it update on your
computer" — reusing the exact `app` DTOs that `sync` writes to `status.json`.

## Stack & data flow
- **`crates/web`** — a **Leptos** app compiled to **WASM** (client-side rendered) +
  an **axum** binary `gameulator-web`.
- axum: `GET /api/status` reads `status.json` and returns it (404/empty-state if
  absent); also serves the built WASM/HTML/CSS assets.
- The Leptos app **polls `/api/status` every ~2s**, deserializes `StatusView`
  (reusing the `app` DTOs — M3 adds `Deserialize` to them, the M2-deferred item),
  and reactively re-renders on change. WebSocket push is deferred — 2s polling is
  invisible for this use and far simpler (no server-side file watch).
- **Config-driven:** a small `WebConfig { status_path, port, poll_ms }`, env/CLI
  overridable — one place to tune runtime behavior.

## Composable components (each its own file, independently tweakable)
The whole point (user: "i am ever unsure what i want until i see it… i like configs
and composable components and highly variable'd css so we can easily swap things"):
- `<Dashboard>` — owns the fetch + 2s poll signal; passes typed props down.
- `<InfoHeader>` — trainer, playtime, a checksum badge.
- `<PartyGrid>` → `<PartyCard>` (per mon) → `<HpBar>`, `<StatusBadges>`, `<MoveList>`.
- `<ItemList>` — reused for bag and PC.
Each takes typed props straight from the DTOs (`PartyMemberView`, `MoveView`,
`ItemView`, `SaveInfoView`). Adding/reordering the dashboard = compose differently;
no component internals change.

## CSS as design tokens (the single restyle point) + light/dark
- **One token block** drives everything: `:root { --card-bg; --text; --accent;
  --hp-high; --hp-mid; --hp-low; per-status colors; per-type colors; --gap;
  --radius; --font; sizes; ... }`.
- **Every component styles ONLY via `var(--token)`** — never a hard-coded color/size.
  So a reskin = edit the token block; a component never changes to re-theme.
- **Light/dark follows the system theme** (dogfoods the tokens): light values are the
  `:root` default; a single `@media (prefers-color-scheme: dark) { :root { ... } }`
  overrides the token *values*. The UI re-themes with the OS automatically — no JS,
  no component edits. (A manual toggle can layer on later via a `[data-theme]`
  attribute overriding the same tokens.)
- A `themes/` seam (later): swappable token sets, since all theming is token values.

## Testing
- **Pure Rust tests:** `Deserialize` round-trip on the `app` DTOs / `StatusView`
  (serialize → deserialize → equal); the axum `/api/status` handler (serves a seeded
  file; empty-state when absent — no 500).
- **Leptos view:** build-to-WASM must compile; then a **cdp screenshot** in Brave
  against a seeded `status.json` for a visual check — after which we tweak the tokens
  and layout together live (the whole reason for the token system).
- The reactive party/HP-bar rendering is verified visually (screenshot), not unit-
  tested (WASM DOM testing is out of scope for v1).

## Toolchain (additive, logged to CHANGELOG)
- `rustup target add wasm32-unknown-unknown`
- `trunk` (`cargo install trunk`) — the WASM bundler/dev-server Leptos CSR uses.

## Prerequisites / M2 carry-ins
- Add `Deserialize` (alongside `Serialize`) to the `app` DTOs (`StatusView`'s
  members: `PartyMemberView`, `MoveView`, `ItemView`, `Condition`, `SaveInfoView`) so
  the browser can reconstruct `StatusView` from `status.json`. `Playtime` already
  derives it.
- `status.json`'s `snapshot` field is an absolute host path (noted in M2); the web
  view should ignore it or show just the filename — don't surface the host path.

## Deferred (not M3 v1)
- WebSocket push (poll is v1).
- Manual light/dark toggle (system-follow is v1).
- Dex grid, type-coverage heatmap, run timeline (need data domains not yet extracted
  — that's the library-expansion roadmap).
- Serving the raw `.sav` bytes for client-side WASM parsing (v1 reads the
  already-parsed `status.json`; the pure lib → WASM path is a future enhancement).
