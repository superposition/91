// SPDX-License-Identifier: GPL-3.0-only
//! Save/load policy at the UI/engine boundary. The caller applies returned
//! commands after releasing state borrows; this module performs no device I/O.
use fm1_formats::{
    objects::SettingsParts,
    session::JamSession,
    storage::{RestoreLimits, Settings},
};
pub fn migrate_settings(raw: &[u8], defaults: &[u8], output: &mut [u8]) -> bool {
    if defaults.len() > 256
        || output.len() != ((24 + defaults.len()).next_multiple_of(2) + 4).next_multiple_of(4)
    {
        return false;
    }
    let limits = RestoreLimits {
        effects: defaults.len() as u16,
        settings_len: output.len(),
    };
    let Ok(s) = Settings::parse(raw, limits) else {
        return false;
    };
    if raw.len() == output.len() {
        output.copy_from_slice(raw);
        return true;
    }
    let mut effects = [0; 256];
    effects[..defaults.len()].copy_from_slice(defaults);
    effects[..s.effects().len()].copy_from_slice(s.effects());
    SettingsParts {
        effects: &effects[..defaults.len()],
        performance: s.performance(),
        targets: s.targets_and_visualizer(),
        pattern: s.pattern(),
        usb: s.usb_flags(),
    }
    .encode(output)
    .is_ok()
}
pub fn voice_count(raw: &[u8]) -> Option<usize> {
    if raw.len() != 2056 || &raw[..4] != b"VOX1" {
        return None;
    }
    let n = u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
    (n <= 16).then_some(n)
}
pub fn preserve_genre(
    effects: &mut [u8],
    stored: &[u8],
    positions: &[u8; 12],
    genre: u8,
    have: bool,
) -> bool {
    if genre == 0 || !have {
        return true;
    }
    if stored.len() != effects.len() || positions.iter().any(|n| *n as usize >= effects.len()) {
        return false;
    }
    for i in positions {
        effects[*i as usize] = stored[*i as usize];
    }
    true
}
pub fn piece_effects(saved: &[u8; 16], live: &[u8; 8], defaults: &[u8; 7]) -> [u8; 7] {
    if live[0] != 0 {
        live[1..8].try_into().unwrap()
    } else if saved[3] & 1 != 0 {
        saved[4..11].try_into().unwrap()
    } else {
        *defaults
    }
}
pub fn boot_session(raw: &[u8]) -> [u8; 20] {
    let mut b = [
        b'J', b'S', b'M', 1, 1, 0, 0, 16, 0, 3, 60, 0, 0, 0, 100, 0, 0, 0, 0, 0,
    ];
    if JamSession::decode(raw).is_ok() {
        b.copy_from_slice(raw);
    }
    b
}
pub fn session_save_allowed(playing: bool, busy: bool, locked: bool) -> bool {
    !(playing || busy || locked)
}
pub fn session_slot(request: i32) -> Option<u8> {
    (0..=4).contains(&request).then_some(request as u8)
}
pub fn resolve_session(bytes: &mut [u8; 20], loaded: bool) -> Option<u8> {
    if !loaded {
        bytes[13] = 0;
    }
    builtin_source(bytes)
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Performance {
    pub usb_fixed: u8,
    pub fm_dy: u8,
    pub scan: u8,
    pub rounds: u8,
    pub ramp: u8,
    pub intro: u8,
    pub midi_map: u8,
    pub channel: u8,
    pub pads: u8,
    pub cc: u8,
    pub visualizer: u8,
    pub centre: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PerformanceBytes {
    pub usb: u16,
    pub performance: u8,
    pub visualizer: u8,
}
impl Performance {
    pub fn encode(self) -> PerformanceBytes {
        let usb = u16::from(self.usb_fixed)
            | (u16::from(self.fm_dy == 0) << 1)
            | (u16::from(self.scan) << 2)
            | (u16::from(self.rounds & 7) << 3)
            | (u16::from(self.ramp & 7) << 6)
            | (u16::from(self.intro & 7) << 9)
            | (u16::from(self.midi_map & 7) << 12)
            | (u16::from(self.pads == 1) << 15);
        let performance = self.channel
            | ((if self.visualizer == 8 {
                1
            } else if self.visualizer == 7 {
                2
            } else {
                0
            }) << 5);
        let visualizer = (if self.visualizer >= 7 {
            7
        } else {
            self.visualizer
        }) | (u8::from(self.pads == 2) << 3)
            | ((self.cc & 3) << 4)
            | (u8::from(self.pads == 3) << 6)
            | (self.centre << 7);
        PerformanceBytes {
            usb,
            performance,
            visualizer,
        }
    }
    pub fn decode(bytes: PerformanceBytes) -> Self {
        let (usb, p, v) = (bytes.usb, bytes.performance, bytes.visualizer);
        Self {
            usb_fixed: (usb & 1) as u8,
            fm_dy: u8::from(usb & 2 == 0),
            scan: (usb >> 2 & 1) as u8,
            rounds: (usb >> 3 & 7) as u8,
            ramp: (usb >> 6 & 7) as u8,
            intro: (usb >> 9 & 7) as u8,
            midi_map: (usb >> 12 & 7) as u8,
            channel: if p & 31 <= 16 { p & 31 } else { 0 },
            pads: if v & 64 != 0 {
                3
            } else if v & 8 != 0 {
                2
            } else {
                (usb >> 15 & 1) as u8
            },
            cc: v >> 4 & 3,
            visualizer: if v & 7 == 7 {
                match p >> 5 & 3 {
                    1 => 8,
                    2 => 7,
                    _ => 9,
                }
            } else {
                v & 7
            },
            centre: v >> 7,
        }
    }
    pub fn knob(self, index: u32) -> Option<u8> {
        match index {
            12 => Some(self.usb_fixed),
            13 => Some(self.fm_dy),
            14 => Some(self.scan),
            15 => Some(self.rounds),
            16 => Some(self.ramp),
            17 => Some(self.intro),
            18 => Some(self.midi_map),
            19 => Some(self.channel),
            20 => Some(self.pads),
            22 => Some(self.cc),
            _ => None,
        }
    }
}
pub fn builtin_source(bytes: &[u8; 20]) -> Option<u8> {
    (bytes[13] == 0 && bytes[17] == b'S' && bytes[16] < 3).then_some(bytes[16])
}
pub fn session_values(bytes: &[u8; 20]) -> [i32; 10] {
    [
        i32::from(bytes[5]) + 12 * i32::from(bytes[6]),
        i32::from(bytes[7]),
        i32::from(bytes[8]),
        i32::from(bytes[9]),
        i32::from(bytes[10]),
        i32::from(bytes[11]),
        i32::from(bytes[12]),
        i32::from(bytes[13]),
        i32::from(u16::from_le_bytes([bytes[14], bytes[15]])),
        i32::from(bytes[4]),
    ]
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Autosave {
    pub now: u32,
    pub last_action: u32,
    pub place: u32,
    pub remembered: u32,
    pub playing: u32,
    pub seamless: u32,
    pub buttons: u32,
    pub keys: u32,
    pub waiting: u32,
    pub voices: u32,
    pub settings: u32,
    pub lazy: u32,
    pub pending: u32,
    pub globals: u32,
    pub global_quiet: u32,
    pub kit_quiet: u32,
}
/// low byte: 0 none, 1 voices, 2 settings, 3 all project, 4 globals;
/// bit8: remember current place, bit9: promote lazy settings to dirty.
pub fn autosave(input: Autosave, clock: &mut [u32; 2]) -> u32 {
    let remember = if input.place != clock[0] {
        clock[0] = input.place;
        clock[1] = input.now;
        false
    } else {
        input.place != input.remembered
            && input.now.wrapping_sub(clock[1]) >= 20_000
            && (input.playing == 0 || input.seamless != 0)
    };
    let mut result = u32::from(remember) << 8;
    let quiet = input.now.wrapping_sub(input.last_action);
    let global = if input.playing != 0 {
        input.global_quiet.wrapping_mul(2)
    } else {
        input.global_quiet
    };
    let blocked = input.buttons != 0 || input.keys != 0 || input.waiting != 0;
    if input.voices != 0 && !blocked && quiet >= global {
        return result | 1;
    }
    let promote = input.lazy != 0 && input.playing == 0;
    result |= u32::from(promote) << 9;
    if (input.settings != 0 || promote) && !blocked && quiet >= global {
        return result | 2;
    }
    if (input.pending == 0 && !remember) || blocked {
        return result;
    }
    if input.playing == 0 && quiet >= input.kit_quiet {
        return result | 3;
    }
    if (input.globals != 0 || remember) && quiet >= global {
        return result | 4;
    }
    result
}
