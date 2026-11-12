# TRS MIDI software port

This is the first bounded implementation for issue #19. `EN_RUST_USB=1` now
uses the safe Rust parser and content-based DMA consumer in the actual
`midi_uart.c` path. The device/host USB, SysEx transport and UAC policy are the
next slices; the whole ticket and physical qualification remain open.

`fm1-hal::midi::Trs` keeps the existing 24-byte state, counters and field offsets.
The existing 128-byte, 16-byte-aligned UART DMA ring and 64-event MIDI input
queue remain in their retained placements. There is no allocator, added audio
or transport payload, additional global production state or per-byte state
packet. UART register setup/count capture and its two `csync` instructions
remain the original thin C/ISA boundary.

The poll still follows content rather than trusting the DMA count. A lagging
count cannot delay a complete note. A DMA lap uses the same wrapping drop/read
arithmetic, clears status/got/SysEx and intentionally keeps need/first. The
undefined realtime byte `0xfd` remains the empty marker: a marker followed by a
real byte is consumed as noise. Running status, realtime priority, one-byte
messages and ignoring SysEx/system common from the TRS jack stay exact.
Initialization clears the ring without resetting parser/counter state.

## Ownership and contexts

The CPU0 MIDI poll owns one scoped mutable parser state. This is TIMER5 outside
the render in the current application. Its synchronous admission callback
accesses only the separate existing MIDI queue; it does not re-enter or retain
parser/ring pointers. The Rust adapter reads/clears bounded raw volatile bytes;
it never constructs a Rust reference into the UART DMA allocation. DMA may
advance between any raw operation, and volatile grants neither exclusion nor
atomicity. No new claim about avoiding physical UART overruns is made.

The named `FM1_HAL_BOOT_QUIESCENT` provider contract now explicitly includes USB
endpoint and UART RX DMA, alongside ALNK/display/SPI and confirmed CPU1 hold.
Logical flags or timeouts cannot authorize BSS/pool reuse. This provider remains
unqualified/default false. A flash transaction is unrelated to UART DMA
completion or DSP/display reset ownership.

## Evidence

Rust is pinned to 1.94.1. `sh tests/rust_usb_trs.sh
../baseline-2026-11-11/source` verifies all 440 immutable source hashes, then
compares frozen C, fresh C0 and the real Rust adapter. The 12,289 cases cover
24,576 direct byte operations and 1,060,122 externally written DMA bytes over
8,192 polls. Every semantic parser field, all 128 DMA bytes, all 64 queue slots,
queue indexes and drained packets match byte for byte, including full queues,
index/counter wrap, count lag, noise, overrun and all byte/status values.

Both standalone WASM consumers also match the same native frozen trace:
8,016,980 bytes, SHA-256
`551a1137552d8d079b655c41e663a829b1b1ad52ce7dc8951f908b5f6006627b`.
All eight existing named TRS regressions pass against both implementations.

Two focused safe-core tests cover content/count independence, realtime/SysEx,
laps, fixed capacity, partial-reset compatibility and invalid-index refusal.
The existing HAL suite totals 10 tests and the bridge totals 11, all passing.
Clippy with warnings denied, formatting and no_std/WASM checks pass. The
strict-provenance Miri retained-UART-root test passes: a raw DMA root remains
usable between parser calls without a temporary Rust ring reference.

The requested integration check also passed before the USB edit: both retained
LCD doubles (APP1 and APP0 with a real-ready provider) and all 78,353 frozen,
fresh-C0 and Rust native/WASM pixel cases, SHA-256
`2ab1f5e8f5f52ef156df08c21f156328f1be1942f3abdc1c9c994376f97f8a8b`.
The authoritative guards from `dbef41f` survive the HAL integration base.

Vendor PI32v2 main/engine objects compile with `EN_RUST_USB=1` and HAL1 using the
existing HAL compile command plus the USB flag. This is not mrustc emission,
whole-image link, target timing or device evidence. Exact logs/artifacts and
source hashes are retained in [usb-trs-proof.json](usb-trs-proof.json).

Remaining work includes device descriptors/EP0 and indexed-SIE ownership,
SysEx/ring publication, host attach/hub/loss/restart diagnostics and UAC
resampling/packet/stream ownership with fixed/master recording. Combined
hardware traffic, electrical UART/USB behavior, DMA stop facts, underruns,
deadlines and physical attach/loss/restart remain explicit acceptance gates.
No device, flash, live server or remote issue was changed.
