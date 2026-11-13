/* SPDX-License-Identifier: GPL-3.0-only */
static void control_write(unsigned,unsigned);static void control_read(unsigned);
#define USB_TEST_SIE_WRITE control_write
#define USB_TEST_SIE_READ control_read
#define USB_LEASE_EMBEDDED 1
#include "usb_lease_reference.c"
static uint8_t records[4096];static uint32_t used;
static uint32_t stage,waiting,polls,delay,fault_stage,fault_code,packet_count,position,packets,model;
static void record(const void *data,uint32_t bytes){const uint8_t *p=data;while(bytes){uint32_t n=sizeof records-used;if(n>bytes)n=bytes;memcpy(records+used,p,n);p+=n;bytes-=n;used+=n;if(used==sizeof records){out_emit(records,used);used=0;}}}
static void control_write(unsigned reg,unsigned value){
 if(!model||ix||reg!=S_CSR0)return;
 if(value==0x0a)stage=0;else if(value==0x20)stage=1;else if(value==0x42)stage=2;else if(value==0x60)stage=3;else{waiting=0;return;}
 waiting=1;polls=0;regs[0][S_CSR0]=stage==0||stage==2?2:0x20;
}
static void control_read(unsigned reg){
 if(!model||ix||reg!=S_CSR0||!waiting)return;
 if(polls++<delay)return;
 if(stage==fault_stage&&fault_code){
  if(fault_code!=255)regs[0][S_CSR0]=(uint8_t)fault_code;
  return;
 }
 waiting=0;
 if(stage==0||stage==2){retire(0);regs[0][S_CSR0]=0;return;}
 regs[0][S_CSR0]=1;
 if(stage==1){
  const uint32_t n=packet_count>64?64:packet_count;
  for(uint32_t i=0;i<n;i++)ep0buf[i]=(uint8_t)((position+i)*13u+17u);
  position+=n;packets++;regs[0][S_COUNT0]=(uint8_t)packet_count;
 }
}
static void one(uint32_t length,uint32_t maximum,uint32_t type,uint32_t data_count,uint32_t wait,uint32_t error_stage,uint32_t error,uint32_t wrap){
 uint8_t output[768];fresh();usb.up=0;uh.on=uh.want=1;uh_d.err=0x55;
 memset(output,0xcc,sizeof output);model=1;stage=waiting=polls=position=packets=0;
 packet_count=data_count;delay=wait;fault_stage=error_stage;fault_code=error;fm1_ms=wrap?0xfffffff0u:1234;
 int r=uh_ctl(3,(uint8_t)type,6,0x1234,0xabcd,(uint16_t)length,output,(uint8_t)maximum);
 verify();CHECK(!violations);
 const uint32_t values[]={length,maximum,type,data_count,wait,error_stage,error,wrap,(uint32_t)r,uh_d.err,fm1_ms,serial,digest,sends[0],position,packets};
 record(values,sizeof values);record(output,sizeof output);record(ep0buf,64);cases++;
}
#ifdef __wasm__
__attribute__((export_name("run")))
#endif
int usb_host_control_run(void){
 static const uint32_t counts[]={0,1,7,8,9,63,64,65,255};static const uint32_t maxima[]={8,16,32,64};
 for(uint32_t length=0;length<=768;length++)for(uint32_t m=0;m<4;m++)for(uint32_t c=0;c<9;c++)for(uint32_t wrap=0;wrap<2;wrap++)one(length,maxima[m],0x80,counts[c],0,9,0,wrap);
 static const uint32_t errors[]={0,4,16,20,255};static const uint32_t delays[]={0,1,499,500,501,502};
 for(uint32_t type=0;type<2;type++)for(uint32_t length=0;length<=65;length+=13)for(uint32_t s=0;s<4;s++)for(uint32_t e=0;e<5;e++)for(uint32_t d=0;d<6;d++)for(uint32_t wrap=0;wrap<2;wrap++)one(length,8,type?0x80:0,8,delays[d],s,errors[e],wrap);
 if(used)out_emit(records,used);return (int)cases;
}
#ifndef __wasm__
int main(void){return usb_host_control_run()<0;}
#endif
