#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Reject a different compiler checkout or an incomplete/local compiler patch."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

here = Path(__file__).resolve().parent
checkout = Path(sys.argv[1]).resolve()
manifest = json.loads((here / 'mrustc-pi32v2-patch.json').read_text())
patch = (here / 'mrustc-pi32v2.patch').read_bytes().replace(b'\r\n', b'\n')
if hashlib.sha256(patch).hexdigest() != manifest['sha256']:
    raise SystemExit('The tracked mrustc patch does not match its manifest')
revision = subprocess.check_output(['git', '-C', str(checkout), 'rev-parse', 'HEAD']).decode().strip()
if revision != manifest['base']:
    raise SystemExit('Expected mrustc ' + manifest['base'] + ', found ' + revision)
actual = subprocess.check_output(['git', '-C', str(checkout), 'diff', '--no-ext-diff', '--binary', 'HEAD', '--']).replace(b'\r\n', b'\n')
if actual != patch:
    raise SystemExit('The mrustc checkout must contain exactly the tracked combined patch; apply it once to the pinned revision')
print(json.dumps(dict(compiler=revision, patch_sha256=manifest['sha256'])))
