/* SPDX-License-Identifier: GPL-3.0-only */
#pragma once
#include <stdint.h>
typedef struct{int32_t n,r0,fm_on;uint32_t fm_w,pad_w;int32_t bass;uint32_t genre;int32_t float_path;} fm1_worker_job_t;
typedef void(*fm1_worker_fn)(const fm1_worker_job_t *);
enum{FM1_WORKER_IDLE,FM1_WORKER_READY,FM1_WORKER_RUNNING,FM1_WORKER_COMPLETED,FM1_WORKER_RETIRED,FM1_WORKER_PREPARING,FM1_WORKER_QUARANTINED};
enum{FM1_WORKER_BASS=1,FM1_WORKER_TANK=2,FM1_WORKER_PAD=4};
extern uint32_t fm1_rust_worker_phase(void);
extern uint32_t fm1_rust_worker_prepare(const fm1_worker_job_t *);
extern uint32_t fm1_rust_worker_execute(uint32_t,fm1_worker_fn);
extern uint32_t fm1_rust_worker_publish(uint32_t,uint32_t);
extern uint32_t fm1_rust_worker_ready(uint32_t,uint32_t);
extern uint32_t fm1_rust_worker_completed(uint32_t);
extern uint32_t fm1_rust_worker_reclaim(uint32_t);
extern uint32_t fm1_rust_worker_retire(uint32_t);
extern uint32_t fm1_rust_worker_quarantine(uint32_t);
extern void fm1_rust_worker_refused(void);
_Static_assert(sizeof(fm1_worker_job_t)==32,"Rust immutable DSP job ABI");
