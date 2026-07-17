#!/usr/bin/env python3
"""
Pokemon Yellow (Gen 1) party reader — for Yellow Legacy or vanilla R/B/Y.
Usage:  python3 read_party.py yourgame.sav
Reads the party block straight out of the .sav and prints species/levels/moves/stats.
"""
import sys, json

# ---- Gen 1 save layout (SRAM bank 1) ----
PARTY_COUNT = 0x2F2C      # 1 byte: how many in party
PARTY_SPECIES = 0x2F2D    # species list (count bytes + 0xFF terminator)
PARTY_DATA = 0x2F34       # start of the 6 x 44-byte party structs
STRUCT_LEN = 44
PARTY_NICK = 0x307E       # nicknames (11 bytes each)
OT_NAME   = 0x2F9C        # OT names (11 bytes each)

MON = {1: 'RHYDON', 2: 'KANGASKHAN', 3: 'NIDORAN♂', 4: 'CLEFAIRY', 5: 'SPEAROW', 6: 'VOLTORB', 7: 'NIDOKING', 8: 'SLOWBRO', 9: 'IVYSAUR', 10: 'EXEGGUTOR', 11: 'LICKITUNG', 12: 'EXEGGCUTE', 13: 'GRIMER', 14: 'GENGAR', 15: 'NIDORAN♀', 16: 'NIDOQUEEN', 17: 'CUBONE', 18: 'RHYHORN', 19: 'LAPRAS', 20: 'ARCANINE', 21: 'MEW', 22: 'GYARADOS', 23: 'SHELLDER', 24: 'TENTACOOL', 25: 'GASTLY', 26: 'SCYTHER', 27: 'STARYU', 28: 'BLASTOISE', 29: 'PINSIR', 30: 'TANGELA', 31: 'MISSINGNO.', 32: 'MISSINGNO.', 33: 'GROWLITHE', 34: 'ONIX', 35: 'FEAROW', 36: 'PIDGEY', 37: 'SLOWPOKE', 38: 'KADABRA', 39: 'GRAVELER', 40: 'CHANSEY', 41: 'MACHOKE', 42: 'MR.MIME', 43: 'HITMONLEE', 44: 'HITMONCHAN', 45: 'ARBOK', 46: 'PARASECT', 47: 'PSYDUCK', 48: 'DROWZEE', 49: 'GOLEM', 50: 'MISSINGNO.', 51: 'MAGMAR', 52: 'MISSINGNO.', 53: 'ELECTABUZZ', 54: 'MAGNETON', 55: 'KOFFING', 56: 'MISSINGNO.', 57: 'MANKEY', 58: 'SEEL', 59: 'DIGLETT', 60: 'TAUROS', 61: 'MISSINGNO.', 62: 'MISSINGNO.', 63: 'MISSINGNO.', 64: "FARFETCH'D", 65: 'VENONAT', 66: 'DRAGONITE', 67: 'MISSINGNO.', 68: 'MISSINGNO.', 69: 'MISSINGNO.', 70: 'DODUO', 71: 'POLIWAG', 72: 'JYNX', 73: 'MOLTRES', 74: 'ARTICUNO', 75: 'ZAPDOS', 76: 'DITTO', 77: 'MEOWTH', 78: 'KRABBY', 79: 'MISSINGNO.', 80: 'MISSINGNO.', 81: 'MISSINGNO.', 82: 'VULPIX', 83: 'NINETALES', 84: 'PIKACHU', 85: 'RAICHU', 86: 'MISSINGNO.', 87: 'MISSINGNO.', 88: 'DRATINI', 89: 'DRAGONAIR', 90: 'KABUTO', 91: 'KABUTOPS', 92: 'HORSEA', 93: 'SEADRA', 94: 'MISSINGNO.', 95: 'MISSINGNO.', 96: 'SANDSHREW', 97: 'SANDSLASH', 98: 'OMANYTE', 99: 'OMASTAR', 100: 'JIGGLYPUFF', 101: 'WIGGLYTUFF', 102: 'EEVEE', 103: 'FLAREON', 104: 'JOLTEON', 105: 'VAPOREON', 106: 'MACHOP', 107: 'ZUBAT', 108: 'EKANS', 109: 'PARAS', 110: 'POLIWHIRL', 111: 'POLIWRATH', 112: 'WEEDLE', 113: 'KAKUNA', 114: 'BEEDRILL', 115: 'MISSINGNO.', 116: 'DODRIO', 117: 'PRIMEAPE', 118: 'DUGTRIO', 119: 'VENOMOTH', 120: 'DEWGONG', 121: 'MISSINGNO.', 122: 'MISSINGNO.', 123: 'CATERPIE', 124: 'METAPOD', 125: 'BUTTERFREE', 126: 'MACHAMP', 127: 'MISSINGNO.', 128: 'GOLDUCK', 129: 'HYPNO', 130: 'GOLBAT', 131: 'MEWTWO', 132: 'SNORLAX', 133: 'MAGIKARP', 134: 'MISSINGNO.', 135: 'MISSINGNO.', 136: 'MUK', 137: 'MISSINGNO.', 138: 'KINGLER', 139: 'CLOYSTER', 140: 'MISSINGNO.', 141: 'ELECTRODE', 142: 'CLEFABLE', 143: 'WEEZING', 144: 'PERSIAN', 145: 'MAROWAK', 146: 'MISSINGNO.', 147: 'HAUNTER', 148: 'ABRA', 149: 'ALAKAZAM', 150: 'PIDGEOTTO', 151: 'PIDGEOT', 152: 'STARMIE', 153: 'BULBASAUR', 154: 'VENUSAUR', 155: 'TENTACRUEL', 156: 'MISSINGNO.', 157: 'GOLDEEN', 158: 'SEAKING', 159: 'MISSINGNO.', 160: 'MISSINGNO.', 161: 'MISSINGNO.', 162: 'MISSINGNO.', 163: 'PONYTA', 164: 'RAPIDASH', 165: 'RATTATA', 166: 'RATICATE', 167: 'NIDORINO', 168: 'NIDORINA', 169: 'GEODUDE', 170: 'PORYGON', 171: 'AERODACTYL', 172: 'MISSINGNO.', 173: 'MAGNEMITE', 174: 'MISSINGNO.', 175: 'MISSINGNO.', 176: 'CHARMANDER', 177: 'SQUIRTLE', 178: 'CHARMELEON', 179: 'WARTORTLE', 180: 'CHARIZARD', 181: 'MISSINGNO.', 182: 'MISSINGNO.', 183: 'MISSINGNO.', 184: 'MISSINGNO.', 185: 'ODDISH', 186: 'GLOOM', 187: 'VILEPLUME', 188: 'BELLSPROUT', 189: 'WEEPINBELL', 190: 'VICTREEBEL'}
MV  = {1: 'POUND', 2: 'KARATE CHOP', 3: 'DOUBLESLAP', 4: 'COMET PUNCH', 5: 'MEGA PUNCH', 6: 'PAY DAY', 7: 'FIRE PUNCH', 8: 'ICE PUNCH', 9: 'THUNDERPUNCH', 10: 'SCRATCH', 11: 'VICEGRIP', 12: 'GUILLOTINE', 13: 'RAZOR WIND', 14: 'SWORDS DANCE', 15: 'CUT', 16: 'GUST', 17: 'WING ATTACK', 18: 'WHIRLWIND', 19: 'FLY', 20: 'BIND', 21: 'SLAM', 22: 'VINE WHIP', 23: 'STOMP', 24: 'DOUBLE KICK', 25: 'MEGA KICK', 26: 'JUMP KICK', 27: 'ROLLING KICK', 28: 'SAND-ATTACK', 29: 'HEADBUTT', 30: 'HORN ATTACK', 31: 'FURY ATTACK', 32: 'HORN DRILL', 33: 'TACKLE', 34: 'BODY SLAM', 35: 'WRAP', 36: 'TAKE DOWN', 37: 'THRASH', 38: 'DOUBLE-EDGE', 39: 'TAIL WHIP', 40: 'POISON STING', 41: 'TWINEEDLE', 42: 'PIN MISSILE', 43: 'LEER', 44: 'BITE', 45: 'GROWL', 46: 'ROAR', 47: 'SING', 48: 'SUPERSONIC', 49: 'SONICBOOM', 50: 'DISABLE', 51: 'ACID', 52: 'EMBER', 53: 'FLAMETHROWER', 54: 'MIST', 55: 'WATER GUN', 56: 'HYDRO PUMP', 57: 'SURF', 58: 'ICE BEAM', 59: 'BLIZZARD', 60: 'PSYBEAM', 61: 'BUBBLEBEAM', 62: 'AURORA BEAM', 63: 'HYPER BEAM', 64: 'PECK', 65: 'DRILL PECK', 66: 'SUBMISSION', 67: 'LOW KICK', 68: 'COUNTER', 69: 'SEISMIC TOSS', 70: 'STRENGTH', 71: 'ABSORB', 72: 'MEGA DRAIN', 73: 'LEECH SEED', 74: 'GROWTH', 75: 'RAZOR LEAF', 76: 'SOLARBEAM', 77: 'POISONPOWDER', 78: 'STUN SPORE', 79: 'SLEEP POWDER', 80: 'PETAL DANCE', 81: 'STRING SHOT', 82: 'DRAGON RAGE', 83: 'FIRE SPIN', 84: 'THUNDERSHOCK', 85: 'THUNDERBOLT', 86: 'THUNDER WAVE', 87: 'THUNDER', 88: 'ROCK THROW', 89: 'EARTHQUAKE', 90: 'FISSURE', 91: 'DIG', 92: 'TOXIC', 93: 'CONFUSION', 94: 'PSYCHIC', 95: 'HYPNOSIS', 96: 'MEDITATE', 97: 'AGILITY', 98: 'QUICK ATTACK', 99: 'RAGE', 100: 'TELEPORT', 101: 'NIGHT SHADE', 102: 'MIMIC', 103: 'SCREECH', 104: 'DOUBLE TEAM', 105: 'RECOVER', 106: 'HARDEN', 107: 'MINIMIZE', 108: 'SMOKESCREEN', 109: 'CONFUSE RAY', 110: 'WITHDRAW', 111: 'DEFENSE CURL', 112: 'BARRIER', 113: 'LIGHT SCREEN', 114: 'HAZE', 115: 'REFLECT', 116: 'FOCUS ENERGY', 117: 'BIDE', 118: 'METRONOME', 119: 'MIRROR MOVE', 120: 'SELFDESTRUCT', 121: 'EGG BOMB', 122: 'LICK', 123: 'SMOG', 124: 'SLUDGE', 125: 'BONE CLUB', 126: 'FIRE BLAST', 127: 'WATERFALL', 128: 'CLAMP', 129: 'SWIFT', 130: 'SKULL BASH', 131: 'SPIKE CANNON', 132: 'CONSTRICT', 133: 'AMNESIA', 134: 'KINESIS', 135: 'SOFTBOILED', 136: 'HI JUMP KICK', 137: 'GLARE', 138: 'DREAM EATER', 139: 'POISON GAS', 140: 'BARRAGE', 141: 'LEECH LIFE', 142: 'LOVELY KISS', 143: 'SKY ATTACK', 144: 'TRANSFORM', 145: 'BUBBLE', 146: 'DIZZY PUNCH', 147: 'SPORE', 148: 'FLASH', 149: 'PSYWAVE', 150: 'SPLASH', 151: 'ACID ARMOR', 152: 'CRABHAMMER', 153: 'EXPLOSION', 154: 'FURY SWIPES', 155: 'BONEMERANG', 156: 'REST', 157: 'ROCK SLIDE', 158: 'HYPER FANG', 159: 'SHARPEN', 160: 'CONVERSION', 161: 'TRI ATTACK', 162: 'SUPER FANG', 163: 'SLASH', 164: 'SUBSTITUTE', 165: 'STRUGGLE'}

def gbstr(b):
    """decode a Gen1 text string (0x50 terminated)"""
    out=""
    CH={0x80:"A",0x81:"B",0x82:"C",0x83:"D",0x84:"E",0x85:"F",0x86:"G",0x87:"H",
        0x88:"I",0x89:"J",0x8A:"K",0x8B:"L",0x8C:"M",0x8D:"N",0x8E:"O",0x8F:"P",
        0x90:"Q",0x91:"R",0x92:"S",0x93:"T",0x94:"U",0x95:"V",0x96:"W",0x97:"X",
        0x98:"Y",0x99:"Z"}
    for c in b:
        if c==0x50 or c==0: break
        out+=CH.get(c,"")
    return out

def main():
    if len(sys.argv)<2:
        print("usage: python3 read_party.py <save.sav>"); return
    data=open(sys.argv[1],"rb").read()
    n=data[PARTY_COUNT]
    if n<1 or n>6:
        print(f"party count looks off ({n}). Is this a Gen 1 .sav?"); return
    print(f"=== PARTY ({n}) ===\n")
    for i in range(n):
        base=PARTY_DATA + i*STRUCT_LEN
        species=data[base]
        level=data[base+0x03]
        # current HP is 2 bytes big-endian at struct start+1
        cur_hp=(data[base+0x01]<<8)|data[base+0x02]
        moves=[data[base+0x08+j] for j in range(4)]
        # stats: max HP, atk, def, spd, spc are 2-byte each starting ~0x22
        maxhp=(data[base+0x22]<<8)|data[base+0x23]
        atk =(data[base+0x24]<<8)|data[base+0x25]
        dfn =(data[base+0x26]<<8)|data[base+0x27]
        spd =(data[base+0x28]<<8)|data[base+0x29]
        spc =(data[base+0x2A]<<8)|data[base+0x2B]
        nick=gbstr(data[PARTY_NICK+i*11:PARTY_NICK+i*11+11])
        name=MON.get(species,f"#{species}")
        mvs=" / ".join(MV.get(m,"") for m in moves if m)
        print(f"{i+1}. {nick or name:11} (Lv {level})  HP {cur_hp}/{maxhp}")
        print(f"   Atk {atk}  Def {dfn}  Spd {spd}  Spc {spc}")
        print(f"   {mvs}\n")

if __name__=="__main__":
    main()
