# Button action policy

The actual `button()` consumer now dispatches through `fm1-app::buttons` under
`EN_RUST_APP=1`; C0 retains its original function. Rust chooses Songbook save,
hold/idea actions, HOME transitions, six page-group cycles, FX double presses,
transport, freeze, octave/lane changes and breakdown/build/drop actions.

Each UI access is scoped to a closure. The narrow bridge ends that borrow
before invoking view changes, storage, engine commands, scene reset or messages.
Those existing C effects may synchronously access UI again. This extends the
frame → C input → Rust button → C effect path without retaining an exclusive
UI reference across any callback. Engine queries copy only CPU0 data under
the named IRQ hooks on the physical platform; effects and Rust work run after
IRQ state is restored.

The six group records use 18 bytes of compact Rust metadata. They temporarily
coexist with the C table used by remaining helpers; subsequent helper ports
must remove that live duplicate. No mutable state, framebuffer, storage payload
or diagnostic counter is added. Existing manual/session save failure messages
and behavior are preserved.

Invalid physical button IDs are refused without mutation. A corrupt transient
last-FX-page index now selects the first page before looking up a label, avoiding
the former undefined out-of-bounds C lookup. Valid FX-page values retain exact
behavior. Lane arithmetic promotes the existing byte before modulo, including
the defined behavior of unusual byte values.

`tests/rust_ui_buttons.sh` executes the frozen R01 function and group/octave
helpers, current C0, and the actual Rust-enabled C function/callback on native
and WASM. Its 28,896 cases cover all 43 pages and 14 buttons, UI flags, clock
wrap and the timing edges. It emits every semantic UI field, all parameter
values and effect/message order without reading padding. Effect doubles mutate
UI and published flags, checking that later decisions observe those mutations.
Four null/index probes and a corrupt FX-page probe supplement the corpus.
An optional R01 path verifies all 440 manifest entries and independently
re-extracts the reference.

`tests/rust_ui_miri.sh` separately verifies Rust-side aliasing under strict
provenance using the existing pinned nightly interpreter. A retained
`UnsafeCell` root executes frame → input callback → button → view callback →
nested button, then checks that later frame operations see all mutations.
This tests actual Rust adapters with Rust implementations of the callback ABI;
the C drivers and physical interrupt behavior are covered separately.

Held-button/key routing, encoder routing, remaining application helpers, page
composition and visualizers are still separate #15 work. C message formatting
and view effects remain explicit transitional callbacks. Native/WASM evidence
does not qualify physical interrupt latency, memory footprint or device use.
