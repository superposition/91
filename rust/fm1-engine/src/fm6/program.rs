// SPDX-License-Identifier: GPL-3.0-only
//! Factory/DY/user program selection and bounded Songbook profiles.
use super::{scaling, tables::*, *};
use fm1_formats::voice;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Selection {
    pub requested: i32,
    pub current: i32,
    pub dy_current: u32,
    pub dy_requested: u32,
    pub user_count: u32,
}
pub fn select(s: &mut Selection) -> bool {
    let mut index = if (0..46).contains(&s.requested) {
        s.requested
    } else {
        0
    };
    if index >= 30 && (index - 30) as u32 >= s.user_count {
        index = 0;
    }
    if s.dy_current != s.dy_requested {
        s.dy_current = s.dy_requested;
        s.current = -1;
    }
    let changed = index != s.current;
    s.current = index;
    changed
}
pub fn load(index: usize, dy: bool, user: Option<&[u8; 128]>, output: &mut [u8; 155]) {
    assert!(index < 46);
    if index >= 30 {
        *output = voice::unpack(user.expect("selected user program"));
    } else if dy {
        *output = voice::unpack(&FM6_DY[index]);
    } else {
        let source = FM_MAP[index] as usize;
        if source < 8 {
            *output = voice::unpack(&FM6_FACTORY[source]);
        } else {
            output.copy_from_slice(&FM6_ROM[source - 8]);
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Profile {
    pub part: u32,
    pub touch: u32,
    pub release: i32,
    pub brightness: i32,
    pub attack: u32,
}
pub fn songbook(packed: &[u8; 128], output: &mut [u8; 155], c: &Profile) {
    *output = voice::unpack(packed);
    output[LPMD] = 0;
    for op in 0..6 {
        if c.part == 0 {
            output[op * OP + KVS] = (c.touch * 7 / 100) as u8;
            output[op * OP + R1 + 3] =
                (output[op * OP + R1 + 3] as i32 + (65 - c.release) / 2).clamp(0, 99) as u8;
            if !scaling::carrier(output[ALG] as u32, 6 - op as u32) {
                output[op * OP + OL] =
                    (output[op * OP + OL] as i32 + (c.brightness - 50) / 4).clamp(0, 99) as u8;
            }
        } else if c.attack != 0 {
            output[op * OP + R1] = (99 - c.attack * 60 / 100) as u8;
        }
    }
}
pub fn gain(index: usize, dy: bool) -> f32 {
    if dy && index < 30 {
        FM_GAIN_DY[index]
    } else {
        FM_GAIN[index]
    }
}
pub fn name(index: usize, dy: bool) -> &'static [u8; 16] {
    if dy {
        &FM6_DY_NAME[index]
    } else {
        &FM_DX_NAME[index]
    }
}
pub fn user_name(packed: &[u8; 128], name: &mut [u8; 16]) {
    name[0] = b'D';
    name[1] = b'X';
    name[2] = b' ';
    let mut n = 10;
    while n > 0 && packed[118 + n - 1] & 0x7f <= 32 {
        n -= 1;
    }
    for k in 0..n {
        let c = packed[118 + k] & 0x7f;
        name[3 + k] = if (32..=126).contains(&c) { c } else { b' ' };
    }
    if n == 0 {
        name[3] = b'?';
        n = 1;
    }
    name[3 + n] = 0;
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct UserState {
    pub pending: u32,
    pub count: u32,
    pub current: i32,
}
pub fn stage(s: &mut UserState, slot: u32) -> bool {
    if slot >= 16 || slot > s.count || s.pending >> slot & 1 != 0 {
        return false;
    }
    if slot == s.count {
        s.count += 1;
    }
    s.pending |= 1 << slot;
    true
}
pub fn commit(s: &mut UserState, slot: u32) -> bool {
    if slot >= 16 || s.pending >> slot & 1 == 0 {
        return false;
    }
    s.pending &= !(1 << slot);
    if slot >= s.count {
        s.count = slot + 1;
    }
    if s.current == 30 + slot as i32 {
        s.current = -1;
    }
    true
}
pub fn wipe(s: &mut UserState) {
    s.count = 0;
    if s.current >= 30 {
        s.current = -1;
    }
}
pub fn copy_packed(source: &[u8; 128], destination: &mut [u8; 128]) {
    for (out, x) in destination.iter_mut().zip(source) {
        *out = *x & 0x7f;
    }
}
const _: [(); 20] = [(); core::mem::size_of::<Selection>()];
const _: [(); 20] = [(); core::mem::size_of::<Profile>()];
const _: [(); 12] = [(); core::mem::size_of::<UserState>()];
