# USB host control transfers (R18 / #19)

This follows [host RX selection](USB_HOST.md) and [retained TX](USB_HOST_TX.md).
The complete active `uh_ctl` transfer and CSR wait/kick policy now run in safe
`fm1-hal::usb_host_control`: SETUP, IN packet traversal, short-packet handling,
status stages, clock/timeout decisions and diagnostic publication. The original
C0 bodies remain selectable. C keeps thin observed-register, DMA, cache-sync and
watchdog bindings; enumeration/hub/application orchestration is still C.

A private 12-byte request holds address/type/request/MPS and four u16 fields,
including initialized reserved bytes. Eight SETUP bytes are written directly to
the existing raw EP0 root; no temporary setup payload array is added. Incoming
known-ready packets are cache-synced, clipped to 64 bytes and copied only into the
requested completed output prefix (up to the existing 768-byte configuration
limit). The bridge never forms a reference to DMA or an uninitialized C output.
The only borrowed inputs are initialized immutable CPU-owned request/context
packets; no address or foreign-memory reference survives a call.

Callbacks are disjoint controller/cache/DMA/watchdog leaves. No application,
engine, UI or view reentry is allowed while those private request inputs exist.
The foreground owner disables active device/host data polling, keeps IRQs enabled
and requires its existing clock to progress. The port introduces no IRQ mask or
new wait. The original strict `elapsed > 500ms` timeout decision stays after a
failed readiness check, so a just-ready packet retains the same priority; this
is not a new hard wall-clock deadline guarantee.

Unknown read is distinct from known zero. Uncertain arm, known kick and
direction-specific completion use the existing endpoint policy. Every failure
seals the source until an independent positive stop, rather than replaying an
uncertain SETUP or accepting a later fresh zero. Initial/data errors update the
existing diagnostic byte; the legacy status-stage diagnostic quirk remains.
Partial initialized output already copied before a status failure is preserved,
and its untouched tail is never read or initialized by the adapter.

## Fixed storage and measured C framing

No allocator, writable global, payload buffer, enlarged ring or runtime cache is
added. The request is 12 bytes/ alignment 2; the context has nine pointers and is
36 bytes on PI32v2 / 72 on native64. Vendor `-Os` places the immutable descriptor
in `.rodata.uh_ctl.context` (36 bytes). Address-taking the existing physical
leaves also exposes 14B `fm1_usb_ep0_send` and 4B `fm1_usb_rx_sync` bodies in the C
object; this is not a whole-image size claim.

The small C wrapper deliberately stays `noinline`. The first object inspection
showed automatic inlining reserve many distinct foreign-request slots and grow
`uh_probe` to 188 B. Keeping the request's lifetime in one C frame avoids that
regression. Compared with `d5eef08`, final USB1/HAL0 C frames including saved
registers are:

| Function | Before | After |
| --- | ---: | ---: |
| `uh_ctl` |40B |16B (return4B + request12B) |
| `uh_probe` |80B |88B |
| `uh_hub_port_probe` |60B |72B |
| `uh_hub_status` |44B |44B |
| `usb_poll` |68B |68B |
| `fm1_timer5_irq` |60B |60B |

These are object facts, not total stack savings: the Rust control/wait and
indirect leaf frames still need adopted-compiler/LTO/full-chain measurement.
Both USB1/HAL0 and HAL1 main/engine objects compile with exact request/context
and register-map assertions. The physical provider remains default-unqualified;
no candidate firmware was linked or run by this work.

## Independent qualification

```sh
sh tests/rust_usb_host_control.sh ../baseline-2026-11-11/source
```

Pinned Rust 1.94.1 and the independent 440-file frozen source exercise the actual
`uh_ctl` consumers with scripted controller observations. Every 0..768 requested
length, each 8/16/32/64 MPS, count 0/1/7/8/9/63/64/65/255, ordinary/wrapping clocks,
delayed readiness at 0/1/499/500/501/502 polls, stalls/no-answer/both/timeout at
every stage, and IN/OUT requests match frozen C, fresh C0, Rust native and both
WASM consumers exactly: **58,248 cases / 52,190,208 bytes**, SHA-256
`239a59ad9b8f1a01f29ab1ffa9d61fe403f7c798790aaafd58f334a67cd07102`.
The final `noinline` wrapper was rerun through that full matrix. The separate
unknown-completion/reset/abort/stop regression retains both exact hashes from
`2855811`; no corrective case is hidden in the normal parity comparison.

All 182 workspace tests pass, with explicit exit 0 records for fmt, warnings-denied
Clippy and no_std/WASM checks. Four focused strict-provenance Miri tests pass,
including a genuinely uninitialized eight-byte C destination with only its
three completed bytes exposed after return, raw retained DMA/callback roots,
wrapping timeout/stop refusal and status diagnostics. Full logs, source/object/
trace hashes and before/after/inlined C prologues are retained in
[usb-host-control-proof.json](usb-host-control-proof.json).

## Remaining #19 boundaries

Host enumeration/hub/search and attach/loss/restart traversal, its timed wait,
endpoint activation/register setup and device poll/reset/retry orchestration
remain active C. CDC service and UAC stream/ring/window/resampling/packet policy
remain C. Host control/RX/TX, descriptors/EP0, SysEx, queue and TRS policies are
now active Rust consumers. Thin physical MMIO/cache/DMA bindings remain audited
C seams.

Actual positive stop providers, combined device/host/control/UAC/TRS traffic,
cache/electrical/clock behavior, fixed-master recording, complete generated
link/stack cost and deadlines remain physical acceptance gates. No device,
flash, live server, remote issue, emitter or main dispatch was changed.
