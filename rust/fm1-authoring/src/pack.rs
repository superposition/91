// SPDX-License-Identifier: GPL-3.0-only
use crate::{factory, get, integer, list, string, Error, Result};
use fm1_formats::jam::{
    JamPack, BASS, DRUM, EDGE, FORM, MIN_V1, MIN_V2, NODE, SOUND, V2_CHART, V2_EXTRA, V2_HEAD,
    V2_VOICE,
};
use serde_json::{json, Value};
pub type Limits<'a> = Option<&'a Value>;
/// A decoded and compile-checked authoring document. Fields are deliberately
/// JSON values so unused metadata survives edits without lossy typed coercion.
#[derive(Clone, Debug)]
pub struct Document(Value);
impl Document {
    pub fn into_value(self) -> Value {
        self.0
    }
    pub fn value(&self) -> &Value {
        &self.0
    }
}
fn write_name(b: &mut [u8], at: usize, len: usize, v: &Value) -> Result<()> {
    let s = string(v)?;
    if s.is_empty() || s.len() >= len || !s.bytes().all(|c| (32..=126).contains(&c)) {
        return Err(Error::value(
            "name must be printable ASCII and fit its field",
        ));
    }
    b[at..at + s.len()].copy_from_slice(s.as_bytes());
    Ok(())
}
fn read_name(b: &[u8], at: usize, len: usize) -> Result<String> {
    let b = &b[at..at + len];
    let b = &b[..b.iter().position(|c| *c == 0).unwrap_or(b.len())];
    if !b.is_ascii() {
        return Err(Error::value("pack name is not ASCII"));
    }
    Ok(String::from_utf8(b.to_vec()).unwrap())
}
fn at(v: &Value, i: usize) -> Result<&Value> {
    list(v)?
        .get(i)
        .ok_or_else(|| Error::value("list index out of range"))
}
pub fn compile(d: &Value, limits: Limits<'_>) -> Result<Vec<u8>> {
    if matches!(d.get("version").and_then(Value::as_f64), Some(2.0 | 3.0)) {
        return compile_v2(d, limits);
    }
    integer(get(d, "version")?, 1, 1, "version")?;
    let mut b = vec![0; MIN_V1];
    b[..5].copy_from_slice(b"JMP1\x01");
    write_name(&mut b, 16, 24, get(d, "name")?)?;
    for (at, key, max) in [
        (5, "nodes", 16),
        (6, "bass", 8),
        (7, "drums", 8),
        (8, "sounds", 32),
    ] {
        b[at] = integer(
            &json!(list(get(d, key)?)?.len()),
            if key == "sounds" { 0 } else { 1 },
            max,
            key,
        )? as u8;
    }
    if list(get(d, "scales")?)?.len() != 2
        || list(get(d, "forms")?)?.len() != 4
        || list(get(d, "cadence")?)?.len() != 2
    {
        return Err(Error::value("need two scales, two cadences, four forms"));
    }
    for (i, scale) in list(&d["scales"])?.iter().enumerate() {
        let a = list(scale)?;
        if a.len() != 8
            || a[0] != 0
            || a.windows(2)
                .any(|w| w[0].as_i64().zip(w[1].as_i64()).is_none_or(|(a, b)| a >= b))
        {
            return Err(Error::value(
                "scale must be eight ascending distinct pitch classes starting at 0",
            ));
        }
        for (j, v) in a.iter().enumerate() {
            b[40 + i * 8 + j] = integer(v, 0, 11, "scale")? as u8;
        }
    }
    for (i, group) in list(&d["cadence"])?.iter().enumerate() {
        if list(group)?.len() != 3 {
            return Err(Error::value("cadence is [tonic, ii, dominant]"));
        }
        for (j, x) in list(group)?.iter().enumerate() {
            let n = integer(x, 0, b[5] as i64 - 1, "cadence node")? as usize;
            b[9 + i * 3 + j] = n as u8;
            if get(&d["nodes"][n], "mode")? != &json!(i) {
                return Err(Error::value("cadence crosses key mode"));
            }
        }
    }
    for (i, n) in list(&d["nodes"])?.iter().enumerate() {
        for (j, (key, hi)) in [
            ("bass", 11),
            ("sixth", 11),
            ("scale", 1),
            ("label", 5),
            ("mode", 1),
        ]
        .iter()
        .enumerate()
        {
            b[NODE + i * 8 + j] = integer(get(n, key)?, 0, *hi, key)? as u8;
        }
        b[NODE + i * 8 + 5] = integer(
            n.get("emphasis").unwrap_or(&json!(0)),
            0,
            15,
            "section emphasis",
        )? as u8;
    }
    let transitions = list(get(d, "transitions")?)?;
    if transitions.len() != b[5] as usize {
        return Err(Error::value("transition row count"));
    }
    for (i, row) in transitions.iter().enumerate() {
        let row = list(row)?;
        if row.len() != b[5] as usize
            || row
                .iter()
                .map(|x| x.as_i64().unwrap_or(0) as i128)
                .sum::<i128>()
                == 0
        {
            return Err(Error::value("each node needs outgoing weights"));
        }
        for (j, w) in row.iter().enumerate() {
            let w = integer(w, 0, 255, "weight")? as u8;
            b[EDGE + i * 16 + j] = w;
            if w != 0 && d["nodes"][i]["mode"] != d["nodes"][j]["mode"] {
                return Err(Error::value("transition crosses key mode"));
            }
        }
    }
    for (i, f) in list(&d["forms"])?.iter().enumerate() {
        let o = FORM + i * 32;
        write_name(&mut b, o, 16, get(f, "name")?)?;
        let roles = list(get(f, "roles")?)?;
        b[o + 16] = integer(&json!(roles.len()), 1, 8, "section count")? as u8;
        for (j, r) in roles.iter().enumerate() {
            b[o + 17 + j] = integer(r, 0, 3, "section role")? as u8;
        }
    }
    for (i, p) in list(&d["bass"])?.iter().enumerate() {
        let o = BASS + i * 36;
        let notes = list(get(p, "notes")?)?;
        let velocities = list(get(p, "velocity")?)?;
        if notes.len() != 16 || velocities.len() != 16 {
            return Err(Error::value("bass pattern needs 16 notes and velocities"));
        }
        for (j, v) in notes.iter().enumerate() {
            let n = integer(v, -128, 12, "bass note")?;
            if n < -12 && ![-128, -127, -126, -125].contains(&n) {
                return Err(Error::value("unknown bass operation"));
            }
            b[o + j] = n as u8;
        }
        for (j, v) in velocities.iter().enumerate() {
            b[o + 16 + j] = integer(v, 0, 127, "velocity")? as u8;
        }
        b[o + 32] = integer(get(p, "gate")?, 1, 100, "gate")? as u8;
    }
    for (i, p) in list(&d["drums"])?.iter().enumerate() {
        let lanes = list(get(p, "lanes")?)?;
        if lanes.len() != 6 {
            return Err(Error::value("six drum lanes required"));
        }
        for (j, v) in lanes
            .iter()
            .chain(std::iter::once(get(p, "comp")?))
            .enumerate()
        {
            let x = integer(v, 0, 65535, "mask")? as u16;
            b[DRUM + i * 16 + j * 2..DRUM + i * 16 + j * 2 + 2].copy_from_slice(&x.to_le_bytes());
        }
        b[DRUM + i * 16 + 14] = integer(
            p.get("comp_gate").unwrap_or(&json!(80)),
            1,
            100,
            "comp gate",
        )? as u8;
    }
    for (i, p) in list(&d["sounds"])?.iter().enumerate() {
        let id = integer(get(p, "parameter")?, 0, 175, "parameter")? as usize;
        if [5, 71, 72, 73, 74, 75].contains(&id) {
            return Err(Error::value(
                "packs cannot overwrite tempo, key or global preferences",
            ));
        }
        let value = get(p, "value")?;
        if let Some(limits) = limits {
            let p = &limits["params"][id];
            integer(
                value,
                p["min"]
                    .as_i64()
                    .ok_or_else(|| Error::value("invalid parameter limits"))?,
                p["max"]
                    .as_i64()
                    .ok_or_else(|| Error::value("invalid parameter limits"))?,
                "sound value",
            )?;
        }
        let value = integer(value, -32768, 32767, "sound value")? as i16;
        b[SOUND + i * 4..SOUND + i * 4 + 2].copy_from_slice(&(id as u16).to_le_bytes());
        b[SOUND + i * 4 + 2..SOUND + i * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    let empty = json!([]);
    let sources = d.get("sources").unwrap_or(&empty);
    if !sources
        .as_array()
        .is_some_and(|a| a.iter().all(Value::is_string))
    {
        return Err(Error::value("sources must be a list of strings"));
    }
    b.extend(serde_json::to_vec(sources)?);
    if b.len() > fm1_types::STORE_MAX {
        return Err(Error::value(format!(
            "pack is {} bytes, limit 3840",
            b.len()
        )));
    }
    // The legacy authoring profile optionally omits parameter-range checking;
    // structural validation remains identical to the instrument's shared view.
    JamPack::parse_structural(&b)
        .map_err(|e| Error::value(format!("invalid authored pack: {e:?}")))?;
    Ok(b)
}
fn compile_v2(d: &Value, limits: Limits<'_>) -> Result<Vec<u8>> {
    let v3 = d.get("version").and_then(Value::as_f64) == Some(3.0);
    if v3 {
        integer(get(d, "version")?, 3, 3, "songbook version")?;
        if list(get(d, "cadence")?)?.len() != 2 || list(get(d, "charts")?)?.len() != 2 {
            return Err(Error::value("need two cadence/chart banks"));
        }
    }
    let mut base = factory();
    for key in ["name", "scales", "forms", "bass", "drums", "sounds"] {
        base[key] = get(d, key)?.clone();
    }
    let mut b = compile(&base, limits)?;
    b.truncate(MIN_V1);
    b.resize(MIN_V2, 0);
    b[3] = if v3 { b'3' } else { b'2' };
    b[4] = if v3 { 3 } else { 2 };
    b[5] = integer(&json!(list(get(d, "nodes")?)?.len()), 1, 32, "nodes")? as u8;
    for (mode, group) in list(get(d, "cadence")?)?.iter().enumerate() {
        if mode >= 2 {
            return Err(Error::value("need two cadences"));
        }
        if list(group)?.len() != 3 {
            return Err(Error::value("cadence needs tonic/ii/dominant"));
        }
        for (n, node) in list(group)?.iter().enumerate() {
            let x = integer(node, 0, b[5] as i64 - 1, "cadence")? as usize;
            b[9 + 3 * mode + n] = x as u8;
            if !v3 && get(&d["nodes"][x], "mode")? != &json!(mode) {
                return Err(Error::value("cadence crosses mode"));
            }
        }
    }
    for (i, node) in list(&d["nodes"])?.iter().enumerate() {
        let o = if i < 16 {
            NODE + i * 8
        } else {
            V2_EXTRA + (i - 16) * 8
        };
        for (k, (name, hi)) in [
            ("bass", 11),
            ("sixth", 11),
            ("scale", 1),
            ("label", 7),
            ("mode", 1),
            ("emphasis", 15),
            ("borrow", 15),
        ]
        .iter()
        .enumerate()
        {
            b[o + k] = integer(node.get(*name).unwrap_or(&json!(0)), 0, *hi, name)? as u8;
        }
        if v3 {
            b[o + 7] = integer(get(node, "quality")?, 0, 16, "quality")? as u8;
        }
    }
    b[V2_HEAD..V2_HEAD + 4].copy_from_slice(if v3 { b"SNG3" } else { b"SNG2" });
    let p = get(d, "parts")?;
    let voices = list(get(d, "voice_programs")?)?;
    b[V2_HEAD + 4] = integer(&json!(voices.len()), 1, 8, "voice programs")? as u8;
    for (i, name) in ["keys", "bass"].iter().enumerate() {
        b[V2_HEAD + 5 + i] = integer(get(p, name)?, 0, voices.len() as i64 - 1, name)? as u8;
    }
    b[V2_HEAD + 7] = integer(get(p, "swell")?, 0, 23, "swell sound")? as u8;
    for (i, name) in ["keys_gain", "bass_gain", "swell_amount"]
        .iter()
        .enumerate()
    {
        b[V2_HEAD + 8 + i] = integer(get(p, name)?, 0, 100, name)? as u8;
    }
    b[V2_HEAD + 11] = integer(p.get("variation").unwrap_or(&json!(1)), 0, 3, "variation")? as u8;
    b[V2_HEAD + 12] = integer(d.get("song_id").unwrap_or(&json!(0)), 0, 2, "song id")? as u8;
    for (offset, name, default, lo) in [
        (16, "keys_gate", 65, 1),
        (17, "bass_gate", 58, 1),
        (18, "touch", 75, 0),
        (19, "brightness", 50, 0),
        (20, "attack", 0, 0),
    ] {
        b[V2_HEAD + offset] = integer(
            p.get(name).unwrap_or(&json!(default)),
            lo,
            100,
            if offset < 18 {
                if offset == 16 {
                    "keys gate"
                } else {
                    "bass gate"
                }
            } else {
                ["touch", "brightness", "attack"][offset - 18]
            },
        )? as u8;
    }
    if v3 {
        b[V2_HEAD + 21] = integer(p.get("voicing").unwrap_or(&json!(0)), 0, 1, "voicing")? as u8;
        b[V2_HEAD + 22] = integer(
            p.get("voice_flags").unwrap_or(&json!(0)),
            0,
            3,
            "voice flags",
        )? as u8;
        for (offset, name) in [(23, "keys_octave"), (24, "bass_octave")] {
            b[V2_HEAD + offset] =
                integer(p.get(name).unwrap_or(&json!(0)), -2, 2, name)? as i8 as u8;
        }
        b[V2_HEAD + 25] = integer(p.get("synth").unwrap_or(&json!(0)), 0, 1, "synth mode")? as u8;
    }
    let charts = get(d, "charts")?;
    for mode in 0..2 {
        if v3 && list(at(charts, mode)?)?.len() != 2 {
            return Err(Error::value("need 8/16-bar chart versions"));
        }
        for (length, bars) in [8, 16].iter().enumerate() {
            if v3 && list(at(at(charts, mode)?, length)?)?.len() != 2 {
                return Err(Error::value("need A/B chart parts"));
            }
            for part in 0..2 {
                let chart = list(at(at(at(charts, mode)?, length)?, part)?)?;
                if chart.len() != bars * 2 {
                    return Err(Error::value("chart needs two destinations per bar"));
                }
                for k in 0..32 {
                    let x = integer(
                        &chart[k.min(chart.len() - 1)],
                        0,
                        b[5] as i64 - 1,
                        "chart node",
                    )? as usize;
                    if !v3 && d["nodes"][x]["mode"] != json!(mode) {
                        return Err(Error::value("chart crosses mode"));
                    }
                    b[V2_CHART + ((mode * 2 + length) * 2 + part) * 32 + k] = x as u8;
                }
            }
        }
    }
    for (i, voice) in voices.iter().enumerate() {
        let data = list(get(voice, "data")?)?;
        if data.len() != 128 {
            return Err(Error::value("DX program needs 128 bytes"));
        }
        for (k, x) in data.iter().enumerate() {
            b[V2_VOICE + i * 128 + k] = integer(x, 0, 127, "DX byte")? as u8;
        }
    }
    b.extend(serde_json::to_vec(d.get("sources").unwrap_or(&json!([])))?);
    if b.len() > fm1_types::STORE_MAX {
        return Err(Error::value("song exceeds 3840 bytes"));
    }
    if v3 {
        JamPack::parse_structural(&b)
            .map_err(|e| Error::value(format!("invalid authored pack: {e:?}")))?;
    }
    // Preserve the legacy v2 authoring profile, including its canonicalization
    // of historical metadata and omitted cadence rows. Strict target validation
    // remains available through the existing `fm1-rs validate` command.
    Ok(b)
}
pub fn decode(b: &[u8], limits: Limits<'_>) -> Result<Document> {
    if b.starts_with(b"JMP2\x02") || b.starts_with(b"JMP3\x03") {
        return decode_v2(b, limits);
    }
    if !(MIN_V1..=fm1_types::STORE_MAX).contains(&b.len()) || !b.starts_with(b"JMP1\x01") {
        return Err(Error::value("unsupported or truncated pack"));
    }
    if !(1..=16).contains(&b[5])
        || !(1..=8).contains(&b[6])
        || !(1..=8).contains(&b[7])
        || b[8] > 32
    {
        return Err(Error::value("invalid pack counts"));
    }
    let mut d = json!({"version":1,"name":read_name(b,16,24)?,"scales":[&b[40..48],&b[48..56]],"cadence":[&b[9..12],&b[12..15]],"nodes":[],"transitions":[],"forms":[],"bass":[],"drums":[],"sounds":[]});
    for i in 0..b[5] as usize {
        let o = NODE + i * 8;
        d["nodes"].as_array_mut().unwrap().push(json!({"bass":b[o],"sixth":b[o+1],"scale":b[o+2],"label":b[o+3],"mode":b[o+4],"emphasis":b[o+5]}));
        d["transitions"]
            .as_array_mut()
            .unwrap()
            .push(json!(&b[EDGE + i * 16..EDGE + i * 16 + b[5] as usize]));
    }
    for i in 0..4 {
        let o = FORM + i * 32;
        let end = o + 17 + b[o + 16] as usize;
        let roles = b
            .get(o + 17..end)
            .ok_or_else(|| Error::value("section count outside pack"))?;
        d["forms"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name":read_name(b,o,16)?,"roles":roles}));
    }
    for i in 0..b[6] as usize {
        let o = BASS + i * 36;
        let notes: Vec<_> = b[o..o + 16].iter().map(|n| *n as i8).collect();
        d["bass"]
            .as_array_mut()
            .unwrap()
            .push(json!({"notes":notes,"velocity":&b[o+16..o+32],"gate":b[o+32]}));
    }
    for i in 0..b[7] as usize {
        let o = DRUM + i * 16;
        let masks: Vec<_> = (0..7)
            .map(|j| u16::from_le_bytes([b[o + j * 2], b[o + j * 2 + 1]]))
            .collect();
        d["drums"].as_array_mut().unwrap().push(
            json!({"lanes":&masks[..6],"comp":masks[6],"comp_gate":if b[o+14]==0{80}else{b[o+14]}}),
        );
    }
    for i in 0..b[8] as usize {
        let o = SOUND + i * 4;
        d["sounds"].as_array_mut().unwrap().push(json!({"parameter":u16::from_le_bytes([b[o],b[o+1]]),"value":i16::from_le_bytes([b[o+2],b[o+3]])}));
    }
    d["sources"] = if b.len() == MIN_V1 {
        json!([])
    } else {
        serde_json::from_slice(&b[MIN_V1..])?
    };
    compile(&d, limits)?;
    Ok(Document(d))
}
fn decode_v2(b: &[u8], limits: Limits<'_>) -> Result<Document> {
    if !(MIN_V2..=fm1_types::STORE_MAX).contains(&b.len()) {
        return Err(Error::value("invalid v2 pack"));
    }
    let v3 = b[4] == 3;
    if v3 {
        JamPack::parse_structural(b)
            .map_err(|e| Error::value(format!("invalid v3 pack: {e:?}")))?;
    }
    // Validate variable counts before indexing. Legacy decode would later reject
    // these counts during recompilation; explicit bounds avoid partial access.
    if b[5] > 32 || b[V2_HEAD + 4] > 8 {
        return Err(Error::value("invalid v2 pack counts"));
    }
    let mut base = compile(&factory(), limits)?;
    base.truncate(MIN_V1);
    base[16..40].copy_from_slice(&b[16..40]);
    base[40..56].copy_from_slice(&b[40..56]);
    base[448..1120].copy_from_slice(&b[448..1120]);
    base[6..9].copy_from_slice(&b[6..9]);
    let mut d = decode(&base, limits)?.into_value();
    d["version"] = json!(b[4]);
    d["nodes"] = json!([]);
    for i in 0..b[5] as usize {
        let o = if i < 16 {
            NODE + i * 8
        } else {
            V2_EXTRA + (i - 16) * 8
        };
        d["nodes"].as_array_mut().unwrap().push(json!({"bass":b[o],"sixth":b[o+1],"scale":b[o+2],"label":b[o+3],"mode":b[o+4],"emphasis":b[o+5],"borrow":b[o+6]}));
        if v3 {
            d["nodes"][i]["quality"] = json!(b[o + 7]);
        }
    }
    d["cadence"] = json!([&b[9..12], &b[12..15]]);
    d.as_object_mut().unwrap().shift_remove("transitions");
    d["charts"] = json!((0..2)
        .map(|m| (0..2)
            .map(|l| (0..2)
                .map(|p| {
                    let o = V2_CHART + ((m * 2 + l) * 2 + p) * 32;
                    b[o..o + if l == 0 { 16 } else { 32 }].to_vec()
                })
                .collect::<Vec<_>>())
            .collect::<Vec<_>>())
        .collect::<Vec<_>>());
    d["voice_programs"] = json!((0..b[V2_HEAD + 4] as usize)
        .map(|i| {
            let o = V2_VOICE + i * 128;
            let name: String = b[o + 118..o + 128]
                .iter()
                .map(|b| {
                    if b.is_ascii() {
                        char::from(*b)
                    } else {
                        '\u{fffd}'
                    }
                })
                .collect();
            json!({"name":name.trim(),"data":&b[o..o+128]})
        })
        .collect::<Vec<_>>());
    d["parts"] = json!({});
    for (i, name) in [
        "keys",
        "bass",
        "swell",
        "keys_gain",
        "bass_gain",
        "swell_amount",
        "variation",
    ]
    .iter()
    .enumerate()
    {
        d["parts"][name] = json!(b[V2_HEAD + 5 + i]);
    }
    for (offset, name) in [
        (16, "keys_gate"),
        (17, "bass_gate"),
        (18, "touch"),
        (19, "brightness"),
        (20, "attack"),
    ] {
        d["parts"][name] = json!(b[V2_HEAD + offset]);
    }
    d["song_id"] = json!(b[V2_HEAD + 12]);
    if v3 {
        d["parts"]["voicing"] = json!(b[V2_HEAD + 21]);
        d["parts"]["voice_flags"] = json!(b[V2_HEAD + 22]);
        d["parts"]["keys_octave"] = json!(b[V2_HEAD + 23] as i8);
        d["parts"]["bass_octave"] = json!(b[V2_HEAD + 24] as i8);
        d["parts"]["synth"] = json!(b[V2_HEAD + 25]);
    }
    d["sources"] = if b.len() == MIN_V2 {
        json!([])
    } else {
        serde_json::from_slice(&b[MIN_V2..])?
    };
    compile(&d, limits)?;
    Ok(Document(d))
}
