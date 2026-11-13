# 0.4.4 Synth Jam integration record

This candidate starts from the committed 0.4.3 source checkpoint `2d27650`. The
[0.4.3 record](RELEASE_0_4_3.md) and its machine-readable evidence remain historical
records; they are not rewritten as proof for this release.

## Behavior and boundaries

On v3 packs, detail page 42 becomes Synth Jam with MODE, PROGRAM, OCTAVE and
SPACE. Band mode retains authored chord playback. Synth mode routes all 27
physical keys to a chromatic keyboard, mutes automatic keyboard comping and
keeps drums and bass running. Five keyboard voices share the existing FM engine;
the bass retains its protected voice.

PLAY controls the backing while manual notes remain playable, including when
the backing is stopped. HOME releases held notes and returns to Band and the
Songbook page without resetting the chart or groove. Program, octave and space
controls work on the current song's settings. Both authored default octaves
remain +1. The Songbook detail header uses the current key and written chord
instead of the inherited legacy piece/mode strip.

The mode is a bounded v3 pack field, `parts.synth`, at header byte 1120+25.
Values are 0 (Band) and 1 (Synth), with missing fields defaulting to 0. USB
control 24 sets mode; control 25 changes the existing reverb amount in `sounds`.
There is no new patch-bank format, browser synthesis engine or duplicate effect
storage. v1/v2 behavior remains separate. Panel notes use
`53 + key_index + 12 * keys_octave`; external MIDI notes retain their raw pitches.

The C/runtime and Rust/input changes are integrated together. The web player
keeps its existing input, audio and lifecycle modules; the public changes are
the current controls/help, release metadata and real framebuffer captures.

## Validation status

The MCU build, memory/recovery gates, held-note DSP tests, UI routing and USB
persistence tests passed. The installed package is `FM-1_7000404`, 611,251 bytes,
SHA-256 `d61d38a7f76d1142cc9a2bad0f39e0dd8cab1f07f31e89601bf16957f719b7d3`.

[Physical verification](release/hardware-0.4.4.json) records a verified restart
and 131 telemetry frames over 32 seconds with zero crashes, xruns or added late
events. Backing stayed active in Synth mode, CPU readings were 45–48%, and both
part octaves remained +1. All four saved banks were byte-identical; the active
bank changed only its mode byte. These observations establish that device run;
held-note sound and input transitions also have separate automated DSP/UI tests.

The final 0.4.4 [web qualification record](release/web-0.4.4.json) verifies the
same package in the site and the production WASM SHA-256
`5232ac420a875eb1f806c23f81edca0020249eee5e0c10e5e3316321bcb92542`.
All five new framebuffer captures were visually reviewed. The site checks pass
for nine routes, 142 local links/assets, 152 retained controls and 18 module
copies, plus metadata, backup, SysEx, OTA and lifecycle contracts. Eleven site
output-safety tests also pass. Actual WASM audio checks cover 1,048,576 HTTP
fallback samples and manual Synth Jam notes with the backing stopped, including
HOME releasing held notes. Interactive browser layout was not rerun for 0.4.4.
No public deployment is inferred from either a local build or device install.

The authoring gate passed 903 differential cases, 167 fake-device cases,
15 acceptance cases, 11 CLI cases, 8 Jam-tool cases and 3 USB transport cases,
plus six full songs totaling 37,712,896 frames with exact restart comparison.
Logs are `build/release-authoring-0.4.4.log` and
`build/release-authoring-tail-0.4.4.log`.

The independent CI baseline exposed a fixture boundary: `jam-helper` and
`songbook` are both among the 94 frozen historical scenarios, but the current
factory contains different songs. Historical builds now inject only the exact
R01 `song_factory.inc`, verified against `tests/reference/source.sha256`; the
frozen source/oracle, scenario files, expected failures and emulator fingerprints
remain unchanged. Current C and Rust run separate, explicitly named JMP3 cases
with exact PCM, framebuffer and artifact comparison. A changed or current
factory table is rejected as a legacy fixture. The optional
`FM1_TEST_FACTORY_HEADER` build input is cleared by the release wrapper.

[Parity evidence](release/parity-0.4.4.json) records the fixture hashes, source
fingerprints and separate result families. The focused final current C oracle,
C runner and Rust runner agree on all output from `jam-helper` and the new
Songbook/Synth case, with zero assertions. The real F4 exit sequence compares
3,475,080 WASM samples and every framebuffer exactly. This caught and fixed a
Rust snapshot refresh that left the Songbook header stale after changing pages.
An independent frozen UI-frame gate also passes 8,192 native/WASM cases and
14,176,176 exact bytes. The full 94-scenario native corpus plus extras and the
full emulator workflow will run on the pushed commit; no final full-suite local
pass is claimed here.

```sh
EN_VERSION=0.4.4 sh web/emu/build_clang.sh
python3 tools/build_codecs.py
python3 tools/make_params.py
python3 tools/make_params.py --check
python3 tools/manual_html.py
node tools/capture_web_screens.mjs build/emu/amb.wasm
python3 tools/make_pages_site.py 0.4.4 build/site
python3 tests/web_site_test.py build/site build/emu
node tests/web_metadata_test.mjs build/site
node tests/host/emu_browser_test.mjs build/emu/amb.wasm
node tests/host/emu_http_test.mjs build/emu/amb.wasm
node tests/host/emu_synth_jam_test.mjs build/emu/amb.wasm
```
