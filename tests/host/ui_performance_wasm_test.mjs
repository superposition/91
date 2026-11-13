// SPDX-License-Identifier: GPL-3.0-only
// Keep every byte exact to the independent C implementation on its own target.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
const [nativePath,frozenPath,c0Path,rustPath]=process.argv.slice(2);
const native=fs.readFileSync(nativePath),recordBytes=native.length/4096;
assert(Number.isInteger(recordBytes));
const output='build/ui-performance/frozen-wasm.bin',fd=fs.openSync(output,'w');
const digest=createHash('sha256'),frameDifferences=[];
let instance,offset=0,cases;
try {
 ({instance}=await WebAssembly.instantiate(fs.readFileSync(frozenPath),{env:new Proxy({emit(p,n){
  assert.equal(n,recordBytes,'one complete semantic record');
  const chunk=Buffer.from(instance.exports.memory.buffer,p,n),want=native.subarray(offset,offset+n);
  assert.equal(want.length,n,'bounded complete corpus');
  // The final u32 is the complete framebuffer hash. All earlier bytes include
  // PCM, explicit UI/project/global fields, controls and store bytes/hash.
  assert(chunk.subarray(0,n-4).equals(want.subarray(0,n-4)),`cross-target C semantic mismatch at case ${offset/n}`);
  if(!chunk.subarray(n-4).equals(want.subarray(n-4)))frameDifferences.push(offset/n);
  digest.update(chunk);fs.writeSync(fd,chunk);offset+=n;
 }},{get:(o,k)=>k in o?o[k]:()=>0})}));
 cases=instance.exports.run();
} finally {fs.closeSync(fd);}
assert.equal(cases,4096);assert.equal(offset,native.length);
assert.deepEqual(frameDifferences,[1152,3456],'frozen C target-specific settings-popup frames');
const hash=digest.digest('hex');
assert.equal(hash,fs.readFileSync('tests/reference/ui-performance-wasm.sha256','utf8').trim().split(/\s/)[0]);
console.log(JSON.stringify({path:frozenPath,cases,bytes:offset,sha256:hash,native_wasm_semantic_bytes:'exact',frozen_target_frame_hash_cases:frameDifferences}));
const result=spawnSync(process.execPath,['tests/host/project_wasm_test.mjs',output,c0Path,rustPath],{stdio:'inherit'});
assert.equal(result.status,0,'complete current C0 and Rust WASM records match frozen WASM');
