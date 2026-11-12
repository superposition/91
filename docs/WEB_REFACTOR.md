# NAIWAN 91 web refactor

Tracked in [web epic #28](https://github.com/superposition/91/issues/28), alongside [Rust migration #1](https://github.com/superposition/91/issues/1). Public pages introduce the songs, sound and playing controls. Implementation status, measurements and architecture belong in development documentation rather than the player interface.

## Findings and first changes

| Finding | First implementation |
| --- | --- |
| Four pages contained literal `:root{{...}}` and `body{{...}}` theme blocks, copied from a Python f-string. Invalid CSS left the inherited dark/red palette active. | Replace those blocks with valid, shared `web/site.css`. |
| Landing HTML and its separate palette lived inside the Python site builder. | Author `web/index.html` as the product page and substitute a small set of build metadata. |
| Editor, pieces and library titles still said PERF-1; inherited minimum versions were presented as NAIWAN releases. | Correct visible product names and selected capability messages; preserve compatible formats and identifiers. |
| Emulator help described HYPNO startup, obsolete genre/store behavior and other products' stream views. | Lead with current Songbook controls, collapse optional MIDI, link the maintained legacy manual, and retain one NAIWAN spectrum view. |
| Backup/telemetry genre labels still showed HYPNO/LOFI while compiled metadata reports CITYPOP/SUNSET. | Align display labels with current `build/params.json`; preserve numeric IDs and saved bytes. |
| README hero, tour and feature collages depicted the earlier fork. | Replace their references with five fresh, deterministic WASM framebuffer captures. |

The current build uses Rust planning, bass decisions, pack/session codecs, command queue, sample-clock cadence, Songbook view copies and staging arbitration. C still dispatches musical events and draws/synthesizes the instrument. These details remain in development documentation; public pages explain choosing a song, key/tempo, Hold/New Idea and shaping keys/bass/swells. Hardware is optional for `/emu/`; the installer and connected editor retain their device-specific roles.

The public CPU/migration badges were intentionally removed after review. Their two non-interactive DOM IDs are recorded as retired in `tests/web-required-ids.json`; all actual controls remain covered by the original ID contract. Developer telemetry remains available through `window.ambLast` without being displayed to players.

## Shared color spectrum

The source of the palette is the firmware's Beach theme in `firmware/src/app/ui.c`, rather than the malformed inherited web override:

| Token | Firmware color | Web use |
| --- | --- | --- |
| Sky | `#8FD3EA` | Page ground |
| Cream | `#FDF3D6` | Cards and panels |
| Navy | `#14263A` | Body text |
| Coral | `#F2542D` | Accents and active state |
| Pad blue | `#1D6FB8` | Primary blue |
| Frost | `#1B7FD6` | Secondary blue |
| Palm | `#0E9E7E` | Green accent |

Web text and small controls use darker derived shades where needed for legibility (`#0F4A80` blue, `#932810` coral, `#087A65` green). The raw instrument framebuffer is never recolored. Its RGB565 quantization can differ slightly from the unquantized CSS values.

Shared styles cover the landing, emulator, optional device tools, library redirect, stream view and served manual. Fonts are repository-owned assets. The firmware's device-panel geometry, MIDI mappings, storage formats, update/recovery markers and upstream credits remain intact.

`web/emu/package.sh` packages the same stylesheet, local fonts and icon inside standalone `build/emu`, so the emulator can also be served directly from that directory. Full-site output reuses that bundle and copies the shared source stylesheet for the other pages. Missing optional manual PDFs are omitted from the served manual rather than left as broken downloads.

## Reproducible current imagery

```sh
node tools/capture_web_screens.mjs build/emu/amb.wasm
```

This boots the actual WASM with seed 91 and master 2800, advances the instrument, captures the performance page and SELECT's four Songbook detail pages, and writes lossless 240×240 PNGs plus `docs/img/naiwan91-captures.json`. PNG bytes come from the same byte-swapped RGB565 buffer that the browser draws. These are instrument framebuffer captures, not browser layout screenshots or invented hardware renders.

## Tickets

- [#29 — W01: inherited surface audit and product boundary](https://github.com/superposition/91/issues/29)
- [#30 — W02: shared spectrum and broken theme correction](https://github.com/superposition/91/issues/30)
- [#31 — W03: product navigation and Songbook playing controls](https://github.com/superposition/91/issues/31)
- [#32 — W04: browser presentation/integration modules](https://github.com/superposition/91/issues/32)
- [#33 — W05: README and current imagery](https://github.com/superposition/91/issues/33)
- [#34 — W06: metadata and capability copy](https://github.com/superposition/91/issues/34)
- [#35 — W07: build qualification and legacy entry points](https://github.com/superposition/91/issues/35)

Rust WASM ownership and migrating browser application logic remain [#17](https://github.com/superposition/91/issues/17) and [#23](https://github.com/superposition/91/issues/23). Build-tool conversion remains [#22](https://github.com/superposition/91/issues/22); the talk remains [#26](https://github.com/superposition/91/issues/26). This track does not create a second synthesis implementation.

## Validation and remaining work

Build the site with the existing compiled emulator and release package, then run the built-site check:

```sh
python3 tools/make_pages_site.py 0.3.0 build/site-web-review
python3 tests/web_site_test.py build/site-web-review
node tests/host/emu_http_test.mjs build/emu/amb.wasm
node tests/web_backup_test.js
```

Branding, theme, player copy and imagery now match the Songbook. The [browser bindings](WEB_EMULATOR_MODULES.md) are separated into nine modules with startup cancellation and resource cleanup. Complete localization/capability metadata, retirement of unused builders and real desktop/mobile browser visual/audio/MIDI QA remain ticketed. Computer Use did not complete the prior browser verification because the browser host was unavailable and native URL verification stopped access. Source/Node/WASM checks and inspected framebuffer images do not establish browser-layout or audio-device QA. No public deployment is implied by a local build or draft PR.

The existing manual renderer contains duplicate anchor IDs. The built-site check compares their exact fingerprint against `docs/manual.html`, rejects any new duplicates, and rejects duplicates outright in the refactored pages. Correcting the manual content renderer remains part of W07; this exception does not permit missing route/assets or new defects.
