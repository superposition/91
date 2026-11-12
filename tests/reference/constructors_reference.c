/* SPDX-License-Identifier: GPL-3.0-only */
/* Struct definitions are compiled from immutable R01, independent of Rust. */
#define EN_WEB 1
#include <stdint.h>
static void web_audio(const int32_t*b,uint32_t n){(void)b;(void)n;}
#include "host/amb_host.c"
static void emit(const char*kind,const uint8_t*b,unsigned n,int*first){unsigned i;if(!*first)putchar(',');*first=0;printf("[\"%s\",[",kind);for(i=0;i<n;i++)printf("%s%u",i?",":"",b[i]);printf("]]");}
int main(void){unsigned seed,i;int first=1;uint8_t b[3840];printf("[");
 for(seed=0;seed<20;seed++){
  glob_t g;set_t s;vox_t v;mmap_t m;old_kit_t kit;
  for(i=0;i<sizeof g;i++)((uint8_t*)&g)[i]=(uint8_t)(seed+i*37);g.magic=GLOB_MAGIC;for(i=0;i<16;i++)g.sl[i]=(uint8_t)((seed+i*17)%191);
#if EN_RUST_FORMATS
  if(fm1_rust_formats_global(&g.tune,g.sl,g.sl_on,g.sl_rsv,(uint8_t*)&g,sizeof g)!=(int32_t)sizeof g)return 2;
#endif
  emit("global",(uint8_t*)&g,sizeof g,&first);emit("global-old",(uint8_t*)&g,12,&first);
  memset(&s,0,sizeof s);s.magic=SET_MAGIC;s.nfxs=EN_NFXS;s.nui=12;for(i=0;i<EN_NFXS;i++)s.fxs[i]=(uint8_t)(seed+i*37);for(i=0;i<12;i++)s.uiv[i]=(uint8_t)(seed+i*41);s.gx_preset=(uint8_t)(seed+21);s.gx_tgt=seed+37;s.pm_tgt=seed+71;s.viz=seed+97;s.gx_pat=seed*947;s.usb=seed*1693;
#if EN_RUST_FORMATS
  if(fm1_rust_formats_settings(s.fxs,EN_NFXS,s.uiv,&s.gx_preset,s.gx_pat,s.usb,(uint8_t*)&s,sizeof s)!=(int32_t)sizeof s)return 3;
#endif
  emit("settings",(uint8_t*)&s,sizeof s,&first);
  memset(&v,0,sizeof v);v.magic=VOX_MAGIC;v.n=seed%17;for(i=0;i<v.n*128;i++)((uint8_t*)v.v)[i]=(uint8_t)(seed+i*37);
#if EN_RUST_FORMATS
  if(fm1_rust_formats_voices((uint8_t*)v.v,v.n,(uint8_t*)&v,sizeof v)!=(int32_t)sizeof v)return 4;
#endif
  emit("voices",(uint8_t*)&v,sizeof v,&first);
  memset(&m,0,sizeof m);m.magic=seed&1?MM_MAGIC2:MM_MAGIC;m.n=seed*7%49;m.pad[0]=seed;m.pad[1]=seed*3;m.pad[2]=seed*5;for(i=0;i<m.n;i++){m.e[i][0]=(uint8_t)((i%16)|16|((i%3)<<5)|((seed&1)?128:0));m.e[i][1]=(uint8_t)(i*11%128);m.e[i][2]=(uint8_t)((seed&1)?i*3%176:i%14);}emit("map",(uint8_t*)&m,sizeof m,&first);
  for(i=0;i<sizeof kit;i++)((uint8_t*)&kit)[i]=(uint8_t)(seed+i*41);kit.magic=OLD_KIT_MAGIC;emit("kit",(uint8_t*)&kit,sizeof kit,&first);
  for(i=0;i<IMG_LEN;i++)image[i]=(uint8_t)(seed+i*37);saved=(uint16_t)(seed*73);image_header(seed%19);image[12]=seed;image[13]=seed*3;image[14]=seed*5;image[15]=seed*7;emit("bank",image,IMG_LEN,&first);
  for(i=0;i<PX_SIZE;i++)b[i]=(uint8_t)(seed+i*37);wr32(b,PX_MAGIC);b[4]=16;b[5]=EN_NGENRE;emit("extras",b,PX_SIZE,&first);wr32(b,BS_MAGIC);emit("bass",b,BS_SIZE,&first);
  wr32(b,KND_MAGIC);emit("kinds",b,4+EN_NGENRE*10*3,&first);
  {unsigned count=1+seed*8,stride=count*2+74,n=20+10*stride;wr32(b,BANK_MAGIC);wr32(b+4,1);wr16(b+8,count);wr16(b+10,seed*71);wr16(b+12,seed*907);wr16(b+14,seed*911);wr16(b+16,seed*919);wr16(b+18,seed*929);for(i=20;i<n;i++)b[i]=(uint8_t)(seed+i*43);emit("legacy",b,n,&first);}
 }
 /* Execute the actual save paths as well as the byte-layout constructors. */
 boot();en_play(0);settings_save();
 for(i=0;i<128;i++)b[i]=(uint8_t)((91+i*37)&127);en_user_voice(0,b);voices_save();
 project_piece_save();project_globals();if(project_flush()!=0)return 5;
 {const unsigned ids[6]={OBJ_SET,OBJ_VOICES,OBJ_GLOB,OBJ_PROJ,OBJ_KINDS,OBJ_BSTEP};const char*names[6]={"consumer-settings","consumer-voices","consumer-globals","consumer-bank","consumer-extras","consumer-bass"};for(i=0;i<6;i++){if(store_len[ids[i]]<0)return 6;emit(names[i],store[ids[i]],(unsigned)store_len[ids[i]],&first);}}
 printf("]\n");return 0;}
