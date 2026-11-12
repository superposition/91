# SPDX-License-Identifier: GPL-3.0-only
"""Portable build-policy differential runner; no hardware or vendor compiler."""
from pathlib import Path
import os,subprocess,sys
root=Path(__file__).resolve().parents[1]
out=root/'build/build-policy'/('windows'if os.name=='nt'else'linux')
subprocess.run(['cargo','+1.94.1','build','--release','--locked','-p','fm1-cli','--target-dir',str(out)],cwd=root,check=True)
binary=out/'release'/('fm1-rs.exe'if os.name=='nt'else'fm1-rs')
subprocess.run([sys.executable,str(root/'tests/build_policy_differential.py'),str(Path(sys.argv[1]).resolve()),str(binary)],cwd=root,check=True)
