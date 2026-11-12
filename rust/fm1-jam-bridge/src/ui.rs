// SPDX-License-Identifier: GPL-3.0-only
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
