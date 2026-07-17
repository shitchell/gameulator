import re, os, glob, json

ROOT = "/home/claude/yl"

# ── type chart straight from Legacy's own table ───────────────────────
TN = {"NORMAL":"Normal","FIGHTING":"Fighting","FLYING":"Flying","POISON":"Poison",
      "GROUND":"Ground","ROCK":"Rock","BUG":"Bug","GHOST":"Ghost","FIRE":"Fire",
      "WATER":"Water","GRASS":"Grass","ELECTRIC":"Electric","PSYCHIC_TYPE":"Psychic",
      "ICE":"Ice","DRAGON":"Dragon"}
MULT = {"SUPER_EFFECTIVE":2.0,"NOT_VERY_EFFECTIVE":0.5,"NO_EFFECT":0.0}
CHART = {}
for ln in open(f"{ROOT}/data/types/type_matchups.asm"):
    m = re.match(r"\s*db (\w+),\s*(\w+),\s*(\w+)", ln)
    if m and m.group(1) in TN and m.group(2) in TN:
        CHART.setdefault(TN[m.group(1)], {})[TN[m.group(2)]] = MULT[m.group(3)]

# ── category: Legacy moved GHOST into the SPECIAL block ───────────────
tc = open(f"{ROOT}/constants/type_constants.asm").read()
order, special_at = [], None
for i, ln in enumerate(tc.splitlines()):
    c = re.match(r"\s*const (\w+)", ln)
    if c: order.append((i, c.group(1)))
    if "DEF SPECIAL EQU" in ln: special_at = i
SPECIAL_TYPES = {TN[n] for i, n in order if special_at and i > special_at and n in TN}
PHYS_TYPES    = {TN[n] for i, n in order if special_at and i < special_at and n in TN}

# ── moves ─────────────────────────────────────────────────────────────
TMS = re.findall(r"add_tm\s+(\w+)", open(f"{ROOT}/constants/item_constants.asm").read())
HMS = re.findall(r"add_hm\s+(\w+)", open(f"{ROOT}/constants/item_constants.asm").read())
TMHM = {m: f"TM{i:02d}" for i, m in enumerate(TMS, 1)}
TMHM.update({m: f"HM{i:02d}" for i, m in enumerate(HMS, 1)})

MOVES = {}
for ln in open(f"{ROOT}/data/moves/moves.asm"):
    m = re.match(r"\s*move\s+(\w+),\s*(\w+),\s*(\d+),\s*(\w+),\s*(\d+),\s*(\d+)", ln)
    if m:
        n, eff, pw, ty, acc, pp = m.groups()
        t = TN.get(ty, ty.capitalize())
        MOVES[n] = dict(name=n, effect=eff, power=int(pw), type=t, acc=int(acc), pp=int(pp),
                        tm=TMHM.get(n, ""),
                        cat="Special" if t in SPECIAL_TYPES else "Physical")

# ── pokemon ───────────────────────────────────────────────────────────
POKE = {}
for f in glob.glob(f"{ROOT}/data/pokemon/base_stats/*.asm"):
    txt = open(f).read()
    dex = re.search(r"db DEX_(\w+)", txt).group(1)
    st = re.search(r"db\s+(\d+),\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+)", txt).groups()
    t1, t2 = re.search(r"db (\w+), (\w+) ; type", txt).groups()
    tm = re.search(r"tmhm (.*?)\n\t; end", txt, re.S)
    tml = [x.strip() for x in tm.group(1).replace("\\","").replace("\n","").split(",") if x.strip()] if tm else []
    hp, atk, df, sp, spc = map(int, st)
    POKE[dex] = dict(key=os.path.basename(f)[:-4], hp=hp, atk=atk, defense=df, spd=sp, spc=spc,
                     t1=TN.get(t1, t1.capitalize()), t2="" if t2 == t1 else TN.get(t2, t2.capitalize()),
                     tmhm=tml, levelup=[], evos=[])

em = open(f"{ROOT}/data/pokemon/evos_moves.asm").read()
FIX = {"NIDORANM":"NIDORAN_M","NIDORANF":"NIDORAN_F","MRMIME":"MR_MIME"}
for b in re.finditer(r"^(\w+)EvosMoves:\n(.*?)\n\tdb 0\n(.*?)\n\tdb 0", em, re.S|re.M):
    nm, ev, lr = b.groups()
    k = FIX.get(nm.upper(), nm.upper())
    if k in POKE:
        POKE[k]["levelup"] = [x.group(2) for x in re.finditer(r"db (\d+), (\w+)", lr)]
        POKE[k]["evos"] = [l.strip() for l in ev.splitlines() if "EVOLVE" in l]
for f in glob.glob(f"{ROOT}/data/pokemon/base_stats/*.asm"):
    txt = open(f).read()
    dex = re.search(r"db DEX_(\w+)", txt).group(1)
    l1 = re.search(r"db ([\w, ]+) ; level 1 learnset", txt)
    if l1:
        for mv in [x.strip() for x in l1.group(1).split(",")]:
            if mv != "NO_MOVE" and mv not in POKE[dex]["levelup"]:
                POKE[dex]["levelup"].append(mv)

# ── BOSS ROSTERS straight from Legacy's trainer parties ───────────────
pt = open(f"{ROOT}/data/trainers/parties.asm").read()
def parties(label):
    b = re.search(rf"^{label}:\n(.*?)(?=\n\w+Data:|\Z)", pt, re.S | re.M)
    out = []
    for ln in b.group(1).splitlines():
        ln = ln.split(";")[0].strip()
        if not ln.startswith("db "): continue
        toks = [t.strip() for t in ln[3:].split(",") if t.strip() and t.strip() != "0"]
        if not toks: continue
        if toks[0] == "$FF":                    # per-mon levels
            team = [(int(toks[i]), toks[i+1]) for i in range(1, len(toks)-1, 2)]
        else:                                    # uniform level
            lv = int(toks[0]); team = [(lv, s) for s in toks[1:]]
        if team: out.append(team)
    return out

GYMS = [("Brock","Gym 1","BrockData",0),("Misty","Gym 2","MistyData",0),
        ("Lt. Surge","Gym 3","LtSurgeData",0),("Erika","Gym 4","ErikaData",0),
        ("Koga","Gym 5","KogaData",0),("Sabrina","Gym 6","SabrinaData",0),
        ("Blaine","Gym 7","BlaineData",0),("Giovanni","Gym 8","GiovanniData",-1),
        ("Lorelei","Elite Four","LoreleiData",0),("Bruno","Elite Four","BrunoData",0),
        ("Agatha","Elite Four","AgathaData",0),("Lance","Elite Four","LanceData",0)]
BOSSES = []
for name, stage, lab, idx in GYMS:
    try: ps = parties(lab)
    except AttributeError: print("MISSING", lab); continue
    team = ps[idx] if ps else []
    for lv, sp in team:
        sp = sp.replace("NIDORANM","NIDORAN_M").replace("NIDORANF","NIDORAN_F")
        if sp in POKE:
            BOSSES.append((name, stage, sp, lv, POKE[sp]["t1"], POKE[sp]["t2"]))
        else: print("  ?? unknown species", sp)

print("=== YELLOW LEGACY vs VANILLA ===")
print(f"types where Ghost is: {'SPECIAL' if 'Ghost' in SPECIAL_TYPES else 'PHYSICAL'}")
print(f"Ghost -> Psychic: {CHART['Ghost'].get('Psychic',1.0)}x   (vanilla: 0x)")
print(f"Bug   -> Poison : {CHART['Bug'].get('Poison',1.0)}x   (vanilla: 2x)")
print(f"moves: {len(MOVES)}  TMs: {len(TMS)}  HMs: {len(HMS)}  pokemon: {len(POKE)}")
print(f"boss pokemon parsed: {len(BOSSES)}\n")
for name, stage, _, _ in GYMS:
    t = [(b[2], b[3]) for b in BOSSES if b[0] == name]
    print(f"  {name:10} {', '.join(f'{s.title()} L{l}' for s, l in t)}")

json.dump({"chart":CHART,"moves":MOVES,"poke":POKE,"bosses":BOSSES,
           "special":sorted(SPECIAL_TYPES),"phys":sorted(PHYS_TYPES),
           "tms":TMS,"hms":HMS}, open("/home/claude/legacy.json","w"))
print("\nsaved legacy.json")
