# Device transmit parity checkpoint

This separate #19 checkpoint ports the actual device EP1 packet fill, MIDI /
SysEx arbitration, SysEx event encoding and try/blocking send policy into safe
no_std `fm1-hal::usb_tx`. The C entrypoints select it with `EN_RUST_USB=1`;
C0 and existing payload placements remain. This is the ordinary-behavior
checkpoint before the separately authorized reset/admission correction.

The Rust fill drains SysEx first, preserves the active-producer MIDI exclusion,
sends complete four-byte events up to 64 bytes, retains unsigned index wrap and
keeps both original MIDI publication barriers. Encoding preserves CIN 4 for
continuation and 5/6/7 for the final 1/2/3-byte event, without changing source
bytes. Try-send retains all-at-once capacity refusal; blocking-send retains
clock-wrap handling and the original greater-than 200 ms check while full.
The bound is 192 input bytes for try and 640 for the existing OTA frame path.
Zero-length, unconfigured and partial-timeout compatibility is explicit in
these fixtures; no reset-boundary fix is hidden in this parity port.

## Boundary and cost

The existing 64-event MIDI and SysEx queues and 64-byte EP1 TX allocation are
unchanged. A scoped stack descriptor carries nine raw roots and three leaf /
clock / idle callbacks: 52 bytes on PI32v2, 104 on the 64-bit native host. Both C
and Rust assert its layout. There is no allocator, new persistent state,
duplicate payload or per-byte FFI/state packet. Send makes one Rust call per
message; fill makes one call per released endpoint packet. The adapter only
borrows its immutable local descriptor. No Rust reference into endpoint,
source, queue or scalar memory is formed or retained.

The idle callback may re-enter a disjoint raw consumer while the producer
remains scoped. It cannot start another producer, modify/retain the descriptor
or mutate source bytes. The raw endpoint must remain alive through DMA; its
original controller handshake is still C. Busy packets are never rewritten in
the retained-read model. Volatile access does not prove completion or grant
reuse after a failed hardware observation. Target stack/cycle/image costs and
peripheral completion behavior remain unqualified.

Known remaining lifecycle corrections are deliberately recorded: a reset can
clear the SysEx queue while the old foreground producer then publishes a tail,
including reset+reconfigure between steps; and `sie_rd` currently returns 0 for
off/timeout, which cannot establish endpoint completion. The next checkpoint
will separate reset/config epoch admission and unknown endpoint status from
ordinary C parity, with explicit frozen witnesses. It must keep masking short,
never wait while masked and preserve the separate polled loader path. The
existing timeout's partial-prefix behavior is retained here.

## Evidence

Pinned Rust 1.94.1:

```sh
sh tests/rust_usb_tx.sh ../baseline-2026-11-11/source
```

The fixture verifies all 440 frozen source hashes and compiles the actual
frozen/current driver with only its physical HAL include substituted. All
native frozen C, fresh C0, actual Rust adapter and C0/Rust WASM traces match in
four OTA/app/loader combinations: 354,404 snapshots and 49,519,664 bytes total.
OTA-off profiles each have 11,681 snapshots / 1,650,708 bytes, SHA-256
`6f92af4a55effbae819c169bd7bc138284615cea6f6627b85d1cf7a0986d9591`;
OTA-on profiles each have 165,521 / 23,109,124 bytes, SHA-256
`f3af81e7e4f8dd5233bf4387ce5c1c13c30e3950fdb897795a37f863625971eb`.

Cases cover empty/full and packet-boundary queue combinations, all individual
fills 0..64, four index/wrap bases, active SysEx producer exclusion, pending /
stalled endpoint status and repeated busy polls. Every retained send keeps its
exact source bytes until simulated completion. Every message length 0..640 is
checked against empty/63/64-event backlogs, configured/unconfigured state,
try/blocking mode, progressing/stalled consumers and millisecond wrap. Traces
include exact endpoint bytes, SIE event order, queue positions/payload hashes,
packet diagnostics, return values, waits and time observations. App and loader
fixtures preserve their current ordinary policy; this is not physical loader
or firmware-image evidence.

All 169 Linux workspace tests pass. Formatting, Clippy with warnings denied and
no_std/WASM checks pass. The safe packet-source test checks every length /
offset through 640 without reading outside its selected prefix. Strict-
provenance Miri passes a full retained queue, wrapping indexes and a producer
re-entered only by its scoped consumer, including original raw source/endpoint
roots and invalid-context refusal (2.37s). Vendor PI32v2 C main/engine objects
compile for USB1/HAL0 and USB1/HAL1 with the 52-byte assertions. No new Rust MCU
emission, whole-image link, stack/deadline or hardware result is claimed.
Retained hashes are in [usb-tx-proof.json](usb-tx-proof.json).

Active C still supplies SIE access, endpoint release/publication, device
reset/suspend/retry/detach and host control/enumeration, CDC rings, UAC
resampling/stream/packet policy and foreground message application. The new
Rust TX policy does not qualify their ownership or physical traffic. No
device, image, live server or remote issue was changed.
