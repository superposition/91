# Historical browser API prototype

This directory contains an older AMB-style API frontend retained by `tests/host/app_test.mjs`. It is not copied into the NAIWAN site. The supported browser instrument is `web/emu`, built into `/emu/` by `tools/make_pages_site.py`.

The default build entrypoint exits with the supported player/site commands. Historical regression builds require `sh web/app/build.sh --legacy-test` and go to `build/legacy-app-test`, keeping their output separate from the current player. `tests/run_tests.sh` retains that explicit coverage when Emscripten is available. Do not publish this directory or treat it as the current interface.

Its source history and authorship are retained. [Web ticket #35](https://github.com/superposition/91/issues/35) separately tracks interactive browser qualification.
