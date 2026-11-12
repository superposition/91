// SPDX-License-Identifier: GPL-3.0-only
use fm1_app::controller::{self, Action, FrameState, Port, Query, Views};
use fm1_app::ui::{State, EFFECT_SETTINGS};
/// # Safety
/// UI is initialized/aligned, foreground-owned and disjoint from immutable
/// defaults96. No concurrent/reentrant UI access, callback or retained borrow.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_ui_initialize(
    state: *mut State,
    defaults: *const [i16; EFFECT_SETTINGS],
    gate: u16,
) -> i32 {
    if state.is_null() || defaults.is_null() {
        return crate::app::reject();
    }
    // SAFETY: initialized plain-field UI and immutable complete default table.
    unsafe { (&mut *state).initialize(&*defaults, gate) };
    crate::app::game_call();
    0
}

type FrameCallback = unsafe extern "C" fn(u32, u32, u32) -> u32;
struct ForegroundPort {
    ui: *mut State,
    frame: *mut FrameState,
    callback: FrameCallback,
}
impl Port for ForegroundPort {
    fn with_ui<R>(&mut self, f: impl FnOnce(&mut State) -> R) -> R {
        // SAFETY: caller owns this initialized foreground carrier; the closure
        // cannot export its reference. No C call occurs until this borrow ends.
        f(unsafe { &mut *self.ui })
    }
    fn with_frame<R>(&mut self, f: impl FnOnce(&mut FrameState) -> R) -> R {
        // SAFETY: disjoint initialized auxiliary words, scoped like UI above.
        f(unsafe { &mut *self.frame })
    }
    fn query(&mut self, query: Query) -> u32 {
        // SAFETY: no UI/aux reference survives a state closure. Reentrant C may
        // access those objects; this Port owns only its disjoint raw-pointer tuple.
        unsafe { (self.callback)(0x100 | query as u32, 0, 0) }
    }
    fn action(&mut self, action: Action, first: u32, second: u32) -> u32 {
        // SAFETY: callbacks execute strictly between all state borrows.
        unsafe { (self.callback)(action as u32, first, second) }
    }
}

/// # Safety
/// UI and frame are initialized, aligned, disjoint, foreground-owned objects.
/// Callback may synchronously access/reenter the app, but may not retain this
/// stack Port or start another foreground frame concurrently. Audio/IRQ code
/// must not access UI or frame; callback observations use published engine data.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_ui_frame(
    state: *mut State,
    frame: *mut FrameState,
    callback: Option<FrameCallback>,
    view_ids: u32,
) -> i32 {
    let Some(callback) = callback else {
        return crate::app::reject();
    };
    if state.is_null() || frame.is_null() {
        return crate::app::reject();
    }
    let mut port = ForegroundPort {
        ui: state,
        frame,
        callback,
    };
    controller::frame(
        &mut port,
        Views {
            jam: view_ids as u8,
            songbook: (view_ids >> 8) as u8,
            arrangement: (view_ids >> 16) as u8,
            swells: (view_ids >> 24) as u8,
        },
    );
    crate::app::game_call();
    0
}
