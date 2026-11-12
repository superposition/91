/* SPDX-License-Identifier: GPL-3.0-only */
#define PROJECT_REFERENCE_LIBRARY 1
#include "project_reference.c"
static void ui_witness(unsigned family,unsigned index,int rc){
 uint8_t values[11];jp_settings_t settings;uint32_t v[18];unsigned i;uint32_t h=2166136261u;
 witness(family,index,rc);for(i=0;i<11;i++)values[i]=*uiv((int)i);v[0]=hbytes(h,values,sizeof values);v[1]=hbytes(h,ui.fxs,EN_NFXS);v[2]=hbytes(h,&set_disk,sizeof set_disk);v[3]=(uint32_t)set_have;
 v[4]=ui.viz|(uint32_t)ui.min_ch<<8|(uint32_t)ui.min_pads<<16|(uint32_t)ui.cc_mode<<24;
 v[5]=ui.usb_fixed|(uint32_t)ui.fm_dy<<8|(uint32_t)ui.scan<<16|(uint32_t)ui.centre_imp<<24;
 v[6]=ui.dj_rounds|(uint32_t)ui.dj_ramp<<8|(uint32_t)ui.intro<<16|(uint32_t)ui.midi_map<<24;
 v[7]=ui.gx_pat|(uint32_t)ui.gx_preset<<16|(uint32_t)ui.gx_tgt<<24;v[8]=ui.pm_tgt;
 v[9]=ui.vox_dirty|(uint32_t)ui.set_dirty<<8|(uint32_t)ui.set_lazy<<16|(uint32_t)ui.store_wait<<24;v[10]=ui.store_kind;
 v[11]=(uint32_t)en_user_count();h=2166136261u;for(i=0;i<(unsigned)en_user_count();i++)h=hbytes(h,en_user_get((int)i),128);v[12]=h;
 en_jam_get(&settings);v[13]=hbytes(2166136261u,&settings,sizeof settings);v[14]=(uint32_t)en_jam_source();v[15]=jam_boot_on;v[16]=store_lock;v[17]=(uint32_t)proj.par[P_TEMPO];emit(v,sizeof v);
}
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int persistence_ui_run(void){unsigned nf,k,i,n,p,mode;uint8_t raw[3840];
 clean(0);
 for(nf=0;nf<=EN_NFXS;nf++)for(mode=0;mode<4;mode++){
  clean(0);memset(raw,0,sizeof raw);wr32(raw,SET_MAGIC);wr16(raw+4,nf);wr16(raw+6,12);
  for(i=0;i<nf+16;i++)raw[8+i]=(uint8_t)(i*37+nf*53);p=24+nf;p+=p&1;wr16(raw+p,nf*791);wr16(raw+p+2,nf*701);
  n=nf==EN_NFXS?sizeof(set_t):p+4;if(mode==1)n--;if(mode==2)raw[6]=11;if(mode==3)raw[0]=0;
  memcpy(store[OBJ_SET],raw,n);store_len[OBJ_SET]=(int)n;settings_load();ui_witness(17,nf*4+mode,0);
  proj.genre=7;for(i=0;i<EN_NFXS;i++)ui.fxs[i]=(uint8_t)(i+nf);settings_save();ui_witness(18,nf*4+mode,0);
 }
 for(n=0;n<20;n++)for(mode=0;mode<3;mode++){
  clean(0);memset(raw,0,2056);wr32(raw,VOX_MAGIC);wr32(raw+4,n);for(i=8;i<2056;i++)raw[i]=(uint8_t)((i*37+n)&127);
  if(mode==2)raw[0]=0;memcpy(store[OBJ_VOICES],raw,2056);store_len[OBJ_VOICES]=mode==1?2055:2056;voices_load();ui_witness(19,n*3+mode,0);voices_save();ui_witness(20,n*3+mode,0);
 }
 for(k=0;k<256;k++)for(mode=0;mode<3;mode++){
  unsigned len;const uint8_t *pack;clean(0);memset(raw,0,20);memcpy(raw,"JSM\1",4);raw[4]=(uint8_t)(k&1);raw[5]=(uint8_t)(k%12);raw[6]=(uint8_t)(k%2);raw[7]=(uint8_t)(k&2?8:16);raw[8]=(uint8_t)(k%4);raw[9]=(uint8_t)((k/4)%4);raw[10]=(uint8_t)(k%101);raw[11]=(uint8_t)(k%2);raw[12]=(uint8_t)(k%8);raw[13]=(uint8_t)(k%5);wr16(raw+14,70+k%121);raw[16]=(uint8_t)(k%4);raw[17]=(uint8_t)(k&4?'S':0);raw[18]=(uint8_t)k;raw[19]=(uint8_t)(k*37);
  if(mode==2)raw[5]=12;memcpy(store[OBJ_JSESSION],raw,20);store_len[OBJ_JSESSION]=20;
  if(mode==1){pack=en_jam_builtin(k%3,&len);for(i=26;i<30;i++){memcpy(store[i],pack,len);store_len[i]=(int)len;}}
  jam_session_load();ui_witness(21,k*3+mode,0);
  en_play(0);ui_witness(22,k*3+mode,jam_session_save_slot((int)(k%8)-1));
 }
 clean(0);
 for(i=0;i<8192;i++){
  static const uint32_t time[16]={0,1999,2000,2999,3000,3999,4000,19999,20000,20001,0xfffffffeu,0xffffffffu,1,100,30000,40000};
  now_ms=time[i%16];ui.act_t=(i&16)?0:now_ms-5000u;proj.genre=(uint8_t)((i/128)%19);proj.piece=(uint8_t)((i/128)%10);glob.rsv=(int16_t)((i/16)%191);
  en_play((int)((i>>5)&1));host_seamless=(int)((i>>6)&1);ui.btn=(i&256)?1:0;ui.keys=(i&512)?1:0;ui.store_wait=(i&1024)?1:0;
  ui.vox_dirty=(uint8_t)(i&1);ui.set_dirty=(uint8_t)((i>>1)&1);ui.set_lazy=(uint8_t)((i>>2)&1);pending=(uint8_t)((i>>3)&1);pending_glob=(uint8_t)((i>>4)&1);pending_px=0;bs_dirty=0;
  autosave();ui_witness(23,i,0);
 }
 {uint32_t v[2]={cases,sizeof(set_t)};emit(v,sizeof v);}return (int)cases;
}
#ifndef __wasm__
int main(void){persistence_ui_run();return 0;}
#endif
