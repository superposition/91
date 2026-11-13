// SPDX-License-Identifier: GPL-3.0-only
use fm1_engine::fm6::{
    held,
    pool::{self, NoteContext, Pool},
    types::*,
};

fn attack(
    slots: &mut [Slot; 6],
    owner: u8,
    pitch: i32,
    part: u32,
    age: &mut u32,
) -> pool::Allocation {
    let mut keys = Lfo::default();
    let mut bass = Lfo::default();
    let mut patch = [0; 155];
    patch[144] = 24;
    let result = pool::note_on(
        Pool {
            slots,
            keys_lfo: &mut keys,
            bass_lfo: &mut bass,
        },
        &patch,
        &NoteContext {
            jam: 1,
            part,
            current: 0,
            shed: 0,
        },
        age,
        pitch,
        0.8,
        128,
    )
    .unwrap();
    if owner != 0 {
        assert!(held::mark(slots, result.slot, owner));
    }
    result
}

#[test]
fn held_identity_uses_existing_padding_and_does_not_expire() {
    assert_eq!(core::mem::size_of::<Slot>(), 408);
    assert_eq!(core::mem::offset_of!(Slot, owner), 391);
    assert_eq!(core::mem::offset_of!(Slot, age), 392);
    let mut slots = [Slot::default(); 6];
    let mut age = 0;
    let keys = attack(&mut slots, 1, 72, 0, &mut age);
    let bass = attack(&mut slots, 0, 36, 1, &mut age);
    assert_eq!(bass.slot, 4);
    assert_ne!(keys.slot, 4);
    for _ in 0..1000 {
        pool::gates(&mut slots, 256, 44100);
    }
    assert_eq!(slots[keys.slot].gate, 1);
    assert_eq!(slots[keys.slot].gate_left, 128);
    assert_eq!(slots[bass.slot].gate, 0);
    assert_eq!(held::release(&mut slots, 1), Some(72));
    assert_eq!(slots[keys.slot].gate, 0);
    assert_eq!(slots[keys.slot].owner, 0);
    assert_eq!(held::release(&mut slots, 1), None);
}

#[test]
fn five_keys_never_claim_bass_and_stolen_owner_cannot_release_replacement() {
    let mut slots = [Slot::default(); 6];
    let mut age = 0;
    attack(&mut slots, 0, 36, 1, &mut age);
    for owner in 1..=5 {
        assert_eq!(
            attack(&mut slots, owner, 60 + owner as i32, 0, &mut age).previous,
            None
        );
    }
    let next = attack(&mut slots, 6, 78, 0, &mut age);
    assert_eq!(
        next.previous,
        Some(held::Note {
            owner: 1,
            pitch: 61
        })
    );
    assert_eq!(slots[4].s.note, 36);
    assert_eq!(slots[4].part, 1);
    assert_eq!(held::release(&mut slots, 1), None);
    assert_eq!(slots[next.slot].owner, 6);
    assert_eq!(slots[next.slot].gate, 1);
    assert_eq!(held::release(&mut slots, 6), Some(78));
}

#[test]
fn same_pitch_sources_and_naturally_silent_keys_retain_correct_midi_ownership() {
    let mut slots = [Slot::default(); 6];
    let mut age = 0;
    let panel = attack(&mut slots, 3, 72, 0, &mut age);
    attack(&mut slots, 188, 72, 0, &mut age);
    assert_eq!(held::count(&slots, 72), 2);
    slots[panel.slot].used = 0; // Percussive DX envelopes can end while key is down.
    assert_eq!(held::count(&slots, 72), 2);
    assert_eq!(held::release(&mut slots, 3), Some(72));
    assert_eq!(held::count(&slots, 72), 1); // No outgoing MIDI off yet.
    assert_eq!(held::release(&mut slots, 188), Some(72));
    assert_eq!(held::count(&slots, 72), 0); // Last owner sends outgoing MIDI off.
    let old = attack(&mut slots, 4, 65, 0, &mut age);
    slots[old.slot].used = 0;
    assert_eq!(
        attack(&mut slots, 5, 77, 0, &mut age).previous,
        Some(held::Note {
            owner: 4,
            pitch: 65
        })
    );
}

#[test]
fn explicit_reset_and_retirement_clear_all_owners_after_host_note_offs() {
    let mut slots = [Slot::default(); 6];
    let mut age = 0;
    for owner in 1..=5 {
        attack(&mut slots, owner, 60, 0, &mut age);
    }
    assert!(!held::mark(&mut slots, 4, 27));
    assert!(!held::mark(&mut slots, 6, 1));
    assert!(!held::mark(&mut slots, 0, 0));
    pool::retire_part(&mut slots, 0);
    assert!(slots.iter().all(|s| s.owner == 0));
    attack(&mut slots, 1, 72, 0, &mut age);
    pool::retire_all(&mut slots);
    assert!(slots.iter().all(|s| s.owner == 0));
    attack(&mut slots, 1, 72, 0, &mut age);
    pool::reset(&mut slots, &mut Lfo::default());
    assert!(slots.iter().all(|s| s.owner == 0));
}
