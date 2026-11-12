// SPDX-License-Identifier: GPL-3.0-only
//! Scoped voice-owner borrows; no pointer/callback/state copy survives a block.
use core::sync::atomic::{AtomicU32, Ordering};
use fm1_engine::voices::{self, Drum, DrumsConfig, Mix, Pluck, PluckConfig};
static COUNTERS: [AtomicU32; 8] = [const { AtomicU32::new(0) }; 8];
pub fn stat(index: u32) -> u32 {
    COUNTERS
        .get(index.wrapping_sub(40) as usize)
        .map(|v| v.load(Ordering::Relaxed))
        .unwrap_or(0)
}
const _: [(); 104] = [(); core::mem::size_of::<Drum>()];
const _: [(); 108] = [(); core::mem::size_of::<DrumsConfig>()];
const _: [(); 28] = [(); core::mem::size_of::<Pluck>()];
const _: [(); 44] = [(); core::mem::size_of::<PluckConfig>()];
/// # Safety
/// State/config/table/seed and four initialized n-f32 mix arrays are aligned and
/// pairwise disjoint. The audio owner exclusively holds state/seed/mix while
/// table/config stay read-only. Six states and1025 sine values are provided.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_drums(
    s: *mut Drum,
    c: *const DrumsConfig,
    t: *const f32,
    seed: *mut u32,
    p: *const *mut f32,
    n: u32,
) {
    let p = unsafe { &*p.cast::<[*mut f32; 4]>() };
    let n = n as usize;
    voices::drums(
        unsafe { &mut *s.cast::<[Drum; 6]>() },
        unsafe { &*c },
        unsafe { &*t.cast::<[f32; 1025]>() },
        unsafe { &mut *seed },
        Mix {
            left: unsafe { core::slice::from_raw_parts_mut(p[0], n) },
            right: unsafe { core::slice::from_raw_parts_mut(p[1], n) },
            reverb: unsafe { core::slice::from_raw_parts_mut(p[2], n) },
            echo: unsafe { core::slice::from_raw_parts_mut(p[3], n) },
        },
    );
    COUNTERS[0].fetch_add(1, Ordering::Relaxed);
}
/// # Safety
/// Two initialized voice states and1025 sine values are provided. The module's
/// disjoint block-borrow contract applies, without the shared drum seed.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_voice_pluck(
    s: *mut Pluck,
    c: *const PluckConfig,
    t: *const f32,
    p: *const *mut f32,
    n: u32,
) -> f32 {
    let p = unsafe { &*p.cast::<[*mut f32; 4]>() };
    let n = n as usize;
    let level = voices::pluck(
        unsafe { &mut *s.cast::<[Pluck; 2]>() },
        unsafe { &*c },
        unsafe { &*t.cast::<[f32; 1025]>() },
        Mix {
            left: unsafe { core::slice::from_raw_parts_mut(p[0], n) },
            right: unsafe { core::slice::from_raw_parts_mut(p[1], n) },
            reverb: unsafe { core::slice::from_raw_parts_mut(p[2], n) },
            echo: unsafe { core::slice::from_raw_parts_mut(p[3], n) },
        },
    );
    COUNTERS[1].fetch_add(1, Ordering::Relaxed);
    level
}
