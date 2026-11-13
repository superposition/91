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

The original web refactor shipped alongside Rust planning, bass decisions, pack/session codecs, command queue, sample-clock cadence, Songbook view copies and staging arbitration. Subsequent releases also migrated input and parts of synthesis; the current boundaries are recorded in the migration documentation. These details remain in development documentation. Public pages explain choosing a standard, setting key/tempo, shaping the keyboard and bass sounds, and playing the keyboard part in Synth Jam. Hardware is optional for `/emu/`; the installer and connected editor retain their device-specific roles.

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

The inherited small coral labels and hover states now use the dark accent token, and selected solid controls use white text. Computed contrast for these foreground/background pairs improves from 3.12 to 7.43 on cream and from 2.91 to 5.27 on green selections. The dark accent on sky is 4.97 and muted body text on sky is 4.70. These calculations qualify the selected color pairs; desktop/mobile layout and focus visibility still require actual browser review in #30/#35.

Shared styles cover the landing, emulator, optional device tools, library redirect, stream view and served manual. Fonts are repository-owned assets. The firmware's device-panel geometry, MIDI mappings, storage formats, update/recovery markers and upstream credits remain intact.

`web/emu/package.sh` packages the same stylesheet, local fonts and icon inside standalone `build/emu`, so the emulator can also be served directly from that directory. Full-site output reuses that bundle and copies the shared source stylesheet for the other pages. Missing optional manual PDFs are omitted from the served manual rather than left as broken downloads.

## Reproducible current imagery

```sh
node tools/capture_web_screens.mjs build/emu/amb.wasm
```

This boots the actual WASM with seed 91 and master 2800, advances the instrument, captures the performance page and SELECT's four Songbook detail pages, and writes lossless 240×240 PNGs plus `docs/img/naiwan91-captures.json`. PNG bytes come from the same byte-swapped RGB565 buffer that the browser draws. These are instrument framebuffer captures, not browser layout screenshots or invented hardware renders.

For 0.4.4, the last capture shows Synth Jam after its MODE knob is turned from BAND to SYNTH. The capture manifest records that input and the exact WASM hash. Historical release manifests retain their original image hashes.

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
python3 tools/make_pages_site.py 0.4.4 build/site-web-review
python3 tests/web_site_test.py build/site-web-review
node tests/host/emu_http_test.mjs build/emu/amb.wasm
node tests/web_backup_test.js
```

Branding, theme, player copy and imagery now match the Songbook. The [browser bindings](WEB_EMULATOR_MODULES.md) are separated into nine modules with startup cancellation and resource cleanup. Authoritative device metadata and canonical site generation are delivered in the same review branch. Browser review also found inherited control descriptions: SELECT still said Tempo and Songbook Save/Hold/New Idea described the older generative controls. The hover and accessibility descriptions now explain the Songbook roles, with the alternate generative-piece roles where applicable.

Actual browser qualification on 2026-11-12 is recorded in [browser-qa.json](web/browser-qa.json). On the LAN HTTP origin, a real Chromium ScriptProcessorNode ran at 44.1 kHz: all 20 analyser windows contained finite, nonzero audio, with 40,960 sampled values and zero page errors. DOM event checks exercised key release/blur, knob arrow/wheel input and stream-view entry/exit. These were delivered through browser evaluation, not physical keyboard or touch input. The 618px-wide page and live instrument framebuffer were visually reviewed without horizontal overflow.

Native Windows computer use then opened one Chrome tab on localhost, clicked Switch on and verified On plus a live framebuffer, and stopped playback through the panel's held-button interaction. The 1644x1248 Chrome-window capture shows the desktop layout and a visible blue focus outline. The separate browser connector lacked its access host; native control recovered after the earlier URL-check failure during tab creation. Neither condition required a site/server change. Physical phone touch, other desktop browsers, real Web MIDI/FM-1 interaction and full cross-route visual/focus QA remain open. Browser signal sampling and a live framebuffer do not establish subjective listening or physical-device qualification. No public deployment is implied by a local build or draft PR.

The existing manual renderer contains duplicate anchor IDs. The built-site check compares their exact fingerprint against `docs/manual.html`, rejects any new duplicates, and rejects duplicates outright in the refactored pages. Correcting the manual content renderer remains part of W07; this exception does not permit missing route/assets or new defects.
