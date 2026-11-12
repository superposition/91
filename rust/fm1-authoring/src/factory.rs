// SPDX-License-Identifier: GPL-3.0-only
use crate::{Error, Result};
use fm1_types::ParamId;
use serde_json::{json, Value};
pub fn factory() -> Value {
    let modes = [
        [
            (0, 0, 0, 0),
            (2, 5, 0, 1),
            (4, 7, 0, 1),
            (5, 5, 0, 0),
            (7, 2, 1, 3),
            (9, 0, 0, 1),
            (11, 2, 1, 2),
        ],
        [
            (0, 0, 1, 4),
            (2, 5, 1, 2),
            (3, 3, 0, 0),
            (5, 8, 0, 1),
            (7, 2, 1, 3),
            (8, 8, 0, 0),
            (11, 2, 1, 2),
        ],
    ];
    let nodes:Vec<_>=modes.iter().enumerate().flat_map(|(mode,rows)|rows.iter().enumerate().map(move |(i,(a,b,c,d))|json!({"bass":a,"sixth":b,"scale":c,"label":d,"mode":mode,"emphasis":if [3,5].contains(&i){10}else if [1,2].contains(&i){4}else{0}}))).collect();
    let mut edges = [[0; 14]; 14];
    let routes: &[&[(usize, u8)]] = &[
        &[(1, 4), (3, 4), (5, 5)],
        &[(4, 12), (6, 1)],
        &[(5, 10), (3, 3)],
        &[(1, 7), (4, 6), (0, 3)],
        &[(0, 12), (5, 3)],
        &[(1, 10), (3, 3)],
        &[(4, 10), (0, 3)],
    ];
    for mode in 0..2 {
        for (i, route) in routes.iter().enumerate() {
            for &(j, w) in *route {
                edges[7 * mode + i][7 * mode + j] = w;
            }
        }
    }
    let patterns: Vec<_> = [
        vec![(0, 0), (3, -125), (6, 0), (8, -126), (10, -125), (14, -127)],
        vec![(0, 0), (2, 0), (6, -125), (8, 0), (11, -126), (14, -127)],
        vec![(0, 0), (4, -126), (6, -125), (8, 0), (10, 12), (15, -127)],
        vec![(0, 0), (3, 0), (6, -125), (8, -126), (12, -125), (14, -127)],
    ]
    .iter()
    .map(|p| {
        let mut notes = [-128; 16];
        let mut velocity = [0; 16];
        for &(i, n) in p {
            notes[i] = n;
            velocity[i] = if i == 0 {
                100
            } else if i % 4 == 0 {
                88
            } else {
                72
            };
        }
        json!({"notes":notes,"velocity":velocity,"gate":58})
    })
    .collect();
    macro_rules! sounds {($($name:ident:$value:expr),*)=>{vec![$(json!({"parameter":ParamId::$name.index(),"value":$value,"name":concat!("P_",stringify!($name))})),*]};}
    let sounds = sounds!(PAD:56,BASS:72,PULSE:60,PADTONE:0,BRIGHT:18,PE_ATK:6,PE_REL:32,DR_INST:23,DR_CUT:42,DR_RES:8,DE_ATK:0,DE_DEC:25,DE_AMT:6,DRIVE:0,DETUNE:0,PL_DEPTH:0,DL_DEPTH:0,PL_CHO:0,BL_LEN:45,BL_GLIDE:0,REVERB:10,SIZE:12,RV_TYPE:0,RV_DAMP:75,ECHO:0,MOTION:0,HUMAN:12,SWING:8,KIT_KICK:4,KIT_SNARE:4,KIT_HAT:4,KIT_PERC:4);
    json!({"version":1,"name":"Harris Citypop","scales":[[0,2,4,5,7,8,9,11],[0,2,3,5,7,8,9,11]],"nodes":nodes,"cadence":[[0,1,4],[7,8,11]],"transitions":edges,"forms":[{"name":"Vamp","roles":[0]},{"name":"Verse / Chorus","roles":[0,1]},{"name":"AABA","roles":[0,0,2,0]},{"name":"Journey","roles":[0,1,2,3]}],"bass":patterns,"drums":[{"lanes":[0x0441,0x1010,0,0x5555,0x0400,0x8000],"comp":0x1111},{"lanes":[0x4541,0x1010,0x1000,0x5555,0x4040,0x8000],"comp":0x1111}],"sounds":sounds,"sources":["Original generated accompaniment; Barry Harris sixth-diminished framework.","https://github.com/aaelony/barry-harris-exercises","https://github.com/BenjaminPoilve/minichord/issues/37","Free MIDI reference (not copied into this factory pack): https://www.patreon.com/musicianparadise/posts/gospel-scripts-150853281"]})
}
pub fn programs() -> Value {
    let source = &fm1_engine::fm6::tables::FM6_DY;
    let mut mellow = source[0];
    let mut bark = source[0];
    let mut bloom = source[0];
    for op in 0..6 {
        mellow[op * 17 + 13] = (mellow[op * 17 + 13] & 3) | (3 << 2);
        bark[op * 17 + 14] = (bark[op * 17 + 14] + if op % 2 == 0 { 5 } else { 0 }).min(99);
        bloom[op * 17 + 1] = bloom[op * 17 + 1].min(35);
        bloom[op * 17 + 7] = 0;
    }
    json!([
        ("Mellow Tine", mellow),
        ("Bark EP", bark),
        ("Bloom EP", bloom),
        ("Round FM", source[2]),
        ("Articulated FM", source[9])
    ]
    .into_iter()
    .map(|(name, mut data)| {
        data[118..128].fill(b' ');
        let count = name.len().min(10);
        data[118..118 + count].copy_from_slice(&name.as_bytes()[..count]);
        json!({"name":name,"data":data.as_slice()})
    })
    .collect::<Vec<_>>())
}
pub fn song(song: usize) -> Result<Value> {
    if song >= 3 {
        return Err(Error::value("song: expected integer 0..2"));
    }
    let mut d = factory();
    d["version"] = json!(2);
    d["name"] = json!(["Velvet Changes", "Afterhours", "Neon Turnaround"][song]);
    let modes = [
        [
            (0, 7, 0, 6),
            (2, 5, 0, 1),
            (4, 7, 0, 1),
            (5, 0, 0, 6),
            (7, 2, 1, 3),
            (9, 0, 0, 1),
            (11, 2, 1, 2),
            (9, 10, 1, 7),
            (5, 5, 1, 4),
            (4, 5, 1, 7),
            (2, 7, 1, 3),
            (6, 9, 1, 2),
            (0, 0, 0, 0),
            (7, 8, 1, 7),
            (5, 5, 0, 0),
            (0, 0, 0, 0),
        ],
        [
            (0, 0, 1, 4),
            (2, 5, 1, 2),
            (3, 10, 0, 6),
            (5, 8, 0, 1),
            (7, 8, 1, 7),
            (8, 8, 0, 0),
            (11, 2, 1, 2),
            (9, 10, 1, 7),
            (5, 5, 1, 4),
            (7, 2, 1, 3),
            (10, 5, 1, 3),
            (6, 9, 1, 2),
            (0, 0, 1, 4),
            (7, 8, 1, 7),
            (8, 3, 0, 6),
            (0, 0, 1, 4),
        ],
    ];
    d["nodes"]=json!(modes.iter().enumerate().flat_map(|(mode,rows)|rows.iter().map(move |(bass,sixth,scale,label)|json!({"bass":bass,"sixth":sixth,"scale":scale,"label":label,"mode":mode,"emphasis":0,"borrow":if [0,1].contains(label){9}else if *label==6{0}else{1}}))).collect::<Vec<_>>());
    d["cadence"] = json!([[0, 1, 4], [16, 17, 20]]);
    let charts = [
        [
            [
                [0, 0, 5, 5, 1, 1, 4, 4, 2, 7, 1, 4, 12, 14, 1, 4],
                [3, 3, 8, 8, 2, 2, 7, 7, 1, 1, 4, 13, 0, 0, 4, 4],
            ],
            [
                [0, 0, 12, 12, 3, 3, 10, 4, 2, 7, 1, 4, 0, 14, 1, 4],
                [3, 3, 10, 10, 2, 7, 1, 4, 3, 8, 2, 7, 1, 4, 0, 0],
            ],
        ],
        [
            [
                [0, 0, 0, 0, 3, 3, 10, 10, 2, 2, 5, 5, 1, 1, 4, 4],
                [8, 8, 1, 4, 0, 0, 7, 7, 1, 1, 4, 4, 0, 0, 4, 4],
            ],
            [
                [0, 0, 0, 0, 3, 3, 10, 10, 2, 2, 5, 5, 1, 1, 4, 4],
                [8, 8, 1, 4, 0, 0, 7, 7, 1, 1, 4, 4, 0, 0, 4, 4],
            ],
        ],
        [
            [
                [0, 0, 9, 9, 5, 5, 10, 10, 1, 1, 4, 4, 0, 7, 1, 4],
                [3, 3, 11, 11, 2, 7, 1, 4, 14, 8, 2, 7, 1, 4, 0, 0],
            ],
            [
                [0, 0, 7, 7, 3, 3, 10, 10, 2, 2, 5, 5, 1, 1, 4, 4],
                [8, 8, 11, 11, 2, 7, 1, 4, 8, 8, 2, 7, 1, 4, 0, 0],
            ],
        ],
    ][song];
    d["charts"] = json!((0..2)
        .map(|mode| [8, 16]
            .into_iter()
            .map(|bars| (0..2)
                .map(|part| {
                    let a = &charts[mode][part];
                    let sequence = if bars == 8 {
                        a.to_vec()
                    } else {
                        let mut b = a[..12].to_vec();
                        b.extend([1, 4, 0, 0]);
                        let copy = b.clone();
                        b.extend(copy);
                        b
                    };
                    sequence.iter().map(|n| n + 16 * mode).collect::<Vec<_>>()
                })
                .collect::<Vec<_>>())
            .collect::<Vec<_>>())
        .collect::<Vec<_>>());
    d["forms"] = json!([{"name":"Authored","roles":if song==2{vec![0,1,0,3]}else{vec![0,0,2,3]}},{"name":"Vamp","roles":[0]},{"name":"AABA","roles":[0,0,2,3]},{"name":"ABAB","roles":[0,1,0,1]}]);
    d["voice_programs"] = programs();
    d["parts"] = json!({"keys":([0,2,1][song]),"bass":if song==2{4}else{3},"swell":([2,5,19][song]),"keys_gain":45,"bass_gain":35,"swell_amount":30,"variation":1,"keys_gate":65,"bass_gate":58});
    d["song_id"] = json!(song);
    for setting in d["sounds"].as_array_mut().unwrap() {
        if [json!(47), json!(163)].contains(&setting["parameter"]) {
            setting["value"] = json!(0);
        }
    }
    for drum in d["drums"].as_array_mut().unwrap() {
        drum["comp"] = json!(if song == 1 { 0x0441 } else { 0x4441 });
        drum["comp_gate"] = json!(65);
    }
    d["sources"]=json!(["Original composition and arrangement; Harris-informed Citypop.","https://jazzworkshops.com/workshop-item/evolutionary-voicings-part-1/","DX voices adapted from PERF-1 public-domain DY bank; see tools/fm6_dy.txt and LICENSING.md."]);
    Ok(d)
}
