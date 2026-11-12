#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Import the independently exported R01 factory corpus; never run at build time."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "rust/fm1-engine/tests/factory/reference.json"
EXPECTED = "6475cd0cb5ced5e1db51dbd10f4a9f0bf23ef759c2d231a892154c5d00ad4873"


def literal(value):
    if isinstance(value, list): return "[" + ",".join(map(literal, value)) + "]"
    return str(value)


def cstring(value):
    return 'b"' + ''.join(chr(byte) if 32 <= byte < 127 and byte not in (34, 92) else '\\x%02x' % byte for byte in value.encode()) + '\\0"'


def c_pointer(value):
    return 'c' + cstring(value)[1:-3] + '".as_ptr().cast::<u8>()'


def main():
    raw = SOURCE.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == EXPECTED, "not the independently verified R01 corpus"
    data = json.loads(raw)
    presets = data["presets"]
    assert len(presets) == 190 and all(len(p["parameters"]) == 176 for p in presets)
    header = "// SPDX-License-Identifier: GPL-3.0-only\n// Imported from verified R01 corpus %s; maintained data, not runtime C access.\n" % EXPECTED
    compact_raw = (SOURCE.parent / 'compact.json').read_bytes()
    assert hashlib.sha256(compact_raw).hexdigest() == '23b53f86923a60ecbeec5d88c6dd835d035930e114e2d09028c5547d84b1a4fb'
    compact = json.loads(compact_raw)
    lines = [header, "use fm1_types::StepMask;", "use super::{Palette, Override};"]
    for key, name in [('base','BASE'),('overrides','OVERRIDES')]:
        lines.append('pub static %s: &[&[Override]] = &[' % name)
        for row in compact[key]:
            assert row[-1] == -1 and len(row) % 2 == 1
            lines.append('&[' + ','.join('Override::new(%d,%d)' % (row[i],row[i+1]) for i in range(0,len(row)-1,2)) + '],')
        lines.append('];')
    for key,name,typ in [('rows','ROWS','u8'),('patterns','PATTERNS','u16'),('psy_down','PSY_DOWN','u16'),('tempo_key_mode','TEMPO_KEY_MODE','u8'),('pluck','PLUCK','u8'),('pluck_rate','PLUCK_RATE','u8'),('pad_shift','PAD_SHIFT','u8')]:
        lines.append('pub static %s: [%s;%d] = %s;' % (name,typ,len(compact[key]),literal(compact[key])))
    assert len(compact['palettes']) == 19*32
    lines.append('pub static PALETTES: [Palette;19] = [')
    for g in range(19):
        row=compact['palettes'][g*32:(g+1)*32]
        lines.append('Palette { '+','.join('%s:%s' % (name,literal(row[start:start+length])) for name,start,length in [('pad',0,5),('drone',5,5),('acid',10,5),('pluck',15,5),('kick',20,3),('snare',23,3),('hat',26,3),('perc',29,3)])+' },')
    lines.append('];')
    lines.append("pub static BOOT_PATTERNS: [[StepMask;6];6] = [" + ','.join('[' + ','.join('StepMask::from_bits(%d)' % value for value in data['boot_patterns'][row*6:row*6+6])+']' for row in range(6)) + "];")
    lines.extend(["pub static MODE_POOLS: [u16;8] = %s;" % literal(data["mode_pools"]), "pub static DONORS: [u8;19] = %s;" % literal(data["donors"]), "pub static ARCS: [([u8;6],[u8;3]);10] = [" + ','.join('(%s,%s)' % (literal(arc[0]),literal(arc[1])) for arc in data['arcs']) + "];"])
    data["names"]["PARAMETER_NAMES"] = [p[0] for p in data["parameters"]]
    lines.append("pub static NAMES: &[&[&[u8]]] = &[")
    for values in data["names"].values(): lines.append("&[" + ','.join(cstring(v) for v in values) + "],")
    lines.append("];")
    (ROOT / "rust/fm1-engine/src/factory_data.rs").write_text('\n'.join(lines)+'\n',encoding='utf-8')
    bridge = [header]
    for index, values in enumerate(data["names"].values()):
        bridge.append('#[export_name="fm1_rust_factory_name_table_%d"]\nstatic NAMES_%d: NamePointers<%d> = NamePointers([%s]);' % (index,index,len(values),','.join(c_pointer(v) for v in values)))
    bridge.append("fn name_pointers(category: usize) -> Option<*const *const u8> { match category {")
    for index in range(len(data["names"])): bridge.append("%d => Some(NAMES_%d.0.as_ptr())," % (index,index))
    bridge.append("_ => None, } }")
    bridge.append("static PARAMETERS: [ParameterAbi;176] = [")
    for index, row in enumerate(data["parameters"]): bridge.append("ParameterAbi { name:%s, min:fm1_types::PARAMETERS[%d].min, max:fm1_types::PARAMETERS[%d].max, default:fm1_types::PARAMETERS[%d].default }," % (c_pointer(row[0]),index,index,index))
    bridge.append("];")
    (ROOT / "rust/fm1-jam-bridge/src/factory_data.rs").write_text('\n'.join(bridge)+'\n',encoding='utf-8')
    mapping = {name:index for index,name in enumerate(data["names"])}
    aliases=[]
    for name,index in mapping.items():
        if name=='EN_CM_SOUND': continue
        count=len(data['names'][name]);shape='[19][10]' if name=='GEN_PIECE_NAME' else '[%d]' % count
        aliases.extend(['extern const uint8_t *const fm1_rust_factory_name_table_%d[%d];' % (index,count),'#define %s (*(const char *const (*)%s)fm1_rust_factory_name_table_%d)' % (name,shape,index)])
    (ROOT / "firmware/src/dsp/rust_factory_names.inc").write_text("/* SPDX-License-Identifier: GPL-3.0-only; immutable Rust arrays retain C shapes. */\n" + '\n'.join(aliases) + '\n',encoding='utf-8')
    numeric_bytes = sum(len(compact[k]) for k in ['rows','tempo_key_mode','pluck','pluck_rate','pad_shift','palettes']) + 2*(len(compact['patterns'])+len(compact['psy_down'])) + 4*sum((len(row)-1)//2 for key in ['base','overrides'] for row in compact[key]) + 72+16+19+90
    print(json.dumps({"presets_computed":190,"expanded_preset_tables":0,"names":len(mapping),"compact_numeric_bytes":numeric_bytes,"factory_cm_category":mapping['EN_CM_SOUND']}))


if __name__ == "__main__": main()
