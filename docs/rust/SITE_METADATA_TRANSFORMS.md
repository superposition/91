# Site and editor metadata transforms

This checkpoint moves the live editor-data and current static-site text
transforms into safe host Rust. It preserves the Spectrum templates, CSS,
screenshots, routes and musician-facing copy as source data. It does not deploy
a site or change the browser instrument.

`tools/make_params.py` now calls `fm1_tooling::assets::metadata` for comment
stripping, parameter enum expressions, UI table extraction, tab assembly,
unplaced-parameter fallback, compact parameter JSON, device metadata and report
counts. The tab recipe is `tools/data/parameter-tabs.json`: bounded data
operations for groups, labels, indexed parameters, filtering and repetition.
The recipe is not evaluated by Python. The Rust evaluator has a 20,000-operation
budget and depth 96; source enum arithmetic has depth 64 and checked i64 values.
The source grammar admits identifiers, decimal/hex integers, parentheses, unary
signs, addition, subtraction and multiplication. It covers the actual frozen and
current enums; arbitrary Python expressions from the old `eval` are deliberately
not executed. Unsupported expressions fail explicitly.

Numeric descriptors are emitted from `fm1_types::PARAMETERS`; FM6 routing comes
from `fm1_engine::fm6::tables::FM6_ALG`. Every supplied C numeric descriptor and
algorithm is checked before delivery. The existing C `gparams` executable still
provides value/display names and the aggregate genre/piece/object/backup snapshot.
Its compilation and process/file delivery remain Python. This is an explicit
transitional C metadata seam, not a replacement name table. The C header enum and
UI table source remain parsed inputs; this checkpoint does not claim they were
replaced by a Rust application metadata registry.

The actual `tools/make_pages_site.py`, `tools/web_site_support.py`,
`tools/web_metadata.py` and retired `web/make_site.py` import path use
`fm1_tooling::assets::site` for ESM stripping, release identity, redirects,
ordered template replacement, manual styling/link removal, PDF card ordering,
sharing-link removal, landing placeholders, Python-compatible JSON formatting,
device JavaScript assembly and codec-runtime freshness/embedding policy.
`tools/data/site-copy.json` holds the five original inline page fragments
verbatim. `tools/rust_site.py` only transports typed values and maps errors;
`web_metadata` supplies synchronous file/JSON delivery callbacks.

The runtime source path preserves the old order: manifest delivery, source hash,
first WASM read/hash, loader text delivery, then a second WASM read for embedding.
Tests cover the historical second-read growth behavior and failures at each
stage. The source-file inventory/hash implementation in root-owned
`tools/build_codecs.py` remains Python and is called through the adapter.

Text and output are bounded at 16 MiB; replacement expansion is checked before
allocation. Metadata permits at most 1,024 parameters/enum entries and 256 UI
groups. JSON preserves insertion order, explicit integer values, Python ASCII
escaping including UTF-16 surrogate pairs, indentation and optional sorted keys.
It supports valid Unicode scalar strings; it does not admit lone surrogate
strings or general floating-point metadata. Release decimal normalization uses
the Python 3.10 Unicode Nd profile. The unchanged release artifacts are ASCII.
The old Python word/whitespace conventions are retained for active regexes,
including combining marks and information-separator whitespace.

The filesystem staging implementation (`output_path`, inventory, ownership,
legacy adoption, backup/rollback and recursive cleanup) is still Python, as are
CLI argument/file-copy orchestration and package preflight. These are the next
site policy slice. The transform checkpoint keeps their safety checks active;
it does not claim all of ticket #22 is complete. No firmware, UI, controller,
HAL, common bootstrap or root-owned build gates were modified.

## Qualification

Run on Linux with pinned Rust 1.94.1, stdlib Python 3.10+, a C99 compiler and Node:

```sh
bash tests/rust_site_metadata.sh /path/to/immutable/R01/source
```

The runner builds the real current C exporter and an independent R01 exporter,
then runs the actual `make_params.py --check`. All 176 descriptor/value-name
rows match R01. Historical genre/piece data also matches; current C0 adds the
already-existing object/backup metadata. The complete R01 parameter layout is
compared after that explicit metadata addition is accounted for.

`tests/site_metadata_differential.py` checks all 440 raw R01 source hashes and
the current C0 builder pins from `fa187ac`. The current C0 scripts are exact Git
LF blobs; their initial Windows CRLF fingerprints are separately recorded.
The suite exercises the actual Python APIs and `main`, native commands,
randomized grammar, Unicode, JSON, table zip/filter/fallback semantics and drift
refusals. `tests/site_build_differential.py` runs both real current builder
entrypoints for all 16 sharing/PDF-presence profiles, compares every generated
file byte/hash, and runs the current route/control/asset/subpath/Node assertions.
It also tests dirty output, missing inputs, retired CLI and runtime error/order
behavior. The obsolete R01 website design is not used as a presentation target.

Full builds use explicit copy-only WASM, firmware and PDF byte fixtures. They do
not instantiate an engine, render/inspect a PDF, test audio, start a server or
publish anything. Browser/audio and physical qualification remain separate.
Windows Python 3.8 retains the original `Path.write_text(newline=...)` failure;
the full Windows fixture build uses a test-only Python 3.10-compatible writer.
Linux executes the unmodified actual entrypoints directly.

The pinned run passes 1,219 metadata/grammar cases on each host, 51 Linux site
cases and 52 Windows site cases (the extra case is the Python 3.8 failure
fingerprint). It includes the 16 complete build profiles plus a PDF-arrival
profile and exact runtime read/error order. The isolated workspace passes 201
Rust tests; Windows tooling passes 23 tests; formatting and strict workspace
Clippy pass. Raw artifact traces describe the recorded worktree inputs and host
newline behavior. CI checks C0/Rust equivalence rather than asserting one
platform-independent site artifact hash.

Useful native commands accept explicit files and never open hardware:

```text
fm1-rs tools editor-metadata INPUT_JSON PARAMS_JSON DEVICE_JS
fm1-rs tools site-transform METHOD INPUT_JSON OUTPUT_TEXT
```

The editor input contains `header`, `ui`, `fm6`, `params`, `names` and `product`.
The site input is the chosen method's argument array. Existing loader/LZ4,
DX/logo, manual, installer and diagnostic commands remain registered.
