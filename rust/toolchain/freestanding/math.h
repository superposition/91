/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#define INFINITY __builtin_inff()
#define NAN __builtin_nanf("")
float sqrtf(float); double sqrt(double);
float floorf(float); double floor(double);
float ceilf(float); double ceil(double);
float truncf(float); double trunc(double);
float roundf(float); double round(double);
float fabsf(float); double fabs(double);
float fmodf(float,float); double fmod(double,double);
float remainderf(float,float); double remainder(double,double);
float copysignf(float,float); double copysign(double,double);
float fmaf(float,float,float); double fma(double,double,double);
float powf(float,float); double pow(double,double);
float sinf(float); double sin(double);
float cosf(float); double cos(double);
float expf(float); double exp(double);
float exp2f(float); double exp2(double);
float logf(float); double log(double);
float log2f(float); double log2(double);
float log10f(float); double log10(double);
float nearbyintf(float); double nearbyint(double);
float rintf(float); double rint(double);
