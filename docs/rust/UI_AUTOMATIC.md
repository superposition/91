# Automatic DJ, FX and mix keys

Five production helpers now use `fm1-app::automatic`: mix-key actions,
next-piece selection, DJ bar/tick policy and automatic FX. Scene actions and
tempo changes reuse the scoped performance port. C0 retains the original
functions behind the independent `EN_RUST_APP` flag.

The DJ's five original byte statics become one caller-owned five-byte carrier:
last step, intro/map/host settings sent, and chord-sync sent. Its initialization
and persistence across `ui_init` are unchanged. The bridge lends UI and cache
only in separate short closures; no typed loan survives a platform, engine or
message callback. Published CPU0 byte/word/float observations are copied under
the named physical IRQ hooks, then IRQ state is restored before Rust work.
Worker-private sample spans are not inspected.

The 28 automatic-effect records keep their compact eight-byte representation:
effect ID, amount threshold, section mask, maximum length and genre-favorite
bits. The 224-byte Rust table replaces the active C table; its C counterpart
is C0-only. Existing verified engine effect/setting IDs are shared as symbolic
constants. The small DJ timing tables retain their byte/word widths.
Candidate and weight arrays remain 28 bytes each on the stack, as in C.
The 27-byte mix-key mapping temporarily coexists with the C rendering lookup;
remaining presentation work must retire that live duplicate. No additional
mutable state, pixel/storage payload or diagnostic counter is added.

The port preserves integer tempo ramps, section/build alignment, intro holds,
sync behavior, queued scenes/pieces, pattern-release restore, weighted FX RNG,
last-two-effect exclusion, drop lead-ins and manual cancellation. Message
formatting stays in bounded C effects; the long DJ-sync caption uses its
constant string directly rather than the former undersized temporary.

`tests/rust_ui_automatic.sh <R01-source>` verifies all 440 frozen source hashes
and executes complete R01/current-C0/Rust consumers on native and WASM. The
4,096 cases compare all fields of all 28 records and every mix-key mapping,
then inject bounded published clock/section/intro observations between completed
renders. Each of the 28 effects is actually scheduled; the seen/required masks
are asserted, with two fixed seeds supplementing rare random choices. Every
record contains PCM, explicit UI/project/global/slot fields, platform/store
state and framebuffer hashes. The same fixed golden applies to both targets.
No padding is serialized and no output difference is tolerated.

Four strict-provenance UI Miri cases include DJ cache access from callbacks,
nested page actions and a map change observed by a later decision. Separate
probes cover missing arguments, invalid metadata fields/indices, untouched
error outputs, and a malformed queued scene. The interpreted callbacks are
Rust implementations of the C ABI; they do not execute C or physical IRQ code.

Remaining #15 work includes parameter/knob/MIDI helpers, bounded text/widget
presentation, page composition and visualizers. Target ROM/RAM/stack,
interrupt latency and physical controls/display/audio require their separate
adopted-compiler and device qualification.
