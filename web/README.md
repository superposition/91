# NAIWAN 91 web source

The served site is built by `tools/make_pages_site.py`. The landing page is authored in `web/index.html`; shared palette/shell styles are in `web/site.css`. Local font files are in `web/fonts`.

| Route | Source / responsibility |
| --- | --- |
| `/` | `web/index.html`: songs, playing controls and screenshots |
| `/emu/` | `web/emu/index.html`: player interface; `web/emu/src/*.mjs`: browser bindings; `worklet.js`: audio and WASM messages |
| `/install/` | `web/amb_installer.html`: optional USB update, backup, restore and diagnostics |
| `/editor/` | `web/editor/index.html`: connected legacy parameter/DX voice editor; Songbook packs use the instrument pages and pack tools |
| `/pieces/` | `web/pieces/index.html`: legacy piece/set library and connected transfers |
| `/library/` | Compatibility redirect to `/pieces/#library` |
| `/manual/` | Generated `docs/manual.html`, with the shared site theme and a current Songbook link |
| `/webapp/installer/`, `/webapp/editor/` | Relative redirects preserving the historical installer/editor addresses, including query and fragment |

The emulator is useful without hardware and starts audio only on **Switch on**. It uses AudioWorklet on secure origins, with the same processor driven by a Web Audio callback for LAN HTTP previews. Preserve both paths. Public copy describes playing the instrument; engineering telemetry stays available through `window.ambLast`.

[Browser modules and lifecycle](../docs/WEB_EMULATOR_MODULES.md) describes mounting, audio startup/cancellation, input subscriptions, MIDI and stream cleanup, and targeted checks. `web/emu/package.sh` copies the same modules into standalone and full-site builds; no bundler or external JavaScript service is needed.

`web/app` is an unshipped AMB-style API regression harness. It requires `sh web/app/build.sh --legacy-test`, writes only `build/legacy-app-test`, and remains covered by the legacy test runner. It is never copied into the site. Its source and upstream credits are retained. `web/make_site.py` now exits without writing anything; compatibility imports forward its two helpers to the maintained `tools/web_site_support.py`.

Build with Python 3.10 or newer:

```sh
python3 tools/make_pages_site.py 0.4.4 build/site
python3 tests/web_build_test.py
python3 tests/web_site_test.py build/site build/emu
```

Build the firmware, emulator, browser codecs, parameter data and manual first with `tools/release.sh 0.4.4`. See [build prerequisites](../BUILDING.md). `web/product.json` is the release metadata source; `python3 tools/make_params.py` regenerates `web/device-data.js`, and `--check` rejects drift. Product release versions are separate from wire/storage format versions.

The full site shares root CSS, fonts and the icon once; standalone `build/emu` retains its own assets. All paths work below a host subpath. `site-build.json` records the release identity, firmware hash and a deterministic inventory; it records a build, not a deployment. Existing generated outputs are verified before replacement. Build into a fresh directory below `build/`, or use `--adopt-existing` once for a verified older pre-manifest bundle. That flag recognizes the old canonical routes and exact redundant emulator assets, rejects unknown files, and verifies every path against the new build before replacement. It retains the entire old directory beside the output and reports its location, preserving edits in known legacy filenames. It never adopts arbitrary nonempty directories. Local changes to manifest-owned files also require a fresh output directory.

The builder stages separately and restores the previous generated output if replacement fails. Repository roots, `build/` itself, build inputs, nested sites, and symlink/junction outputs are rejected. [Build qualification and remaining browser checks](../docs/WEB_BUILD.md) are tracked in [ticket #35](https://github.com/superposition/91/issues/35).

Names inside protocol/storage formats are compatibility contracts, not branding leftovers: `.p1bk`, `.perfmap`, loader markers, model identities, numeric object IDs, parameter IDs and wire framing remain stable. Upstream author/licence attributions remain with the code. [The web refactor audit and tickets](../docs/WEB_REFACTOR.md) track the remaining work.
