# Encoder policy

The real `accel()` consumer calls `fm1-app::controls::accelerate` when
`EN_RUST_APP=1`. C0 retains its original function. The safe core updates the
existing seven timestamps and run counters in place, with no callbacks,
allocation, extra state or new diagnostic counter.

The first few detents remain single steps. Per-detent intervals below 12, 25
and 45 milliseconds select gains 8, 5 and 3 after the run reaches four;
ranges below 100 cap the gain at 3, while ranges at most 24 and disabled
acceleration retain single steps. Time subtraction wraps as before. Saturation
and the original byte conversion of long runs are preserved.

Invalid encoder IDs and overflowing signed results return typed errors before
changing UI state. The narrow C wrapper returns a zero adjustment on rejection.
These malformed inputs formerly indexed outside the arrays or caused undefined
signed C arithmetic; the defined control domain is unchanged. Browser producer
accumulation is a separate platform boundary under #17.

`tests/rust_ui_controls.sh` executes the captured R01 function, current C0 and
the actual Rust-enabled C consumer on native and WASM. Its 296,352 cases cross
all seven encoders, timing thresholds, long runs, small/large ranges, signs,
disabled acceleration, wrapping timestamps and full-width defined products.
The corpus includes all timestamps/run bytes and a hash of each semantic UI
field, excluding padding. Six additional null, index and overflow probes check
unchanged UI/output on rejection. The optional R01 argument verifies its 440-file
manifest and re-extracts the original function.

`tests/host/emu_controls_test.mjs` exercises the nested production path:
Rust frame → C input/turn → Rust encoder → C parameter application. A second
identical Rust instrument provides the frame-counter baseline, proving one
actual encoder call without another counter. Normal knob turns match C0 audio
and framebuffer bytes; an oversized malformed detent is refused and a valid
subsequent turn still works. This is source-level WASM execution, not browser,
MIDI or device QA. Button, key and remaining input/page policies are still
separate work under #15.

Page classification also runs through safe Rust for the actual C `voice_of`,
`kit_view`, `oct_par` and `group_of` consumers. Octave choices return the existing
shared `ParamId`; unknown pages keep the legacy fallbacks. Six immutable group
records are shared by Rust policy helpers. Classification invokes no callback
or diagnostic mutation and runs outside IRQ exclusion. Its independent R01,
current-C and Rust native/WASM corpus covers all byte-sized pages, negative
values and both signed extremes (770 records), plus unknown-operation probes.
