# Installer and diagnostic policy (#22 checkpoint)

`tools/fm1_install.py`, `tools/fm1log.py` and `tools/fm1sum.py` now call
`fm1-tooling` for their installer transactions and diagnostic decisions. The
existing Python command lines, fake-device APIs and OS MIDI/file adapters remain
usable. This checkpoint does not close all of R21 / issue #22.

## Executed boundary

| Existing entry point | Maintained Rust behavior | Thin Python behavior |
| --- | --- | --- |
| `fm1_install.py` | Identity/model validation; Windows port pairing; safe discovery candidates; short/full handshake fallback; package identity/loader-marker policy; request bounds; finish acknowledgements; check/write/loader/restart/verification; original timeout/error/progress order | `argparse`, package file read, confirmation/presentation, backend/link methods, clock/sleep, caller callbacks |
| `fm1log.py` | Four compatible frame profiles and old short-frame defaults; bit fields; port choice; keepalive threshold and polling constants; benchmark arithmetic/format/error prefixes; event order, CSV columns/row and live display | MIDI reads/writes, wall-clock observations, CSV sink, terminal output and interrupt cleanup |
| `fm1sum.py` | Stable peak selection; grouping and cross-group counter deltas; timing histograms; fixed-cost equations; diagnostic stages; all five report sections | CSV input and printing |

The native CLI also has useful offline commands:

```text
fm1-rs tools package-info PACKAGE.fwsc [--force]
fm1-rs tools decode 70 RAW-SYSEX-DATA
fm1-rs tools summary TELEMETRY.csv
```

`decode` takes the SysEx data bytes without F0/F7. Frame kinds are decimal 70
(status), 90 (extra), 91 (diagnostic), 71 (benchmark). Native summaries use the same
platform newline bytes as the existing Python report command.

## State and I/O

The installer follows this call graph:

```text
find -> handshake -> [check -> wait_for(loader)] -> write
     -> wait_for(running) -> verify exact identity
```

An already running loader skips check. All of these methods execute in Rust;
they are not generated continuations around the old Python implementation.
`Updater<'_, A>` borrows a synchronous `Adapter`. It carries the selected port
and delay observations. A transaction decodes its byte image once, and `serve`
borrows that image while answering bounded 512-byte requests. Existing package,
seven-bit, OTA and telemetry byte codecs come from `fm1-formats` (#6).

The adapter exchanges copied JSON observations over a length-prefixed stream.
Rust keeps its transaction stack between requests; it does not resend a whole
package with each read continuation. The initial JSON representation and the
decoded image coexist for the transaction. Python holds a per-call list of OS
link objects; Rust sees integer handles. No Rust reference names a Python/OS
object, and Rust retains no link or package after the call returns.

Adapter actions are inputs/outputs/open, send/read/drain/lost/close, clock/sleep,
progress and the caller's identity predicate. A predicate may call pure helpers:
an independent offline Rust stack services that query while the transaction
waits. Mutable transaction reentry is rejected. Exceptions unwind the current
call; the framed stream remains usable after a normal policy failure or a
callback interrupt. A broken/truncated process stream is discarded and can be
started again. No hidden retry repeats a write transaction.

This preserves existing details such as acknowledging either finish address
before checking the expected phase, ignoring unrelated frames, checking idle
expiry only after an empty read, and recording progress after a successful send.
This checkpoint does not redesign those protocol choices or OS driver behavior.

Telemetry rows use the shared field arrays and preserve event order. Reports
group with a `BTreeMap`; stable sorting retains input order for equal peaks.
Integer sums stay integer until the original floating-point equations require a
division. Benchmark zero divisors preserve the previously printed prefix and
the Python exception message.

## Bounds and qualification

The std host crate forbids unsafe code. A framed request/response is at most 32 MiB;
byte inputs at most 1,100,000 bytes; a served request at most 512 bytes. Native CSV
input is at most 32 MiB,100,000 rows and 256 columns; cells are bounded at 1 MiB.
Report integers use an explicit signed 2^40 bound, wider than the u32 live words,
so all accepted sums/deltas fit i64. Time observations must be finite. Out-of-bound
host inputs are rejected explicitly. The native CSV reader validates row shape
and quotes; the retained Python API still uses Python's CSV reader.

The proof runner verifies all 440 immutable R01 source hashes, then compares
2,739 cases. This includes 64 actual virtual-clock transactions, complete sent
packets/backend calls/progress order and elapsed virtual time; malformed and
legacy frames; 100 frozen live-row/display executions; benchmark partial-output
errors; synthetic packages; native command outputs; interrupt/reentry and
malformed-stream recovery. The existing threaded fake installer suite and
logger selftest also pass. Linux and Windows identities and exact artifact
hashes are recorded in `installer-diagnostics-proof.json`.

```sh
bash tests/rust_tooling.sh /absolute/path/to/immutable/R01/source
```

The Linux gate uses Python 3.10 stdlib and Rust 1.94.1. The CLI dependency graph
keeps the native simulator C-app feature disabled. The gate needs no third-party
Python package and never imports a MIDI backend. Windows is checked separately
with Python 3.8.3.
151 workspace tests, formatting and strict Clippy pass on the isolated source.
The selected browser-codec dependency graph contains no `fm1-tooling` or authoring
crate. No firmware, hardware, server, public export or browser code was changed.

## Remaining issue #22 work

The owned build/release/SDK/memory/disassembly gates; LZ4/package construction;
ROM rescue and diagnostic debugger transaction policy; install/rollback physical
delivery facts; toolchain acquisition/launchers; and font/pattern/table/logo,
asset/site/manual/document generators still need their own reviewed ports.
Existing byte-codec migrations do not make those orchestration paths complete.
The original pinned stub and vendor SDK remain unchanged.

No port was opened and no device was read, reset or flashed for this checkpoint.
Fake-device equality is host-policy evidence. Actual driver delivery, USB
enumeration, audio timing and a physical restore/install remain separate facts.
