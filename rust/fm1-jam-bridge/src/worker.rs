// SPDX-License-Identifier: GPL-3.0-only
//! Generation-checked job dispatch and stage publication for existing C DSP.
use fm1_rt::atomic::{AtomicU32, Ordering};
use fm1_rt::job::{Generation, JobMailbox, Phase, Stage};
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Job {
    n: i32,
    r0: i32,
    fm_on: i32,
    fm_w: u32,
    pad_w: u32,
    bass: i32,
    genre: u32,
    float_path: i32,
}
static MAILBOX: JobMailbox<Job> = JobMailbox::new(Job {
    n: 0,
    r0: 0,
    fm_on: 0,
    fm_w: 0,
    pad_w: 0,
    bass: 0,
    genre: 0,
    float_path: 0,
});
static COUNTERS: [AtomicU32; 8] = [const { AtomicU32::new(0) }; 8];
pub fn stat(index: u32) -> u32 {
    COUNTERS
        .get(index.wrapping_sub(56) as usize)
        .map(|x| x.load(Ordering::Relaxed))
        .unwrap_or(0)
}
fn stale() -> u32 {
    COUNTERS[6].fetch_add(1, Ordering::Relaxed);
    0
}
fn stage(value: u32) -> Option<Stage> {
    match value {
        1 => Some(Stage::Bass),
        2 => Some(Stage::Tank),
        4 => Some(Stage::Pad),
        _ => None,
    }
}
#[no_mangle]
pub extern "C" fn fm1_rust_worker_phase() -> u32 {
    match MAILBOX.phase() {
        Phase::Idle => 0,
        Phase::Ready => 1,
        Phase::Running => 2,
        Phase::Completed => 3,
        Phase::Retired => 4,
        Phase::Preparing => 5,
        Phase::Quarantined => 6,
    }
}
/// # Safety
/// One initialized 32-byte job descriptor is readable for this copy. Its fixed
/// spans have been exclusively prepared by render; no previous job is active.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_worker_prepare(job: *const Job) -> u32 {
    let job = unsafe { *job };
    if !(0..=256).contains(&job.n)
        || !(0..1579).contains(&job.r0)
        || job.fm_w >= 512
        || job.pad_w >= 512
        || !(0..=1).contains(&job.bass)
        || job.genre >= 19
    {
        return stale();
    }
    match MAILBOX.prepare(job) {
        Ok(g) => {
            COUNTERS[0].fetch_add(1, Ordering::Relaxed);
            g.raw()
        }
        Err(_) => 0,
    }
}
/// # Safety
/// Callback executes exactly this job over the existing exclusively held DSP
/// state/output spans. It retains no job pointer and releases each Rust kernel
/// borrow before publication. Main cannot alter inputs/state until released
/// stages or full completion are acquired; failed backend join is NOT a lease
/// release unless execution and all future callbacks are confirmed stopped.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_worker_execute(
    generation: u32,
    callback: unsafe extern "C" fn(*const Job),
) -> u32 {
    let Some(g) = Generation::from_raw(generation) else {
        return stale();
    };
    let Some(lease) = MAILBOX.claim(g) else {
        return stale();
    };
    COUNTERS[1].fetch_add(1, Ordering::Relaxed);
    let job = lease.job();
    unsafe { callback(&job) };
    if lease.complete() {
        COUNTERS[3].fetch_add(1, Ordering::Relaxed);
        1
    } else {
        stale()
    }
}
/// # Safety
/// This is called only by the claimed worker callback after every mutable
/// reference to this stage's outputs has ended. Those outputs are not touched
/// again by the worker before the main owner finishes this job.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_worker_publish(generation: u32, value: u32) -> u32 {
    let (Some(g), Some(stage)) = (Generation::from_raw(generation), stage(value)) else {
        return stale();
    };
    if unsafe { MAILBOX.publish_external(g, stage) } {
        COUNTERS[2].fetch_add(1, Ordering::Relaxed);
        1
    } else {
        stale()
    }
}
#[no_mangle]
pub extern "C" fn fm1_rust_worker_ready(generation: u32, value: u32) -> u32 {
    let (Some(g), Some(stage)) = (Generation::from_raw(generation), stage(value)) else {
        return 0;
    };
    u32::from(MAILBOX.stage_ready(g, stage))
}
#[no_mangle]
pub extern "C" fn fm1_rust_worker_completed(generation: u32) -> u32 {
    u32::from(Generation::from_raw(generation).is_some_and(|g| MAILBOX.completed(g)))
}
/// # Safety
/// Backend join has confirmed this exact callback has returned. Main will not
/// prepare a new job until it finishes reading all released output spans.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_worker_reclaim(generation: u32) -> u32 {
    if Generation::from_raw(generation).is_some_and(|g| MAILBOX.reclaim_completed(g)) {
        1
    } else {
        stale()
    }
}
/// # Safety
/// Backend has permanently stopped this exact job, all nested DSP callbacks,
/// and every future stale invocation. A timeout or negative/unknown join is
/// insufficient. Main may reset/reuse external DSP spans only after success.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_worker_retire(generation: u32) -> u32 {
    let Some(g) = Generation::from_raw(generation) else {
        return stale();
    };
    if unsafe { MAILBOX.retire_after_stop(g) } {
        COUNTERS[5].fetch_add(1, Ordering::Relaxed);
        1
    } else {
        stale()
    }
}
#[no_mangle]
pub extern "C" fn fm1_rust_worker_quarantine(generation: u32) -> u32 {
    if Generation::from_raw(generation).is_some_and(|g| MAILBOX.quarantine(g)) {
        COUNTERS[7].fetch_add(1, Ordering::Relaxed);
        1
    } else {
        stale()
    }
}
#[no_mangle]
pub extern "C" fn fm1_rust_worker_refused() {
    COUNTERS[4].fetch_add(1, Ordering::Relaxed);
}
const _: [(); 32] = [(); core::mem::size_of::<Job>()];
const _: [(); 96] = [(); core::mem::size_of::<JobMailbox<Job>>()];
