# NAIWAN 91 web source

The served site is built by `tools/make_pages_site.py`. The landing page is authored in `web/index.html`; shared palette/shell styles are in `web/site.css`. Local font files are in `web/fonts`.

| Route | Source / responsibility |
| --- | --- |
| `/` | `web/index.html`: current product, screenshots and migration scope |
| `/emu/` | `web/emu/index.html`: panel/input/browser storage; `worklet.js`: audio and WASM messages |
| `/install/` | `web/amb_installer.html`: optional USB update, backup, restore and diagnostics |
| `/editor/` | `web/editor/index.html`: connected parameter/DX voice editor |
| `/pieces/` | `web/pieces/index.html`: legacy piece/set library and connected transfers |
| `/library/` | Compatibility redirect to `/pieces/#library` |
| `/manual/` | Generated `docs/manual.html`, with the shared site theme and a current Songbook link |

The emulator is useful without hardware and starts audio only on **Switch on**. It uses AudioWorklet on secure origins, with the same processor driven by a Web Audio callback for LAN HTTP previews. Preserve both paths. The Rust/C status labels must describe the actual build.

`web/app` is an unshipped, older AMB-style API prototype. `web/make_site.py` retains imported helper functions used by the current builder; its old command-line layout is not the NAIWAN site build. Their retirement/quarantine is tracked in [web ticket #35](https://github.com/superposition/91/issues/35).

Names inside protocol/storage formats are compatibility contracts, not branding leftovers: `.p1bk`, `.perfmap`, loader markers, model identities, numeric object IDs, parameter IDs and wire framing remain stable. Upstream author/licence attributions remain with the code. [The web refactor audit and tickets](../docs/WEB_REFACTOR.md) track the remaining work.
