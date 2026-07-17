#!/usr/bin/env python3
"""
Pokemon Yellow LEGACY save reader (also works on vanilla R/B/Y).

Usage:
  python3 read_save.py "game.srm"          # party + notable items (bag & PC)
  python3 read_save.py "game.srm" --full   # + every single item in bag and PC

Reliable fields: trainer name, party (species/level/stats/moves), bag, PC items, playtime.
"""
import sys

O_NAME=0x2598
O_BAGDATA=0x25CA
O_PT_H=0x2CED; O_PT_M=0x2CEF
O_PCOUNT=0x2F2C; O_PARTY=0x2F34; O_NICK=0x307E
O_PC=0x2834            # PC stored items (count + item/qty pairs + 0xFF)
STRUCT=44; LEVEL_OFF=0x21

MON={1: 'RHYDON', 2: 'KANGASKHAN', 3: 'NIDORAN♂', 4: 'CLEFAIRY', 5: 'SPEAROW', 6: 'VOLTORB', 7: 'NIDOKING', 8: 'SLOWBRO', 9: 'IVYSAUR', 10: 'EXEGGUTOR', 11: 'LICKITUNG', 12: 'EXEGGCUTE', 13: 'GRIMER', 14: 'GENGAR', 15: 'NIDORAN♀', 16: 'NIDOQUEEN', 17: 'CUBONE', 18: 'RHYHORN', 19: 'LAPRAS', 20: 'ARCANINE', 21: 'MEW', 22: 'GYARADOS', 23: 'SHELLDER', 24: 'TENTACOOL', 25: 'GASTLY', 26: 'SCYTHER', 27: 'STARYU', 28: 'BLASTOISE', 29: 'PINSIR', 30: 'TANGELA', 31: 'MISSINGNO.', 32: 'MISSINGNO.', 33: 'GROWLITHE', 34: 'ONIX', 35: 'FEAROW', 36: 'PIDGEY', 37: 'SLOWPOKE', 38: 'KADABRA', 39: 'GRAVELER', 40: 'CHANSEY', 41: 'MACHOKE', 42: 'MR.MIME', 43: 'HITMONLEE', 44: 'HITMONCHAN', 45: 'ARBOK', 46: 'PARASECT', 47: 'PSYDUCK', 48: 'DROWZEE', 49: 'GOLEM', 50: 'MISSINGNO.', 51: 'MAGMAR', 52: 'MISSINGNO.', 53: 'ELECTABUZZ', 54: 'MAGNETON', 55: 'KOFFING', 56: 'MISSINGNO.', 57: 'MANKEY', 58: 'SEEL', 59: 'DIGLETT', 60: 'TAUROS', 61: 'MISSINGNO.', 62: 'MISSINGNO.', 63: 'MISSINGNO.', 64: "FARFETCH'D", 65: 'VENONAT', 66: 'DRAGONITE', 67: 'MISSINGNO.', 68: 'MISSINGNO.', 69: 'MISSINGNO.', 70: 'DODUO', 71: 'POLIWAG', 72: 'JYNX', 73: 'MOLTRES', 74: 'ARTICUNO', 75: 'ZAPDOS', 76: 'DITTO', 77: 'MEOWTH', 78: 'KRABBY', 79: 'MISSINGNO.', 80: 'MISSINGNO.', 81: 'MISSINGNO.', 82: 'VULPIX', 83: 'NINETALES', 84: 'PIKACHU', 85: 'RAICHU', 86: 'MISSINGNO.', 87: 'MISSINGNO.', 88: 'DRATINI', 89: 'DRAGONAIR', 90: 'KABUTO', 91: 'KABUTOPS', 92: 'HORSEA', 93: 'SEADRA', 94: 'MISSINGNO.', 95: 'MISSINGNO.', 96: 'SANDSHREW', 97: 'SANDSLASH', 98: 'OMANYTE', 99: 'OMASTAR', 100: 'JIGGLYPUFF', 101: 'WIGGLYTUFF', 102: 'EEVEE', 103: 'FLAREON', 104: 'JOLTEON', 105: 'VAPOREON', 106: 'MACHOP', 107: 'ZUBAT', 108: 'EKANS', 109: 'PARAS', 110: 'POLIWHIRL', 111: 'POLIWRATH', 112: 'WEEDLE', 113: 'KAKUNA', 114: 'BEEDRILL', 115: 'MISSINGNO.', 116: 'DODRIO', 117: 'PRIMEAPE', 118: 'DUGTRIO', 119: 'VENOMOTH', 120: 'DEWGONG', 121: 'MISSINGNO.', 122: 'MISSINGNO.', 123: 'CATERPIE', 124: 'METAPOD', 125: 'BUTTERFREE', 126: 'MACHAMP', 127: 'MISSINGNO.', 128: 'GOLDUCK', 129: 'HYPNO', 130: 'GOLBAT', 131: 'MEWTWO', 132: 'SNORLAX', 133: 'MAGIKARP', 134: 'MISSINGNO.', 135: 'MISSINGNO.', 136: 'MUK', 137: 'MISSINGNO.', 138: 'KINGLER', 139: 'CLOYSTER', 140: 'MISSINGNO.', 141: 'ELECTRODE', 142: 'CLEFABLE', 143: 'WEEZING', 144: 'PERSIAN', 145: 'MAROWAK', 146: 'MISSINGNO.', 147: 'HAUNTER', 148: 'ABRA', 149: 'ALAKAZAM', 150: 'PIDGEOTTO', 151: 'PIDGEOT', 152: 'STARMIE', 153: 'BULBASAUR', 154: 'VENUSAUR', 155: 'TENTACRUEL', 156: 'MISSINGNO.', 157: 'GOLDEEN', 158: 'SEAKING', 159: 'MISSINGNO.', 160: 'MISSINGNO.', 161: 'MISSINGNO.', 162: 'MISSINGNO.', 163: 'PONYTA', 164: 'RAPIDASH', 165: 'RATTATA', 166: 'RATICATE', 167: 'NIDORINO', 168: 'NIDORINA', 169: 'GEODUDE', 170: 'PORYGON', 171: 'AERODACTYL', 172: 'MISSINGNO.', 173: 'MAGNEMITE', 174: 'MISSINGNO.', 175: 'MISSINGNO.', 176: 'CHARMANDER', 177: 'SQUIRTLE', 178: 'CHARMELEON', 179: 'WARTORTLE', 180: 'CHARIZARD', 181: 'MISSINGNO.', 182: 'MISSINGNO.', 183: 'MISSINGNO.', 184: 'MISSINGNO.', 185: 'ODDISH', 186: 'GLOOM', 187: 'VILEPLUME', 188: 'BELLSPROUT', 189: 'WEEPINBELL', 190: 'VICTREEBEL'}
MV={1: 'POUND', 2: 'KARATE CHOP', 3: 'DOUBLESLAP', 4: 'COMET PUNCH', 5: 'MEGA PUNCH', 6: 'PAY DAY', 7: 'FIRE PUNCH', 8: 'ICE PUNCH', 9: 'THUNDERPUNCH', 10: 'SCRATCH', 11: 'VICEGRIP', 12: 'GUILLOTINE', 13: 'RAZOR WIND', 14: 'SWORDS DANCE', 15: 'CUT', 16: 'GUST', 17: 'WING ATTACK', 18: 'WHIRLWIND', 19: 'FLY', 20: 'BIND', 21: 'SLAM', 22: 'VINE WHIP', 23: 'STOMP', 24: 'DOUBLE KICK', 25: 'MEGA KICK', 26: 'JUMP KICK', 27: 'ROLLING KICK', 28: 'SAND-ATTACK', 29: 'HEADBUTT', 30: 'HORN ATTACK', 31: 'FURY ATTACK', 32: 'HORN DRILL', 33: 'TACKLE', 34: 'BODY SLAM', 35: 'WRAP', 36: 'TAKE DOWN', 37: 'THRASH', 38: 'DOUBLE-EDGE', 39: 'TAIL WHIP', 40: 'POISON STING', 41: 'TWINEEDLE', 42: 'PIN MISSILE', 43: 'LEER', 44: 'BITE', 45: 'GROWL', 46: 'ROAR', 47: 'SING', 48: 'SUPERSONIC', 49: 'SONICBOOM', 50: 'DISABLE', 51: 'ACID', 52: 'EMBER', 53: 'FLAMETHROWER', 54: 'MIST', 55: 'WATER GUN', 56: 'HYDRO PUMP', 57: 'SURF', 58: 'ICE BEAM', 59: 'BLIZZARD', 60: 'PSYBEAM', 61: 'BUBBLEBEAM', 62: 'AURORA BEAM', 63: 'HYPER BEAM', 64: 'PECK', 65: 'DRILL PECK', 66: 'SUBMISSION', 67: 'LOW KICK', 68: 'COUNTER', 69: 'SEISMIC TOSS', 70: 'STRENGTH', 71: 'ABSORB', 72: 'MEGA DRAIN', 73: 'LEECH SEED', 74: 'GROWTH', 75: 'RAZOR LEAF', 76: 'SOLARBEAM', 77: 'POISONPOWDER', 78: 'STUN SPORE', 79: 'SLEEP POWDER', 80: 'PETAL DANCE', 81: 'STRING SHOT', 82: 'DRAGON RAGE', 83: 'FIRE SPIN', 84: 'THUNDERSHOCK', 85: 'THUNDERBOLT', 86: 'THUNDER WAVE', 87: 'THUNDER', 88: 'ROCK THROW', 89: 'EARTHQUAKE', 90: 'FISSURE', 91: 'DIG', 92: 'TOXIC', 93: 'CONFUSION', 94: 'PSYCHIC', 95: 'HYPNOSIS', 96: 'MEDITATE', 97: 'AGILITY', 98: 'QUICK ATTACK', 99: 'RAGE', 100: 'TELEPORT', 101: 'NIGHT SHADE', 102: 'MIMIC', 103: 'SCREECH', 104: 'DOUBLE TEAM', 105: 'RECOVER', 106: 'HARDEN', 107: 'MINIMIZE', 108: 'SMOKESCREEN', 109: 'CONFUSE RAY', 110: 'WITHDRAW', 111: 'DEFENSE CURL', 112: 'BARRIER', 113: 'LIGHT SCREEN', 114: 'HAZE', 115: 'REFLECT', 116: 'FOCUS ENERGY', 117: 'BIDE', 118: 'METRONOME', 119: 'MIRROR MOVE', 120: 'SELFDESTRUCT', 121: 'EGG BOMB', 122: 'LICK', 123: 'SMOG', 124: 'SLUDGE', 125: 'BONE CLUB', 126: 'FIRE BLAST', 127: 'WATERFALL', 128: 'CLAMP', 129: 'SWIFT', 130: 'SKULL BASH', 131: 'SPIKE CANNON', 132: 'CONSTRICT', 133: 'AMNESIA', 134: 'KINESIS', 135: 'SOFTBOILED', 136: 'HI JUMP KICK', 137: 'GLARE', 138: 'DREAM EATER', 139: 'POISON GAS', 140: 'BARRAGE', 141: 'LEECH LIFE', 142: 'LOVELY KISS', 143: 'SKY ATTACK', 144: 'TRANSFORM', 145: 'BUBBLE', 146: 'DIZZY PUNCH', 147: 'SPORE', 148: 'FLASH', 149: 'PSYWAVE', 150: 'SPLASH', 151: 'ACID ARMOR', 152: 'CRABHAMMER', 153: 'EXPLOSION', 154: 'FURY SWIPES', 155: 'BONEMERANG', 156: 'REST', 157: 'ROCK SLIDE', 158: 'HYPER FANG', 159: 'SHARPEN', 160: 'CONVERSION', 161: 'TRI ATTACK', 162: 'SUPER FANG', 163: 'SLASH', 164: 'SUBSTITUTE', 165: 'STRUGGLE'}
BASEPP={1: 35, 2: 25, 3: 35, 4: 25, 5: 20, 6: 20, 7: 15, 8: 15, 9: 15, 10: 35, 11: 30, 12: 5, 13: 10, 14: 30, 15: 30, 16: 35, 17: 35, 18: 20, 19: 15, 20: 20, 21: 20, 22: 25, 23: 20, 24: 30, 25: 10, 26: 25, 27: 15, 28: 15, 29: 15, 30: 25, 31: 35, 32: 5, 33: 35, 34: 15, 35: 20, 36: 20, 37: 20, 38: 15, 39: 30, 40: 35, 41: 20, 42: 30, 43: 30, 44: 25, 45: 40, 46: 20, 47: 15, 48: 20, 49: 20, 50: 20, 51: 30, 52: 25, 53: 15, 54: 30, 55: 25, 56: 10, 57: 15, 58: 15, 59: 5, 60: 20, 61: 20, 62: 20, 63: 5, 64: 35, 65: 20, 66: 25, 67: 20, 68: 20, 69: 20, 70: 15, 71: 25, 72: 20, 73: 10, 74: 40, 75: 25, 76: 10, 77: 35, 78: 30, 79: 15, 80: 20, 81: 40, 82: 20, 83: 15, 84: 30, 85: 15, 86: 20, 87: 5, 88: 25, 89: 10, 90: 5, 91: 20, 92: 10, 93: 25, 94: 15, 95: 20, 96: 40, 97: 30, 98: 30, 99: 20, 100: 20, 101: 15, 102: 10, 103: 40, 104: 15, 105: 20, 106: 30, 107: 20, 108: 20, 109: 10, 110: 40, 111: 40, 112: 30, 113: 30, 114: 30, 115: 20, 116: 30, 117: 10, 118: 10, 119: 20, 120: 5, 121: 15, 122: 30, 123: 20, 124: 20, 125: 20, 126: 5, 127: 15, 128: 10, 129: 20, 130: 15, 131: 15, 132: 35, 133: 20, 134: 15, 135: 5, 136: 20, 137: 30, 138: 15, 139: 40, 140: 20, 141: 25, 142: 10, 143: 10, 144: 10, 145: 30, 146: 20, 147: 15, 148: 20, 149: 15, 150: 40, 151: 40, 152: 10, 153: 5, 154: 20, 155: 20, 156: 10, 157: 15, 158: 15, 159: 30, 160: 30, 161: 15, 162: 10, 163: 20, 164: 10, 165: 10}
ITEMS={0: 'No Item', 1: 'Master Ball', 2: 'Ultra Ball', 3: 'Great Ball', 4: 'Poke Ball', 5: 'Town Map', 6: 'Bicycle', 7: 'Surfboard', 8: 'Safari Ball', 9: 'Pokedex', 10: 'Moon Stone', 11: 'Antidote', 12: 'Burn Heal', 13: 'Ice Heal', 14: 'Awakening', 15: 'Parlyz Heal', 16: 'Full Restore', 17: 'Max Potion', 18: 'Hyper Potion', 19: 'Super Potion', 20: 'Potion', 21: 'Boulderbadge', 22: 'Cascadebadge', 23: 'Thunderbadge', 24: 'Rainbowbadge', 25: 'Soulbadge', 26: 'Marshbadge', 27: 'Volcanobadge', 28: 'Earthbadge', 29: 'Escape Rope', 30: 'Repel', 31: 'Old Amber', 32: 'Fire Stone', 33: 'Thunder Stone', 34: 'Water Stone', 35: 'Hp Up', 36: 'Protein', 37: 'Iron', 38: 'Carbos', 39: 'Calcium', 40: 'Rare Candy', 41: 'Dome Fossil', 42: 'Helix Fossil', 43: 'Secret Key', 44: 'Item 2C', 45: 'Bike Voucher', 46: 'X Accuracy', 47: 'Leaf Stone', 48: 'Card Key', 49: 'Nugget', 50: 'Item 32', 51: 'Poke Doll', 52: 'Full Heal', 53: 'Revive', 54: 'Max Revive', 55: 'Guard Spec', 56: 'Super Repel', 57: 'Max Repel', 58: 'Dire Hit', 59: 'Coin', 60: 'Fresh Water', 61: 'Soda Pop', 62: 'Lemonade', 63: 'S S Ticket', 64: 'Gold Teeth', 65: 'X Attack', 66: 'X Defend', 67: 'X Speed', 68: 'X Special', 69: 'Coin Case', 70: 'Oaks Parcel', 71: 'Itemfinder', 72: 'Silph Scope', 73: 'Poke Flute', 74: 'Lift Key', 75: 'Exp All', 76: 'Old Rod', 77: 'Good Rod', 78: 'Super Rod', 79: 'Pp Up', 80: 'Ether', 81: 'Max Ether', 82: 'Elixer', 83: 'Max Elixer', 84: 'Floor B2F', 85: 'Floor B1F', 86: 'Floor 1F', 87: 'Floor 2F', 88: 'Floor 3F', 89: 'Floor 4F', 90: 'Floor 5F', 91: 'Floor 6F', 92: 'Floor 7F', 93: 'Floor 8F', 94: 'Floor 9F', 95: 'Floor 10F', 96: 'Floor 11F', 97: 'Floor B4F', 196: 'HM-Cut', 197: 'HM-Fly', 198: 'HM-Surf', 199: 'HM-Strength', 200: 'HM-Flash', 201: 'TM-Mega Punch', 202: 'TM-Razor Wind', 203: 'TM-Swords Dance', 204: 'TM-Flamethrower', 205: 'TM-Mega Kick', 206: 'TM-Toxic', 207: 'TM-Horn Drill', 208: 'TM-Body Slam', 209: 'TM-Take Down', 210: 'TM-Double Edge', 211: 'TM-Bubblebeam', 212: 'TM-Water Gun', 213: 'TM-Ice Beam', 214: 'TM-Blizzard', 215: 'TM-Hyper Beam', 216: 'TM-Pay Day', 217: 'TM-Submission', 218: 'TM-Counter', 219: 'TM-Seismic Toss', 220: 'TM-Rage', 221: 'TM-Mega Drain', 222: 'TM-Solarbeam', 223: 'TM-Dragon Rage', 224: 'TM-Thunderbolt', 225: 'TM-Thunder', 226: 'TM-Earthquake', 227: 'TM-Fissure', 228: 'TM-Dig', 229: 'TM-Psychic M', 230: 'TM-Teleport', 231: 'TM-Mimic', 232: 'TM-Double Team', 233: 'TM-Reflect', 234: 'TM-Bide', 235: 'TM-Metronome', 236: 'TM-Selfdestruct', 237: 'TM-Egg Bomb', 238: 'TM-Fire Blast', 239: 'TM-Swift', 240: 'TM-Skull Bash', 241: 'TM-Softboiled', 242: 'TM-Dream Eater', 243: 'TM-Sky Attack', 244: 'TM-Rest', 245: 'TM-Thunder Wave', 246: 'TM-Psywave', 247: 'TM-Explosion', 248: 'TM-Rock Slide', 249: 'TM-Tri Attack', 250: 'TM-Substitute'}

# Items worth surfacing by default (evolution stones, key TMs, vitamins, rare items).
# Everything else (common balls/potions/status heals) is hidden unless --full.
NOTABLE_KEYWORDS = ("TM-","HM-","Stone","Rare Candy","Nugget","Master Ball",
                    "Protein","Iron","Calcium","Carbos","Hp Up","Ppup","Pp Up",
                    "Exp All","Ether","Elixer","Elixir","Max Revive")

CH={0x50:"",0x7F:" ",**{0x80+i:c for i,c in enumerate("ABCDEFGHIJKLMNOPQRSTUVWXYZ")},
    0xE8:".",0x9A:"(",0x9B:")",0x9C:":",0xE3:"-",0xF3:"/",0xF4:",",0x9D:";"}
def gbstr(b):
    out=""
    for c in b:
        if c in (0x50,0x00): break
        out+=CH.get(c,"")
    return out.strip()

def read_items(d, start, limit=60):
    """Read an item list (item,qty pairs) until 0xFF terminator."""
    out=[]; i=0
    while i<limit:
        it=d[start+i*2]
        if it==0xFF: break
        out.append((it, d[start+i*2+1])); i+=1
    return out

def show_items(title, lst, full):
    if not full:
        lst=[(it,q) for it,q in lst
             if any(k.lower() in ITEMS.get(it,"").lower() for k in NOTABLE_KEYWORDS)]
    print(f"=== {title} ({len(lst)}{'' if full else ' notable'}) ===")
    for it,q in lst:
        print(f"  {ITEMS.get(it,f'#{it}'):22} x{q}")
    if not lst: print("  (none)")
    print()

def extract(d):
    """Parse the save into a structured dict (the source of truth)."""
    save={"trainer":gbstr(d[O_NAME:O_NAME+11]),
          "playtime":f"{d[O_PT_H]}h {d[O_PT_M]}m","party":[]}
    n=d[O_PCOUNT]
    if 1<=n<=6:
        for i in range(n):
            b=O_PARTY+i*STRUCT
            sp=d[b]
            cur=(d[b+1]<<8)|d[b+2]
            st=d[b+0x04]; conds=[]
            if st&0x07: conds.append(f"SLEEP({st&0x07})")
            if st&0x08: conds.append("POISON")
            if st&0x10: conds.append("BURN")
            if st&0x20: conds.append("FREEZE")
            if st&0x40: conds.append("PARALYZE")
            if cur==0: conds=["FAINTED"]
            moves=[]
            for j in range(4):
                m=d[b+8+j]
                if not m: continue
                pb=d[b+0x1D+j]; p=pb&0x3F; u=(pb>>6)&0x3
                base=BASEPP.get(m,0)
                moves.append({"name":MV.get(m,f"#{m}"),"pp":p,
                              "maxpp":base*(5+u)//5 if base else p})
            nick=gbstr(d[O_NICK+i*11:O_NICK+i*11+11]); nm=MON.get(sp,f"#{sp}")
            save["party"].append({
                "slot":i+1,"species":nm,
                "nick":nick if nick and nick!=nm else None,
                "level":d[b+LEVEL_OFF],"hp":cur,"maxhp":(d[b+0x22]<<8)|d[b+0x23],
                "atk":(d[b+0x24]<<8)|d[b+0x25],"def":(d[b+0x26]<<8)|d[b+0x27],
                "spd":(d[b+0x28]<<8)|d[b+0x29],"spc":(d[b+0x2A]<<8)|d[b+0x2B],
                "status":conds,"moves":moves})
    save["bag"]=[{"item":ITEMS.get(it,f"#{it}"),"qty":q} for it,q in read_items(d,O_BAGDATA)]
    save["pc"]=[{"item":ITEMS.get(it,f"#{it}"),"qty":q} for it,q in read_items(d,O_PC+1)]
    return save

def render_full(s, full):
    print("="*46)
    print(f"  TRAINER: {s['trainer'] or '(unnamed)'}")
    print(f"  Playtime: {s['playtime']}")
    print("="*46)
    if s["party"]:
        print(f"\n=== PARTY ({len(s['party'])}) ===\n")
        for m in s["party"]:
            tag=f"{m['nick']} ({m['species']})" if m['nick'] else m['species']
            stag=f"  [{', '.join(m['status'])}]" if m['status'] else ""
            print(f"{m['slot']}. {tag:20} Lv{m['level']}  HP {m['hp']}/{m['maxhp']}{stag}")
            print(f"   Atk {m['atk']}  Def {m['def']}  Spd {m['spd']}  Spc {m['spc']}")
            mvstr=" / ".join(f"{mv['name']} ({mv['pp']}/{mv['maxpp']})" for mv in m['moves'])
            print(f"   {mvstr}\n")
    def items(title, lst):
        if not full:
            lst=[x for x in lst if any(k.lower() in x['item'].lower() for k in NOTABLE_KEYWORDS)]
        print(f"=== {title} ({len(lst)}{'' if full else ' notable'}) ===")
        for x in lst: print(f"  {x['item']:22} x{x['qty']}")
        if not lst: print("  (none)")
        print()
    items("BAG", s["bag"]); items("PC STORAGE", s["pc"])
    if not full: print("(run with --full to see every ball/potion/heal too)")

def render_compact(s):
    """One line per mon — stable NAME prefix, diff-friendly."""
    print(f"# {s['trainer']}  {s['playtime']}")
    for m in s["party"]:
        st=f" [{','.join(m['status'])}]" if m['status'] else ""
        mv=" · ".join(f"{mv['name']} {mv['pp']}/{mv['maxpp']}" for mv in m['moves'])
        print(f"{m['species']:10} L{m['level']:<2} {m['hp']:>3}/{m['maxhp']:<3}"
              f" ATK{m['atk']:<3} DEF{m['def']:<3} SPD{m['spd']:<3} SPC{m['spc']:<3}{st}  {mv}")

def main():
    args=[a for a in sys.argv[1:] if not a.startswith("--")]
    flags=set(a for a in sys.argv if a.startswith("--"))
    if not args:
        print("usage: python3 read_save.py <save.srm> [--full|--compact|--json]"); return
    s=extract(open(args[0],"rb").read())
    if "--json" in flags:
        import json; print(json.dumps(s,indent=2))
    elif "--compact" in flags:
        render_compact(s)
    else:
        render_full(s, "--full" in flags)

if __name__=="__main__":
    try:
        main()
    except BrokenPipeError:
        pass
