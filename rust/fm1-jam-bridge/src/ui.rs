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
struct PerformancePort {
    ui: *mut State,
    callback: FrameCallback,
}
struct AutomaticPort {
    foreground: PerformancePort,
    dj: *mut fm1_app::automatic::DjState,
}
impl fm1_app::performance::Port for AutomaticPort {
    fn with_ui<R>(&mut self, f: impl FnOnce(&mut State) -> R) -> R {
        fm1_app::performance::Port::with_ui(&mut self.foreground, f)
    }
    fn query(&mut self, q: fm1_app::performance::Query, a: u32) -> u32 {
        fm1_app::performance::Port::query(&mut self.foreground, q, a)
    }
    fn action(&mut self, a: fm1_app::performance::Action, x: u32, y: u32) {
        fm1_app::performance::Port::action(&mut self.foreground, a, x, y)
    }
    fn notice(&mut self, n: fm1_app::performance::Notice, x: u32, y: u32) {
        fm1_app::performance::Port::notice(&mut self.foreground, n, x, y)
    }
}
impl fm1_app::automatic::DjPort for AutomaticPort {
    fn with_dj<R>(&mut self, f: impl FnOnce(&mut fm1_app::automatic::DjState) -> R) -> R {
        // SAFETY: initialized, aligned foreground cache disjoint from UI. No
        // callback occurs until this exclusive five-byte loan has ended.
        f(unsafe { &mut *self.dj })
    }
}
/// # Safety
/// UI and the initialized five-byte DJ cache are disjoint foreground roots.
/// Synchronous callbacks may access these roots between loans, retaining no
/// pointer/reference to this stack adapter. IRQ/worker code does not access them.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_ui_dj_tick(
    state: *mut State,
    dj: *mut fm1_app::automatic::DjState,
    callback: Option<FrameCallback>,
) -> i32 {
    let Some(callback) = callback else {
        return crate::app::reject();
    };
    if state.is_null() || dj.is_null() {
        return crate::app::reject();
    }
    fm1_app::automatic::dj_tick(&mut AutomaticPort {
        foreground: PerformancePort {
            ui: state,
            callback,
        },
        dj,
    });
    crate::app::game_call();
    0
}
/// # Safety
/// Output is one initialized/exclusively writable u32, disjoint from immutable
/// factory metadata. No callback or retained reference; errors leave it intact.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_ui_auto_metadata(
    index: u32,
    field: u32,
    output: *mut u32,
) -> i32 {
    if output.is_null() {
        return crate::app::reject();
    }
    let Some(value) = fm1_app::automatic::auto_metadata(index as usize, field) else {
        return crate::app::reject();
    };
    // SAFETY: exclusive caller-owned scalar, with no outstanding metadata loan.
    unsafe { output.write(value) };
    0
}
impl fm1_app::performance::Port for PerformancePort {
    fn with_ui<R>(&mut self, f: impl FnOnce(&mut State) -> R) -> R {
        // SAFETY: initialized foreground-owned UI; no call occurs during the loan.
        f(unsafe { &mut *self.ui })
    }
    fn query(&mut self, q: fm1_app::performance::Query, argument: u32) -> u32 {
        // SAFETY: every exclusive state loan has ended before the callback.
        unsafe { (self.callback)(0x100 + q as u32, argument, 0) }
    }
    fn action(&mut self, a: fm1_app::performance::Action, x: u32, y: u32) {
        // SAFETY: effects can reenter/access UI only between scoped loans.
        unsafe { (self.callback)(a as u32, x, y) };
    }
    fn notice(&mut self, n: fm1_app::performance::Notice, x: u32, y: u32) {
        // SAFETY: no state reference survives into message/view callbacks.
        unsafe { (self.callback)(0x200 + n as u32, x, y) };
    }
}
/// # Safety
/// State is initialized, aligned and foreground-owned. Synchronous callbacks
/// may access/reenter leaf app APIs, but retain no reference to this stack port.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_ui_performance(
    state: *mut State,
    operation: u32,
    argument: i32,
    callback: Option<FrameCallback>,
) -> i32 {
    let Some(callback) = callback else {
        return crate::app::reject();
    };
    if state.is_null() {
        return crate::app::reject();
    }
    if !fm1_app::performance::dispatch(
        &mut PerformancePort {
            ui: state,
            callback,
        },
        operation,
        argument,
    ) {
        return crate::app::reject();
    }
    crate::app::game_call();
    0
}
struct InputPort {
    ui: *mut State,
    seen: *mut u32,
    held: *mut u8,
    callback: FrameCallback,
}
impl fm1_app::input::Port for InputPort {
    fn with_ui<R>(&mut self, f: impl FnOnce(&mut State) -> R) -> R {
        // SAFETY: caller-owned initialized foreground UI; no callback inside the loan.
        f(unsafe { &mut *self.ui })
    }
    fn with_cache<R>(&mut self, f: impl FnOnce(&mut u32, &mut u8) -> R) -> R {
        // SAFETY: separate live initialized/aligned scalar roots, disjoint from UI.
        f(unsafe { &mut *self.seen }, unsafe { &mut *self.held })
    }
    fn query(&mut self, q: fm1_app::input::Query, a: u32, b: u32) -> u32 {
        // SAFETY: no UI/cache loan survives a state closure.
        unsafe { (self.callback)(0x100 + q as u32, a, b) }
    }
    fn action(&mut self, a: fm1_app::input::Action, x: u32, y: u32) -> u32 {
        // SAFETY: callbacks may access UI/cache between all exclusive loans.
        unsafe { (self.callback)(a as u32, x, y) }
    }
    fn notice(&mut self, n: fm1_app::input::Notice, x: u32, y: u32) {
        // SAFETY: text/view effects run only after state loans return.
        unsafe { (self.callback)(0x200 + n as u32, x, y) };
    }
}
/// # Safety
/// UI/seen/held are initialized, aligned, disjoint foreground-owned roots.
/// The synchronous callback may reenter leaf app APIs and retains no pointer
/// to this stack adapter. IRQ/worker code never accesses these carriers.
#[no_mangle]
pub unsafe extern "C" fn fm1_rust_ui_input(
    state: *mut State,
    seen: *mut u32,
    held: *mut u8,
    callback: Option<FrameCallback>,
) -> i32 {
    let Some(callback) = callback else {
        return crate::app::reject();
    };
    if state.is_null() || seen.is_null() || held.is_null() {
        return crate::app::reject();
    }
    let mut port = InputPort {
        ui: state,
        seen,
        held,
        callback,
    };
    let rejected = fm1_app::input::poll(&mut port);
    for _ in 0..rejected {
        crate::app::reject();
    }
    crate::app::game_call();
    rejected as i32
}
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
    thread_local! {
        static SEEN: Cell<*mut u32> = const {Cell::new(core::ptr::null_mut())};
        static HELD: Cell<*mut u8> = const {Cell::new(core::ptr::null_mut())};
        static INPUT_CALLBACKS: Cell<u32> = const {Cell::new(0)};
        static BAD_ENCODER: Cell<bool> = const {Cell::new(false)};
        static SCENE_DENSITY: Cell<u32> = const {Cell::new(0)};
        static TAP_NOW: Cell<u32> = const {Cell::new(0)};
        static DJ_ACTIVE: Cell<*mut fm1_app::automatic::DjState> = const {Cell::new(core::ptr::null_mut())};
        static DJ_MAP: Cell<u32> = const {Cell::new(0)};
    }
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            ACTIVE.with(|state| state.set(core::ptr::null_mut()));
            SEEN.with(|state| state.set(core::ptr::null_mut()));
            HELD.with(|state| state.set(core::ptr::null_mut()));
            DJ_ACTIVE.with(|state| state.set(core::ptr::null_mut()));
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
    unsafe extern "C" fn input(op: u32, _a: u32, _b: u32) -> u32 {
        use fm1_app::input::{Action as A, Query as Q};
        INPUT_CALLBACKS.with(|count| count.set(count.get() + 1));
        // SAFETY: disjoint retained UnsafeCell roots. Every callback accesses all
        // three roots, detecting any exclusive loan surviving a query/effect.
        unsafe {
            (*active()).frame += 1;
            SEEN.with(|pointer| *pointer.get() = (*pointer.get()).wrapping_add(0));
            HELD.with(|pointer| *pointer.get() ^= 0);
        }
        if op == 0x100 + Q::Now as u32 {
            return 45000;
        }
        if op == 0x100 + Q::FlexDone as u32 {
            return 1;
        }
        if op == 0x100 + Q::FlexEffect as u32 {
            return 5;
        }
        if op == 0x100 + Q::Encoder as u32 {
            return if _a == 1 && BAD_ENCODER.with(Cell::get) {
                i32::MAX as u32
            } else {
                0
            };
        }
        if op == A::Midi as u32 {
            // SAFETY: the input adapter holds only raw pointer tuples here;
            // its state and cache loans ended before this reentrant leaf call.
            unsafe {
                fm1_rust_ui_button(active(), fm1_app::buttons::id::HOME, Some(button));
            }
        }
        0
    }
    #[test]
    fn input_callbacks_reenter_ui_and_caches_between_scoped_borrows() {
        // SAFETY: all UI fields admit zero; padding is never inspected.
        let state = Box::new(UnsafeCell::new(unsafe { core::mem::zeroed::<State>() }));
        let seen = Box::new(UnsafeCell::new(7u32));
        let held = Box::new(UnsafeCell::new(1u8));
        ACTIVE.with(|pointer| pointer.set(state.get()));
        SEEN.with(|pointer| pointer.set(seen.get()));
        HELD.with(|pointer| pointer.set(held.get()));
        let _reset = Reset;
        // SAFETY: separate initialized roots remain alive for all callbacks.
        unsafe {
            (*state.get()).fx_lock = 1 << 5;
        }
        for malformed in [false, true, false] {
            BAD_ENCODER.with(|flag| flag.set(malformed));
            INPUT_CALLBACKS.with(|count| count.set(0));
            // SAFETY: the previous poll returned; avoid interpreting this
            // repeated synthetic leaf call as the separate double-Home gesture.
            unsafe {
                (*state.get()).home_t = 0;
                (*state.get()).view = 0;
            }
            // SAFETY: complete live disjoint roots, synchronous callbacks.
            let result =
                unsafe { fm1_rust_ui_input(state.get(), seen.get(), held.get(), Some(input)) };
            assert_eq!(result, i32::from(malformed));
            assert!(INPUT_CALLBACKS.with(Cell::get) > 10);
            // SAFETY: all nested calls have returned; no state/cache loan lives.
            unsafe {
                assert_eq!(*seen.get(), 1);
                assert_eq!(*held.get(), 0);
                assert_eq!((*state.get()).fx_lock, 0);
                assert_eq!((*state.get()).view, 36);
                assert_eq!((*state.get()).mood, 0);
            }
        }
        INPUT_CALLBACKS.with(|count| count.set(0));
        // SAFETY: each call must reject its missing argument before any access;
        // all non-null roots still satisfy the full contract.
        unsafe {
            assert_eq!(
                fm1_rust_ui_input(core::ptr::null_mut(), seen.get(), held.get(), Some(input)),
                -1
            );
            assert_eq!(
                fm1_rust_ui_input(state.get(), core::ptr::null_mut(), held.get(), Some(input)),
                -1
            );
            assert_eq!(
                fm1_rust_ui_input(state.get(), seen.get(), core::ptr::null_mut(), Some(input)),
                -1
            );
            assert_eq!(
                fm1_rust_ui_input(state.get(), seen.get(), held.get(), None),
                -1
            );
        }
        assert_eq!(INPUT_CALLBACKS.with(Cell::get), 0);
    }
    unsafe extern "C" fn performance_leaf(_op: u32, _a: u32, _b: u32) -> u32 {
        // SAFETY: the nested view adapter ended every UI loan before this call.
        unsafe { (*active()).frame += 1 };
        0
    }
    unsafe extern "C" fn performance(op: u32, a: u32, b: u32) -> u32 {
        use fm1_app::performance::{Action as A, Operation as O, Query as Q};
        if op == 0x100 + Q::Now as u32 {
            return TAP_NOW.with(Cell::get);
        }
        if op == A::Tracks as u32 {
            // SAFETY: outer scene state loans ended. This nested view runs its
            // own short loans and callback without retaining the outer adapter.
            unsafe {
                fm1_rust_ui_performance(active(), O::View as u32, 12, Some(performance_leaf));
                (*active()).scene_g[2][0] = 91;
            }
        } else if op == A::Knob as u32 {
            if a == fm1_types::ParamId::DENS.index() as u32 {
                SCENE_DENSITY.with(|density| density.set(b));
            }
            // SAFETY: callback mutates the live root between exclusive loans.
            unsafe { (*active()).fxk[0] += 1 };
        }
        0
    }
    #[test]
    fn scene_callbacks_reenter_view_before_later_scene_reads() {
        use fm1_app::performance::Operation as O;
        // SAFETY: the UI's plain fields admit zero; padding is never read.
        let state = Box::new(UnsafeCell::new(unsafe { core::mem::zeroed::<State>() }));
        ACTIVE.with(|pointer| pointer.set(state.get()));
        let _reset = Reset;
        // SAFETY: root is live with no loan at this boundary.
        unsafe { (*state.get()).fxk[0] = 3 };
        SCENE_DENSITY.with(|density| density.set(0));
        // SAFETY: initialized foreground UI, callback and nested call synchronous.
        assert_eq!(
            unsafe {
                fm1_rust_ui_performance(state.get(), O::SceneApply as u32, 2, Some(performance))
            },
            0
        );
        assert_eq!(SCENE_DENSITY.with(Cell::get), 91);
        // SAFETY: every outer/nested loan ended when the call returned.
        unsafe {
            assert_eq!((*state.get()).view, 12);
            assert_eq!((*state.get()).fxk[0], 16);
            assert_eq!((*state.get()).glide_f[0], 16.0);
            assert_eq!((*state.get()).glide_on, 15);
        }
        for (operation, argument) in [
            (O::SceneStore as u32, -1),
            (O::SceneApply as u32, 4),
            (O::SceneRecall as u32, i32::MAX),
            (O::View as u32, -1),
            (O::View as u32, 43),
            (99, 0),
        ] {
            // SAFETY: invalid operation/index is rejected before access/effects.
            assert_eq!(
                unsafe {
                    fm1_rust_ui_performance(state.get(), operation, argument, Some(performance))
                },
                -1
            );
        }
        // Same-clock taps preserve history while rejecting only the undefined
        // tempo division. A later nonzero interval remains usable.
        TAP_NOW.with(|now| now.set(100));
        // SAFETY: no live UI loan; reset the isolated tap history.
        unsafe { (*state.get()).tap_n = 0 };
        for (now, expected) in [(100, 0), (100, -1), (600, 0)] {
            TAP_NOW.with(|clock| clock.set(now));
            // SAFETY: the same initialized root and synchronous callback.
            assert_eq!(
                unsafe {
                    fm1_rust_ui_performance(state.get(), O::Tap as u32, 0, Some(performance))
                },
                expected
            );
        }
        // SAFETY: missing pointers are rejected before callback/access.
        unsafe {
            assert_eq!(
                fm1_rust_ui_performance(
                    core::ptr::null_mut(),
                    O::SceneTick as u32,
                    0,
                    Some(performance)
                ),
                -1
            );
            assert_eq!(
                fm1_rust_ui_performance(state.get(), O::SceneTick as u32, 0, None),
                -1
            );
        }
    }
    unsafe extern "C" fn automatic(op: u32, a: u32, _b: u32) -> u32 {
        use fm1_app::performance::{Action as A, Operation as O, Query as Q};
        INPUT_CALLBACKS.with(|count| count.set(count.get() + 1));
        // SAFETY: distinct retained roots. No exclusive UI/cache loan may
        // survive any metadata, message or effect callback.
        unsafe {
            (*active()).frame += 1;
            DJ_ACTIVE.with(|cache| (*cache.get()).host ^= 0);
        }
        if op == 0x100 + Q::Now as u32 {
            return 100;
        }
        if op == 0x100 + Q::Playing as u32 {
            return 1;
        }
        if op == 0x100 + Q::IntroFraction as u32 {
            return 1.0f32.to_bits();
        }
        if op == 0x100 + Q::DriftBars as u32 {
            return 3.0f32.to_bits();
        }
        if op == 0x100 + Q::BuildLength as u32 {
            return 4;
        }
        if op == 0x100 + Q::HostMessage as u32 {
            return 3;
        }
        if op == A::Intro as u32 {
            // SAFETY: cache/UI loans ended before this effect; the next map
            // decision must observe this changed field.
            unsafe {
                (*active()).midi_map = 2;
            }
        } else if op == A::MidiMap as u32 {
            DJ_MAP.with(|map| map.set(a));
        } else if op == A::DjSync as u32 {
            // SAFETY: reenter page policy while the outer DJ cache is retained
            // as a raw pointer, without an outstanding typed reference.
            unsafe {
                fm1_rust_ui_performance(active(), O::View as u32, 13, Some(automatic));
            }
        }
        0
    }
    #[test]
    fn dj_callbacks_reenter_ui_and_cache_between_scoped_loans() {
        // SAFETY: every plain UI field admits zero; padding is not read.
        let state = Box::new(UnsafeCell::new(unsafe { core::mem::zeroed::<State>() }));
        let cache = Box::new(UnsafeCell::new(fm1_app::automatic::DjState::initial()));
        ACTIVE.with(|pointer| pointer.set(state.get()));
        DJ_ACTIVE.with(|pointer| pointer.set(cache.get()));
        let _reset = Reset;
        // SAFETY: no typed loan lives at setup, roots remain allocated.
        unsafe {
            (*cache.get()).last = 15;
            (*state.get()).intro = 2;
            (*state.get()).midi_map = 1;
            (*state.get()).usb_host = 1;
            (*state.get()).dj_on = 1;
            (*state.get()).dj_len = 4;
            (*state.get()).scene_q = 1;
            (*state.get()).mx_bars = 4;
            (*state.get()).mx_n = 2;
        }
        DJ_MAP.with(|map| map.set(0));
        // SAFETY: complete initialized disjoint roots and synchronous callbacks.
        assert_eq!(
            unsafe { fm1_rust_ui_dj_tick(state.get(), cache.get(), Some(automatic)) },
            0
        );
        assert_eq!(DJ_MAP.with(Cell::get), 2);
        // SAFETY: all application loans ended on return.
        unsafe {
            assert_eq!(
                *cache.get(),
                fm1_app::automatic::DjState {
                    last: 0,
                    intro: 2,
                    map: 2,
                    host: 1,
                    sync: 1
                }
            );
            assert_eq!((*state.get()).view, 13);
            assert_eq!((*state.get()).scene_q, 0);
            assert_eq!((*state.get()).mx_n, 3);
            (*cache.get()).last = 15;
            (*state.get()).scene_q = 255;
            (*state.get()).glide_on = 0xa5;
        }
        // SAFETY: malformed queued scene is cleared without indexing its arrays.
        assert_eq!(
            unsafe { fm1_rust_ui_dj_tick(state.get(), cache.get(), Some(automatic)) },
            0
        );
        // SAFETY: callback sequence returned, no typed loan remains.
        unsafe {
            assert_eq!((*state.get()).scene_q, 0);
            assert_eq!((*state.get()).glide_on, 0xa5);
        }
        INPUT_CALLBACKS.with(|count| count.set(0));
        // SAFETY: each missing argument is rejected before access/callbacks.
        unsafe {
            assert_eq!(
                fm1_rust_ui_dj_tick(core::ptr::null_mut(), cache.get(), Some(automatic)),
                -1
            );
            assert_eq!(
                fm1_rust_ui_dj_tick(state.get(), core::ptr::null_mut(), Some(automatic)),
                -1
            );
            assert_eq!(fm1_rust_ui_dj_tick(state.get(), cache.get(), None), -1);
        }
        assert_eq!(INPUT_CALLBACKS.with(Cell::get), 0);
        let mut value = 0xdead_beefu32;
        for (index, field) in [(28, 0), (0, 6), (27, 5), (u32::MAX, 0)] {
            // SAFETY: exclusive initialized scalar; invalid metadata leaves it intact.
            assert_eq!(
                unsafe { fm1_rust_ui_auto_metadata(index, field, &mut value) },
                -1
            );
            assert_eq!(value, 0xdead_beef);
        }
        // SAFETY: null output is rejected before any access.
        assert_eq!(
            unsafe { fm1_rust_ui_auto_metadata(0, 0, core::ptr::null_mut()) },
            -1
        );
    }
}
