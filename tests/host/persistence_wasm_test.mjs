// SPDX-License-Identifier: GPL-3.0-only
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const expected=readFileSync('build/persistence/c0.bin');
for(const path of process.argv.slice(2)) {
  const chunks=[];
  let instance;
  ({instance}=await WebAssembly.instantiate(readFileSync(path),{env:{emit(p,n){chunks.push(Buffer.from(new Uint8Array(instance.exports.memory.buffer,p,n)));}}}));
  const cases=instance.exports.run();
  const actual=Buffer.concat(chunks);
  assert.deepEqual(actual,expected,path);
  console.log(JSON.stringify({path,cases,bytes:actual.length,sha256:createHash('sha256').update(actual).digest('hex')}));
}
