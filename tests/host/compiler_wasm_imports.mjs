// SPDX-License-Identifier: GPL-3.0-only
import fs from 'node:fs';
import assert from 'node:assert/strict';
const module=await WebAssembly.compile(fs.readFileSync(process.argv[2]));
const imports=WebAssembly.Module.imports(module),exports=WebAssembly.Module.exports(module);
assert.deepEqual(imports,[],'unimplemented target imports cannot become zero-returning environment stubs');
assert.equal(exports.length,24,'original bounded browser export surface');
console.log(JSON.stringify({imports,exports:exports.length}));
