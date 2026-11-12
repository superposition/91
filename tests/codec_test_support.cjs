// SPDX-License-Identifier: GPL-3.0-only
// Legacy CommonJS source tests execute the actual embedded WASM client.
const fs = require('node:fs'), path = require('node:path');
exports.prelude = () => fs.readFileSync(path.join(__dirname, '../build/codecs/runtime.mjs'), 'utf8').replace(/^export /gm, '') + '\n';
exports.strip = source => source.replace(/^export /gm, '').replace(/^import .*;\r?\n/gm, '');
