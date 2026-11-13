# Songbook chord labels while switched off

The copied Songbook view now retains the current and planned chord names when
Songbook is switched off. The original page asks for both names in that state;
the snapshot adapter had filled them only when `en_jam_on` was nonzero. This
left blank text on the copied-view path despite matching musical and UI state.
The correction calls the same existing bounded name getters in both states.

`tests/rust_jam_labels.sh <R01-source>` verifies all 440 immutable source hashes
and compares the complete original app/engine, current C0 and Rust-enabled
consumer on native and WASM. Its 64 profiles cover all four Songbook on/off
and playback stopped/playing combinations, with key and hold changes. Each
record contains both names, PCM and every framebuffer byte. The current
snapshot is also checked directly against the legacy getters and initialized
name-buffer bounds. A fixed independently generated golden hash protects the
reference output.

The 220-byte snapshot, public exports and source exclusion contract are
unchanged. There is no additional runtime buffer or retained reference. These
tests use the serialized native/WASM app boundary; they do not qualify physical
interrupt exclusion or display/audio hardware.
