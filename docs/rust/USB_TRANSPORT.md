# USB-MIDI receive and SysEx collectors

`EN_RUST_USB=1` now runs the actual device EP1 receive traversal and all three
SysEx collectors through safe, no_std `fm1-hal::usb_transport`. This checkpoint
continues #19 after TRS (`99766ff`), EP0 (`5dedfb1`) and the explicit retained
message-length correction (`80f65c6`). The complete ticket remains open.

The core handles cable/CIN admission, complete four-byte event traversal,
realtime filtering, parameter/backup commands, independent short-command
collection, DX/Jam/backup message collection and OTA/telemetry collection. It
preserves the original capacities, overflow/drop rules, status-byte treatment,
restart/termination behavior, queue limits and publication order. DX type
classification uses the shared `fm1-formats` codec through a bounded byte
accessor; only header indexes 0..4 are visited. No message/header copy is added.
The normal slice-based classifier still uses the same implementation.

## Storage and call boundary

All payloads and scalar placements stay in their existing C allocations. There
is no allocator, expanded cache, new persistent state or duplicate payload.
The real EP1 path makes one Rust call per captured packet (at most 64 bytes),
not one FFI/state-packet copy per byte. A direct single-byte wrapper remains
for existing consumers/tests; it is not used by the active Rust EP1 traversal.

A scoped caller-stack descriptor contains build flags, 21 raw roots and the
existing no-reentry C publication-barrier callback: 92 bytes on PI32v2 and 184
on the 64-bit native test host. C and Rust assert this ABI. This is bounded
transitional stack/indirection cost; target emission, image size, stack peak
and timing have not been qualified for this checkpoint.

| Root slots | Existing storage |
| --- | --- |
|0..1|16-byte short command payload; five adjacent length/on/request bytes|
|2..7|16-byte parameter payload, position/on,32-event queue and indexes|
|8..12|4,112-byte DX payload, position/on/ready/kind|
|13..17|640-byte OTA payload, position/collect/ready/published length|
|18..20|64-event MIDI input queue and indexes|

Optional feature roots are NULL and never accessed when disabled. The adapter
validates mandatory roots, scalar alignment and input bounds before mutation.
It independently bounds every raw payload access and wraps queue indexes with
the same unsigned arithmetic as C. No endpoint, message, queue or scalar Rust
reference is formed; only the immutable local descriptor is borrowed. No
pointer survives the synchronous call. The publication callback is a leaf
compiler/device-boundary binding and must not re-enter the parser or retain
its descriptor. The original C publication barrier is preserved; volatile
access does not invent atomicity or general multi-threaded queue ownership.

The current CPU0 poll serializes receive/TRS producers outside render. Main
consumers may release only completed payloads and consumed queue slots. Busy
DX/Jam storage retains both length and payload until dispatch releases it.
F0 restarts independent short commands without touching that lease. OTA keeps
its separately published frame length while a subsequent attempted collection
resets the private position, matching C. Controller RX synchronization and
packet release, lifecycle and physical DMA facts remain explicit C seams.

## Frozen and integration evidence

Pinned Rust 1.94.1 commands:

```sh
sh tests/rust_usb_transport.sh ../baseline-2026-11-11/source
sh tests/rust_usb_transport.sh ../baseline-2026-11-11/source edge
sh tests/rust_usb_busy_dispatch.sh ../baseline-2026-11-11/source
```

The driver fixture verifies all 440 immutable source hashes. It compiles the
actual frozen/current `usb.c` with only its HAL include substituted. The frozen
receive oracle applies exactly the named ready-length correction; its separate
unmodified-frozen witness proves the old F0 bug before that correction is used.
No other frozen policy is changed.

Across DX/OTA off/on combinations, frozen-plus-correction native C, fresh C0,
the real Rust adapter and both C0/Rust WASM consumers match byte for byte.
The full run has 59,981 snapshots per variant (239,924 total), 245,135,872 trace
bytes, 238,126 direct SysEx bytes and 4,177 EP1 packets per variant. It exercises
all 65,536 cable/CIN/status pairs, every byte mutation of 12 command forms,
short/truncated/oversized packets, complete and full queues, index wrap,
repeated delimiters, busy collectors, overlong bodies and exact16/640/4,112
capacity edges. Traces include every semantic field, complete queue contents,
ordered register events, periodic full-payload hashes and full payload bytes
at message boundaries. Hashes are additional interim checks, not a claim that
each byte is serialized after every individual input byte.

The edge run adds 579 snapshots per variant (2,316 total/1,078,704 bytes), with
explicit successful bulk, single, slot, clear, backup and Jam classification,
then incoming F0/F7 while each payload remains ready. Every ready payload and
its exact length stays unchanged. The actual foreground Busy retry fixture
passes with both corrected C and the Rust collector+format codec: it retains
a denied Jam request through incoming F0/F7, retries while still Busy and
succeeds once admission opens. `main_fm1.c` is unchanged by this checkpoint.

All 165 Linux workspace tests pass. Clippy with warnings denied, formatting and
no_std/WASM checks pass. Strict-provenance Miri passes retained external roots,
pending message lifetime, input/slot bounds and invalid-context no-mutation
checks (0.58s). The safe shared-header test verifies no accessor visits payload
or an index beyond the actual message length. Vendor PI32v2 main/engine C
objects compile with USB1+HAL1, including the 92-byte context assertions; this
is not Rust MCU emission, whole-image link or physical evidence. Details and
source/trace/artifact hashes are in [usb-transport-proof.json](usb-transport-proof.json).

Remaining active C includes SIE access/EP0 hardware publication and device
lifecycle, output/CDC queues and ordering, OTA SysEx transmit arbitration,
foreground message consumers, host enumeration/hub/control/attach/loss/restart
and UAC producer resampling/stream/packet ownership. The input/parameter queue
admission policy moved here; a portable atomic-ring claim is not made. The
next output boundary must also preserve the incoming IRQ state independently
of HAL selection. Physical combined MIDI/audio, fixed-master recording, DMA
completion, attach/loss/restart, electrical behavior and deadlines remain
acceptance gates. No device, firmware image, live server or remote issue changed.
