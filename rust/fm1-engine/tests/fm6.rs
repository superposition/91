// SPDX-License-Identifier: GPL-3.0-only
//! Component outputs recorded from immutable R01 C, not recomputed Rust goldens.
use fm1_engine::fm6::{self, envelopes, lfo, operators, pool, program, types::*, *};
use serde_json::Value;

struct Tables {
    sin: [f32; 1025],
    ml: [u16; 2048],
    me: [u16; 1024],
    ol: [u16; 512],
    oe: [u16; 256],
}
impl Tables {
    fn new() -> Self {
        let mut t = Self {
            sin: [0.; 1025],
            ml: [0; 2048],
            me: [0; 1024],
            ol: [0; 512],
            oe: [0; 256],
        };
        fm6::initialize_tables(&mut t.sin, &mut t.ml, &mut t.me, &mut t.ol, &mut t.oe);
        t
    }
    fn view(&self) -> Lookup<'_> {
        Lookup {
            sin: &self.sin,
            mki_log: &self.ml,
            mki_exp: &self.me,
            opl_log: &self.ol,
            opl_exp: &self.oe,
        }
    }
}
struct Buffers {
    bus: [[f32; 64]; 2],
    sum: [f32; 64],
    ibus: [[i32; 64]; 2],
    isum: [i32; 64],
}
impl Buffers {
    fn new() -> Self {
        let mut b = Self {
            bus: [[0.; 64]; 2],
            sum: [0.; 64],
            ibus: [[0; 64]; 2],
            isum: [0; 64],
        };
        for i in 0..64 {
            b.sum[i] = (i as i32 - 31) as f32 * 0.03125;
            b.isum[i] = (i as i32 - 31) * 524288;
            for k in 0..2 {
                b.bus[k][i] = ((i * 7 + k * 3) % 31) as f32 * 0.125 - 15. * 0.125;
                b.ibus[k][i] = ((i * 7 + k * 3) % 31) as i32 * 2097152 - 15 * 2097152;
            }
        }
        b
    }
    fn view(&mut self) -> Scratch<'_> {
        Scratch {
            bus: &mut self.bus,
            sum: &mut self.sum,
            ibus: &mut self.ibus,
            isum: &mut self.isum,
        }
    }
}
fn state(engine: u32) -> Voice {
    let mut s = Voice::default();
    for k in 0..6 {
        s.ph[k] = (u32::MAX - 31).wrapping_add(k as u32 * 997);
        s.fq[k] = 170003 + k as i32 * 1193;
        s.g[k] = if engine == 0 {
            0.25
        } else if engine == 1 {
            12000.
        } else {
            64.
        };
        s.dg[k] = if engine == 0 {
            -0.001953125
        } else if engine == 1 {
            -64.
        } else {
            0.25
        };
    }
    s.fb = [-0.125, 0.0625];
    s.fbs = 7;
    s
}
struct Hash(u32);
impl Hash {
    fn new() -> Self {
        Self(2166136261)
    }
    fn bytes(&mut self, v: &[u8]) {
        for x in v {
            self.0 = (self.0 ^ *x as u32).wrapping_mul(16777619);
        }
    }
    fn u(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }
    fn i(&mut self, v: i32) {
        self.u(v as u32);
    }
    fn floats(&mut self, v: &[f32]) {
        for x in v {
            self.u(x.to_bits());
        }
    }
    fn ints(&mut self, v: &[i32]) {
        for x in v {
            self.i(*x);
        }
    }
    fn eg(&mut self, e: &Envelope) {
        self.i(e.level);
        self.i(e.target);
        self.i(e.inc);
        self.i(e.hold);
        self.bytes(&[e.ix, e.rising]);
    }
    fn peg(&mut self, e: &PitchEnvelope) {
        self.i(e.pl);
        self.i(e.pt);
        self.i(e.pi);
        self.bytes(&[e.pix, e.prise]);
        self.bytes(&e.pr);
        self.bytes(&e.pv);
    }
    fn op(&mut self, s: &Voice, b: &Buffers) {
        for x in s.ph {
            self.u(x);
        }
        self.floats(&s.g);
        self.floats(&s.fb);
        for x in &b.bus {
            self.floats(x);
        }
        self.floats(&b.sum);
        for x in &b.ibus {
            self.ints(x);
        }
        self.ints(&b.isum);
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("fm6-components.json")).unwrap()
}
fn check(golden: &Value, index: &mut usize, hash: Hash) {
    assert_eq!(
        golden[*index].as_u64().unwrap() as u32,
        hash.0,
        "vector {}",
        *index
    );
    *index += 1;
}
#[test]
fn isolated_operators_aliasing_routes_feedback_and_partial_blocks() {
    let expected = fixture();
    let t = Tables::new();
    let mut index = 0;
    for eng in 0..3 {
        for k in 0..6 {
            for input in 0..3 {
                for output in 0..3 {
                    for add in [false, true] {
                        for kind in 0..if eng == 1 { 4 } else { 2 } {
                            for n in [1, 3, 63, 64] {
                                let mut s = state(eng);
                                let mut b = Buffers::new();
                                let r = operators::Route { input, output, add };
                                match kind {
                                    0 => operators::operator(
                                        &mut s,
                                        &t.view(),
                                        &mut b.view(),
                                        k,
                                        r,
                                        eng,
                                        n,
                                    ),
                                    1 => operators::feedback_operator(
                                        &mut s,
                                        &t.view(),
                                        &mut b.view(),
                                        k,
                                        r,
                                        eng,
                                        n,
                                    ),
                                    2 => operators::integer_operator(
                                        &mut s,
                                        &t.view(),
                                        &mut b.view(),
                                        k,
                                        r,
                                        n,
                                    ),
                                    _ => operators::integer_feedback_operator(
                                        &mut s,
                                        &t.view(),
                                        &mut b.view(),
                                        k,
                                        r,
                                        n,
                                    ),
                                }
                                let mut h = Hash::new();
                                h.op(&s, &b);
                                check(&expected["operators"], &mut index, h);
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(index, expected["operators"].as_array().unwrap().len());
}
#[test]
fn two_and_three_operator_feedback_loops() {
    let expected = fixture();
    let t = Tables::new();
    let mut index = 0;
    for integer in [false, true] {
        for count in 2..=3 {
            for output in 0..3 {
                for n in [1, 3, 63, 64] {
                    let mut s = state(1);
                    s.loop_ = count;
                    let mut b = Buffers::new();
                    if integer {
                        operators::integer_loop_operator(
                            &mut s,
                            &t.view(),
                            &mut b.view(),
                            output,
                            n,
                        );
                    } else {
                        operators::loop_operator(&mut s, &t.view(), &mut b.view(), output, n);
                    }
                    let mut h = Hash::new();
                    h.op(&s, &b);
                    check(&expected["loops"], &mut index, h);
                }
            }
        }
    }
    assert_eq!(index, 48);
}
#[test]
fn all_envelope_rates_levels_hold_and_release() {
    let expected = fixture();
    let mut index = 0;
    for rate in 0..100 {
        for level in [0, 50, 99] {
            for rs in [-7, 0, 7] {
                let mut op = [0; 21];
                for q in 0..4 {
                    op[q] = ((rate + q * 17) % 100) as u8;
                    op[4 + q] = level;
                }
                let mut e = Envelope::default();
                envelopes::advance(&mut e, &op, 4000, rs, 0);
                let mut h = Hash::new();
                h.eg(&e);
                for j in 0..256 {
                    if j == 96 {
                        envelopes::advance(&mut e, &op, 4000, rs, 3);
                    }
                    h.i(envelopes::step(&mut e, &op, 4000, rs, j < 96));
                    h.eg(&e);
                }
                check(&expected["envelopes"], &mut index, h);
            }
        }
    }
    assert_eq!(index, 900);
}
#[test]
fn all_pitch_rates_extreme_levels_and_release() {
    let expected = fixture();
    let mut index = 0;
    for rate in 0..100 {
        for level in [0, 50, 99] {
            let mut patch = [0; 155];
            for j in 0..4 {
                patch[PR1 + j] = ((rate + j * 17) % 100) as u8;
                patch[PL1 + j] = level;
            }
            let mut e = PitchEnvelope::default();
            envelopes::pitch_set(&mut e, &patch);
            let mut h = Hash::new();
            h.peg(&e);
            for j in 0..256 {
                if j == 96 {
                    envelopes::pitch_advance(&mut e, 3);
                }
                h.i(envelopes::pitch_step(&mut e, j < 96));
                h.peg(&e);
            }
            check(&expected["pitch"], &mut index, h);
        }
    }
    assert_eq!(index, 300);
}
#[test]
fn every_lfo_wave_rate_and_delay_wrap() {
    let expected = fixture();
    let t = Tables::new();
    let mut index = 0;
    for wave in 0..8 {
        for rate in 0..100 {
            for delay in [0, 99] {
                let mut patch = [0; 155];
                patch[LFW] = wave;
                patch[LFS] = rate;
                patch[LFD] = delay;
                let mut l = Lfo {
                    ph: u32::MAX - 100,
                    dly: u32::MAX - 100,
                    rnd: 255,
                    ..Default::default()
                };
                let mut h = Hash::new();
                for _ in 0..256 {
                    lfo::step(&mut l, &patch, &t.view());
                    h.u(l.ph);
                    h.u(l.dly);
                    h.i(l.val);
                    h.i(l.depth);
                    h.bytes(&[l.rnd]);
                }
                check(&expected["lfo"], &mut index, h);
            }
        }
    }
    assert_eq!(index, 1600);
}
#[test]
fn user_boundaries_names_mask_selection_and_reset_contract() {
    let mut s = program::UserState::default();
    assert!(!program::stage(&mut s, 16));
    assert!(!program::stage(&mut s, 1));
    assert!(program::stage(&mut s, 0));
    assert!(!program::stage(&mut s, 0));
    s.current = 30;
    assert!(program::commit(&mut s, 0));
    assert_eq!(s.current, -1);
    assert_eq!(s.pending, 0);
    assert!(!program::commit(&mut s, 0));
    let mut source = [255; 128];
    source[118..].fill(0);
    let mut dest = [0; 128];
    program::copy_packed(&source, &mut dest);
    assert_eq!(dest[0], 127);
    let mut name = [7; 16];
    program::user_name(&dest, &mut name);
    assert_eq!(&name[..5], b"DX ?\0");
    assert_eq!(name[5], 7);
    let mut selection = program::Selection {
        requested: 45,
        current: 5,
        user_count: 0,
        ..Default::default()
    };
    assert!(program::select(&mut selection));
    assert_eq!(selection.current, 0);
    assert!(!program::select(&mut selection));
    selection.dy_requested = 1;
    assert!(program::select(&mut selection));
    assert_eq!(selection.dy_current, 1);
    s.current = 0;
    program::wipe(&mut s);
    assert_eq!(s.current, 0);
    s.current = 30;
    program::wipe(&mut s);
    assert_eq!(s.current, -1);
    let mut slots = [Slot::default(); 6];
    slots[0].age = 81;
    slots[0].gate = 1;
    slots[0].s.ph[0] = 91;
    let mut lfo = Lfo {
        rnd: 77,
        ..Default::default()
    };
    pool::reset(&mut slots, &mut lfo);
    assert_eq!(slots[0].age, 81);
    assert_eq!(slots[0].gate, 1);
    assert_eq!(slots[0].s.ph[0], 0);
    assert_eq!(lfo.rnd, 0);
}
#[test]
fn all_factory_programs_carriers_and_immutable_table_shapes() {
    for dy in [false, true] {
        for i in 0..30 {
            let mut patch = [0; 155];
            program::load(i, dy, None, &mut patch);
            assert!(patch[ALG] < 32);
            assert!(program::name(i, dy).contains(&0));
            assert!(program::gain(i, dy) > 0.);
            let mut has_carrier = false;
            for op in 0..6 {
                has_carrier |= fm6::scaling::carrier(patch[ALG] as u32, 6 - op);
            }
            assert!(has_carrier);
        }
    }
    for algorithm in 0..32 {
        let row = tables::FM6_ALG[algorithm];
        assert!(row.iter().any(|x| x & 3 == 0));
        for flags in row {
            assert!(flags & 3 < 3);
            assert!((flags >> 4) & 3 < 3);
        }
    }
}

#[test]
fn protected_allocation_stolen_phase_and_legacy_age_endpoint() {
    let mut patch = [0; 155];
    program::load(0, false, None, &mut patch);
    patch[OKS] = 1;
    patch[LKS] = 1;
    let mut slots = [Slot::default(); 6];
    let mut keys = Lfo::default();
    let mut bass = Lfo::default();
    let mut age = 0;
    let mut note =
        |context: &pool::NoteContext, slots: &mut [Slot; 6], keys: &mut Lfo, bass: &mut Lfo| {
            pool::note_on(
                pool::Pool {
                    slots,
                    keys_lfo: keys,
                    bass_lfo: bass,
                },
                &patch,
                context,
                &mut age,
                60,
                0.8,
                500,
            )
            .unwrap()
        };
    let mut context = pool::NoteContext {
        jam: 1,
        current: 0,
        ..Default::default()
    };
    for expected in [0, 1, 2, 3, 5] {
        assert_eq!(
            note(&context, &mut slots, &mut keys, &mut bass).slot,
            expected
        );
    }
    assert_eq!(keys.ph, (1 << 31) - 1);
    assert_eq!(bass.ph, 0);
    context.part = 1;
    assert_eq!(note(&context, &mut slots, &mut keys, &mut bass).slot, 4);
    assert_eq!(bass.ph, (1 << 31) - 1);
    context.part = 0;
    slots[0].s.ph[0] = 998;
    slots[0].s.gout[0] = 0.75;
    slots[0].s.fb = [0.4, 0.8];
    let result = note(&context, &mut slots, &mut keys, &mut bass);
    assert_eq!(result.slot, 0);
    assert!(result.stolen);
    assert_eq!(slots[0].s.ph[0], 998);
    assert_eq!(slots[0].s.gout[0], 0.75);
    assert_eq!(slots[0].s.fb, [0.4, 0.8]);
    context.part = 1;
    slots[4].age = u32::MAX;
    let result = note(&context, &mut slots, &mut keys, &mut bass);
    assert_eq!(result.slot, 0);
    assert!(slots[4].used != 0 && slots[0].part == 1);
    assert!(pool::part_busy(&slots, 1));
    pool::retire_part(&mut slots, 1);
    assert!(!pool::part_busy(&slots, 1));
    assert!(pool::part_busy(&slots, 0));
    pool::retire_all(&mut slots);
    assert!(slots.iter().all(|x| x.used == 0));
}

#[test]
fn all_initialized_ram_lookup_bits_match_frozen_c() {
    let expected: Value = serde_json::from_str(include_str!("fm6-tables.json")).unwrap();
    let t = Tables::new();
    for (i, v) in t.sin.iter().enumerate() {
        assert_eq!(v.to_bits() as u64, expected["FM6_SIN"][i].as_u64().unwrap());
    }
    for (name, values) in [
        ("FM6_MKI_LOG", &t.ml[..]),
        ("FM6_MKI_EXP", &t.me[..]),
        ("FM6_OPL_LOG", &t.ol[..]),
        ("FM6_OPL_EXP", &t.oe[..]),
    ] {
        for (i, v) in values.iter().enumerate() {
            assert_eq!(*v as u64, expected[name][i].as_u64().unwrap());
        }
    }
}
