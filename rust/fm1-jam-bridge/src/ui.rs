// SPDX-License-Identifier: GPL-3.0-only
use fm1_app::controller::{self, Action, FrameState, Port, Query, Views};
use fm1_app::ui::{State, EFFECT_SETTINGS};
#[no_mangle]
pub extern "C" fn fm1_rust_ui_classify(operation: u32, view: i32) -> i32 {
    match operation {
        0 => fm1_app::controls::voice_of(view).map_or(-1, i32::from),
        1 => i32::from(fm1_app::controls::kit_view(view)),
        2 => fm1_app::controls::octave_parameter(view).index() as i32,
        3 => fm1_app::controls::group_of(view).map_or(-1, |index| index as i32),
        _ => -1,
    }
}
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

/// # Safety
/// UI is initialized and exclusively foreground-owned; output is one disjoint
/// writable i32. No callback occurs and no reference survives this call.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_ui_accel(
    state: *mut State,
    role: u32,
    step: i32,
    range: i32,
    now: u32,
    disabled: u32,
    output: *mut i32,
) -> i32 {
    if state.is_null() || output.is_null() {
        return crate::app::reject();
    }
    // SAFETY: caller owns the initialized UI; pure arithmetic invokes no C.
    let result = unsafe {
        fm1_app::controls::accelerate(&mut *state, role as usize, step, range, now, disabled != 0)
    };
    let Ok(result) = result else {
        return crate::app::reject();
    };
    // SAFETY: one exclusive disjoint output; the UI borrow already ended.
    unsafe { output.write(result) };
    crate::app::game_call();
    0
}

type FrameCallback = unsafe extern "C" fn(u32, u32, u32) -> u32;
struct ButtonPort {
    ui: *mut State,
    callback: FrameCallback,
}
impl fm1_app::buttons::Port for ButtonPort {
    fn with_ui<R>(&mut self, f: impl FnOnce(&mut State) -> R) -> R {
        // SAFETY: foreground-owned initialized state, with no callback or
        // retained reference until this closure has completely returned.
        f(unsafe { &mut *self.ui })
    }
    fn query(&mut self, q: fm1_app::buttons::Query, arg: u32) -> u32 {
        // SAFETY: there is no outstanding UI borrow during any callback.
        unsafe { (self.callback)(0x100 + q as u32, arg, 0) }
    }
    fn action(&mut self, a: fm1_app::buttons::Action, x: u32, y: u32) -> u32 {
        // SAFETY: callbacks may reenter/access UI only between state closures.
        unsafe { (self.callback)(a as u32, x, y) }
    }
    fn notice(&mut self, n: fm1_app::buttons::Notice, x: u32, y: u32) {
        // SAFETY: message effects also run after the exclusive UI borrow ends.
        unsafe { (self.callback)(0x200 + n as u32, x, y) };
    }
}
/// # Safety
/// State is initialized/aligned and foreground-owned. Callback is synchronous,
/// may reenter/access state, and retains no reference to this stack adapter.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_ui_button(
    state: *mut State,
    button: u32,
    callback: Option<FrameCallback>,
) -> i32 {
    let Some(callback) = callback else {
        return crate::app::reject();
    };
    if state.is_null() {
        return crate::app::reject();
    }
    if !fm1_app::buttons::button(
        &mut ButtonPort {
            ui: state,
            callback,
        },
        button,
    ) {
        return crate::app::reject();
    }
    crate::app::game_call();
    0
}
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, UnsafeCell};
    thread_local! {static ACTIVE: Cell<*mut State> = const {Cell::new(core::ptr::null_mut())};}
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            ACTIVE.with(|state| state.set(core::ptr::null_mut()));
        }
    }
    fn active() -> *mut State {
        ACTIVE.with(Cell::get)
    }
    unsafe extern "C" fn leaf(op: u32, _a: u32, _b: u32) -> u32 {
        if op == 0x100 {
            return 45000;
        }
        if (0x100..0x200).contains(&op) {
            return 0;
        }
        // SAFETY: the test's live UnsafeCell root; caller's scoped UI borrow
        // must have ended before entering this callback.
        unsafe { (*active()).frame += 1 };
        0
    }
    unsafe extern "C" fn button(op: u32, a: u32, b: u32) -> u32 {
        if op == fm1_app::buttons::Action::View as u32 {
            let pointer = active();
            // SAFETY: no reference is retained over the nested application call.
            unsafe {
                (*pointer).view = a as u8;
            }
            unsafe {
                fm1_rust_ui_button(pointer, fm1_app::buttons::id::PLAY, Some(leaf));
            }
            return 0;
        }
        // SAFETY: identical callback contract, no retained reference.
        unsafe { leaf(op, a, b) }
    }
    unsafe extern "C" fn frame(op: u32, _a: u32, _b: u32) -> u32 {
        if op == 0x100 {
            return 45000;
        }
        if op >= 0x100 {
            return 0;
        }
        if op == Action::Input as u32 {
            let pointer = active();
            // SAFETY: these writes end before a nested button action reenters UI.
            unsafe {
                (*pointer).frame = 100;
                (*pointer).mood = 73;
            }
            unsafe {
                fm1_rust_ui_button(pointer, fm1_app::buttons::id::HOME, Some(button));
            }
        } else if op == Action::Main as u32 {
            // SAFETY: exclusively borrowed only inside this callback.
            unsafe {
                let state = &mut *active();
                state.frame += 1;
                state.saver_on = 1;
            }
        } else if op == Action::Leds as u32 {
            // SAFETY: the previous callback and every UI closure have returned.
            unsafe {
                (*active()).frame += 1;
            }
        }
        0
    }
    #[test]
    fn callbacks_reenter_ui_only_between_scoped_borrows() {
        // SAFETY: all 193 fields are plain integers, floats or byte arrays;
        // zero is valid for every field. No padding is inspected.
        let state = Box::new(UnsafeCell::new(unsafe { core::mem::zeroed::<State>() }));
        let auxiliary = Box::new(UnsafeCell::new(FrameState::default()));
        ACTIVE.with(|active| active.set(state.get()));
        let _reset = Reset;
        // SAFETY: live, disjoint initialized roots retained throughout nested
        // synchronous frame -> input -> button -> view -> button callbacks.
        let result = unsafe {
            fm1_rust_ui_frame(
                state.get(),
                auxiliary.get(),
                Some(frame),
                34 | (38 << 8) | (39 << 16) | (42 << 24),
            )
        };
        assert_eq!(result, 0);
        // SAFETY: no call-scoped UI reference survives the application call.
        unsafe {
            assert_eq!((*state.get()).frame, 106);
            assert_eq!((*state.get()).mood, 73);
            assert_eq!((*state.get()).view, 36);
            assert_eq!((*state.get()).saver_on, 1);
        }
    }
}
