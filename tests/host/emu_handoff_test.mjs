// SPDX-License-Identifier: GPL-3.0-only
// Prove that the running Songbook UI consumes Rust copies, including all pages.
import assert from 'node:assert/strict';
import fs from 'node:fs';
let memory;
const wasi = new Proxy({
  clock_time_get: (_id,_p,out) => {new DataView(memory.buffer).setBigUint64(out,0n,true);return 0;},
  fd_write: (_fd,_iov,_n,out) => {new DataView(memory.buffer).setUint32(out,0,true);return 0;}
},{get:(object,key)=>key in object?object[key]:()=>0});
const {instance} = await WebAssembly.instantiate(fs.readFileSync(process.argv[2]),{
  wasi_snapshot_preview1:wasi,env:new Proxy({},{get:()=>()=>0})
});
const ex=instance.exports;memory=ex.memory;
if(ex._initialize)ex._initialize();
assert.equal(Object.keys(ex).length,24);
ex.web_seed(91);ex.web_master(2800);ex.web_boot();
const render=(n)=>{for(let i=0;i<n;i++)ex.web_render(128);};
render(1600);
assert(ex.web_rust_stat(14)>0,'Songbook consumes Rust-owned snapshot copies');
for(let page=0;page<4;page++) {
  const before=ex.web_rust_stat(14);
  ex.web_enc(0,1);render(30);
  ex.web_enc(4,1);render(30);
  assert(ex.web_rust_stat(14)>before,'each Songbook detail page consumes snapshots');
}
const published=ex.web_rust_stat(13),copied=ex.web_rust_stat(14);
assert.equal(published,copied,'serialized frame handoff does not contend');
assert.equal(ex.web_rust_stat(17),0,'legal staging transitions');
assert.equal(memory.buffer.byteLength,33554432);
console.log(JSON.stringify({snapshot_bytes:220,published,copied,detail_pages:4,
  staging_claims:ex.web_rust_stat(15),staging_activations:ex.web_rust_stat(16),
  staging_rejections:ex.web_rust_stat(17),memory_bytes:memory.buffer.byteLength}));
