/* SPDX-License-Identifier: GPL-3.0-only */
/* Zig's optimized C mode defines NDEBUG. Test callers must retain all checks. */
#ifdef NDEBUG
#error "Generated-code compatibility fixtures require active assertions"
#endif
#include <assert.h>
