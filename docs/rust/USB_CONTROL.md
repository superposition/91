# USB descriptors and endpoint-zero policy

This is the second bounded implementation for issue #19. `EN_RUST_USB=1`
selects the safe Rust descriptor generator, SETUP request decisions and EP0
packet/ZLP framing in the actual `usb.c` device path. It builds on the TRS
adapter in `99766ff`. The original C implementation remains selected by C0.
This does not finish USB host, MIDI/SysEx queues or UAC stream ownership.

`fm1-hal` remains no_std, dependency-free in production and forbids unsafe code.
The descriptor generator keeps compact immutable pieces and streams at most
64 bytes into the existing 68-byte aligned EP0 allocation. It adds no expanded
descriptor cache, transport payload or persistent production state. Every
original descriptor byte is preserved: app/recovery names, UTF-16 strings,
PID selection (including CDC's existing fixed PID), bcdDevice, interface and
endpoint numbering, lengths and CDC/UAC combinations.

The request decision uses copied SETUP words, configuration and build flags.
It returns an action/value word without IO, references, callbacks or retained
pointers. C then publishes state and performs the existing ordered hardware
actions. The port deliberately retains the original low-byte configuration and
interface checks, permissive CDC requests, isochronous halt handling, ignored
SET_CUR frequency payload, deferred SET_ADDRESS and reset/abort behavior.
It does not tighten request validation during the compatibility migration.

The framing functions preserve request clipping, empty transfers, DataEnd and
the extra zero-length packet after a full 64-byte final packet when required.
A legitimate NULL source denotes a generated descriptor; the captured original
SETUP and remaining byte count identify each later chunk. Ordinary short reply
sources remain C pointers. No integer is disguised as a pointer.

## Ownership and retained buffers

In the current app, TIMER5 runs device EP0 and the MIDI poll only outside the
audio render. Its nested render path services only UAC EP4. They do not
interleave SIE indexing. Main-loop host negotiation turns device polling off;
that lifecycle and its physical timing still need the remaining #19 work.

The bridge never forms a Rust reference into EP0 DMA RAM. Its bounded raw
volatile stores do not grant exclusion or claim DMA completion. The existing
controller handshake must prove the previous packet released the buffer before
it is rewritten. Ordinary source pointers, descriptor SETUP capture, packet
publication, stall/SetupEnd transitions, configuration hardware sequences and
class-data reception remain explicit C boundary work. The retained-read test
keeps a foreign root alive until simulated completion and verifies every sent
byte stayed unchanged. A separate Miri case covers a retained raw root and
initializing an uninitialized destination without a temporary mutable slice.

The provider contract also includes USB endpoint/UART RX DMA in boot quiescence;
logical flags, a timeout or a flash lease cannot prove that memory is reusable.
HAL1 remains default off and physical completion/provider behavior unqualified.

## Evidence

Run with Rust 1.94.1:

```sh
sh tests/rust_usb_control.sh ../baseline-2026-11-11/source
```

The script verifies all 440 frozen source hashes. The fixture compiles the real
frozen and current USB driver, replacing only its physical HAL include with a
retained-buffer/SIE double. It compares frozen native C, fresh C0, the real Rust
adapter and both C0/Rust WASM consumers. All eight CDC/UAC/app/recovery variants
match: 67,182 cases each, 537,456 total and 6,742,928 trace bytes in total.

Cases include all 65,536 descriptor selectors, all descriptor data through
actual GET_DESCRIPTOR, request-length clipping and large transfers, standard
and class request/value/interface matrices, line/frequency data reception,
SET_ADDRESS's next-poll commit, busy/idle polls, SentStall, SetupEnd and
abandoned class data. Traces retain exact payload, register access order and
values, buffer identities, state and diagnostics. Class-capability cases run
with all compile-time combinations, including loader strings. Three safe-core
tests cover bounds/invalid-no-write, packet termination and acceptance quirks.

All 163 current Linux workspace tests pass; the HAL suite totals 13 and the
bridge 12. Rustfmt, Clippy with warnings denied and no_std/WASM checks pass.
Strict-provenance Miri passes the retained EP0 destination case (0.48s).
The vendor PI32v2 compiler accepts main/engine C objects with USB1+HAL1. This is
not Rust MCU emission, an image link, target stack/cycle evidence or a device
claim. Per-variant trace hashes, source hashes and retained evidence are in
[usb-control-proof.json](usb-control-proof.json).

Remaining active C includes SIE access/lifecycle and EP0 hardware publication;
MIDI, parameter/DX/OTA SysEx collection and queue ownership/publication; CDC
rings; host control/enumeration/hub/attach/error/loss/restart; UAC producer
resampling, stream/packet policy, retained endpoint ownership and diagnostics.
The class request selector is ported here, not those stream/transport kernels.
Combined hardware MIDI/audio traffic, fixed-master recording, electrical and
DMA behavior, underruns and deadlines remain physical acceptance gates. No
device, flash, live server or remote ticket was changed.
