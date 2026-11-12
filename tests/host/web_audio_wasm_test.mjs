// SPDX-License-Identifier: GPL-3.0-only
import assert from 'node:assert/strict';
import fs from 'node:fs';
import crypto from 'node:crypto';
const [oracle,...modules]=process.argv.slice(2);
const expected=crypto.createHash('sha256').update(fs.readFileSync(oracle)).digest('hex');
const expectedBytes=fs.statSync(oracle).size;
for(const path of modules){
  let instance,bytes=0;const hash=crypto.createHash('sha256');
  const module=new WebAssembly.Module(fs.readFileSync(path));
  assert.deepEqual(WebAssembly.Module.imports(module),[{module:'env',name:'write',kind:'function'}]);
  instance=new WebAssembly.Instance(module,{env:{write(pointer,length){
    hash.update(Buffer.from(instance.exports.memory.buffer,pointer,length));bytes+=length;
  }}});
  assert.equal(instance.exports.main(),0);
  assert.equal(bytes,expectedBytes);
  const sha256=hash.digest('hex');assert.equal(sha256,expected);
  console.log(JSON.stringify({path,ring_cases:4096,background_colors:65536,bytes,sha256}));
}
