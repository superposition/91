// SPDX-License-Identifier: GPL-3.0-only
//! Synth Jam key ownership over the existing six FM slots; no separate held map.
//! Owner 0 is automatic, 1..27 are panel keys, 128..255 are incoming MIDI notes.
//! Hosts coalesce outgoing MIDI by pitch: first owner on, last owner off.
use super::types::Slot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Note {
    pub owner: u8,
    pub pitch: u8,
}

pub fn note(slot: &Slot) -> Option<Note> {
    // A percussive patch may become silent before key-up. Keep its MIDI
    // ownership until release or reuse, even when the DSP marks used=0.
    if slot.owner != 0 {
        Some(Note {
            owner: slot.owner,
            pitch: slot.s.note,
        })
    } else {
        None
    }
}

pub fn mark(slots: &mut [Slot; 6], index: usize, owner: u8) -> bool {
    if owner == 0 || index >= slots.len() || slots[index].used == 0 || slots[index].part != 0 {
        return false;
    }
    slots[index].owner = owner;
    true
}

/// Release only this physical/MIDI key, using its original sounding pitch.
/// A release of a stolen key is harmless because the slot has a new owner.
pub fn release(slots: &mut [Slot; 6], owner: u8) -> Option<u8> {
    if owner == 0 {
        return None;
    }
    for slot in slots {
        if slot.owner == owner {
            let pitch = slot.s.note;
            slot.owner = 0;
            slot.gate = 0;
            slot.kill_left = i32::MAX;
            return Some(pitch);
        }
    }
    None
}

pub fn count(slots: &[Slot; 6], pitch: u8) -> u32 {
    slots
        .iter()
        .filter(|slot| note(slot).is_some_and(|n| n.pitch == pitch))
        .count() as u32
}
