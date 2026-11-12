// SPDX-License-Identifier: GPL-3.0-only
//! Bounded handoffs. Musical state belongs to its render/application owner;
//! this module owns only the publication protocol for compact command words.
#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

use crate::atomic::{AtomicU32, Ordering};
use core::cell::UnsafeCell;
use fm1_types::{CommandWord, COMMAND_CAPACITY};

pub mod atomic;
pub mod handoff;
pub mod job;

/// Single-producer/single-consumer ring, with the legacy 4 KiB payload.
/// Obtain unique endpoints with [`Self::split`]. No allocation or blocking.
pub struct CommandQueue {
    slots: UnsafeCell<[CommandWord; COMMAND_CAPACITY]>,
    written: AtomicU32,
    read: AtomicU32,
}

// SAFETY: safe clients have one non-clonable producer and consumer. Publication
// releases initialized slots; consumption acquires them. The consumer releases
// each slot before producer reuse. Direct ABI users must uphold the same roles.
unsafe impl Sync for CommandQueue {}

const _: [(); 4104] = [(); core::mem::size_of::<CommandQueue>()];
const _: () = assert!(COMMAND_CAPACITY.is_power_of_two() && COMMAND_CAPACITY < (1 << 31));

impl Default for CommandQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandQueue {
    pub const fn new() -> Self {
        Self {
            slots: UnsafeCell::new([CommandWord::from_bits(0); COMMAND_CAPACITY]),
            written: AtomicU32::new(0),
            read: AtomicU32::new(0),
        }
    }

    pub fn split(&mut self) -> (Producer<'_>, Consumer<'_>) {
        (Producer { queue: self }, Consumer { queue: self })
    }

    pub fn reset(&mut self) {
        // The unique borrow excludes every live safe endpoint.
        unsafe { self.reset_exclusive() };
    }

    /// Reset publication indices, retaining allocated storage.
    ///
    /// # Safety
    /// Exclude all producer/consumer access for this call. Intended for boot
    /// before interrupts/workers start, not a live queue flush.
    pub unsafe fn reset_exclusive(&self) {
        self.written.store(0, Ordering::Relaxed);
        self.read.store(0, Ordering::Relaxed);
    }

    /// Publish from a foreign producer without retaining a Rust endpoint.
    /// Returns the accepted depth, or the unchanged word when full.
    ///
    /// # Safety
    /// There must be only one producer call at a time, no reset, and at most
    /// one concurrent consumer. The safe split API enforces those roles.
    pub unsafe fn push_from_producer(&self, word: CommandWord) -> Result<u32, CommandWord> {
        let written = self.written.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Acquire);
        let occupied = written.wrapping_sub(read);
        if occupied >= COMMAND_CAPACITY as u32 {
            return Err(word);
        }
        let slot = written as usize & (COMMAND_CAPACITY - 1);
        // SAFETY: this slot is producer-owned until publication. Cast the raw
        // array pointer rather than forming an exclusive reference to the
        // entire array, whose other slots may be read by the consumer.
        unsafe { self.slots.get().cast::<CommandWord>().add(slot).write(word) };
        self.written
            .store(written.wrapping_add(1), Ordering::Release);
        Ok(occupied + 1)
    }

    /// Consume from a foreign render owner without retaining an endpoint.
    ///
    /// # Safety
    /// There must be only one consumer call at a time, no reset, and at most
    /// one concurrent producer. The slot must not be read after it is released.
    pub unsafe fn pop_from_consumer(&self) -> Option<CommandWord> {
        let read = self.read.load(Ordering::Relaxed);
        let written = self.written.load(Ordering::Acquire);
        if read == written {
            return None;
        }
        let slot = read as usize & (COMMAND_CAPACITY - 1);
        // SAFETY: acquire observed publication of this initialized slot, and
        // the producer cannot reuse it until this consumer's release below.
        let word = unsafe { self.slots.get().cast::<CommandWord>().add(slot).read() };
        self.read.store(read.wrapping_add(1), Ordering::Release);
        Some(word)
    }
}

pub struct Producer<'a> {
    queue: &'a CommandQueue,
}

impl Producer<'_> {
    pub fn try_push(&mut self, word: CommandWord) -> Result<(), CommandWord> {
        // SAFETY: split's unique borrow produces one non-clonable endpoint;
        // &mut self excludes concurrent calls through this producer.
        unsafe { self.queue.push_from_producer(word).map(|_| ()) }
    }
}

pub struct Consumer<'a> {
    queue: &'a CommandQueue,
}

impl Consumer<'_> {
    pub fn try_pop(&mut self) -> Option<CommandWord> {
        // SAFETY: the unique consumer handle and &mut self enforce its role.
        unsafe { self.queue.pop_from_consumer() }
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[test]
    fn concurrent_publication_never_tears_or_reorders_words() {
        let mut queue = CommandQueue::new();
        let (mut producer, mut consumer) = queue.split();
        std::thread::scope(|scope| {
            scope.spawn(move || {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
                for value in 0u32..500_000 {
                    let word = CommandWord::from_bits(value.wrapping_mul(2_654_435_761));
                    while producer.try_push(word).is_err() {
                        assert!(std::time::Instant::now() < deadline, "producer stalled");
                        core::hint::spin_loop();
                    }
                }
            });
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            for value in 0u32..500_000 {
                let actual = loop {
                    if let Some(word) = consumer.try_pop() {
                        break word;
                    }
                    assert!(std::time::Instant::now() < deadline, "consumer stalled");
                    core::hint::spin_loop();
                };
                assert_eq!(actual.bits(), value.wrapping_mul(2_654_435_761));
            }
            assert_eq!(consumer.try_pop(), None);
        });
    }

    #[test]
    fn wraparound_full_empty_and_bursts_match_an_independent_fifo() {
        for origin in [0, 1, u32::MAX - 1023, u32::MAX - 7] {
            let mut queue = CommandQueue::new();
            queue.read.store(origin, Ordering::Relaxed);
            queue.written.store(origin, Ordering::Relaxed);
            let (mut producer, mut consumer) = queue.split();
            let mut expected = VecDeque::new();
            let mut seed = 91u32;
            for round in 0..256 {
                // Large preset-like bursts intentionally exercise drop-newest.
                for _ in 0..if round % 7 == 0 { 1536 } else { 176 } {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    let word = CommandWord::from_bits(seed);
                    let accepted = producer.try_push(word);
                    if expected.len() == COMMAND_CAPACITY {
                        assert_eq!(accepted, Err(word));
                    } else {
                        assert_eq!(accepted, Ok(()));
                        expected.push_back(word);
                    }
                }
                for _ in 0..if round % 5 == 0 { 1400 } else { 128 } {
                    assert_eq!(consumer.try_pop(), expected.pop_front());
                }
            }
            while let Some(word) = expected.pop_front() {
                assert_eq!(consumer.try_pop(), Some(word));
            }
            assert_eq!(consumer.try_pop(), None);
        }
    }

    #[test]
    fn frozen_c_command_ids_and_queue_traces_match() {
        use fm1_types::CommandKind;
        let reference: serde_json::Value =
            serde_json::from_str(include_str!("../tests/legacy.json")).unwrap();
        for (index, kind) in reference["kinds"].as_array().unwrap().iter().enumerate() {
            assert_eq!(
                CommandKind::try_from(index as u8).unwrap() as u64,
                kind.as_u64().unwrap()
            );
        }
        let hash = |mut h: u32, value: u32| {
            for byte in value.to_le_bytes() {
                h = (h ^ byte as u32).wrapping_mul(16_777_619);
            }
            h
        };
        for case in reference["cases"].as_array().unwrap() {
            let origin = case["origin"].as_u64().unwrap() as u32;
            let mut queue = CommandQueue::new();
            queue.read.store(origin, Ordering::Relaxed);
            queue.written.store(origin, Ordering::Relaxed);
            let (mut producer, mut consumer) = queue.split();
            let (mut h, mut seed) = (2_166_136_261u32, 91u32);
            let (mut accepted, mut dropped, mut popped) = (0u32, 0u32, 0u32);
            for round in 0..256 {
                for _ in 0..if round % 7 == 0 { 1536 } else { 176 } {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    let word = CommandWord::new((seed >> 24) as u8, (seed >> 8) as u8, seed as i16);
                    let ok = producer.try_push(word).is_ok();
                    if ok {
                        accepted += 1;
                    } else {
                        dropped += 1;
                    }
                    h = hash(h, u32::from(ok));
                }
                for _ in 0..if round % 5 == 0 { 1400 } else { 128 } {
                    let word = consumer.try_pop();
                    h = hash(h, u32::from(word.is_some()));
                    if let Some(word) = word {
                        h = hash(h, word.bits());
                        popped += 1;
                    }
                }
            }
            while let Some(word) = consumer.try_pop() {
                h = hash(h, word.bits());
                popped += 1;
            }
            assert_eq!(h, case["hash"].as_u64().unwrap() as u32);
            assert_eq!(accepted, case["accepted"].as_u64().unwrap() as u32);
            assert_eq!(dropped, case["dropped"].as_u64().unwrap() as u32);
            assert_eq!(popped, case["popped"].as_u64().unwrap() as u32);
        }
    }
}
