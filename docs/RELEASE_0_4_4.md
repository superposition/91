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
SHA-256 `be511345bf19e3aa3843f1e9a62d6bae68de08b18000520d59da39c27a82dfaf`.

[Physical verification](release/hardware-0.4.4.json) records a verified restart
and 129 telemetry frames over 32 seconds with zero crashes, xruns or added late
events. Backing stayed active in Synth mode, CPU readings were 45–49%, and both
part octaves remained +1. All four saved banks and the active bank were
byte-identical. These observations establish that device run;
held-note sound and input transitions also have separate automated DSP/UI tests.

The final 0.4.4 [web qualification record](release/web-0.4.4.json) verifies the
same package in the site and the production WASM SHA-256
`f6bbaa22cff650475f9e96e303cb0d5bb681f57bad8a72a239ed51343c34927d`.
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
14,176,176 exact bytes. Those local checks preceded the remote run below.

## CI follow-up

[Run 43388392909](https://github.com/superposition/91/actions/runs/43388392909)
on `3e245ad` passed six jobs: native simulator, Linux and Windows portable,
Miri, authoring, and web metadata/build safety. Native coverage includes all
94 historical scenarios, three extra cases and two current JMP3 cases. The
emulator job passed its first 28 stages, including full instrument playback and
browser codecs, then stopped at the historical device-wire test: it expected
the original bank's reverb value 10 while the new bank uses 38.

The fixture repairs change test scripts and isolated test adapters. Historical
USB, persistence and full-app UI comparisons receive the SHA-verified R01
factory table; current JMP3 tests keep their production bank. The native handoff
build also declares `EN_HOST`, and isolated Runner/button adapters provide the
callbacks now required by the code under test. No golden values or inherited
assertions are rebased.

The full-app comparison also found that hashing the pack version on every
knob redraw changed an internal redraw fingerprint for legacy packs. A small
runtime guard restricts that additional version hash to v3 packs. All four
historical UI corpora now match their original golden bytes across native and
WASM C/Rust/frozen implementations, with 4,096 cases per corpus. The current
Songbook/Synth and F4 comparisons also pass exactly after the guard. The
production WASM, firmware package and site have been rebuilt and qualified.
The new package is installed with the bounded physical evidence above. All five
regenerated PNGs match the initially reviewed images byte for byte. Initial
[web](release/web-0.4.4-initial.json),
[parity](release/parity-0.4.4-initial.json),
[firmware](release/firmware-0.4.4-initial.json) and
[physical](release/hardware-0.4.4-initial.json) records preserve the first build.

Device-wire and handoff checks now pass against C, Rust and the frozen source,
retaining USB response fingerprint `eff97ff8`. All remaining local workflow
commands now pass: 13 persistence/DSP/HAL commands, 21 UI/environment commands
and 11 USB commands, with original failure logs and corrected reruns retained. The
[follow-up evidence](release/ci-followup-0.4.4.json) distinguishes these local
results from the remote run; a green remote rerun is not yet established.

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
