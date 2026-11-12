/* SPDX-License-Identifier: GPL-3.0-only */
/* Compile unchanged against verified immutable R01 and the current consumers. */
#define EN_WEB 1
#include <stdint.h>
static void web_audio(const int32_t *b,uint32_t n){(void)b;(void)n;}
#include "host/amb_host.c"
#undef __attribute__
static uint32_t hbytes(uint32_t h,const void *b,unsigned n){const uint8_t *p=b;while(n--)h=(h^*p++)*16777619u;return h;}
static uint32_t hslot(uint32_t h,const slot_t*s){h=hbytes(h,s->par,sizeof s->par);h=hbytes(h,&s->pool,2);h=hbytes(h,s->pats,sizeof s->pats);h=hbytes(h,s->mod,sizeof s->mod);h=hbytes(h,s->acl,18);h=hbytes(h,s->cml,12);h=hbytes(h,s->prg,6);h=hbytes(h,s->prk,3);return hbytes(h,s->bs,16);}
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit"))) extern void project_emit(const void*,uint32_t);
static void emit(const void*p,unsigned n){project_emit(p,n);}
#else
static void emit(const void*p,unsigned n){fwrite(p,1,n,stdout);}
#endif
static unsigned cases;
static void witness(unsigned family,unsigned index,int rc){
 uint32_t v[16],h=2166136261u;unsigned i;
 h=hbytes(h,proj.par,sizeof proj.par);h=hbytes(h,&proj.pool,2);h=hbytes(h,proj.acl,18);h=hbytes(h,proj.cml,12);h=hbytes(h,proj.prg,6);h=hbytes(h,proj.prk,3);h=hbytes(h,proj.bs,16);h=hbytes(h,&proj.leds,3);h=hbytes(h,&proj.edited,2);
 for(i=0;i<10;i++)h=hslot(h,&proj.slot[i]);v[0]=family;v[1]=index;v[2]=(uint32_t)rc;v[3]=h;
 v[4]=hbytes(2166136261u,image,sizeof image);v[5]=hbytes(2166136261u,&glob,sizeof glob);v[6]=hbytes(2166136261u,px,sizeof px);v[7]=hbytes(2166136261u,pxl,sizeof pxl);
 h=2166136261u;for(i=0;i<OBJ_NOBJ;i++){h=hbytes(h,&store_len[i],sizeof store_len[i]);if(store_len[i]>0)h=hbytes(h,store[i],(unsigned)store_len[i]);}v[8]=h;v[9]=saved;v[10]=pending|(uint32_t)pending_glob<<8|(uint32_t)pending_px<<16;v[11]=bs_dirty;v[12]=store_writes;v[13]=glob_from_old;v[14]=hbytes(2166136261u,(const void*)en_pats,sizeof en_pats);v[15]=hbytes(2166136261u,(const void*)en_mod,sizeof en_mod);emit(v,sizeof v);cases++;
}
static void clean(uint32_t seed){unsigned i;memset(store,0,sizeof store);for(i=0;i<OBJ_NOBJ;i++)store_len[i]=-1;memset(&glob,0,sizeof glob);glob_from_old=0;saved=0;pending=pending_glob=pending_px=0;bs_dirty=0;store_lock=0;store_writes=0;set_have=0;memset(&set_disk,0,sizeof set_disk);host_seed=seed;now_ms=0;boot();en_play(0);}
static void edit(unsigned seed){unsigned i,l;for(i=0;i<P_NPARAMS;i++){const en_param_t *pi=en_param_info(i);proj.par[i]=(int16_t)(pi->lo+(int)((seed+i*17)%(unsigned)(pi->hi-pi->lo+1)));}proj.pool=0xab5;for(i=0;i<6;i++)for(l=0;l<6;l++)en_pats[i][l]=(uint16_t)(seed+i*701+l*109);for(i=0;i<6;i++)for(l=0;l<16;l++)en_mod[i][l]=(uint8_t)(seed+i*37+l*53);for(i=0;i<6;i++)proj.prg[i]=(uint8_t)(seed+i);for(i=0;i<3;i++)proj.prk[i]=(uint8_t)(seed+i*11);for(i=0;i<16;i++)proj.bs[i]=(uint8_t)(seed+i*19);en_bass_steps_set(proj.bs);proj.edited=1023;}
#if defined(__wasm__) && !defined(PROJECT_REFERENCE_LIBRARY)
__attribute__((export_name("run")))
#endif
int project_run(void){unsigned g,p,i,n,k;uint8_t raw[3840];
 for(g=0;g<19;g++)for(p=0;p<10;p++){
  clean(0);project_set_genre((int)g);project_piece((int)p);edit(g*101+p*17);project_piece_save();project_globals();witness(0,g*10+p,0);witness(1,g*10+p,project_flush());
  project_piece((int)p+1);project_piece((int)p);witness(2,g*10+p,0);
  project_set_genre((int)g+1);project_set_genre((int)g);witness(3,g*10+p,0);
  project_piece_factory();witness(4,g*10+p,project_flush());
 }
 for(i=0;i<256;i++){
  clean(0);glob.rsv=(int16_t)(i%191 | (i&1?256:0));glob.sl_on=(uint8_t)i;for(k=0;k<16;k++)glob.sl[k]=(uint8_t)(i+k*13);project_globals();project_flush_globals();host_seed=i*7919u+1;boot();witness(5,i,0);
 }
 for(n=0;n<=574;n++){
  clean(0);memset(store[OBJ_KINDS],0,sizeof store[OBJ_KINDS]);wr32(store[OBJ_KINDS],KND_MAGIC);for(i=4;i<n;i++)store[OBJ_KINDS][i]=(uint8_t)(i*37+n);store_len[OBJ_KINDS]=(int)n;
  proj.genre=(uint8_t)(n%19);image_load(proj.genre);witness(6,n,px_save());
 }
 for(n=0;n<3050;n+=7){
  clean(0);for(i=0;i<3048;i++)raw[i]=(uint8_t)(i*37+n);wr32(raw,PX_MAGIC);raw[4]=16;memcpy(store[OBJ_KINDS],raw,3048);store_len[OBJ_KINDS]=(int)(n>3048?3048:n);proj.genre=(uint8_t)(n%19);image_load(proj.genre);witness(7,n,px_save());
  wr32(raw,BS_MAGIC);memcpy(store[OBJ_BSTEP],raw,3048);store_len[OBJ_BSTEP]=(int)(n>3048?3048:n);saved=0x155;bs_load(proj.genre);bs_dirty=0x3ff;witness(8,n,bs_save());
 }
 for(i=0;i<5;i++)for(k=0;k<5;k++){
  unsigned count=(unsigned[]){1,73,76,98,154}[i],stride=count*2+74;
  clean(0);memset(raw,0,sizeof raw);wr32(raw,BANK_MAGIC);wr32(raw+4,1);wr16(raw+8,count);wr16(raw+10,0xffff);wr16(raw+12,5);wr16(raw+14,1);wr16(raw+16,4);wr16(raw+18,k);
  for(p=0;p<10;p++){uint8_t*q=raw+20+p*stride;for(n=0;n<count;n++)wr16(q+n*2,(unsigned)en_param_info(n)->def);if(count>76)wr16(q+76*2,p%5);wr16(q+count*2,0xab5);for(n=0;n<36;n++)wr16(q+count*2+2+n*2,p*791+n*109);}
  memcpy(store[0],raw,20+10*stride);store_len[0]=(int)(20+10*stride);image_load(0);witness(9,i*5+k,0);witness(10,i*5+k,project_flush());
 }
 for(i=0;i<128;i++){
  clean(0);edit(i);project_piece_save();project_flush();raw[0]=(uint8_t)i;
  if(i%5==0)store[0][16+74+i%176]=254;else if(i%5==1)store[0][12]=0;else if(i%5==2)store[0][13]=0;else if(i%5==3)store_len[0]=3776;else store[0][5]=3;
  image_load(0);witness(11,i,0);
 }
 for(i=0;i<32;i++){
  clean(0);edit(i);project_piece_save();project_globals();store_lock=1;witness(12,i,project_flush());
  pending=pending_px=pending_glob=0;bs_dirty=1;witness(13,i,bs_save());store_lock=0;witness(14,i,project_flush());
 }
 for(i=0;i<16;i++){clean(0);memset(raw,0,sizeof(old_kit_t));wr32(raw,OLD_KIT_MAGIC);raw[OLD_KIT_THEME]=(uint8_t)i;memcpy(store[0],raw,sizeof(old_kit_t));store_len[0]=sizeof(old_kit_t);image_load(0);witness(15,i,0);}
 for(i=0;i<256;i++){clean(0);glob.magic=GLOB_MAGIC;glob.rsv=(int16_t)(i|256);glob.tune=(int16_t)(i-100);glob.theme=(int16_t)(i%7);for(k=0;k<16;k++)glob.sl[k]=(uint8_t)(i+k);memcpy(store[4],&glob,32);store_len[4]=i&1?12:32;memset(&glob,0,sizeof glob);glob_load();witness(16,i,0);}
 {uint32_t v[2]={cases,sizeof(project_t)};emit(v,sizeof v);}return (int)cases;
}
#if !defined(__wasm__) && !defined(PROJECT_REFERENCE_LIBRARY)
int main(void){project_run();return 0;}
#endif
