# Gameulator build targets.
# RGBDS: path to a locally-built RGBDS 0.6.1 toolchain (trailing slash required —
# the disassembly's makefile prepends $(RGBDS) directly to tool names).
# Single edit point for the toolchain location.
RGBDS ?= $(HOME)/.local/src/rgbds/

SUBMODULE := vendor/pokemon-yellow-legacy
ROM       := $(SUBMODULE)/pokeyellow.gbc
GAME_DIR  := games/Pokemon/Yellow Legacy/rom

.PHONY: rom web

# Build the Yellow Legacy ROM from the pinned disassembly submodule and copy it
# into the (gitignored) game rom directory.
# Preflight: the disassembly requires exactly RGBDS 0.6.1 — newer versions break
# the build — so assert the version before doing anything.
rom:
	@"$(RGBDS)rgbasm" --version | grep -q '0.6.1' || { echo "ERROR: need RGBDS 0.6.1 at RGBDS=$(RGBDS) (got: $$("$(RGBDS)rgbasm" --version 2>/dev/null || echo none)). Newer versions break the build."; exit 1; }
	$(MAKE) -C "$(SUBMODULE)" RGBDS="$(RGBDS)"
	mkdir -p "$(GAME_DIR)"
	cp "$(ROM)" "$(GAME_DIR)/"

# Run the web dashboard end-to-end: build the WASM frontend (release) then serve
# it over the real save's status.json. Run from the repo root — the server's
# relative default paths resolve there. Blocks (Ctrl-C to stop); interactive use
# only, not part of the build.
web:
	cd crates/web && trunk build --release
	cargo run -p web-server --bin gameulator-web -- \
		--dist-dir crates/web/dist \
		--status-path "games/Pokemon/Yellow Legacy/saves/status.json"
