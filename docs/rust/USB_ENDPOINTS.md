# Retained USB endpoint lifetimes (R18 / #19)

This checkpoint follows [the TX port](USB_TX.md) and [reset epoch correction](USB_RESET.md).
It corrects an existing app-side lifetime assumption: a SIE read returning zero
could mean either a real idle endpoint or an unknown/off/timed-out transaction.
The second case never grants reuse of a retained DMA source. The original C0 and
polled update-loader paths remain selectable and unchanged.

The safe `fm1-hal::usb_endpoint` policy is active in real device EP0, MIDI EP1,
CDC EP3, UAC EP4 and host control/TX completion paths. One scalar bridge entry
returns a bounded decision; it creates no references to foreign DMA memory.
Existing endpoint arrays and host RX/controller roots remain raw C allocations.
There is no allocator, payload copy, new ring, runtime cache or context expansion.

## Observed IO and leases

The app keeps the original bounded 20,000-poll SIE operations and timeout counters,
but records success separately from the byte value. A successful INDEX write
precedes any endpoint observation. Failure before admission changes no payload or
queue index. Before the first possible DMA arm, the source lease becomes uncertain;
a subsequent known kick establishes acceptance. An unknown kick cannot invent it.

Four two-bit phases occupy one existing trailing padding byte in `usb`:

| Phase | Confirmed clear | Confirmed busy | Unknown | Confirmed kick | Uncertain arm | Confirmed stopped |
| --- | --- | --- | --- | --- | --- | --- |
| Ready | Ready, allow | Pending | Ready, deny | Pending | Uncertain | Ready, allow |
| Uncertain | Uncertain, deny | Pending | Uncertain | Pending | Uncertain | Ready, allow |
| Pending | Ready, allow | Pending | Pending | Pending | Pending | Ready, allow |
| Sealed | Sealed, deny | Sealed | Sealed | Sealed | Sealed | Ready, allow |

Sealing is explicit after a failed host control transfer or unknown host TX
operation. Host EP0 retirement is direction-specific: a known TxPktRdy clear or
RxPktRdy set at the expected stage. A fresh zero TX status cannot retire an
uncertain earlier IN request. Ordinary host ACK, retained FIFO retry, toggle,
queue advancement, timeout/error limits and diagnostic stage reporting retain
C0 behavior. No new spin or wait occurs inside an IRQ exclusion interval.

Reset, flush, non-idle EP0 abort, CLEAR_HALT and rebind invalidate acceptance before touching the
controller. Idle phases remain ready; every retained phase becomes sealed. A
reset-cleared busy bit is therefore not mistaken for normal packet completion.
If a device configuration/stream reset encounters retained data, the app stops
admission and defers its session until confirmed whole-controller stop/restart.
A controller restart starts unconfigured. Initial idle configuration remains
unchanged. Bus reset preserves the old source while dropping stale queued data
and advancing the existing reset epoch in its original order.

Device EP0 keeps its pending source and metadata if the initial acknowledgment
is unknown; reused `e0_tx=2` means that acknowledgment must be retried before
copying the first chunk. After any arm, the buffer remains leased until confirmed
retirement rather than replaying an uncertain chunk. SentStall/SetupEnd metadata handling remains in
its original order without writing a leased source; non-idle abort seals that
source before the CSR acknowledgment can clear its busy bit. UAC retains the two existing
alternating packet buffers and previous ring/resampler behavior.

## Whole-controller stop boundary

`USB_DMA_STOPPED()` is a **positive, nonblocking backend fact**, not an off-request
return, a successful MMIO write, a timeout, an observed zero or a logical mode flag.
It may return 1 only when no USB endpoint TX/RX DMA can subsequently touch any old
EP0/MIDI/CDC/UAC/host source or destination root. No Rust reference is used to claim
that fact. The physical binding currently returns 0 by default. USB1 app startup,
retry, role rebind and host fault recovery are therefore deliberately unqualified
and deferred until an actual provider is integrated and qualified. Loader/C0
startup is unaffected. Already active healthy traffic progresses using confirmed
busy/free observations and does not need the stop provider.

Role changes stop admission (`usb.up/config`, `uh.run/out`) and make a bounded off
request first. While stop remains unknown, old mode identity, queue ownership,
controller roots, pending source bytes and requested role remain retained; no
attach, endpoint rebind, session reset or queue discard follows. A later positive
stop fact permits retirement and the existing attach/reset sequence. This is a
software/provider contract, not physical USB stop evidence.

The current CPU0 ownership contract remains: TIMER5 owns active indexed SIE
access; foreground host control runs only with device polling and host data
polling disabled. Nested audio recording uses only EP4 and its existing ring
consumer, with no simultaneous `usb_poll` reentry. Volatile accesses do not
invent cross-core exclusion or DMA completion.

## Layout and measured cost

Actual native64 and vendor32 compilation prove:

| Layout | Before | After |
| --- | --- | --- |
| `usb` sizeof, native64 | 104 B | 104 B |
| telemetry offset, native64 | 98 | 98 |
| lease offset, native64 | padding | 99 |
| `usb` sizeof, PI32v2 | 96 B | 96 B |
| telemetry offset, PI32v2 | 94 | 94 |
| lease offset, PI32v2 | padding | 95 |

The policy adds one logical byte inside existing padding: **zero allocated BSS or
payload growth**. The immutable decision table is 32 B; no expanded cache exists.
The existing TX context stays 64 B on PI32v2. Scalar ABI alone does not prove a
zero-cost call or unchanged stack depth.

Vendor object prologues (same `-Os` / strict-FP main build, USB1/HAL0, before at
`3fff50d`) show these C frames, including pushed registers:

| Function | Before | After |
| --- | --- | --- |
| `e0_chunk` | 36 B | 36 B |
| `fm1_timer5_irq` | 60 B | 60 B |
| `usb_poll` | 56 B | 60 B |
| `uac_service` | 40 B | 48 B |
| `uh_ctl` | 32 B | 40 B |
| `uh_csr0` | 20 B | 28 B |
| `usb_start`, opaque test stop provider | 16 B | 20 B |

The small register-save increases retain observed status and lease decisions
across the scalar policy call. They add no payload temporary. An unresolved,
compile-only opaque stop provider preserves the role/attach code for inspection;
the real default-deny binding may optimize it away. These are object-frame
measurements, not LTO call-chain peaks, measured target stack use or deadline
qualification. The combined integration separately measures the actual Rust
leaf: moving its immutable table to read-only storage reduces the isolated
frame from 48 to 8 B. [The inspection context and before/after evidence](USB_ENDPOINT_LEAF.md)
do not replace a complete firmware call-chain bound.

## Reproduction and evidence

Pinned Rust 1.94.1, the independently hashed 440-file frozen source, fresh C0 and
real current Rust-enabled C consumers are used:

```sh
sh tests/rust_usb_lease.sh ../baseline-2026-11-11/source
sh tests/rust_usb_lease.sh ../baseline-2026-11-11/source healthy
sh tests/rust_usb_control.sh ../baseline-2026-11-11/source
sh tests/rust_usb_reset.sh ../baseline-2026-11-11/source
sh tests/rust_usb_tx.sh ../baseline-2026-11-11/source
```

The retained-DMA double injects both accepted and unaccepted failures at every
INDEX/read/arm/kick position, off SIE, fresh-zero after unknown arm, late busy/free,
EP0 chunk continuation, reset/flush/reconfiguration invalidation, complete host
IN/OUT/status stages, uncertain host ACK,
RX-only and bidirectional role changes, repeated unconfirmed off, and eventual
independent positive stop. Checks remain active under `-DNDEBUG`.

Unmodified frozen and fresh C0 match: 272 snapshots / 29,376 B, witnessing
78 retained-source violation observations across 92 snapshots. Corrected Rust has 266
snapshots / 28,728 B and zero violations; refused transitions intentionally remove
some old branches. Each matches its own native/WASM trace, without a tolerance.
Healthy real host/control/device traffic matches frozen/fresh C0/Rust native
and both WASM consumers exactly:
29,330 snapshots / 3,167,640 B, including register-order/payload digest, queues,
diagnostics and IRQ state, with the stop provider false.

All eight normal descriptor/control profiles remain frozen-exact: 537,428 cases.
The seven non-idle app SentStall/SetupEnd cases per feature profile now use the
separate retained-source correction oracle above (including eventual stop and
recovery); idle aborts and every loader case retain C0 comparison coverage.
No scenario is silently tolerated or dropped. Repeated retained-read/rearm
violations are observations, not a count of independent physical events. Reset correction retains all 3,090 frozen stale-tail witnesses;
its updated Rust trace additionally refuses sessions whose reset invalidated
retained DMA, with the fresh 7,718,084-B hash recorded in the proof. This extends
the earlier reset-only result intentionally, without changing frozen C.
Healthy TX plus every loader case remains frozen-exact:
277,484 snapshots / 39,903,824 B. All 170 Windows workspace tests pass; fmt,
Clippy with warnings denied and no_std/WASM checks pass. Strict-provenance Miri
passes both the retained-TX-root/reentrant-consumer test and exhaustive lease-byte
independence/transition test. Vendor PI32v2 main/engine objects compile for USB1
with both HAL0 and HAL1, including fixed offset assertions. Detailed source/log/
artifact hashes are in [usb-endpoints-proof.json](usb-endpoints-proof.json).

## Remaining #19 call map and acceptance

This is an endpoint lifetime correction, not completion of #19. Host descriptor
parsing, enumeration/hub/search and lifecycle traversal, host packet admission/
ACK/retry policy, CDC service traversal and UAC ring/resampling/packet/stream
policy still have active C bodies. Device polling/retry/reset orchestration and
the bounded physical SIE/MMIO/DMA bindings also remain C. The already ported TRS,
SysEx/RX, MIDI queues, device descriptors/control and TX stay active Rust seams.

Software attach/loss/restart policy coverage will expand with those ports.
Actual USB controller stop/retirement, endpoint and UART RX DMA/cache behavior,
combined MIDI/UAC/control traffic, fixed-master recording, electrical behavior,
RAM placement, full generated stack/link cost, underruns and target deadlines
remain physical acceptance gates. No device, image, live server or remote issue
was changed. This work makes no claim about the prior device crash.
