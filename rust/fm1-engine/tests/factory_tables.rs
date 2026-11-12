// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::factory;
use fm1_types::{
    AcidLine, AcidStep, ChordKinds, GenreId, PieceId, PluckLine, PluckStep, Progression, StepId,
    PARAMETERS,
};
use serde_json::Value;
fn corpus() -> Value {
    serde_json::from_str(include_str!("factory/reference.json")).unwrap()
}
#[test]
fn every_factory_output_matches_independent_r01_c() {
    let c = corpus();
    for row in c["presets"].as_array().unwrap() {
        let g = GenreId::new(row["genre"].as_u64().unwrap() as usize).unwrap();
        let i = PieceId::new(row["piece"].as_u64().unwrap() as usize).unwrap();
        let p = factory::preset(g, i);
        for (index, value) in p.parameters.iter().enumerate() {
            assert_eq!(
                *value as i64,
                row["parameters"][index].as_i64().unwrap(),
                "genre{} piece{} parameter{}",
                g.index(),
                i.index(),
                index
            );
        }
        assert_eq!(p.pool as u64, row["pool"].as_u64().unwrap());
        for (index, mask) in factory::patterns(g, i).iter().flatten().enumerate() {
            assert_eq!(mask.bits() as u64, row["patterns"][index].as_u64().unwrap());
        }
        for (index, modifier) in factory::modifiers(g, i).iter().flatten().enumerate() {
            assert_eq!(
                modifier.bits() as u64,
                row["modifiers"][index].as_u64().unwrap()
            );
        }
    }
    for (index, parameter) in PARAMETERS.iter().enumerate() {
        let p = &c["parameters"][index];
        assert_eq!(parameter.name, p[0].as_str().unwrap());
        assert_eq!(parameter.min as i64, p[1].as_i64().unwrap());
        assert_eq!(parameter.max as i64, p[2].as_i64().unwrap());
        assert_eq!(parameter.default as i64, p[3].as_i64().unwrap());
    }
    let order = [
        "EN_NOTE_NAME",
        "EN_MODE_NAME",
        "EN_LANE_NAME",
        "BL_FIG_NAME",
        "GENRE_NAME",
        "ETIME_NAME",
        "LFO_RATE_NAME",
        "LFO_SHAPE_NAME",
        "PAD_DEST_NAME",
        "DRONE_DEST_NAME",
        "NOISE_NAME",
        "TRIG_NAME",
        "THEME_NAME",
        "EN_PAD_SOUND",
        "EN_DRONE_SOUND",
        "EN_ACID_SOUND",
        "EN_KIT_NAME",
        "EN_CHORD_NAME",
        "EN_MAP_NAME",
        "EN_CM_SOUND",
        "FM_DX_NAME",
        "EN_CM_STYLE",
        "EN_CM_RATE",
        "EN_PT_NAME",
        "EN_FX_NAME",
        "GEN_PIECE_NAME",
    ];
    for (category, key) in order.iter().enumerate() {
        for (index, name) in c["names"][key].as_array().unwrap().iter().enumerate() {
            let actual = factory::name(category, index).unwrap();
            assert_eq!(
                &actual[..actual.len() - 1],
                name.as_str().unwrap().as_bytes()
            );
            assert_eq!(actual.last(), Some(&0));
        }
        assert!(factory::name(category, c["names"][key].as_array().unwrap().len()).is_none());
    }
    for (index, value) in factory::mode_pools().iter().enumerate() {
        assert_eq!(*value as u64, c["mode_pools"][index].as_u64().unwrap());
    }
    for (index, value) in factory::boot_patterns().iter().flatten().enumerate() {
        assert_eq!(
            value.bits() as u64,
            c["boot_patterns"][index].as_u64().unwrap()
        );
    }
    for i in 0..10 {
        let (a, k) = factory::arc(PieceId::new(i).unwrap());
        for (j, v) in a.iter().enumerate() {
            assert_eq!(*v as u64, c["arcs"][i][0][j].as_u64().unwrap());
        }
        for (j, v) in k.iter().enumerate() {
            assert_eq!(*v as u64, c["arcs"][i][1][j].as_u64().unwrap());
        }
    }
    assert!(GenreId::new(19).is_err());
    assert!(PieceId::new(10).is_err());
    assert!(factory::name(99, 0).is_none());
    assert!(factory::mode(8).is_none());
    assert_eq!(factory::numeric_storage_bytes(), 6542);
}
#[test]
fn every_packed_step_matches_frozen_cross_byte_vectors() {
    let c = corpus();
    for seed in 0..512 {
        let mut line = AcidLine::EMPTY;
        for slot in 0..16 {
            let value = ((seed + 37 * slot) & 511) as u16;
            let step = AcidStep::from_bits(value);
            assert_eq!(
                AcidStep::new(
                    step.offset(),
                    step.octave(),
                    step.gate(),
                    step.accent(),
                    step.slide()
                )
                .unwrap(),
                step
            );
            line.set(StepId::new(slot).unwrap(), step);
            assert_eq!(line.step(StepId::new(slot).unwrap()), step);
        }
        for (index, value) in line.bytes().iter().enumerate() {
            assert_eq!(*value as u64, c["acid"][seed][index].as_u64().unwrap());
        }
    }
    for seed in 0..64 {
        let mut line = PluckLine::EMPTY;
        for slot in 0..16 {
            let step = PluckStep::from_bits(((seed + 13 * slot) & 63) as u8);
            line.set(StepId::new(slot).unwrap(), step);
            assert_eq!(line.step(StepId::new(slot).unwrap()), step);
        }
        for (index, value) in line.bytes().iter().enumerate() {
            assert_eq!(*value as u64, c["pluck"][seed][index].as_u64().unwrap());
        }
    }
    assert!(AcidStep::new(16, 0, false, false, false).is_err());
    assert!(AcidStep::new(0, 4, false, false, false).is_err());
    assert!(PluckStep::new(16, 0).is_err());
    assert!(PluckStep::new(0, 4).is_err());
    assert!(StepId::new(16).is_err());
    for slot in 0..8 {
        for value in 0..8 {
            let mut p = Progression::from_bytes([255; 6]);
            p.set_degree(slot, value).unwrap();
            assert_eq!(p.degree(slot).unwrap(), value);
            assert_eq!(p.bytes()[5] & 0xf8, 0xf8);
            let mut k = ChordKinds::EMPTY;
            k.set(slot, value).unwrap();
            assert_eq!(k.kind(slot).unwrap(), value);
        }
        for value in 0..4 {
            let mut p = Progression::EMPTY;
            p.set_bars_log2(slot, value).unwrap();
            assert_eq!(p.bars_log2(slot).unwrap(), value);
        }
    }
    let mut p = Progression::from_bytes([0xa5; 6]);
    for value in 0..4 {
        p.set_unit(value).unwrap();
        assert_eq!(p.unit(), value);
    }
    p.set_playing(true);
    assert!(p.playing());
    p.set_playing(false);
    assert!(!p.playing());
    assert!(p.set_degree(8, 0).is_err());
    assert!(p.set_degree(0, 8).is_err());
    assert!(p.set_bars_log2(0, 4).is_err());
    assert!(p.set_unit(4).is_err());
}

#[test]
fn every_progression_field_preserves_the_frozen_c_layout() {
    let rows: Value = serde_json::from_str(include_str!("factory/boundaries.json")).unwrap();
    for row in rows.as_array().unwrap() {
        let kind = row[0].as_u64().unwrap();
        let slot = row[1].as_u64().unwrap() as usize;
        let value = row[2].as_u64().unwrap() as u8;
        let mut progression = Progression::from_bytes([0xa5; 6]);
        let mut kinds = ChordKinds::from_bytes([0xa5; 3]);
        match kind {
            0 => progression.set_degree(slot, value).unwrap(),
            1 => progression.set_bars_log2(slot, value).unwrap(),
            2 => progression.set_unit(value).unwrap(),
            3 => progression.set_playing(value != 0),
            4 => kinds.set(slot, value).unwrap(),
            _ => unreachable!(),
        }
        let mut bytes = progression.bytes();
        if kind == 4 {
            bytes[..3].copy_from_slice(&kinds.bytes());
        }
        for (index, byte) in bytes.iter().enumerate() {
            assert_eq!(*byte as u64, row[3][index].as_u64().unwrap());
        }
    }
    for bit in [0u16, u16::MAX] {
        let mask = fm1_types::StepMask::from_bits(bit);
        for step in 0..16 {
            assert_eq!(mask.contains(step), bit >> step & 1 != 0);
        }
        assert!(!mask.contains(16));
    }
    for bits in 0..=u16::MAX {
        let mask = fm1_types::ScaleMask::from_bits(bits);
        assert_eq!(mask.bits(), bits & 0xfff);
        assert!(!mask.contains(12));
    }
    assert_eq!(core::mem::size_of::<fm1_types::PatternBank>(), 72);
    assert_eq!(core::mem::size_of::<factory::FactoryPreset>(), 354);
    assert_eq!(core::mem::size_of::<AcidLine>(), 18);
    assert_eq!(core::mem::size_of::<PluckLine>(), 12);
    assert_eq!(core::mem::size_of::<Progression>(), 6);
    assert_eq!(core::mem::size_of::<ChordKinds>(), 3);
    assert!(fm1_types::PatternId::new(6).is_err());
    assert!(fm1_types::LaneId::new(6).is_err());
    assert!(StepId::new(usize::MAX).is_err());
    let mut p = Progression::from_bytes([0xa5; 6]);
    let unchanged = p;
    assert!(p.set_degree(usize::MAX, 0).is_err());
    assert!(p.set_degree(0, 8).is_err());
    assert_eq!(p, unchanged);
    let mut k = ChordKinds::from_bytes([0xa5; 3]);
    let unchanged = k;
    assert!(k.set(8, 0).is_err());
    assert!(k.set(0, 8).is_err());
    assert_eq!(k, unchanged);
    let mut acid = AcidLine::EMPTY;
    let mut pluck = PluckLine::EMPTY;
    for slot in 0..16 {
        acid.set(StepId::new(slot).unwrap(), AcidStep::from_bits(511));
        pluck.set(StepId::new(slot).unwrap(), PluckStep::from_bits(63));
    }
    assert_eq!(acid.bytes(), [255; 18]);
    assert_eq!(pluck.bytes(), [255; 12]);
    assert!(!PluckLine::EMPTY.any());
    assert!(pluck.any());
}
