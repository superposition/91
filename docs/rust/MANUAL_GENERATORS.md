# Manual generation policy (#22 checkpoint)

The existing `manual_html.py`, `make_quick_manual.py` and `make_manual_pdf.py`
entry points now execute parsing, page composition and PDF workflow policy in
safe `fm1-tooling` Rust. `rust_manual.py` binds file operations, console output,
Pypdf extraction, clocks and subprocess execution. No Edge process or actual PDF
was created or rendered for this checkpoint.

## Inputs and HTML

`tools/data/manual.css`, `quick-manual.css` and `manual-copy.json` retain the
current styles and copy. Markdown remains the source of manual content. The
current Python C0 scripts at `0f0d4a1` are byte-identical to immutable R01;
`tests/reference/manual_current_c0/manifest.json` pins those exact bytes without
keeping a second Python implementation. Tests exercise both R01 and current
`MANUAL.md`, `QUICK.md` and `GUIDE.md` inputs.

The full and quick manuals keep their separate grammars. The full converter
retains contents links, numbered slugs, figures/groups, details/callouts, table
swatches and its original inline replacement order. The quick converter keeps
protected code-span placeholders, its different table-header rule, raw HTML,
page delimiters, footer extraction and one-time font substitution. Existing
Python APIs, default configuration and `EMBED`/`IMG_BASE` state publication remain.
Figures still use the original MIME/class rules and URI/path operations.

Pinned host-only `fancy-regex=0.16.2` implements the small existing regex grammar;
this does not switch to another Markdown dialect. The normalizer uses Python's
word categories (`L`, `N`, underscore), decimal category `Nd`, whitespace with
U+001C..U+001F, and the original CRLF/Unicode splitlines rules. Tests include
composed/decomposed accents, combining marks, numeric word characters and extra
control whitespace. Slugs preserve captured decimal text. Generated code-span
indices are ASCII; externally injected NUL/non-ASCII index tokens are outside
that internal sentinel profile. Regex category tables come from the pinned
Rust dependency, rather than reproducing every Python Unicode database version.

`Output` tracks emitted byte length while collecting blocks. Checked joins also
bound figure groups, lists, tables and page composition before repeated content
can amplify an input into an unbounded allocation. A bounded cache holds at most
256 compiled regexes, with a 1,000,000-step backtracking limit.

## PDF boundary and ordering

Rust constructs the exact Edge arguments, removes a preexisting output, requests
the original unchecked 120-second subprocess call, then polls 120 times with
0.5-second sleeps. The first sleep precedes the first existence check. Any two
equal positive size observations satisfy the original stability rule; a missing
intervening observation leaves the prior size intact. Timeout/error propagation
and the final SystemExit message remain. A nonzero Edge exit alone is ignored.

Pypdf opening, page access and extraction are adapter primitives. Rust chooses
the raw-byte fallback when the import is unavailable, counts pages, takes each
page's first text, sorts visitor items, computes transformed Y coordinates and
measures section height. It preserves tuple tie ordering, signed-zero equality,
continuation-page accumulation, footer selection, arithmetic association,
rounding and the `267 mm` overflow decision. Constructor/extraction errors retain
their original exception and partial printed prefix. Missing Pypdf during
`--measure` retains its original explicit failure.

The two byte-count profiles remain distinct: quick fallback uses an ASCII word
boundary after `/Page`; full reporting consumes one following non-`s` byte and
therefore excludes an unfinished `/Page` at EOF. Binary tests cover every possible
following byte and adjacent matches. A whole PDF crosses the adapter as bounded
hex bytes to accommodate ordinary document sizes within the framed protocol;
counting and report decisions remain Rust code.

No mutable document or geometry borrow is exposed to a Python callback. Requests
and extraction results are owned values; callbacks perform synchronous primitive
I/O. The regex cache lock is released before any callback or nested parser call.
Each PDF call has an independent adapter/reader; `rust_tooling` rejects mutable
transaction reentry.

## Commands and supported bounds

```sh
cargo +1.94.1 build --release --locked -p fm1-cli
target/release/fm1-rs tools manual-body INPUT.md OUTPUT.html
target/release/fm1-rs tools quick-html INPUT.md OUTPUT.html
target/release/fm1-rs tools pdf-count RAW-PDF [--full]
python3 -I tools/manual_html.py INPUT.md OUTPUT.html [--fragment]
python3 -I tools/make_quick_manual.py INPUT.md --html-only
python3 tools/make_manual_pdf.py [--edge PATH]
```

The new native commands write UTF-8/LF and use Python-style universal newline
normalization for file input. `manual-body` emits the full converter's body;
`quick-html` emits its complete quick document with relative image URLs, matching
the quick API's default image base. The retained Python CLI keeps its original
platform newline behavior and resolved image URLs. Its lazy native binding can
also use `FM1_TOOLING_CLI` / `FM1_CODEC_CLI`.

Python 3.10 is the maintained CLI proof runtime. On this workstation's Windows
Python 3.8, the original quick CLI fails at `Path.write_text(newline=...)`; that
failure fingerprint is preserved. The native commands work there. Deterministic
filesystem doubles also exercise complete main/PDF flows under the pinned
Python 3.10 file API semantics on both platforms.

The host frame limit remains 32 MiB. Text and emitted HTML are bounded to 16 MiB;
PDF byte transport to 12 MiB, images to the existing 1,100,000-byte blob limit,
page count to 4,096, visitor records to 100,000 per page and finite coordinate
magnitudes to 10^12. Larger inputs receive explicit refusals. These are std host
tools, with no new MCU/WASM dependency or public ABI change.

## Evidence and remaining scope

`bash tests/rust_manual.sh FROZEN_SOURCE` needs Python stdlib and Rust 1.94.1.
It requires no Pypdf, Pillow, Edge, browser, MIDI, audio device or C application.
`PYTHON` can select an interpreter. Platform-specific Cargo target directories
keep Windows and Linux artifacts separate.

The HTML suite has 996 exact cases, including isolated real script processes
that write only scratch outputs. The PDF suite has 855 exact cases with fake
file growth, clocks, subprocesses and page extraction: zero/absent/changing
files, deletion, timeouts, subprocess/library errors, both count profiles, page
text, transformed geometry, continuation pages, numeric/rounding thresholds,
argument/environment precedence and printed-prefix order. All 440 R01 source
hashes and the current-C0 pins are checked first. The machine-readable proof
records host identities, sources, binaries, repeat hashes and workspace checks.

This qualifies source and policy parity. It does not measure page layout with a
particular Edge/font installation or claim a newly rendered eight-page PDF.
No production document, web asset, site or device was modified. Whole #22 remains
open for the root-owned build/emitter, site, parameter and bootstrap work.
