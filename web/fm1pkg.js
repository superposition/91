// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Leo Kuroshita (@kurogedelic), Hügelton Instruments
//
// FM-1 .fwsc packages in the browser: the package identity and the image the
// device reads during an update.

import { getCodecs } from "./codec-runtime.mjs";
export function productOf(fwsc) { return String.fromCharCode(...getCodecs().call(14, [], [fwsc])); }
export function logicalImage(fwsc) { return getCodecs().call(15, [], [fwsc]); }
