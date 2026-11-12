// SPDX-License-Identifier: GPL-3.0-only
//! Block configuration borrows existing scalar/parameter inputs only.
use crate::voice_data::*;
use fm1_engine::voices::{self, config};
const _: [(); 100] = [(); core::mem::size_of::<config::DroneInput>()];
const _: [(); 60] = [(); core::mem::size_of::<config::AcidInput>()];
const _: [(); 104] = [(); core::mem::size_of::<config::PadInput>()];
const _: [(); 32] = [(); core::mem::size_of::<config::PadBusInput>()];
const _: [(); 84] = [(); core::mem::size_of::<config::DrumsInput>()];
const _: [(); 28] = [(); core::mem::size_of::<config::PluckInput>()];
/// # Safety
/// params points to176 initialized/aligned i16 values. raw and output point
/// to initialized/aligned input and exclusively writable output respectively.
/// All three regions are disjoint and remain live for the call. No callback,
/// state mutation or pointer/reference retention occurs.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_drone_config(
    params: *const i16,
    raw: *const config::DroneInput,
    out: *mut voices::drone::Config,
) {
    unsafe {
        out.write(config::drone(
            &*params.cast::<config::Params>(),
            &*raw,
            &FM1_VOICE_DRONE_SOUND,
            &FM1_VOICE_DRONE_FR,
        ))
    };
    crate::voices::config_count();
}
/// # Safety
///176 initialized/aligned parameters, initialized/aligned raw and aligned
/// exclusively writable output are disjoint/live. No borrow/callback escapes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_acid_config(
    params: *const i16,
    raw: *const config::AcidInput,
    out: *mut voices::acid::Config,
) {
    unsafe {
        out.write(config::acid(
            &*params.cast::<config::Params>(),
            &*raw,
            &FM1_VOICE_ACID_SOUND,
            &FM1_VOICE_ACID_FM,
        ))
    };
    crate::voices::config_count();
}
/// # Safety
///176 initialized/aligned parameters and aligned exclusively writable output
/// are disjoint/live for this call. No borrow/callback escapes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_breath_config(
    params: *const i16,
    playing: u32,
    out: *mut voices::pad::Breath,
) {
    unsafe {
        out.write(config::breath(
            &*params.cast::<config::Params>(),
            playing,
            &FM1_VOICE_PAD_SOUND,
        ))
    };
}
/// # Safety
///176 initialized/aligned parameters, initialized/aligned raw and aligned
/// exclusively writable output are disjoint/live. raw.genre is0..18. No
/// reference/pointer/callback escapes and no voice state is mutated here.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_pad_config(
    params: *const i16,
    raw: *const config::PadInput,
    out: *mut voices::pad::Config,
) {
    unsafe {
        out.write(config::pad(
            &*params.cast::<config::Params>(),
            &*raw,
            &FM1_VOICE_PAD_SOUND,
            &crate::melodic::FM1_RUST_SECTIONS,
            &FM1_VOICE_BREATH,
            &FM1_VOICE_VOWEL,
        ))
    };
    crate::voices::config_count();
}
/// # Safety
///176 initialized/aligned parameters, initialized/aligned raw and aligned
/// exclusively writable output are disjoint/live. No borrow/callback escapes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_pad_bus_config(
    params: *const i16,
    raw: *const config::PadBusInput,
    out: *mut voices::pad::BusConfig,
) {
    unsafe {
        out.write(config::pad_bus(
            &*params.cast::<config::Params>(),
            &*raw,
            &FM1_VOICE_PAD_SOUND,
        ))
    };
}
/// # Safety
/// Initialized/aligned raw input, four-pointer table, each immutable kit row
/// and aligned exclusively writable output are live/disjoint. No borrow or
/// callback escapes; immutable kit references may point to the same row.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_drums_config(
    raw: *const config::DrumsInput,
    kits: *const *const voices::data::Kit,
    out: *mut voices::DrumsConfig,
) {
    let kits = unsafe { &*kits.cast::<[*const voices::data::Kit; 4]>() };
    unsafe { out.write(config::drums(&*raw, kits.map(|p| &*p))) };
    crate::voices::config_count();
}
/// # Safety
///176 initialized/aligned parameters, initialized/aligned raw and aligned
/// exclusively writable output are disjoint/live. No borrow/callback escapes.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_pluck_config(
    params: *const i16,
    raw: *const config::PluckInput,
    out: *mut voices::PluckConfig,
) {
    unsafe {
        out.write(config::pluck(
            &*params.cast::<config::Params>(),
            &*raw,
            &FM1_VOICE_CM_SOUND,
        ))
    };
    crate::voices::config_count();
}
