#!/data/data/com.termux/files/usr/bin/bash
# pokedump.sh — dump current Pizza Boy save + diff against the previous dump.
#
# Workflow: SAVE in-game (Pizza Boy writes the .sav), then run:  ./pokedump.sh
# Produces a timestamped readout in $DUMPDIR and shows/copies the diff vs last time.

set -euo pipefail

# ---- config: edit these two if paths change ----
SAVE="$HOME/storage/shared/ROMs/saves/Pokemon Yellow Legacy (v1.0.10).sav"
PARSER="$HOME/bin/srmread-pokemon-yellow.py"
DUMPDIR="$HOME/storage/shared/ROMs/pokedumps"
# ------------------------------------------------

mkdir -p "$DUMPDIR"

if [[ ! -f "$SAVE" ]]; then
    echo "!! save not found: $SAVE" >&2
    echo "   (did you save in-game in Pizza Boy first?)" >&2
    exit 1
fi

stamp="$(date +%Y-%m-%d_%H%M%S)"
out="$DUMPDIR/$stamp.txt"

# generate the readout
python3 "$PARSER" "$SAVE" --full > "$out"
echo "wrote $out"

# find the two most recent dumps (timestamp names sort chronologically)
mapfile -t recent < <(printf '%s\n' "$DUMPDIR"/*.txt | sort | tail -n 2)

if [[ "${#recent[@]}" -lt 2 ]]; then
    echo "(first dump — nothing to diff against yet)"
    exit 0
fi

echo
echo "=== diff: $(basename "${recent[0]}") -> $(basename "${recent[1]}") ==="
# diff returns 1 when files differ; don't let set -e kill us
diff "${recent[0]}" "${recent[1]}" | tee >(termux-clipboard-set) || true
echo
echo "(diff copied to clipboard)"
