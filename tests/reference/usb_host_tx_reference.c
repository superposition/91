/* SPDX-License-Identifier: GPL-3.0-only */
#define USB_LEASE_EMBEDDED 1
#include "usb_lease_reference.c"
static uint8_t records[4096];static uint32_t used;
static void record(const void *data,uint32_t bytes){const uint8_t *p=data;
 while(bytes){uint32_t n=sizeof records-used;if(n>bytes)n=bytes;memcpy(records+used,p,n);p+=n;bytes-=n;used+=n;
  if(used==sizeof records){out_emit(records,used);used=0;}}}
static void tx_state(uint32_t group,uint32_t pending_count,uint32_t csr,uint32_t errors,uint32_t age,uint32_t maximum,uint32_t wrap){
 verify();CHECK(!violations);
 const uint32_t v[]={group,pending_count,csr,errors,age,maximum,wrap,uh.tx_count,uh.tx_errors,uh.tx_ms,uh.run,uh.fault,mo_r,mo_w,sends[1],serial,digest,regs[1][S_TXCSR1]};
 record(v,sizeof v);record(midi_out_q,sizeof midi_out_q);record(ep1tx,sizeof ep1tx);cases++;
}
static void init(uint32_t depth,uint32_t maximum,uint32_t wrap){
 fresh();usb.up=0;uh.on=uh.want=uh.run=1;uh.out_mps=(uint8_t)maximum;uh_out=1;ix=1;
 mo_r=wrap?0xfffffff8u:11u;mo_w=mo_r+depth;
 for(uint32_t i=0;i<64;i++)midi_out_q[i]=0x7f009009u|(i<<16);
 memset(ep1tx,0xa5,sizeof ep1tx);fm1_ms=wrap?0xfffffff0u:1234u;
}
static void pending_cases(void){
 static const uint8_t error_values[]={0,15,255};static const uint32_t ages[]={0,499,500};static const uint32_t maxima[]={4,64};
 for(uint32_t pending_count=1;pending_count<=16;pending_count++)for(uint32_t csr=0;csr<256;csr++)for(uint32_t e=0;e<3;e++)for(uint32_t a=0;a<3;a++)for(uint32_t m=0;m<2;m++)for(uint32_t wrap=0;wrap<2;wrap++){
  init(32,maxima[m],wrap);uh.tx_count=(uint8_t)pending_count;uh.tx_errors=error_values[e];uh.tx_ms=fm1_ms-ages[a];
  for(uint32_t i=0;i<pending_count;i++){uint32_t word=midi_out_q[(mo_r+i)%64];memcpy(ep1tx+i*4,&word,4);}
  send(1,ep1tx,pending_count*4);
#if EN_RUST_USB
  usb_endpoint_armed(1,1);
#endif
  if(!(csr&(0x20u|0x84u|3u)))retire(1); /* real observed ACK before any possible new source write */
  regs[1][S_TXCSR1]=(uint8_t)csr;uh_poll();
  tx_state(1,pending_count,csr,error_values[e],ages[a],maxima[m],wrap);
 }
}
static void new_packets(void){
 for(uint32_t depth=0;depth<=64;depth++)for(uint32_t maximum=0;maximum<=64;maximum++)for(uint32_t wrap=0;wrap<2;wrap++){
  init(depth,maximum,wrap);uh_poll();tx_state(2,depth,0,0,0,maximum,wrap);
 }
 /* Normal sequential packet ownership across busy/NAK/ACK, ring index and time wraps. */
 for(uint32_t maximum=4;maximum<=64;maximum+=4){
  init(32,maximum,1);
  for(uint32_t turn=0;turn<64;turn++){
   if(roots[1])retire(1);uh_poll();tx_state(3,turn,0,0,0,maximum,1);
   if(roots[1]){
    regs[1][S_TXCSR1]=1;uh_poll();tx_state(4,turn,1,0,0,maximum,1);
    regs[1][S_TXCSR1]=0x80;uh_poll();tx_state(5,turn,0x80,0,0,maximum,1);
   }
   fm1_ms+=7;
   if(mo_w-mo_r<48){for(uint32_t i=0;i<4;i++)midi_out_q[mo_w++%64]=0x7f409009u|(turn<<16);}
  }
 }
}
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int usb_host_tx_run(void){pending_cases();new_packets();if(used)out_emit(records,used);return (int)cases;}
#ifndef __wasm__
int main(void){return usb_host_tx_run()<0;}
#endif
