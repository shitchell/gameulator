# Gameulator build targets.
# RGBDS: path to a locally-built RGBDS 0.6.1 toolchain (trailing slash required —
# the disassembly's makefile prepends $(RGBDS) directly to tool names).
# Single edit point for the toolchain location.
RGBDS ?= $(HOME)/.local/src/rgbds/

SUBMODULE := vendor/pokemon-yellow-legacy
ROM       := $(SUBMODULE)/pokeyellow.gbc
GAME_DIR  := games/Pokemon/Yellow Legacy/rom

.PHONY: rom

# Build the Yellow Legacy ROM from the pinned disassembly submodule and copy it
# into the (gitignored) game rom directory.
rom:
	$(MAKE) -C $(SUBMODULE) RGBDS=$(RGBDS)
	mkdir -p "$(GAME_DIR)"
	cp $(ROM) "$(GAME_DIR)/"
