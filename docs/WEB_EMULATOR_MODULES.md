# Emulator browser modules

[Web ticket W04 (#32)](https://github.com/superposition/91/issues/32) separates the browser bindings from the instrument. The public page stays focused on playing songs. It loads native ES modules from `web/emu/src/`; no bundler, network library or second synthesis implementation is involved.

| Module | Responsibility and resources |
| --- | --- |
| `app.mjs` | Mount the instrument, connect features, handle Switch on/save/reset, and dispose on navigation. A restored back/forward-cache page mounts fresh controls. |
| `panel.mjs` | Draw the existing SVG panel, create HTML touch targets, display RGB565 frames and lights. Remove generated targets on disposal. |
| `controls.mjs` | Pointer, wheel and keyboard input; held/latched masks; master volume. Publish input events to subscribers instead of letting stream view replace input functions. Release held controls and remove listeners on disposal. |
| `audio.mjs` | One AudioContext and processor connection. Select AudioWorklet when available or the existing HTTP callback otherwise. Bound the complete startup sequence, cancel pending work, and close nodes/ports/context on failure or disposal. |
| `storage.mjs` | Existing base64 object storage and MIDI map persistence; tolerate unavailable local storage. |
| `midi.mjs` | Optional MIDI, learning, map import/export and FM-1 remote lights. Detach input handlers, close MIDI ports opened by this feature, release holds, stop acknowledgements and revoke downloads on disposal. |
| `stream.mjs` | NAIWAN spectrum/stage, input-move labels and analyser connection. Cancel animation/timers, disconnect the analyser and return the panel to its original parent on disposal. |
| `constants.mjs` | Existing button/encoder numbers and key ordering. |
| `lifecycle.mjs` | Idempotent listener/resource cleanup shared by mounted features. |

`worklet.js` still owns the processor adapter and HTTP callback. Its added `dispose` message releases processor references and stops processing; a load completing after disposal cannot revive it. Startup errors return an error message so the player can retry. Rendering, storage/frame payloads, telemetry fields and command dispatch are unchanged.

## Compatibility contracts

- The WASM export set remains at 24; no engine or FFI source changes belong to this ticket.
- Keep `hyp1-emu-store`, `hyp1-emu-midimap`, base64 bytes, `.perfmap` version 1 and all remote MIDI channel/note/CC numbers stable.
- Frame telemetry remains available in `window.ambLast`. The retired `cpu` and `migration` elements stay absent; all 152 actual controls across the served site remain covered by the existing ID contract.
- Audio starts only after **Switch on**. AudioContext resume begins in that click handler before asynchronous loading, preserving the user gesture. Whole-startup cancellation covers fetch, module load, resume and the processor-ready message, including browser promises that never settle.
- `web/emu/package.sh` copies the HTML, worklet and all modules for both standalone `build/emu` and the full site. Module dependencies resolve relative to their files, including when hosted under a repository URL prefix.

## Qualification

With the Rust-enabled WASM build present:

```sh
node tests/host/emu_browser_test.mjs build/emu/amb.wasm
node tests/host/emu_http_test.mjs build/emu/amb.wasm
python3 tests/web_site_test.py build/site-check build/emu
```

The new Node test uses browser API doubles with the real WASM/processor. It covers both audio selections, exact PCM equality, real frames/telemetry, startup timeout/cancellation/retry, repeated mount/dispose, pointer/keyboard input, knob drag/wheel, MIDI mapping/remote cleanup, storage encoding and stream analyser/animation cleanup. The existing HTTP test independently compares 1,048,576 samples against 128-frame processor quanta and checks callback disposal. Site validation checks the served routes, control IDs, copied module bytes, JavaScript syntax and local module dependencies in both bundles. Pages and emulator CI run these checks.

The isolated implementation was built with `RUSTUP_TOOLCHAIN=1.94.1`, the pinned local WASM clang wrapper and `EN_VERSION=0.3.0`. The site qualification uses the existing versioned FM-1 package as an installer asset; it does not flash or qualify hardware.

Actual browser checks are recorded in [web/browser-qa.json](web/browser-qa.json): Chromium HTTP signal sampling and evaluated input/stream events, plus native Windows computer-use startup and panel interaction in desktop Chrome. Developer frames now also expose melodic, effects and factory execution counters through `window.ambLast.rust`; no public status badges or new WASM exports are added. Real phone touch, explicit real AudioWorklet scheduling, HTTP playback on another computer and optional Web MIDI/FM-1 behavior still require qualification. API doubles and framebuffer tests do not establish those results. W04 remains open for that remaining evidence.
