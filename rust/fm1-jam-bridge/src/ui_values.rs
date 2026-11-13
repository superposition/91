// SPDX-License-Identifier: GPL-3.0-only
use crate::ui::{FrameCallback, PerformancePort};
use fm1_app::{
    performance,
    ui::State,
    values::{self, Operation},
};
type Reader = unsafe extern "C" fn(u32, *mut u8, u32) -> i32;
struct ValuesPort {
    foreground: PerformancePort,
    reader: Reader,
}
impl performance::Port for ValuesPort {
    fn with_ui<R>(&mut self, f: impl FnOnce(&mut State) -> R) -> R {
        performance::Port::with_ui(&mut self.foreground, f)
    }
    fn query(&mut self, q: performance::Query, a: u32) -> u32 {
        performance::Port::query(&mut self.foreground, q, a)
    }
    fn action(&mut self, a: performance::Action, x: u32, y: u32) {
        performance::Port::action(&mut self.foreground, a, x, y)
    }
    fn notice(&mut self, n: performance::Notice, x: u32, y: u32) {
        performance::Port::notice(&mut self.foreground, n, x, y)
    }
}
impl values::Port for ValuesPort {
    fn read(&mut self, kind: values::Read, bytes: &mut [u8]) -> bool {
        // SAFETY: this initialized local packet is disjoint from UI/producer
        // roots. The synchronous reader writes only this derived span, retains
        // no pointer and cannot reenter code that accesses this local packet.
        unsafe { (self.reader)(kind as u32, bytes.as_mut_ptr(), bytes.len() as u32) == 0 }
    }
}
/// # Safety
/// State is initialized, aligned and foreground-owned. Output (when used) is
/// one writable i32, or two for Range, disjoint from state and callback sources.
/// Callbacks can reenter UI between scoped loans but retain no port or reader
/// destination pointer. Reader writes only the exact initialized supplied span.
/// Errors leave output untouched; no state reference survives a callback.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_ui_value(
    state: *mut State,
    operation: u32,
    code: i32,
    argument: i32,
    callback: Option<FrameCallback>,
    reader: Option<Reader>,
    output: *mut i32,
) -> i32 {
    if state.is_null() || operation > Operation::MoodApply as u32 {
        return crate::app::reject();
    }
    let writes =
        operation != Operation::AcidSync as u32 && operation != Operation::MoodApply as u32;
    if writes && output.is_null() {
        return crate::app::reject();
    }
    let result = match operation {
        x if x == Operation::Parameter as u32 => {
            // SAFETY: pure query; no callback/retained loan.
            Some((unsafe { values::parameter(&*state, code) }, None))
        }
        x if x == Operation::AcidKind as u32 => {
            // SAFETY: pure checked query over initialized foreground state.
            unsafe { values::acid_kind(&*state, code as usize) }.map(|v| (v, None))
        }
        x if x == Operation::MoodValue as u32 => {
            // SAFETY: one scalar copy; the loan ends before any output write.
            values::mood_value(unsafe { (*state).mood }, code, argument).map(|v| (v, None))
        }
        x if x == Operation::Range as u32 || x == Operation::MoodApply as u32 => {
            let Some(callback) = callback else {
                return crate::app::reject();
            };
            let mut port = PerformancePort {
                ui: state,
                callback,
            };
            if x == Operation::Range as u32 {
                values::range(&mut port, code).map(|(lo, hi)| (lo, Some(hi)))
            } else {
                values::mood_apply(&mut port).then_some((0, None))
            }
        }
        _ => {
            let (Some(callback), Some(reader)) = (callback, reader) else {
                return crate::app::reject();
            };
            let mut port = ValuesPort {
                foreground: PerformancePort {
                    ui: state,
                    callback,
                },
                reader,
            };
            if operation == Operation::Get as u32 {
                values::get(&mut port, code).map(|v| (v, None))
            } else {
                values::acid_sync(&mut port).then_some((0, None))
            }
        }
    };
    let Some((value, high)) = result else {
        return crate::app::reject();
    };
    if writes {
        // SAFETY: the caller owns this disjoint result span. All source loans
        // and callbacks have ended; only Range writes the second element.
        unsafe {
            output.write(value);
            if let Some(high) = high {
                output.add(1).write(high);
            }
        }
    }
    0
}
