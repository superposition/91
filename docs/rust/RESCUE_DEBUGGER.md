# Rescue and debugger policy (#22 checkpoint)

The existing `fm1_rescue.py` and `fm1_debug.py` commands now execute their recovery
and diagnostic policy in safe Rust. Python binds the actual USB/MIDI libraries,
file/network I/O, clocks, terminal and thread/Event primitives. No physical
device was accessed for this checkpoint.

## Recovery boundary

The live CLI uses Rust for the exact V15 package and RAM-loader hash pins,
package extraction/size validation, UID requirement, ROM capture polling and
retry/refusal decisions, BOT framing/status checks, Uboot commands, backup and
restoration decisions, confirmation keepalive and final verification.

```text
verify pinned files -> capture 4C4A:8057 -> inquiry WL82 -> start RAM loader
 -> check key/type/flash ID -> read and save 1 MiB backup
 -> compare protected head -> select changed firmware sectors
 -> optional confirmation with keepalive -> write high addresses first
 -> verify each sector -> compare complete expected flash -> run app
```

The Python API and CLI arguments remain available. `--write` and `--ask` retain
their existing behavior. Check mode still loads the RAM helper and saves a full
backup; it does not write firmware sectors. A head difference refuses writing
after saving the backup. Restore accepts the existing flash IDs `856014` and
`B36014`, key `980F`, type 3. Only changed sectors in `[0x4000,0x93000)` enter the
restoration write plan. The first `0x4000` bytes and everything from `0x93000`
onward are preserved. These are the original flash boundaries; the app's CRSTUB
sector is inside the writable region.

The existing three attempts for a `Fail` during a sector write, whole-session
retry rules, refusal substring rules, timestamped backup names, progress and
error messages are preserved. A successful check returns success even when no
flash write was requested. Low-level `Uboot`/`Bot` protocol methods remain
compatibility interfaces; restoration policy is enforced by the sector/restore
methods and the CLI's pinned inputs.

BOT keeps the original tag increment/wrap order, command/data/status order,
13-byte status signature/tag/status checks and ignored status residue. The CBW
is 31 bytes. Generic CDBs are 16 bytes, with big-endian command/address/length
fields and the original little-endian CRC16 field. Inquiry's 6-byte CDB retains
zero padding in the CBW. Read/program fragments are 512 bytes.

Two explicit bounds stop an empty or oversized flash fragment. The original
empty-fragment read loop made no progress; the reference double limits that
loop to record its failure fingerprint. The new refusal occurs after the first
invalid fragment and performs no flash write. Ordinary short, positive fragments
still accumulate in the original order. Host frames/byte buffers retain the
existing 32 MiB/1,100,000-byte limits; recovery attempts are bounded at 1,000.

## State and callback lifetimes

`RecoveryOptions` carries the borrowed backup directory, write/confirm intent,
attempt count and wait duration. `Device<'_, A>` borrows a synchronous `Adapter`.
It handles either the real BOT binding or an existing duck-typed device API.
Restoration holds byte vectors for the desired image and full snapshot, and a
sorted sector vector: 147 sectors can differ, of which 143 are writable. A read
fragment borrows the current image slice; a complete image is not serialized
with every 512-byte command. Backup/readback payloads cross the pipe as copied
JSON byte arrays, so this is a bounded std host tool rather than an MCU path.

Each real `Bot` creates its command lock before acquiring the USB configuration.
The lock covers the tag change, CBW, data and status transaction. Rust retains
no USB object or Python reference. Recoverable Python exception classes are
classified through their MRO; Rust scopes which classes it handles. An exception
consumed by a retry cannot leak into a later successful return. Cancellation and
other `BaseException` subclasses leave capture immediately.

During confirmation, an independent Rust process runs the keepalive wait/command
policy. It uses the same serialized Bot binding while the foreground Rust
transaction waits. It cannot borrow the foreground's image or mutable Rust
state. Python sets the stop Event and joins the job before writing resumes.

Hashing uses pinned RustCrypto `sha2=0.10.9` with the software backend. It and its
locked dependencies are host-only; the selected WASM-codec and MCU-bridge graphs
exclude tooling/SHA2. The project crate forbids unsafe code. The recovery stub,
SDK, firmware, controller and compiler/emitter sources were not changed.

## Debugger boundary

Rust selects the original case-sensitive `FM-1`/`X0X` output name, drains input,
sends the retained packet, filters unrelated replies and polls against the
original wall-clock deadline. Python opens/closes the Mido contexts and supplies
message/clock observations. Peek/poke/clock request plans and reports are Rust.
Truncated reports preserve their already printed prefix. Zero clock ticks retain
the original division error instead of producing nonfinite text.

Malformed debugger words retain #6's shared `CodecError` profile; raw R01 Python
raised `IndexError` for a truncated word. That inherited byte-profile difference
is explicitly tested using the frozen presentation and retained shared codec.
Normal packets/reports are compared directly with R01. This checkpoint does not
change firmware register permissions or broaden the debugger port filter.

The native CLI adds two offline commands:

```text
fm1-rs tools rescue-plan V15-PACKAGE.fwsc FLASH-SNAPSHOT.bin
fm1-rs tools debug-plan peek|poke|clock A [B]
```

They produce a plan and perform no device I/O. Existing Python commands remain
the OS bindings for actual USB/MIDI use. The rescue adapter retains its existing
macOS/PyUSB prerequisites; Windows virtual proofs do not qualify a Windows ROM
driver route or physical loader timing.

## Evidence and remaining scope

`bash tests/rust_rescue.sh FROZEN_R01_SOURCE` runs with Python stdlib and pinned
Rust 1.94.1. Fake transports replace USB/MIDI and no RAM-loader download is needed.
Both Linux/Python 3.10.12 and Windows/Python 3.8.3 pass 626 new cases, including the
actual cached V15 hash/image, all 256 sector addresses, eight program interruption
points followed by repair, a complete BOT-backed 1 MiB backup/restore/verify,
capture/driver/reset faults, full main retry paths and keepalive loss/cancellation.
Two further invalid-fragment refusals are recorded separately. All 440 frozen
source hashes and all 2,739 earlier installer/telemetry cases pass. 156 workspace
tests, formatting and strict Clippy pass. Exact artifacts and platform-specific
trace hashes are recorded in `rescue-debugger-proof.json`; native path separators
account for platform-specific report hashes.

Actual libusb/driver delivery, vendor RAM-loader execution and physical timing
are separate from this host proof. The current device was not read, reset or
flashed. The bootstrap shell script, build/package/LZ4/SDK gates, acquisition and
asset/site/manual generators remain issue #22 work. This checkpoint does not
close the whole ticket.
