# Retained USB-MIDI host TX (R18 / #19)

This follows [host selection/RX](USB_HOST.md) and the pinned
[endpoint lifetime correction](USB_ENDPOINTS.md). The active `uh_tx_poll`
ACK/FIFO retry/timeout policy and bounded output-packet traversal now run in
safe `fm1-hal::usb_host::transmit`. The original frozen C0 function remains
selectable. C still observes indexed SIE completion, requests physical DMA and
handles failure/quarantine after an uncertain kick; no hardware/provider claim
is added.

The port preserves the original priority: a stall or 500ms expiry fails before
retry, Error/NAK retries the retained FIFO, busy/FIFO-not-empty waits, and only a
known ACK retires queued slots. Error alone increments its existing 16-attempt
limit. The u8 pre-increment wrap quirk is retained and explicitly witnessed.
Both original clock-read points remain: pending timeout is checked before retry,
and a new timestamp is read after the next packet is filled. A stalled packet
short-circuits before a clock read. Clock and ring-index wrapping stay u32.

An initialized private 8-byte caller packet captures the existing timestamp,
count and error fields. Rust borrows only that disjoint caller storage while
running the policy. The context has seven pointers: the existing output queue,
write/read indices, existing 64-byte endpoint source, clock, publication barrier
and disjoint lease byte. Queue/DMA/clock/lease accesses use bounded raw volatile
roots; no foreign-memory reference survives a call or covers a retained payload.
The lease decision calls the existing safe endpoint policy directly. It does
not reenter C while metadata is borrowed and does not change the pinned endpoint
source.

One FFI call handles an entire host TX poll/packet. A confirmed ACK publishes
before advancing the queue read index; the next bounded packet is copied from
remaining queued words in its original order. Unknown/sealed source retirement
cannot advance the queue or rewrite the source. C publishes returned metadata
before any DMA arm, marks uncertainty before the first possible DMA touch, and
establishes acceptance only after the known kick. Retry leaves the same payload,
slots and data toggle retained. All waits remain bounded status decisions;
there is no new spin or IRQ exclusion interval.

## Storage and actual C object facts

No allocator, writable global, new payload, ring, audio buffer or packet cache is
added. Existing queue-to-endpoint copying is unchanged. The metadata packet is
8 bytes (alignment 4) with explicit initialized reserved bytes. The context is
28 bytes on PI32v2 / 56 on native64. The vendor `-Os` C compiler places it in
28 bytes of immutable `.rodata.uh_tx_poll.context`, alongside the previous 20-byte
RX descriptor. It is not claimed free.

Compared with `509e564`, vendor USB1/HAL0 C `usb_poll` grows 60→68 bytes including
pushed registers, matching the private metadata packet; TIMER5 remains 60 and
`uh_ctl` 40. Main/engine compile with USB1/HAL0 and HAL1 and exact context/state
layout assertions. These are compile-only C object measurements. New Rust
frames, the endpoint policy's placement, LTO, total call-chain peak and target
latency remain separate qualification work. The root's separately measured
endpoint-static optimization is not silently applied to this branch or used to
infer costs of this port.

## Frozen/native/WASM and memory checks

```sh
sh tests/rust_usb_host_tx.sh ../baseline-2026-11-11/source
```

Pinned Rust 1.94.1 and the independently verified 440-file frozen source qualify
the actual `uh_poll`/`uh_tx_poll` consumers, including all pending counts 1..16 and
all 256 CSR bytes, error counts 0/15/255, ages 0/499/500, packet limits 4/64,
ordinary/wrapping clocks and indices, every 0..64 queue depth/packet limit, and
sequential busy/NAK/ACK traffic with producer refills. The trace serializes
metadata, queue/source bytes and register/payload digest. Frozen C, fresh C0,
Rust native and both WASM consumers match exactly:
**158,978 cases / 62,319,376 bytes**, SHA-256
`1d17801c0c5469fa6a03629c68554d5928b971a54de699e7282b79f1ec615483`.
Unknown-completion/retained-source correction reruns in an isolated output folder
and preserves both exact hashes and all refusal/recovery assertions from
`2855811`.

All 178 workspace tests pass; fmt, warnings-denied Clippy and no_std/WASM checks
have retained explicit exit 0 records. Strict-provenance Miri passes all eight
focused host tests: prior uninitialized descriptor output and raw RX roots plus
TX source retention through busy/ACK, unknown-retirement refusal, separate
clock reads, FIFO/error/timeout priority and slot publication/wrapping. Main and
engine objects compile for both HAL configurations. Exact source/log/artifact
hashes are in [usb-host-tx-proof.json](usb-host-tx-proof.json).

## Remaining #19 call map

Host enumeration/control/hub/search and attach/loss/restart traversal remain C.
CDC service traversal and UAC stream/ring/window/resampling/packet policy remain
C, as do device poll/reset/retry orchestration and the thin physical register,
cache and DMA request bindings. Host descriptor/RX and now host TX policy are
active Rust consumers, alongside the previously ported device/EP0/SysEx/queue/
TRS paths.

Actual positive stop/retirement providers, combined physical host/device/MIDI/
UAC/TRS traffic, electrical/cache behavior, fixed-master recording, generated
Rust/whole-link/stack cost and deadlines remain hardware acceptance gates. No
device, flash, live server, remote issue, emitter or main dispatch was changed.
