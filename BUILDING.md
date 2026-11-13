# Building NAIWAN 91

NAIWAN 91 builds on Linux x86-64 (or macOS/Windows through Docker or WSL: JieLi's toolchain is Linux x86-64 only). A release build makes:

| File | What |
| --- | --- |
| `build/amb.bin` | the firmware app |
| `build/loader/ota.bin` | the update loader |
| `build/naiwan91-VERSION.fwsc` | the installable package (app + loader) |
| `build/emu/` | the browser emulator (WebAssembly) |

## Prerequisites

- Python 3.10 or newer with Pillow. The full site/font workflow also uses fonttools (`python3 -m pip install Pillow fonttools`). MIDI authoring and its comparison suite additionally use `mido` and `numpy`; USB access needs `python-rtmidi`.
- A native C compiler and POSIX shell for host checks. On Windows, run firmware and shell builds in WSL2; Python and Rust authoring tools also support native Windows. Emscripten is optional for the archived `web/app` API regression harness; its check is skipped when `emcc` is absent.
- **The JieLi Linux toolchain** (clang 4.0.1 for pi32v2, from JieLi's package server): `tools/get_toolchain.sh` installs it to `~/.jieli/toolchain` (`JIELI_TOOLCHAIN` overrides). On macOS or with `JIELI_DOCKER=1` each tool runs in a `linux/amd64` `debian:bookworm-slim` container.
- **The JieLi AC79 SDK** (Apache-2.0). The package uses three of its files (`cpu/wl82/tools/uboot.boot`, `cfg_tool.bin`, `cfg/eq_cfg_hw.bin`); they are not part of this tree:

  ```
  git clone --depth 1 --branch AC79NN_SDK_V1.2.1_2023-12-13 https://gitee.com/Jieli-Tech/fw-AC79_AIoT_SDK.git ~/fw-AC79_AIoT_SDK
  ```

  (`AC79_SDK` overrides the location.)
- A clang with the `wasm32` target and `wasm-ld` (for the emulator), Node.js 22 (the web tests), and Rust 1.94.1 with its `wasm32-unknown-unknown` target. Rust is used by build/codec tooling as well as the native simulator and browser instrument; the physical release still uses the JieLi C compiler and package path.

  ```sh
  rustup toolchain install 1.94.1 --profile minimal --target wasm32-unknown-unknown
  rustup override set 1.94.1
  ```

## Build and test

```
tools/release.sh 0.4.4            # parameter data, the manual's page, the tests, the package, the emulator
tools/release.sh 0.4.4 --no-tests # without the tests
./build.sh                        # a development build (version DEV)
tests/run_tests.sh                # the tests alone
```

The tests run, on the build machine: the maths library against libm; the instrument (`tests/host/amb_test.c`); flash storage; the SysEx protocol and the editor's parameter and DX7 voice code; Felucca's update path against the package; the dual-core job handoff (identical output with the work on either core); and the whole app in a simulator (`tests/scenarios/*.amb`, about 70 scripts), with screenshots and audio in `build/scenarios/`. `host/build_host.sh` builds the simulator alone; `build/host/amb_host SCRIPT OUTDIR` runs one script (the command list is at the top of `host/amb_host.c`).

For an independent native compatibility comparison, reconstruct the pinned reference before running the simulator suite:

```sh
python3 tools/frozen_source.py build/r01-source
sh tests/rust_simulator.sh "$PWD/build/r01-source"
```

This requires Git history containing the checkpoint pinned in `tools/frozen_source.py`. Every reference file is verified against `tests/reference/source.sha256`. CI fetches that history explicitly. Legacy comparisons give the current C and Rust builds the SHA-verified historical factory table, leaving the frozen oracle and its expected results unchanged. Separate current-factory cases compare C and Rust with the new standards and Synth Jam. Both families compare exact audio, frames and state; the release wrapper clears the test-only factory override before building. A successful build is not a hardware qualification; see the [0.4.4 integration record](docs/RELEASE_0_4_4.md).

The build fails if a soft-double routine is linked (a `double` crept in) and checks the image, RAM and pool sizes. Build options: `EN_CORE2=0` builds single-core (the reverb on the second core is the default), `EN_CDC=1` adds Felucca's USB serial function, `EN_DIAG=1` adds the status log's third frame (timers inside the render's stages, the chunk sizes, the work between chunks) for a performance hunt.

The DY voices (`firmware/src/dsp/fm6_dy.h`) are generated from DX7 bank files that are not in this tree: `python3 -I tools/gen_fm6_dy.py tools/fm6_dy.txt BANKS --rom ROMDIR` reads the manifest (slot, name, bank file, voice, source), writes the header, `build/fm6_dy.bin` and the test bank `build/NAIWAN 91-test-voices.syx`, and refuses a voice whose operators match a Yamaha ROM voice in ROMDIR. `tests/host/dycal FILE` (built like the other host tools) prints each voice's level and the gain it would need against the plain sounds; `FM_GAIN_DY` in `amb.c` is set from it and `fmcal`'s rms column so each DY voice sits where its DX voice does.

The current Songbook has a separate reproducible bank: `tools/build_standards.py` authors the seven charts and `tools/build_citypop.py` selects the custom voices from `packs/citypop-sources/`. The numbered JSON/JMP packs and generated factory table are kept with the source. These packs preserve explicit chord qualities and the voices' programmed velocity response and pitch modulation. See [Songbook packs](docs/JAM_PACKS.md) for import and regeneration commands, and [licensing/provenance](LICENSING.md) for the source-bank credits. Retain older pack fixtures: they exercise format compatibility.

## The site

```
python3 tools/make_pages_site.py 0.4.4 build/site
python3 tests/web_build_test.py
python3 tests/web_site_test.py build/site build/emu
```

makes the landing page, installer, player, editor and manual as a static folder with relative links. The canonical Pages workflow reads its default version from `web/product.json`; an explicit builder version must match the package identity. Full-site assets are shared once, while `build/emu` is also usable as a self-contained bundle.

Output directories must be below `build/`. Generated outputs carry a verified inventory, and local changes stop replacement. Use a fresh directory for older builds, or `--adopt-existing` once to migrate a recognized pre-manifest site while retaining its entire old directory as a backup. See [web build qualification](docs/WEB_BUILD.md) for the entrypoint, subpath, metadata and remaining browser checks. Building a folder does not publish it; GitHub Pages deployment or another static host is a separate step.

The printable quick manual (`docs/QUICK.md`, eight A4 pages) is made by `python tools/make_quick_manual.py` on Windows, which prints its HTML to `build/quick-manual.pdf` with Microsoft Edge; the site build copies the PDF to `manual/NAIWAN 91-quick-manual.pdf` when it is there. Bump the footer line at the top of `QUICK.md` with each release.

## Install

From the site, or from the command line (needs `pip3 install mido python-rtmidi`):

```
python3 tools/fm1_install.py build/naiwan91-0.4.4.fwsc
python3 tools/fm1_install.py --info          # identity of the connected FM-1
```

Installing firmware is at your own risk. Hold OCT- and OCT+ for 5 seconds for the update mode. If the FM-1 no longer starts but reaches the chip's update mode (4C4A:8057 on USB), `tools/fm1_rescue.sh` puts stock firmware back from a Mac; otherwise recovery needs [FM-1-transporter](https://github.com/kurogedelic/FM-1-transporter).

## What protects an install, and what does not

The installer's checks (identity read from the package's own markers, the loader mark, the CRCs, the head-equal check, the identity read back after the reboot) and the firmware's safe mode (two failed boots, then USB-only, then UBOOT by SysEx or OCT- + OCT+ held for five seconds) cover a bad *app*. They do not cover a corrupt start-up sector, a bad SPL, or power lost partway through the write with no resume record: no hardware recovery (ISP pads, boot strapping, a key chord) has been demonstrated for this chip, and the interrupted-write safety is by design, not yet tested with a power cut. Keep a way to restore the stock firmware to hand, and do not use `--force` on a package that was not built by this tree.

## Where things are

- `firmware/src/dsp/` the engine (`amb.c`: voices, drums, effects, the master bus; `fm6*.inc`: the DX7-style FM engine), `firmware/src/app/` the UI and the project (`ui.c`, `project.c`, `viz.c`), `firmware/hal/` the hardware layer, `firmware/loader/` the update loader.
- `rust/fm1-formats/` packed data and validation; `fm1-engine/` musical scheduling and synthesis; `fm1-app/` controls and UI; `fm1-rt/` bounded runtime; `fm1-hal/` platform bindings. `fm1-jam-bridge/` exposes the engine to platform callers, `fm1-sim/` runs native scenarios and `fm1-codecs-wasm/` supplies browser data codecs.
- `rust/fm1-authoring/` and `fm1-tooling/` implement authoring/build policy; `tools/` provides file, process and USB adapters, chart/voice generation and telemetry (`fm1log.py`, `fm1sum.py`).
- `web/emu/src/` contains the player lifecycle, audio, MIDI, storage and controls. Installer, connected editor and legacy piece-library pages use the shared device/protocol modules. [Web source](web/README.md) documents route boundaries; `docs/MANUAL.md` is the manual.
- Stored data: `firmware/src/app/project.c` (the packed 376-byte piece records, one flash object per genre), `firmware/src/storage.c` (the A/B sector objects and the flash map).

`build/` and Cargo `target/` are ignored generated outputs. Do not treat every file under `build/` as disposable: `build/hardware/` can contain unique physical recordings and qualification evidence. Keep those records outside any cache-cleaning operation. Publish source, source banks, compact test fixtures and generation recipes; keep release binaries and audio evidence as separate artifacts.
