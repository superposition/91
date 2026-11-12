# Connected-tool metadata and capability checks

[W06 (#34)](https://github.com/superposition/91/issues/34) gives the installer, editor and pieces page one product/instrument description and one connection/error vocabulary. Normal page copy stays focused on sounds, saved pieces and controls.

`web/product.json` defines the display name, source URL, supported installer languages and default published release. `tools/make_params.py` compiles `tests/host/gparams.c` against the engine, reads the UI/algorithm tables, and writes the existing `build/params.json` plus the tracked `web/device-data.js`. The generated data includes genre names, parameter count, DX user-voice base/slot count, backup object IDs and transfer limits from the actual C definitions. Regenerate it with `python3 tools/make_params.py`; `--check` fails if the tracked metadata differs from the current instrument/product tables. A native C compiler is required.

`tools/web_metadata.py` embeds those data and the shared `web/fm1device.js` helpers into each connected page. `DEVICE_RELEASE` comes from the builder's validated explicit release/package values. The `product.json` release is a default; it cannot override an explicit build version. Installer download/version text and each page's embedded release object use the same supplied values. Capabilities never follow a release-number comparison.

The shared client opens matching MIDI ports, handles connection loss, cancels pending requests, prevents simultaneous read/write operations and releases ports it opened. It distinguishes an insecure-origin connection request from a browser without Web MIDI. These errors appear when a connection is requested; LAN browser playback remains available. Installer statuses/errors support English and Japanese; the editor and pieces page remain English. All common error keys have both installer translations.

The device's about response reports parameter/genre counts and a protocol revision. Revisions 2 and 3 advertise the existing piece-extra and bass-step layouts. It does **not** advertise MAP2 support, so a failed settings-map write reports what was refused rather than inventing a minimum NAIWAN release. Songbook backup objects remain behind their separate positive capability probe. Empty data, a malformed import, no response and a disconnected device have distinct paths.

The editor verifies a complete parameter response before enabling connected actions, bounds controls to the peer's reported parameter count and validates voice response lengths/checksums/slot numbers. Its former `12 + 22` DX offset selected factory voices and overwrote factory labels after the instrument grew to 30 built-in FM voices. The generated current offset is **42**; user slots remain **0–15**. Tests execute both name refresh and the actual parameter packet, preserving factory entries **34–41**. A voice-read timeout now reports missing support instead of claiming an empty voice bank.

Offline edits never queue a device write. Reconnect clears pending edits/timers, and `flush` independently requires readiness and peer bounds. Discrete voice reads and writes share the operation lock; an active live voice write still coalesces subsequent edits into the latest voice. Tests run the actual editor functions and read/tab bindings to cover these races.

No protocol framing or payload layout changes: object IDs, P1BK, MAP1/MAP2, HYP3/VOX1/PCX1, SysEx numbers, loader/recovery markers and existing numeric parameters remain compatible. Frozen vectors from the pre-change code verify identical representative backup and map bytes. The OTA implementation and stock-image checksums are unchanged; only connection preflight and operation locking wrap those actions.

Source-capable qualification:

```sh
python3 tools/make_params.py --check
node tests/web_metadata_test.mjs [build/site-check] [explicit-version]
node tests/web_backup_test.js
node tests/web_piece_test.js
python3 tests/web_site_test.py build/site-check build/emu
```

The metadata test uses API doubles and real protocol vectors for connect/disconnect/reconnect, permission denial, missing/insecure MIDI, unsupported capability, concurrent/busy operations, empty voice data, malformed voice/map/import data, both installer languages, metadata drift, old format bytes and the DX correction. Built-site checks cover the shared metadata/release injection and all 152 retained controls. Default `0.3.0` and explicit `0.3.0-demo` outputs are qualified separately; the latter shares the same valid firmware identity and differs from the configured default.

Physical USB connection, real SysEx round trips, device persistence and stock/loader recovery remain hardware acceptance checks. No device was opened or flashed for this implementation. Browser/device interaction should be recorded separately from these source/API results.
