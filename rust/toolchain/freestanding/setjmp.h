/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
/* Abort-only proof: an opaque declaration for unused codegen prelude pointers.
 * No setjmp/longjmp implementation is provided; reachable unwinding must fail. */
typedef struct mrustc_unwind_unavailable jmp_buf;
