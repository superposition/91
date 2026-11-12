# UI storage retries

The foreground UI now treats `PLAT_STORE_BUSY` (`-10`) as a retry request.
Settings, voice-bank and MIDI-map loads return before applying missing/corrupt
fallbacks. A pending initialization returns before clearing UI or remote state.
The frame retries reads and initialization before input, snapshots or rendering;
`ui_storage_loading()` also lets external request dispatch refuse edits while
saved state is unknown.

Settings and voice saves acquire the storage transaction before collecting or
encoding data. Denied admission and failed writes re-arm the existing dirty flag;
the last successfully written settings cache changes only after a successful
save. All write errors retain pending edits, including non-Busy errors. This is an
intentional correction to the former silent dirty-flag loss. Existing saved bytes,
object numbers, codecs, source buffers and successful operation ordering remain
unchanged.

A denied genre switch retains its absolute target and replays all engine, mood,
seed, piece and effect actions only after `project_try_genre` succeeds. A set-list
switch acquires admission before changing its position and retains the requested
slot **and its captured entry**, so later slot edits cannot change the deferred
selection. One byte of read/reinit flags and one signed 16-bit action word are the
only persistent retry state; no second storage payload is added.

A queued set-list change also retains its tempo-glide request in a spare bit of
that word. Repeated denial preserves the current glide; admission switches to
the captured slot/entry and starts the new glide from the tempo currently heard.
If playback has stopped while waiting, the piece changes without starting a
glide. An admitted request keeps its original effect order.

A restore retains state 1, its source pointer, object and length while Busy, then
retries the same write. `ui_flush()` returns while that operation remains pending.
`ui_flush_pending()` covers reads, selection retries, queued writes, dirty UI RAM,
project changes and active restore. Callers must defer detach/reinitialization
until the query clears, with the existing successful-restore/store-lock policy
handled by the recovery dispatcher. This query does not itself lease the shared
incoming transfer buffer; the dispatcher owns that lease and serialization.

Admission is balanced and nestable. It makes worker/XIP access safe for storage;
CPU0 audio interrupts can still run. It does not authorize engine reset, reclaim
private DSP spans, or replace the existing stopped-audio reset lifecycle. Display
completion is independent: existing `LCD_READY()` quarantine gates still apply.
No hardware writes or physical timing claims are part of this checkpoint.

Run `sh tests/rust_ui_busy.sh` with the pinned Rust 1.94.1 toolchain. Its native
harness includes the complete current UI, project/session code and native store,
interposing only admission and three IO functions. Both C0 and all-Rust engine
paths verify denied initialization, untouched reads, inhibited input/writeback,
failed saves, restore source retention, complete genre/set-list replay and balanced
admission. A separate 1,024-case test executes the actual detach-guard query.
Normal frozen-oracle audio/framebuffer regression qualification is recorded
separately; this checkpoint does not claim a complete Rust UI controller.
