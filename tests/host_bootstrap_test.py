# SPDX-License-Identifier: GPL-3.0-only
"""Exercise native host bootstrap with real poisoned parent-Cargo state."""
from pathlib import Path
import hashlib,json,os,subprocess,sys,tempfile
ROOT=Path(__file__).resolve().parents[1];BASE=Path(sys.argv[1]).resolve()
out=ROOT/'build/host-bootstrap';out.mkdir(parents=True,exist_ok=True)
platform='windows'if os.name=='nt'else'linux'
env=dict(os.environ,X0X_FONTSET='terminus')
for key in ['FM1_CODEC_CLI','FM1_TOOLING_CLI','CARGO_BUILD_TARGET','RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTC','RUSTC_WRAPPER','RUSTC_WORKSPACE_WRAPPER','RUSTDOC']:
    env.pop(key,None)
env.update(CARGO_FEATURE_C_APP='1',CARGO_FEATURE_RUST_ENGINE='1',FM1_SIM_SOURCE=str(out/'does-not-exist'))
# fm1-cli depends on fm1-sim without c-app. A parent marker must not enable it.
subprocess.run(['cargo','+1.94.1','build','--release','--locked','-p','fm1-cli','--target-dir',str(out/platform)],cwd=ROOT,env=env,check=True,timeout=180)
with tempfile.TemporaryDirectory(prefix='font-',dir=str(out))as scratch:
    scratch=Path(scratch).resolve();assert ROOT in scratch.parents
    expected=scratch/'frozen.h';actual=scratch/'rust.h'
    subprocess.run([sys.executable,str(BASE/'tools/gen_font.py'),str(expected)],cwd=BASE,env=env,check=True,timeout=60)
    # The actual generator must bootstrap a native tool even inside a foreign
    # compiler/build-script environment, then produce the unchanged header.
    env.update(RUSTC=str(scratch/'invalid-rustc'),RUSTC_WRAPPER=str(scratch/'invalid-wrapper'),
               CARGO_BUILD_TARGET='wasm32-unknown-unknown',RUSTFLAGS='--cfg parent_build_only',
               CARGO_MAKEFLAGS='invalid-parent-jobserver',MAKEFLAGS='invalid-parent-jobserver')
    subprocess.run([sys.executable,str(ROOT/'tools/gen_font.py'),str(actual)],cwd=ROOT,env=env,check=True,timeout=180)
    data=actual.read_bytes();assert data==expected.read_bytes()
    result=dict(platform=platform,poisoned_cli_graph='pass',nested_font_bootstrap='pass',font_bytes=len(data),sha256=hashlib.sha256(data).hexdigest())
    (out/(platform+'.json')).write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
