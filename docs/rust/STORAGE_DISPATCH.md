# Foreground requests during transient flash admission

`PLAT_STORE_BUSY` means the worker cannot yet yield flash access. It does not
mean that an object is absent, corrupt or permanently unwritable. The foreground
dispatchers retain the existing request and retry on a later loop turn.

- A backup read uses an impossible length in its existing read state until the
  store lease is admitted. It sends no empty-object response on Busy. Once read,
  its existing buffer remains a stable snapshot under USB backpressure.
- Stored Songbook selection remains queued. A Jam read/save request stays in
  the existing DX request buffer when a transient denial returns internal zero.
  Existing wire Busy statuses and successful response bytes are unchanged.
- A Jam save holds one balanced outer lease over write, readback verification
  and session commit. Every denial/error releases ownership and the lease.
- A valid restore waits at its occupied queue head until pending UI saves have
  finished. One scalar leases the shared backup buffer during that wait; later
  transfers cannot overwrite it. The consumer index is never rolled backward.
- Normal updater/ROM transitions stop playback and enqueue panic once, then
  wait for `ui_flush_pending()` to clear. Reinitialization/detachment cannot
  discard deferred edits. Safe Mode bypasses the application flush. A completed
  explicit restore also bypasses old-RAM writeback after the store is locked.

This checkpoint requires the UI Busy retry implementation and project/session
admission contract when integrated. Its dispatch test substitutes only the UI
pending/flush observations and terminal USB/ROM actions; the separate actual UI
denial test qualifies dirty-flag/read/restore retries. The existing backup,
Songbook and DX payload buffers are reused; one byte is added for the deferred
restore lease. Updater request values reuse their existing byte flags.

Validation:

```sh
sh tests/rust_storage_dispatch.sh
sh tests/rust_device_wire.sh /path/to/frozen/source /path/to/qualified.fwsc
```

The dispatch gate compiles the actual foreground functions and Jam protocol
with fake store admission, both C and Rust codecs. It exercises admission and
backend Busy, retained reads, exact snapshot frames, queue-head preservation,
balanced saves, successful restore locking, deferred detach/update, Safe Mode
and post-restore transitions. The device-wire gate retains the frozen/current/
Rust frame hash and ordinary Jam save/readback behavior. Vendor PI32v2 C units
also compile; this does not qualify physical timing, device USB or flash writes.
