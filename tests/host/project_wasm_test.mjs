// SPDX-License-Identifier: GPL-3.0-only
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const expected=readFileSync(process.argv[2]);
for(const path of process.argv.slice(3)){
 let instance,offset=0;const hash=createHash('sha256');
 ({instance}=await WebAssembly.instantiate(readFileSync(path),{env:new Proxy({emit(p,n){
  const chunk=Buffer.from(instance.exports.memory.buffer,p,n);
  assert(offset+n<=expected.length,`${path}: output exceeds ${expected.length} bytes at ${offset}`);
  const want=expected.subarray(offset,offset+n);
  if(!chunk.equals(want)){
   let i=0;while(chunk[i]===want[i])i++;
   throw new Error(`${path}: first differing byte at ${offset+i}: actual ${chunk[i]}, expected ${want[i]}`);
  }
  hash.update(chunk);offset+=n;
 }},{get:(o,k)=>k in o?o[k]:()=>0})}));
 const cases=instance.exports.run();
 assert(cases>=0,'production witness/guard checks pass');
 assert.equal(offset,expected.length,`${path}: complete output`);
 console.log(JSON.stringify({path,cases,bytes:offset,sha256:hash.digest('hex')}));
}
