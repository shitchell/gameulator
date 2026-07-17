# Gameulator Milestone 3 — Web Dashboard Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (or
> superpowers:subagent-driven-development) to implement this plan task-by-task.
> Design: `docs/plans/2026-07-17-milestone3-web-design.md`.

**Goal:** A browser dashboard (`gameulator-web`) that live-shows the current save —
trainer/playtime/party/bag/PC — by polling the `status.json` that `sync` writes,
reusing the `app` view DTOs, with composable Leptos components and a CSS design-token
system that follows the system light/dark theme.

**Architecture:** Two crates. `crates/web` = a Leptos CSR frontend (WASM, built by
`trunk`) that polls `/api/status` every 2s and reactively renders the deserialized
`StatusView`. `crates/web-server` = a small axum binary that serves the built frontend
assets + `/api/status` (reads the `status.json` file). The shared contract
(`StatusView` + member DTOs) lives in `app` with `Serialize + Deserialize`.

**Tech Stack:** Rust, Leptos 0.6 (CSR feature), `trunk` + `wasm32-unknown-unknown`,
`gloo-net` (fetch) + `gloo-timers` (poll), axum + tokio + tower-http (backend),
serde/serde_json, hand-rolled token CSS.

**Governing principle (user):** composable components + "highly variable'd CSS" — a
single token block is the ONE restyle point; components style only via `var(--token)`.

---

## Conventions
- Pure-Rust tasks (DTO Deserialize, the `/api/status` handler) are **TDD**. The
  Leptos/WASM view tasks are verified by **compiling to WASM** (`trunk build`) + a
  **cdp screenshot** in Brave (DOM unit-testing WASM is out of scope for v1).
- Commit after each green/verified step. `cargo test --workspace` stays green; the
  WASM frontend is checked with `cargo check --target wasm32-unknown-unknown -p web`
  and `trunk build`.
- The frontend calls NO `app` functions — only its DTO *types*. Bundle-size from
  pulling in `app`+`pokegen1` is accepted for v1 (DCE strips the unused logic/data).

---

## Task 1: Shared `StatusView` in `app` + `Deserialize` on the view DTOs

**Files:**
- Modify: `crates/app/src/lib.rs` (add `Deserialize` to the DTO derives; add `StatusView`)
- Modify: `crates/sync/src/status.rs` (use `app::StatusView`; drop the local def)

**Step 1 (TDD):** In `app`, add `Deserialize` alongside `Serialize` on `Condition`,
`PartyMemberView`, `MoveView`, `ItemView`, `SaveInfoView`. Move `StatusView` here from
`sync/status.rs`:
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusView {
    pub trainer: String,
    pub playtime: Playtime,          // pokegen1::Playtime already derives both
    pub checksum_ok: bool,
    pub party: Vec<PartyMemberView>,
    pub last_change: String,
    pub snapshot: Option<String>,
}
```
Add a round-trip test: build a `StatusView`, `serde_json::to_string` → `from_str` →
assert equal. (Proves the browser can reconstruct it.)

**Step 2:** `sync/status.rs`: `use app::StatusView;` and remove its local struct; its
`write_status` builds `app::StatusView` (fields unchanged). Keep sync's tests green.

**Step 3:** `cargo test -p app -p sync` green; `cargo test --workspace` green; clippy
clean.

**Step 4: Commit** `feat(app): shared StatusView + Deserialize on view DTOs`.

---

## Task 2: Scaffold `web-server` (axum) + `WebConfig` + toolchain

**Files:**
- Create: `crates/web-server/{Cargo.toml, src/main.rs, src/lib.rs}`
- Modify: root `Cargo.toml` (add `crates/web-server` member; add `axum = "0.7"`,
  `tokio = { version = "1", features = ["rt-multi-thread","macros"] }`,
  `tower-http = { version = "0.5", features = ["fs"] }` to `[workspace.dependencies]`)

**Step 1:** Toolchain (additive, log to `/usr/share/CHANGELOG.md`):
`rustup target add wasm32-unknown-unknown` and `cargo install trunk` (needed by Task 5).

**Step 2:** `web-server` crate: bin `gameulator-web`; deps axum/tokio/tower-http/
anyhow/clap/serde (workspace). `WebConfig { status_path: PathBuf, dist_dir: PathBuf,
port: u16, poll_ms: u32 }` with a `Default`/`from_args` (clap): `--status-path`
(default `games/Pokemon/Yellow Legacy/saves/status.json`), `--dist-dir` (default
`crates/web/dist`), `--port` (default 8770), `--poll-ms` (default 2000, exposed to the
frontend — see Task 6).

**Step 3:** `main.rs` stub: parse `WebConfig`, print it, `Ok(())` (server wired in
Tasks 3-4). `cargo build -p web-server` succeeds.

**Step 4: Commit** `chore(web-server): scaffold axum crate + WebConfig`.

---

## Task 3: `/api/status` handler (TDD)

**Files:** `crates/web-server/src/lib.rs` (a `status_handler` + a `router(cfg)` fn).

**Step 1 (TDD):** `router(cfg) -> axum::Router`. `GET /api/status`:
- if `cfg.status_path` exists → read it, return `200` with body = the file bytes and
  `content-type: application/json`.
- if absent → return `200` with a small empty-state JSON `{"present": false}` (NOT a
  500/404 — the watcher may not have written it yet; the frontend shows "waiting").
Test with `axum::body`/`tower::ServiceExt::oneshot`: seed a temp `status.json` → assert
200 + body contains the seeded content; absent → 200 + `present: false`.

**Step 2-4:** Implement, green, clippy clean. **Commit** `feat(web-server): /api/status handler`.

---

## Task 4: Serve the frontend assets (SPA static + fallback)

**Files:** `crates/web-server/src/lib.rs` (extend `router`), `main.rs` (serve + run).

**Step 1:** Add `tower_http::services::ServeDir` on `cfg.dist_dir` with a fallback to
`dist_dir/index.html` (so the SPA loads at `/`). Keep `/api/status` taking precedence.
**Step 2:** `main.rs`: build the router, bind `0.0.0.0:{port}`, `axum::serve`, print
the URL. Also expose `GET /api/config` returning `{"poll_ms": cfg.poll_ms}` so the
frontend reads the interval (one config source).
**Step 3:** Test the router serves a seeded `dist/index.html` at `/` (oneshot).
**Step 4:** `cargo test -p web-server` green. **Commit** `feat(web-server): serve SPA assets + /api/config`.

---

## Task 5: Scaffold the Leptos CSR frontend + verify WASM build

**Files:**
- Create: `crates/web/{Cargo.toml, index.html, Trunk.toml, src/main.rs}`, `crates/web/style.css`
- Modify: root `Cargo.toml` members (add `crates/web`) — NOTE: `web` targets wasm; make
  sure the workspace still builds natively (it's a separate target; `cargo build`
  skips wasm-only crates unless targeted, but confirm no native-build breakage — if the
  leptos csr deps break `cargo build --workspace`, exclude `web` from default members
  or gate it; report what you did).

**Step 1:** `web` Cargo.toml: `leptos = { version = "0.6", features = ["csr"] }`,
`app = { path = "../app" }`, `gloo-net = "0.6"`, `gloo-timers = { version = "0.3",
features=["futures"] }`, `serde_json`, `wasm-bindgen`, `console_error_panic_hook`.
`index.html` (trunk entry linking `style.css`), `Trunk.toml` (dist dir = `dist`).

**Step 2:** `src/main.rs`: `console_error_panic_hook`, `leptos::mount_to_body(|| view!
{ <Dashboard/> })`; a placeholder `<Dashboard>` returning `view! { <h1>"Gameulator"</h1> }`.

**Step 3 (verify):** `cargo check -p web --target wasm32-unknown-unknown` compiles;
`trunk build` (from `crates/web`) produces `dist/`. Report the dist contents.

**Step 4: Commit** `chore(web): scaffold Leptos CSR frontend (compiles to WASM)`.

---

## Task 6: Fetch + 2s poll → `StatusView` signal

**Files:** `crates/web/src/main.rs` (or `src/dashboard.rs`).

**Step 1:** In `<Dashboard>`: a `create_signal::<Option<app::StatusView>>` and a
loading/error/empty state. Fetch `/api/status` via `gloo_net::http::Request::get`,
parse the body: `{"present": false}` → empty-state; else `serde_json::from_str::<app::StatusView>`.
Poll on an interval (read `/api/config`'s `poll_ms`, default 2000) via
`gloo_timers::callback::Interval` (or a leptos effect) — re-fetch + update the signal.
On deserialize/network error, show a small error line but keep polling.

**Step 2 (verify):** `trunk build` compiles. (Runtime verified in Task 9's screenshot.)

**Step 3: Commit** `feat(web): poll /api/status into a StatusView signal`.

---

## Task 7: Composable components (typed props from the DTOs)

**Files:** `crates/web/src/components/{mod,info_header,party_card,hp_bar,status_badges,move_list,item_list}.rs`.

**Step 1:** Build small components, each taking typed props from the DTOs, each styled
ONLY via CSS classes (styles live in Task 8's token CSS — components carry NO inline
colors/sizes):
- `<InfoHeader trainer playtime checksum_ok/>` — playtime as `{h}h {m}m`, a checksum
  badge (class `ok`/`bad`).
- `<PartyCard mon: PartyMemberView/>` → renders name (`nick (species)` or species),
  `Lv{level}`, `<HpBar hp max_hp/>`, stats, `<StatusBadges .../>`, `<MoveList moves/>`.
- `<HpBar hp max_hp/>` — a `<div class="hp-bar">` with an inner width `%` and a class
  by ratio (`hp-high`/`hp-mid`/`hp-low`) so color comes from tokens.
- `<StatusBadges status: Vec<Condition>/>` — one badge per condition; render
  `Condition` → label HERE (Sleep(n)/POISON/… — the view owns the label mapping, same
  rule as the CLI); class per condition for token coloring. Empty → nothing.
- `<MoveList moves/>` — `name (pp)` per move.
- `<ItemList title items/>` — reused for bag + PC.
- `<Dashboard>` composes: `<InfoHeader/>`, a `<PartyGrid>` of `<PartyCard>`, then bag +
  PC `<ItemList>`. Empty-state + error line preserved.

**Step 2 (verify):** `trunk build` compiles.
**Step 3: Commit** `feat(web): composable party/info/item components`.

---

## Task 8: CSS design-token system + system light/dark

**Files:** `crates/web/style.css`.

**Step 1:** A single `:root { }` token block (LIGHT defaults): background/surface/text
colors, `--accent`, `--hp-high/--hp-mid/--hp-low`, per-status colors (`--st-sleep`,
`--st-poison`, `--st-burn`, `--st-freeze`, `--st-paralyze`, `--st-fainted`), `--gap`,
`--radius`, `--font`, font sizes, card shadow. Then ONE
`@media (prefers-color-scheme: dark) { :root { /* dark overrides of the SAME tokens */ } }`.
Every component rule styles via `var(--token)` — NO hard-coded colors/sizes anywhere
else. The HP-bar/status/checksum classes pull their color from the tokens.

**Step 2 (verify):** `trunk build` compiles; the CSS is linked from `index.html`.
**Step 3: Commit** `feat(web): token CSS + system light/dark theme`.

---

## Task 9: End-to-end run + cdp screenshot verification

**Files:** none (verification) — may add a `Makefile`/README `web` target.

**Step 1:** `trunk build` (frontend → `crates/web/dist`). Launch the server:
`cargo run -p web-server --bin gameulator-web -- --status-path <a seeded status.json>`.
(Seed a `status.json` by running the real save through the pipeline, OR copy the one
`gameulator-sync` produced from the real save, OR generate via `app`/`sync`.)
**Step 2:** Screenshot in Brave via cdp: `cdp navigate <tab> http://localhost:8770`
then `cdp screenshot <tab> /tmp/gameulator-web.png`, and Read the PNG to eyeball the
party render (light AND — if togglable via OS — dark). Confirm the party (MEWTWO L100
etc.), playtime, checksum badge, and bag/PC render.
**Step 3:** Note this is the hand-off point for tweaking styling together. Add a
`make web` (build frontend + run server) convenience target. **Commit** `build(web): make web target + e2e verification`.

---

## Task 10: Milestone-3 wrap
**Step 1:** README "Web dashboard (Milestone 3)" section: `make web` / the two-crate
build (`trunk build` in `crates/web`, then `cargo run -p web-server`), the token-CSS
restyle point, system light/dark. Update §Status (M3 done: live poll dashboard;
WebSocket/dex-grid/etc. deferred).
**Step 2:** CHANGELOG entry for the wasm target + trunk install.
**Step 3:** Verify: `cargo test --workspace` green; `cargo clippy --workspace
--all-targets` clean; `cargo fmt --all --check` clean; `cargo check -p web --target
wasm32-unknown-unknown` clean; `trunk build` OK.
**Step 4: Commit** `docs: Milestone 3 web dashboard usage + wrap`.

---

## Notes for the executor
- **Components carry NO colors/sizes** — every visual value is a `var(--token)`; the
  ONE token block (Task 8) is the restyle point. This is the user's core M3 ask.
- **Light/dark = token values only** (a `prefers-color-scheme` media query overriding
  the same tokens); no component or JS changes to re-theme.
- **The frontend reuses `app::StatusView`** — never re-defines the shape. `sync` writes
  it; the browser reads it; `app` owns it.
- **WASM tasks are verified by compile + screenshot**, not DOM unit tests. The cdp
  screenshot (Task 9) needs the user's Brave with remote debugging (per their setup) —
  this is the natural "test/tweak together" hand-off.
- Deferred (not M3): WebSocket push, manual theme toggle, dex/coverage/timeline views,
  client-side WASM `.sav` parsing.
