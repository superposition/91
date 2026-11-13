# 0.4.3 integration record

This is the release-candidate integration record, dated 2026-11-13. Source changes, automated validation, a packaged build, deployment and physical FM-1 qualification are separate steps. This document does not claim that 0.4.3 has been installed or tested on hardware.

## Preserving the existing work

The original `fw` working directory reported many changes against `master` at `5b78ae77897d8bc08981c0a620ab2cce192a2d20`. A read-only Git-object comparison found that **all 1,317 publishable files already exactly matched checkpoint `3f2d40e4c2d6398248e13f4a06e26c3ad625493c`**. This includes all 908 files shown as untracked against the older branch. There were zero different, unique or missing files.

The comparison took the union of `git ls-files` and `git ls-files --others --exclude-standard`, hashed the existing files with `git hash-object --stdin-paths` without `-w`, and compared those Git-normalized blobs to `git ls-tree -rz` at the checkpoint. It did not change the original files or index. The [machine-readable result](release/source-preservation-20261113.json) records counts, commit identities and the normalized inventory checksum.

The release worktree therefore starts from that checkpoint rather than copying a seemingly dirty old branch over the completed web and Rust migration. The three factory standards, seven custom-voice packs and new controls are integrated on top. Qualified snapshots and the original working directory remain separate from this integration.

## Release boundaries

- The public player describes Blue Bossa, Autumn Leaves and Satin Doll. Four additional packs can be loaded into the user slots; they are not represented as four extra factory slots.
- JMP3 preserves authored chord qualities. CLOSE, DROP2 and INVERT change chord voicing; New Idea, Bass Feel and Variation change bass motifs. They do not replace the chart's harmony.
- Each bank supplies six keyboard sounds and two bass sounds. The seven packs use 30 distinct custom programs, with source-bank credits and generator recipes retained. Code licensing and downloaded patch provenance are recorded separately in [LICENSING.md](../LICENSING.md).
- The 0.4.3 control change gives keys and bass separate octave settings from −2 to +2 on knob 2 of their Songbook pages. Touch and bass Attack remain advanced pack controls. New packs request keys +1 octave, bass +1, and plate settings Reverb 38, Size 68, Damping 42, Predelay 80. The bass register was raised after the first candidate review; canonical bass VMEM transpose stays 12 and the runtime octave supplies the shift. Build and device qualification must verify these candidate settings.
- The browser player already has separate lifecycle, audio, controls, MIDI and storage modules. The connected editor remains a legacy parameter/DX voice editor; it is not presented as a graphical JMP3 chart editor. The old `web/app` is an explicit, unshipped API regression harness.
- Generated sites, firmware, host recordings and Cargo output stay outside the source inventory. Physical captures under ignored `build/hardware/` may be unique evidence and are preserved.
- The current web fonts are Quicksand and Nunito. All 21 bundled WOFF2 name tables retain their creator notices and SIL OFL link. `web/fonts/OFL.txt` now accompanies them in the source and copied bundles; the licence table no longer describes the older font families as the current site typography.

## Release and CI corrections

`tools/release.sh` now propagates the emulator builder's failure instead of returning the status of a trailing `tail` command. A regression test runs the real wrapper in a temporary fixture with a successful package build and a failing emulator, then checks that the release exits with the failure status. Its success case also verifies the version reaches the emulator.

The native simulator CI job fetches Git history, reconstructs the hash-pinned R01 source at `43db648fb9b31f944b7f29fb24e70f152a3b5b13`, and passes that explicit independent directory to the comparison. A local `sh tests/rust_simulator.sh` also reconstructs this reference by default, verifies every file and rejects the current tree as a reference. Explicit external baselines undergo the same hash verification. The frozen corpus remains unchanged; new musical behavior has its own focused tests.

`web/product.json` declares 0.4.3. Its generated `web/device-data.js` must agree with it. The static-site builder checks the supplied package's actual identity and records its hash and file inventory; changing the filename is not a version bump. The historical 0.3.0 W07 results in [WEB_BUILD.md](WEB_BUILD.md) remain historical evidence.

## Validation status

Completed on the release worktree:

- Release wrapper: 2 tests pass, including a failed emulator after a successful package build.
- Simulator reference boundaries: 3 tests pass for default reconstruction, explicit reference tampering and current-tree rejection. The actual 440-file frozen source was reconstructed with native Git and every SHA-256 verified in WSL. This verifies the reference inputs, not the full scenario comparison.
- Web output/build safety: all 11 tests pass in WSL.
- Browser codecs built; product/parameter generation and `--check` pass for 176 parameters and release 0.4.3.
- Node 22 checks pass for metadata/capabilities in both locales, DX SysEx, backup/maps, OTA identities and idle device lifecycle. These checks do not use a physical device.
- The default Rust-backed 0.4.3 emulator built with Rust 1.94.1 and Zig 0.17.1. Before the bass-default revision, browser API tests passed secure/HTTP startup, cancellation, timeout/retry and input/MIDI/stream/storage lifecycle; HTTP fallback matched 1,048,576 samples against the processor path.
- After the data-only bass-default revision, the emulator and complete site were rebuilt. All five framebuffer captures were regenerated; visual inspection confirms both OCTAVE controls show +1. The updated site passes 9 routes, 142 local links/assets, 6 inline scripts, 18 module copies, 42 CSS asset dependencies and all 152 preserved controls, including inventory and hosted-subpath checks. Metadata passes in both locales. Font notices are present in standalone and full-site bundles.

The [machine-readable web evidence](release/web-0.4.3.json) preserves the first candidate's browser checks separately from the data-only revision. The final WASM SHA-256 is `1e4472818b8d97def3ed8e0a373fa256855497a82efbb78463774d2fa8265a56`; the packaged firmware SHA-256 is `3aff7ba0a0f290a787b5ac1e2d13dedaa331b850eff66b15ad4dde599200f4a9`. The site copies match these exact files. The firmware build agent's separate `build/qualified-package.json` contains MCU/package and focused DSP, UI and transport results.

The WSL system `node` is older than the required version on this workstation; web checks used the installed Windows Node 22.14.0. The Windows-created worktree's Git pointer also requires native Git for reconstruction (or a correctly configured WSL Git worktree). Neither issue affects a normal Linux CI checkout. Interactive browser/audio-device and physical-device results remain separate. No commit, push, Pages deployment or MIDI operation is part of this documentation/build cleanup.

The captured detail pages still retain the legacy `C05 / G Mixolyd` top strip. The Songbook performance screen shows its own key and written chord; replacing the inherited detail-page strip is a remaining UI cleanup, not something altered in these images.

Reproduction commands for the web/release checks:

```sh
python3 tests/release_script_test.py
python3 tests/simulator_reference_test.py
python3 tests/web_build_test.py
python3 tools/build_codecs.py
python3 tools/make_params.py
python3 tools/make_params.py --check
node tests/web_metadata_test.mjs
```

After the integrated firmware and emulator are built:

```sh
python3 tools/make_pages_site.py 0.4.3 build/site
python3 tests/web_site_test.py build/site build/emu
node tests/web_metadata_test.mjs build/site
node tests/host/emu_browser_test.mjs build/emu/amb.wasm
node tests/host/emu_http_test.mjs build/emu/amb.wasm
```
