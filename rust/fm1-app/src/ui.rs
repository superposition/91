// SPDX-License-Identifier: GPL-3.0-only
//! Foreground UI state policy. Platform/engine calls happen outside its borrow.
pub use crate::ui_state::State;
pub const EFFECT_SETTINGS: usize = 96;
impl State {
    /// Initializes the same carrier in place, without a second UI object.
    pub fn initialize(&mut self, defaults: &[i16; EFFECT_SETTINGS], gate: u16) {
        self.clear();
        self.pat_key = -1;
        self.gr_all = 50;
        self.smp_gap = 1;
        self.smp_cur = -1;
        for (out, value) in self.fxs.iter_mut().zip(defaults.iter()) {
            *out = *value as u8;
        }
        self.gx_pat = gate;
        self.scene_quant = 1;
        self.dj_len = 1;
        self.dj_break = 1;
        self.dj_fillon = 1;
        self.mood = 50;
        self.jam_oct = 2;
        self.fm_dy = 1;
    }
}
