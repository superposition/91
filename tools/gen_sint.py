#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Print the engine's Rust-generated sine table; no floating-point policy here."""
import sys
import rust_tooling

if __name__ == "__main__":
    sys.stdout.write(rust_tooling.call("asset.sine"))
