/* SPDX-License-Identifier: GPL-3.0-only */
/* Actual host descriptor helper and driver RX, not a reimplemented C oracle. */
#define USB_LEASE_EMBEDDED 1
#include "usb_lease_reference.c"
static uint8_t records[4096];static uint32_t used;
static void record(const void *data,uint32_t bytes){
 const uint8_t *p=data;
 while(bytes){uint32_t n=sizeof records-used;if(n>bytes)n=bytes;memcpy(records+used,p,n);p+=n;bytes-=n;used+=n;
  if(used==sizeof records){out_emit(records,used);used=0;}}
}
static void selected(const uint8_t *input,uint32_t count){
 uh_midi_t m; /* actual uninitialized caller-owned result */
 int result=uh_parse_midi(input,count,&m);
 const uint32_t v[]={count,(uint32_t)result,m.found,m.cfg,m.iface,m.alt,m.in_ep,m.in_type,m.in_interval,m.in_mps,m.out_ep,m.out_type,m.out_interval,m.out_mps};
 record(v,sizeof v);cases++;
}
static uint32_t random_word(uint32_t *rng){*rng=*rng*1664525u+1013904223u;return *rng;}
static void descriptors(void){
 static const uint8_t recipe[]={9,2,59,0,2,7,0,0x80,50,9,4,1,0,2,1,3,0,0,7,5,0x81,2,64,0,0,7,5,2,3,5,0,1,9,4,2,1,2,1,3,0,0,7,5,0x83,2,32,0,0,7,5,4,2,16,0,0};
 uint8_t data[768];selected(0,0);
 for(uint32_t n=0;n<=sizeof recipe;n++)selected(recipe,n);
 for(uint32_t field=0;field<sizeof recipe;field++)for(uint32_t value=0;value<256;value++){
  memcpy(data,recipe,sizeof recipe);data[field]=(uint8_t)value;
  selected(data,sizeof recipe);selected(data,field);selected(data,field+1);
 }
 /* Every endpoint address/type/size/interval boundary, plus protocol/class. */
 static const uint16_t sizes[]={0,1,3,4,5,7,8,9,15,16,31,32,63,64,65,127,128,255,256,1023,65535};
 for(uint32_t address=0;address<256;address++)for(uint32_t type=0;type<16;type++)for(uint32_t s=0;s<sizeof sizes/sizeof sizes[0];s++)for(uint32_t interval=0;interval<2;interval++){
  memcpy(data,recipe,sizeof recipe);data[20]=(uint8_t)address;data[21]=(uint8_t)type;data[22]=(uint8_t)sizes[s];data[23]=(uint8_t)(sizes[s]>>8);data[24]=interval?255:0;
  selected(data,25);
 }
 /* Long audio/vendor prefixes and malformed/truncated mixed descriptors. */
 uint32_t rng=0x5eedad17u;
 for(uint32_t trial=0;trial<8192;trial++){
  for(uint32_t i=0;i<sizeof data;i++)data[i]=(uint8_t)(random_word(&rng)>>24);
  uint32_t n=random_word(&rng)%769u,pos=random_word(&rng)%700u;
  if(pos+sizeof recipe<=n)memcpy(data+pos,recipe,sizeof recipe);
  selected(data,n);
 }
 for(uint32_t prefix=0;prefix+sizeof recipe<=sizeof data;prefix+=2){
  for(uint32_t i=0;i<prefix;i+=2){data[i]=2;data[i+1]=0x24;}
  memcpy(data+prefix,recipe,sizeof recipe);selected(data,prefix+sizeof recipe);
 }
}
static void packets(void){
 static const uint32_t tails[]={0,0x7f7f0000u,0x80800000u,0xffff0000u};
 for(uint32_t value=0;value<65536;value++)for(uint32_t i=0;i<4;i++){
  uint32_t word=value|tails[i],result=uh_midi_ok(word);record(&result,4);cases++;
 }
}
static void receive_cases(void){
 uint32_t rng=0xc0ffee77u;
 for(uint32_t count=0;count<=64;count++)for(uint32_t free=0;free<=64;free++)for(uint32_t wrap=0;wrap<2;wrap++)for(uint32_t kind=0;kind<8;kind++){
  fresh();usb.up=0;uh.on=uh.want=uh.run=1;uh.in_ep=3;ix=1;
  mi_w=wrap?0xfffffff8u:32u;mi_r=mi_w-(64u-free);
  for(uint32_t i=0;i<64;i++)midi_in_q[i]=0xaabb0000u+i;
  for(uint32_t i=0;i<64;i+=4){
   uint32_t word;
   switch(kind){
   case 0:word=0x7f409019u+(i<<16);break;
   case 1:word=0x7d00f004u;break;
   case 2:word=0xffffff0fu;word=(word&~0xff00u)|((0xf8u+(i/4u)%8u)<<8);break;
   case 3:word=random_word(&rng);break;
   case 4:word=0xff42caac;break;
   case 5:word=0x0080a00au;break;
   default:word=0x7f409009u;break;
   }
   memcpy(uh_rx+i,&word,4);
  }
  regs[1][S_RXCSR1]=kind==6?0x40:kind==7?0x0c:1;
  if(kind==7)uh.errors=15;
  regs[1][S_RXCOUNT1]=(uint8_t)count;regs[1][S_RXCOUNT2]=0;uh_poll();
  const uint32_t v[]={count,free,wrap,kind,mi_w,mi_r,uh.run,uh.fault,uh.errors,uh.pkts,uh.events,serial,digest,regs[1][S_RXCSR1]};
  record(v,sizeof v);record(midi_in_q,sizeof midi_in_q);record(uh_rx,64);cases++;
 }
}
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int usb_host_run(void){descriptors();packets();receive_cases();if(used)out_emit(records,used);return (int)cases;}
#ifndef __wasm__
int main(void){return usb_host_run()<0;}
#endif
