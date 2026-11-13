# Static-site build qualification — W07

The maintained build is `tools/make_pages_site.py` with the authored landing page in `web/index.html`. [Ticket #35](https://github.com/superposition/91/issues/35) keeps interactive browser qualification and deployment evidence separate from these source/build checks.

## Entrypoints and compatibility

`tools/web_site_support.py` now contains the package identity decoder and module-inlining helper formerly imported from the Felucca builder. `web/make_site.py` preserves those import names and upstream attribution, but its CLI exits with the supported command before reading a package or writing output.

The current builder preserves `/library/` and adds relative redirects for `/webapp/installer/` and `/webapp/editor/`. Query strings and fragments survive those redirects. Every served link is checked below a synthetic `/nested/91/` host prefix.

The source inventory showed `web/app` remains a legacy API regression harness used by `tests/run_tests.sh`. It is retained, explicitly requires `--legacy-test`, and writes `build/legacy-app-test` instead of the ambiguous `build/app`. The canonical site never copies it. The default archived build command exits with current player/site instructions, and the regression test runner retains its explicit historical coverage.

## Bounded output replacement

The builder accepts output directories strictly below its own repository's `build/`. It rejects the repository/build roots, emulator and other build inputs, nested generated sites, and symlink/junction paths. It builds in an exclusively created staging directory; every recursive cleanup or directory move uses rechecked absolute paths within that boundary.

`site-build.json` records a deterministic file/directory inventory and release version, package identity, filename and full firmware SHA-256. A normal rebuild verifies the old inventory before replacing it. Unknown files or local edits stop the replacement and remain untouched. A failed final rename restores the previous generated directory.

For older bundles without that manifest, `--adopt-existing` is an explicit one-time migration. It recognizes all seven canonical old routes and compares every path with the fresh build before replacing anything. Only the new compatibility redirects may be absent. Redundant old emulator CSS/font/icon files are accepted only when they exactly match their existing root counterparts; arbitrary additional files and changed redundant copies are rejected. Since that inventory cannot detect edits inside a known legacy filename, the entire pre-manifest directory is retained beside the output after migration, and the CLI reports its location. It is not deleted automatically or included in the new deterministic manifest. A regression test preserves an edited legacy `index.html` byte-for-byte in that backup. A fresh output directory is always an alternative. No interactive approval prompt is required.

This manifest describes a built artifact. It does not claim that the artifact is deployed. The Pages deployment job records its published environment separately; these changes did not publish or alter the LAN preview.

## Assets and release metadata

Full-site pages use the shared root stylesheet, fonts and icon once. Standalone `build/emu` remains self-contained. Both bundles carry the same browser modules and worklet; source/module checks validate dependencies against the appropriate root. CSS font URLs are checked too.

The supplied package marker must match the version's `FM-1_7XXYYZZ` identity contract from `tools/build.py`, including `X.Y`, patch and suffix compatibility. Renaming the 0.3.0 package to 0.4.0 is rejected before output creation. The Pages workflow reads the default release from the W06 product metadata instead of retaining its old 0.1.0 constant; an explicit builder version remains authoritative and is checked against the package.

`--no-sharing` remains supported and removes links with fragments as well as plain directory links. Its reduced bundle has its own route/control checks.

## Checked results

The following is the historical W07 qualification, using a 611,251-byte `naiwan91-0.3.0.fwsc` package and its WASM artifact. It does not qualify a later release. Current integration evidence is in [the 0.4.3 record](RELEASE_0_4_3.md).

- Eleven output/entrypoint tests pass on Linux, covering unsafe paths, unknown files, local changes, deterministic rebuilds, failed-rename rollback, legacy adoption, aliased paths, retired CLI behavior, release syntax and compatibility subpaths. On Windows, nine pass and the unavailable symlink/shell checks are explicitly skipped.
- Two full builds produce byte-identical `site-build.json` output.
- Full site: nine routes, 141 local links/assets, six inline scripts, eighteen browser-module copies, forty-two CSS font references and all 152 active frozen control IDs pass.
- No-sharing site: seven routes, 125 local links/assets and all 95 active IDs for those routes pass.
- Release package version/identity/hash and five current screenshot references are verified. The archived prototype is absent from the site.
- Browser API doubles, secure/HTTP lifecycle tests and the 1,048,576-value HTTP audio comparison pass. SysEx, backup/maps, OTA and idle protocol tests pass.

```sh
python3 tools/make_pages_site.py 0.3.0 build/site
python3 tests/web_build_test.py
python3 tests/web_site_test.py build/site build/emu
python3 tools/make_pages_site.py 0.3.0 build/site-no-sharing --no-sharing
python3 tests/web_site_test.py build/site-no-sharing build/emu --no-sharing
node tests/host/emu_browser_test.mjs build/emu/amb.wasm
node tests/host/emu_http_test.mjs build/emu/amb.wasm
```

Interactive desktop/mobile layout, keyboard focus, actual audio-device and optional MIDI qualification were not performed in that W07 check. Emscripten was unavailable for rebuilding the archived API harness; its explicit runner coverage is preserved. No physical hardware, server restart or publication was performed. The W06 metadata source and generator are now integrated; use the release version in `web/product.json` for a new build.
