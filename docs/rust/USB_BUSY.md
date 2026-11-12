# Retained USB message length

A complete DX/backup/Jam message waiting in `dx_state==2` owns its existing
payload and `dx_pos` length until foreground dispatch releases it. Incoming
F0 now resets the length only when a new collection is accepted. The separate
short-command collector still restarts, and the busy payload is never touched.
This adds no state, payload or allocation.

This is an intentional correction to the frozen C behavior. Previously every
F0 set `dx_pos=0` even while the payload remained busy. The Busy-aware main
request dispatcher now retains transiently denied requests, so losing their
length could corrupt the retry contract. Ordinary accepted-message bytes,
classification, queue behavior and dispatch remain unchanged.

`sh tests/rust_usb_busy_dispatch.sh ../baseline-2026-11-11/source` verifies all
440 frozen source hashes and compiles the actual SysEx collectors together with
the current actual foreground `dx_service`, backup/Jam request dispatcher and
real engine/host state. It first witnesses the frozen zero-length quirk (then
repairs only that test continuation). The corrected path receives F0/F7 while
a Jam request is denied, preserves all eight pending bytes and length, retries
while still Busy, and succeeds once admission is available. The existing full
Busy backup/read/save/restore/detach assertions run in both variants too.

Both native runs passed on the pinned software base `d12ccfe` merged with the
TRS and EP0 checkpoints. This is a software lifetime fix, not physical USB,
DMA, timing or target-image qualification. No device or firmware image was
changed. The upcoming Rust collector must preserve this corrected invariant
and distinguish this one intentional difference in the frozen oracle.
