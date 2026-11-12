// SPDX-License-Identifier: GPL-3.0-only
//! One immutable job, three released output stages, and explicit retirement.
//! This owns descriptor publication. External DSP buffers remain subject to the
//! bridge's span/stop contracts; a timeout alone never releases those buffers.
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, Ordering};

const IDLE: u32 = 0;
const READY: u32 = 1;
const RUNNING: u32 = 2;
const COMPLETED: u32 = 3;
const RETIRED: u32 = 4;
const PREPARING: u32 = 5;
pub const MAX_GENERATION: u32 = u32::MAX >> 3;
const fn tag(generation: u32, phase: u32) -> u32 {
    generation << 3 | phase
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Generation(u32);
impl Generation {
    pub fn from_raw(value: u32) -> Option<Self> {
        if value != 0 && value <= MAX_GENERATION {
            Some(Self(value))
        } else {
            None
        }
    }
    pub fn raw(self) -> u32 {
        self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Ready,
    Running,
    Completed,
    Retired,
    Preparing,
    Quarantined,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrepareError {
    Busy,
    Retired,
    Exhausted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Bass = 1,
    Tank = 2,
    Pad = 4,
}
#[repr(C, align(32))]
struct OwnerLine {
    tag: AtomicU32,
    quarantined: AtomicU32,
}
#[repr(C, align(32))]
struct WorkerLine {
    stages: AtomicU32,
}

/// Fixed-size, cache-line-separated mailbox. A generation is never reused:
/// exhaustion permanently retires the asynchronous channel instead of ABA.
#[repr(C)]
pub struct JobMailbox<T: Copy + Send> {
    owner: OwnerLine,
    worker: WorkerLine,
    payload: UnsafeCell<T>,
}
// SAFETY: the atomic tag grants exactly one writer while Preparing and one
// worker while Running. Payload is copied only after acquiring Ready. No
// payload reference escapes; a live lease prevents reuse until completion.
unsafe impl<T: Copy + Send> Sync for JobMailbox<T> {}
impl<T: Copy + Send> JobMailbox<T> {
    pub const fn new(initial: T) -> Self {
        Self {
            owner: OwnerLine {
                tag: AtomicU32::new(0),
                quarantined: AtomicU32::new(0),
            },
            worker: WorkerLine {
                stages: AtomicU32::new(0),
            },
            payload: UnsafeCell::new(initial),
        }
    }
    pub fn phase(&self) -> Phase {
        let current = self.owner.tag.load(Ordering::Acquire);
        if matches!(current & 7, READY | RUNNING | COMPLETED)
            && self.owner.quarantined.load(Ordering::Acquire) == current >> 3
        {
            return Phase::Quarantined;
        }
        match current & 7 {
            IDLE => Phase::Idle,
            READY => Phase::Ready,
            RUNNING => Phase::Running,
            COMPLETED => Phase::Completed,
            RETIRED => Phase::Retired,
            _ => Phase::Preparing,
        }
    }
    pub fn prepare(&self, job: T) -> Result<Generation, PrepareError> {
        let current = self.owner.tag.load(Ordering::Acquire);
        if current & 7 == RETIRED {
            return Err(PrepareError::Retired);
        }
        if current & 7 != IDLE {
            return Err(PrepareError::Busy);
        }
        self.owner
            .tag
            .compare_exchange(
                current,
                tag(current >> 3, PREPARING),
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .map_err(|_| PrepareError::Busy)?;
        let generation = (current >> 3) + 1;
        if generation > MAX_GENERATION {
            self.owner
                .tag
                .store(tag(MAX_GENERATION, RETIRED), Ordering::Release);
            return Err(PrepareError::Exhausted);
        }
        // SAFETY: Preparing excludes every worker, reader and other publisher.
        unsafe { self.payload.get().write(job) };
        self.worker
            .stages
            .store(tag(generation, 0), Ordering::Relaxed);
        self.owner.quarantined.store(0, Ordering::Relaxed);
        self.owner
            .tag
            .store(tag(generation, READY), Ordering::Release);
        Ok(Generation(generation))
    }
    pub fn claim(&self, generation: Generation) -> Option<WorkerLease<'_, T>> {
        self.owner
            .tag
            .compare_exchange(
                tag(generation.0, READY),
                tag(generation.0, RUNNING),
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .ok()?;
        // SAFETY: acquiring Ready observes this job's complete publication.
        // Running excludes overwrite until this lease finishes or is truly
        // stopped under retire_after_stop's external safety contract.
        let job = unsafe { self.payload.get().read() };
        Some(WorkerLease {
            mailbox: self,
            generation,
            job,
        })
    }
    pub fn stage_ready(&self, generation: Generation, stage: Stage) -> bool {
        let released = self.worker.stages.load(Ordering::Acquire);
        let current = self.owner.tag.load(Ordering::Acquire);
        released >> 3 == generation.0
            && released & (stage as u32) != 0
            && current >> 3 == generation.0
            && matches!(current & 7, RUNNING | COMPLETED)
            && self.owner.quarantined.load(Ordering::Acquire) != generation.0
    }
    pub fn completed(&self, generation: Generation) -> bool {
        self.owner.tag.load(Ordering::Acquire) == tag(generation.0, COMPLETED)
    }
    pub fn reclaim_completed(&self, generation: Generation) -> bool {
        if self
            .owner
            .tag
            .compare_exchange(
                tag(generation.0, COMPLETED),
                tag(generation.0, IDLE),
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .is_err()
        {
            return false;
        }
        let _ = self.owner.quarantined.compare_exchange(
            generation.0,
            0,
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
        true
    }
    /// Cancel only a job which no worker has claimed. A rejected platform
    /// submit may instead execute it inline using the same one-shot claim.
    pub fn cancel_unclaimed(&self, generation: Generation) -> bool {
        self.owner
            .tag
            .compare_exchange(
                tag(generation.0, READY),
                tag(generation.0, IDLE),
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .is_ok()
    }
    pub fn quarantine(&self, generation: Generation) -> bool {
        let current = self.owner.tag.load(Ordering::Acquire);
        if current >> 3 != generation.0 || !matches!(current & 7, READY | RUNNING | COMPLETED) {
            return false;
        }
        self.owner
            .quarantined
            .store(generation.0, Ordering::Release);
        true
    }
    /// # Safety
    /// The platform has joined this exact job OR permanently stopped its
    /// execution, including all nested callbacks and future stale invocations.
    /// No worker reference/access may survive. An elapsed timeout is not proof.
    /// The asynchronous channel remains retired; no later prepare can reuse it.
    pub unsafe fn retire_after_stop(&self, generation: Generation) -> bool {
        let current = self.owner.tag.load(Ordering::Acquire);
        if current >> 3 != generation.0 || !matches!(current & 7, READY | RUNNING | COMPLETED) {
            return false;
        }
        self.owner
            .tag
            .compare_exchange(
                current,
                tag(generation.0, RETIRED),
                Ordering::AcqRel,
                Ordering::Relaxed,
            )
            .is_ok()
    }
    /// Publication for the foreign worker which is inside a live WorkerLease.
    /// # Safety
    /// Only that worker calls this, after dropping every mutable reference to
    /// this stage's output. It must not touch that output again before join.
    pub unsafe fn publish_external(&self, generation: Generation, stage: Stage) -> bool {
        if self.owner.tag.load(Ordering::Acquire) != tag(generation.0, RUNNING) {
            return false;
        }
        let released = self.worker.stages.load(Ordering::Relaxed);
        if released >> 3 != generation.0 {
            return false;
        }
        self.worker
            .stages
            .store(released | stage as u32, Ordering::Release);
        true
    }
}
pub struct WorkerLease<'a, T: Copy + Send> {
    mailbox: &'a JobMailbox<T>,
    generation: Generation,
    job: T,
}
impl<T: Copy + Send> WorkerLease<'_, T> {
    pub fn job(&self) -> T {
        self.job
    }
    pub fn publish(&mut self, stage: Stage) -> bool {
        // SAFETY: this non-clonable lease uniquely holds the worker role. It
        // publishes protocol state only; external buffers have a separate ABI
        // contract, and no foreign buffer reference is manufactured here.
        unsafe { self.mailbox.publish_external(self.generation, stage) }
    }
    pub fn complete(self) -> bool {
        self.mailbox
            .owner
            .tag
            .compare_exchange(
                tag(self.generation.0, RUNNING),
                tag(self.generation.0, COMPLETED),
                Ordering::Release,
                Ordering::Relaxed,
            )
            .is_ok()
    }
}
// Dropping an unfinished lease deliberately leaves Running: panic/loss never
// grants the main owner permission to overwrite worker memory.
impl<T: Copy + Send> Drop for WorkerLease<'_, T> {
    fn drop(&mut self) {}
}

const _: [(); 96] = [(); core::mem::size_of::<JobMailbox<[u32; 6]>>()];
const _: [(); 32] = [(); core::mem::align_of::<JobMailbox<[u32; 6]>>()];

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};
    fn job(n: u32) -> [u32; 8] {
        core::array::from_fn(|i| n.wrapping_mul(0x9e3779b9).rotate_left(i as u32))
    }
    #[test]
    fn refusal_inline_claim_and_completion_do_not_replay() {
        let mailbox = JobMailbox::new([0; 8]);
        let first = mailbox.prepare(job(1)).unwrap();
        assert_eq!(mailbox.phase(), Phase::Ready);
        let mut lease = mailbox.claim(first).unwrap();
        assert_eq!(lease.job(), job(1));
        assert!(mailbox.claim(first).is_none());
        assert_eq!(mailbox.prepare(job(2)), Err(PrepareError::Busy));
        assert!(!mailbox.cancel_unclaimed(first));
        assert!(!mailbox.stage_ready(first, Stage::Bass));
        assert!(lease.publish(Stage::Bass));
        assert!(mailbox.stage_ready(first, Stage::Bass));
        assert!(!mailbox.stage_ready(first, Stage::Tank));
        assert!(!mailbox.reclaim_completed(first));
        assert!(lease.complete());
        assert_eq!(mailbox.phase(), Phase::Completed);
        assert!(mailbox.reclaim_completed(first));
        assert_eq!(mailbox.phase(), Phase::Idle);
        assert!(!mailbox.reclaim_completed(first));
    }
    #[test]
    fn stale_callbacks_stages_completion_and_cancellation_cannot_claim_next_job() {
        let mailbox = JobMailbox::new([0; 8]);
        let old = mailbox.prepare(job(1)).unwrap();
        assert!(mailbox.cancel_unclaimed(old));
        let current = mailbox.prepare(job(2)).unwrap();
        assert_ne!(old, current);
        assert!(mailbox.claim(old).is_none());
        assert!(!mailbox.cancel_unclaimed(old));
        assert!(!mailbox.quarantine(old));
        assert!(!mailbox.reclaim_completed(old));
        // SAFETY: this deliberate stale external publication has no output
        // borrow and cannot affect the unmatched current generation.
        assert!(!unsafe { mailbox.publish_external(old, Stage::Pad) });
        let mut lease = mailbox.claim(current).unwrap();
        assert_eq!(lease.job(), job(2));
        assert!(!mailbox.stage_ready(old, Stage::Pad));
        assert!(lease.publish(Stage::Pad));
        assert!(lease.complete());
        assert!(mailbox.reclaim_completed(current));
    }
    #[test]
    fn timeout_never_grants_reuse_and_confirmed_stop_is_permanent() {
        let mailbox = JobMailbox::new([0; 8]);
        let generation = mailbox.prepare(job(1)).unwrap();
        let lease = mailbox.claim(generation).unwrap();
        assert!(mailbox.quarantine(generation));
        assert_eq!(mailbox.phase(), Phase::Quarantined);
        assert!(!mailbox.stage_ready(generation, Stage::Bass));
        assert_eq!(mailbox.prepare(job(2)), Err(PrepareError::Busy));
        assert!(!mailbox.reclaim_completed(generation));
        drop(lease);
        assert_eq!(mailbox.phase(), Phase::Quarantined);
        // SAFETY: the simulated stopped worker has dropped its sole lease;
        // no thread, callback or buffer reference can access this job again.
        assert!(unsafe { mailbox.retire_after_stop(generation) });
        assert_eq!(mailbox.phase(), Phase::Retired);
        assert_eq!(mailbox.prepare(job(2)), Err(PrepareError::Retired));
        assert!(mailbox.claim(generation).is_none());
        assert!(!mailbox.reclaim_completed(generation));
    }
    #[test]
    fn quarantined_worker_can_finish_but_only_confirmed_reclaim_releases_it() {
        let mailbox = JobMailbox::new([0; 8]);
        let generation = mailbox.prepare(job(1)).unwrap();
        let mut lease = mailbox.claim(generation).unwrap();
        assert!(mailbox.quarantine(generation));
        assert!(lease.publish(Stage::Tank));
        assert!(!mailbox.stage_ready(generation, Stage::Tank));
        assert!(lease.complete());
        assert!(mailbox.completed(generation));
        assert_eq!(mailbox.phase(), Phase::Quarantined);
        assert!(mailbox.reclaim_completed(generation));
        assert_eq!(mailbox.phase(), Phase::Idle);
        assert!(mailbox.prepare(job(2)).is_ok());
    }
    #[test]
    fn generation_exhaustion_retires_instead_of_aba() {
        let mailbox = JobMailbox::new([0; 8]);
        mailbox
            .owner
            .tag
            .store(tag(MAX_GENERATION - 1, IDLE), Ordering::Relaxed);
        let last = mailbox.prepare(job(1)).unwrap();
        assert_eq!(last.raw(), MAX_GENERATION);
        assert!(mailbox.claim(last).unwrap().complete());
        assert!(mailbox.reclaim_completed(last));
        assert_eq!(mailbox.prepare(job(2)), Err(PrepareError::Exhausted));
        assert_eq!(mailbox.phase(), Phase::Retired);
        assert!(Generation::from_raw(0).is_none());
        assert!(Generation::from_raw(MAX_GENERATION + 1).is_none());
    }
    struct Outputs(UnsafeCell<[[u32; 32]; 3]>);
    // SAFETY: the worker writes only its current stage. Main reads a stage
    // only after acquiring its release, and does not permit a new job until
    // all reads finish and completion is acquired. No whole-buffer ref exists.
    unsafe impl Sync for Outputs {}
    #[test]
    fn concurrent_payload_and_three_stage_outputs_are_coherent() {
        let mailbox = Arc::new(JobMailbox::new([0; 8]));
        let requested = Arc::new(AtomicU32::new(0));
        let outputs = Arc::new(Outputs(UnsafeCell::new([[0; 32]; 3])));
        let mb = mailbox.clone();
        let req = requested.clone();
        let out = outputs.clone();
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(20);
            for n in 1..=100000u32 {
                while req.load(Ordering::Relaxed) != n {
                    assert!(Instant::now() < deadline);
                    thread::yield_now();
                }
                let generation = Generation::from_raw(n).unwrap();
                let mut lease = mb.claim(generation).unwrap();
                assert_eq!(lease.job(), job(n));
                for (stage, index) in [(Stage::Bass, 0), (Stage::Tank, 1), (Stage::Pad, 2)] {
                    {
                        let target = unsafe { &mut *out.0.get().cast::<[u32; 32]>().add(index) };
                        for (i, v) in target.iter_mut().enumerate() {
                            *v = n.wrapping_mul(0x85ebca6b).rotate_left(i as u32) ^ index as u32;
                        }
                    }
                    assert!(lease.publish(stage));
                }
                assert!(lease.complete());
            }
        });
        let deadline = Instant::now() + Duration::from_secs(20);
        for n in 1..=100000u32 {
            let generation = mailbox.prepare(job(n)).unwrap();
            assert_eq!(generation.raw(), n);
            requested.store(n, Ordering::Relaxed);
            for (stage, index) in [(Stage::Bass, 0), (Stage::Tank, 1), (Stage::Pad, 2)] {
                while !mailbox.stage_ready(generation, stage) {
                    assert!(Instant::now() < deadline);
                    thread::yield_now();
                }
                let observed = unsafe { outputs.0.get().cast::<[u32; 32]>().add(index).read() };
                for (i, v) in observed.iter().enumerate() {
                    assert_eq!(
                        *v,
                        n.wrapping_mul(0x85ebca6b).rotate_left(i as u32) ^ index as u32
                    );
                }
            }
            while !mailbox.completed(generation) {
                assert!(Instant::now() < deadline);
                thread::yield_now();
            }
            assert!(mailbox.reclaim_completed(generation));
        }
        worker.join().unwrap();
    }
}
