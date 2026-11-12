/* SPDX-License-Identifier: GPL-3.0-only */
/* Compile unchanged against immutable R01, current C0 and the Rust adapter. */
#include <stdint.h>
#include <string.h>
#ifndef __wasm__
#include <stdio.h>
#endif
#ifndef STORAGE_SOURCE
#define STORAGE_SOURCE "../../firmware/src/storage.c"
#endif
#include "../../firmware/src/app/plat.h"
static uint8_t nor[0x100000], a[3840], b[3840], got[3842];
static uint32_t trace, reads, programs, operations;
static int fail_read=-1, fail_write=-1, fail_erase, partial, fail_code=-9;
static uint32_t hash(uint32_t h,const void *p,uint32_t n){const uint8_t *s=p;while(n--)h=(h^*s++)*16777619u;return h;}
static void event(uint32_t op,uint32_t off,uint32_t n,int rc,const void *data){uint32_t v[4]={op,off,n,(uint32_t)rc};trace=hash(trace,v,sizeof v);if(data)trace=hash(trace,data,n);operations++;}
static int st_read(uint32_t off,void *dst,uint32_t n){int rc=0;if(off>sizeof nor||n>sizeof nor-off)rc=-41;else if((int)reads==fail_read)rc=fail_code;else memcpy(dst,nor+off,n);reads++;event(0,off,n,rc,rc?0:dst);return rc;}
static int st_erase(uint32_t off){int rc=fail_erase?fail_code:0;if(off>sizeof nor-4096)rc=-42;else memset(nor+off,255,rc?(uint32_t)partial:4096);event(1,off,4096,rc,0);return rc;}
static int st_prog(uint32_t off,const void *src,uint32_t n){uint32_t i,limit=n;int rc=0;if(off>sizeof nor||n>sizeof nor-off||(off&255)+n>256)rc=-43;else {if((int)programs==fail_write){rc=fail_code;limit=partial<(int)n?(uint32_t)partial:n;}for(i=0;i<limit;i++)nor[off+i]&=((const uint8_t*)src)[i];}programs++;event(2,off,n,rc,src);return rc;}
#include STORAGE_SOURCE
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit"))) extern void persist_emit(const void *,uint32_t);
static void emit(const void *p,uint32_t n){persist_emit(p,n);}
#else
static void emit(const void *p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
static void reset_faults(void){trace=2166136261u;reads=programs=operations=0;fail_read=fail_write=-1;fail_erase=partial=0;fail_code=-9;}
static void change_header(uint32_t obj,uint32_t copy,uint32_t sequence,uint16_t slot,uint32_t reserved){st_hdr_t *h=(st_hdr_t*)(nor+st_sector(obj,copy));h->seq=sequence;h->slot=slot;h->rsv[0]=reserved;h->hcrc=st_crc32(h,28);}
static void run_case(uint32_t id,uint32_t obj,uint32_t len,uint32_t kind,uint32_t cut,uint32_t part){
    uint32_t i,values[14];int rc,n;
    memset(nor,255,sizeof nor);reset_faults();
    for(i=0;i<sizeof a;i++){a[i]=(uint8_t)(id+i*37);b[i]=(uint8_t)(id*13+i*53);}
    st_save(obj,a,len);st_save(obj,b,len);
    if(kind==1)change_header(obj,0,2,73,0x12345678); /* tie -> A, ignored slot/reserved */
    if(kind==2){change_header(obj,0,0xffffffffu,0,0);change_header(obj,1,0xfffffffeu,1,0);} /* wrapping save */
    if(kind==3&&len)nor[st_sector(obj,1)+256+len/2]^=8;
    if(kind==4)nor[st_sector(obj,1)+28]^=1;
    if(kind==5){nor[st_sector(obj,0)]=0;nor[st_sector(obj,1)]=0;}
    if(kind==6&&len){nor[st_sector(obj,0)+256]^=1;nor[st_sector(obj,1)+256]^=2;}
    reset_faults();
    if(kind==7){fail_write=(int)cut;partial=(int)part;}
    if(kind==8){fail_erase=1;partial=(int)part;}
    if(kind==9){fail_read=(int)cut;fail_code=3;} /* nonzero positive hook result */
    if(kind==10){fail_write=(int)cut;fail_code=17;partial=(int)part;}
    if(kind==11){fail_read=(int)cut;fail_code=-6;}
    rc=st_save(obj,a,len);
    values[0]=id;values[1]=obj;values[2]=len;values[3]=kind;values[4]=cut;values[5]=part;values[6]=(uint32_t)rc;
    values[7]=trace;values[8]=operations;values[9]=reads;values[10]=programs;
    reset_faults();memset(got,0x69,sizeof got);
    n=st_load(obj,got+1,(id%5==0)?len/2:3840);
    values[11]=(uint32_t)n;values[12]=hash(2166136261u,got,sizeof got);
    values[13]=hash(2166136261u,nor+st_sector(obj,0),8192);
    emit(values,sizeof values);
}
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int persistence_run(void){
    static const uint32_t lengths[]={0,1,31,255,256,257,600,3836,3840};
    uint32_t id=0,o,l,k,c,p,v[8];
    for(o=0;o<31;o++)for(l=0;l<9;l++)for(k=0;k<12;k++){
        uint32_t cuts=(k==7||k==10)?18:(k==9||k==11)?8:1;
        for(c=0;c<cuts;c++)for(p=0;p<((k==7||k==8||k==10)?4u:1u);p++)
            run_case(id++,o,lengths[l],k,c,(uint32_t[]){0,1,17,256}[p]);
    }
    reset_faults();v[0]=st_save(0,a,3841);v[1]=operations;v[2]=id;v[3]=sizeof(st_hdr_t);
    v[4]=(uint32_t)st_save(0,0,0);v[5]=(uint32_t)st_load(0,0,0);v[6]=trace;v[7]=hash(2166136261u,nor+st_sector(0,0),8192);emit(v,sizeof v);
    return (int)id;
}
#ifndef __wasm__
int main(void){persistence_run();return 0;}
#endif
