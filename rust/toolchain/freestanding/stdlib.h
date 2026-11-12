/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#include <stddef.h>
_Noreturn void abort(void);
_Noreturn void exit(int);
void *malloc(size_t);
void *calloc(size_t,size_t);
void *realloc(void*,size_t);
void free(void*);
int abs(int);
long labs(long);
long long llabs(long long);
