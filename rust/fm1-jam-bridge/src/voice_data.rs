// SPDX-License-Identifier: GPL-3.0-only
//! Read-only recipe symbols; C0 has its original private definitions.
use fm1_engine::voices::data::*;
#[no_mangle]
pub static FM1_VOICE_KITS: [Kit; 14] = KITS;
#[no_mangle]
pub static FM1_VOICE_PAD_SOUND: [PadSound; 24] = PAD_SOUND;
#[no_mangle]
pub static FM1_VOICE_DRONE_SOUND: [DroneSound; 24] = DRONE_SOUND;
#[no_mangle]
pub static FM1_VOICE_ACID_SOUND: [AcidSound; 31] = ACID_SOUND;
#[no_mangle]
pub static FM1_VOICE_ACID_FM: [AcidFm; 31] = ACID_FM;
#[no_mangle]
pub static FM1_VOICE_CM_SOUND: [PluckSound; 12] = CM_SOUND;
#[no_mangle]
pub static FM1_VOICE_DRONE_FR: [f32; 24] = DRONE_FR;
#[no_mangle]
pub static FM1_VOICE_PAD_PAN: [f32; 4] = PAD_PAN;
#[no_mangle]
pub static FM1_VOICE_BREATH: [f32; 4] = BREATH;
#[no_mangle]
pub static FM1_VOICE_START_T: [f32; 4] = START_T;
#[no_mangle]
pub static FM1_VOICE_VOWEL: [[f32; 2]; 4] = VOWEL;
#[no_mangle]
pub static FM1_VOICE_HAT_INC: [f32; 4] = HAT_INC;
#[no_mangle]
pub static FM1_VOICE_LFO_RATE_BEATS: [f32; 12] = LFO_RATE_BEATS;
#[no_mangle]
pub static FM1_VOICE_DRUM_GM: [u8; 6] = DRUM_GM;
#[no_mangle]
pub static FM1_VOICE_EN_TRIM: [[u8; 10]; 19] = EN_TRIM;

#[no_mangle]
pub static FM1_VOICE_SINT: [f32; 1025] = SINT;
