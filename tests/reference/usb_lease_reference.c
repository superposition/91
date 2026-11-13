/* SPDX-License-Identifier: GPL-3.0-only */
/* Real device/host driver with retained DMA and independently confirmed stop. */
#include <stdint.h>
#include <string.h>
#ifndef __wasm__
#include <stdio.h>
#include <stdlib.h>
#define CHECK(x) do{if(!(x)){fprintf(stderr,"line %u case %u: %s\n",__LINE__,cases,#x);abort();}}while(0)
#else
#define CHECK(x) do{if(!(x))__builtin_trap();}while(0)
#endif
static uint32_t cases,fm1_ms,stopped,irq=256;
static void fm1_wdt_feed(void){fm1_ms++;}
static void fm1_delay_ms(uint32_t n){fm1_ms+=n;}
static uint32_t irq_enter(void){uint32_t r=irq;irq=0;return r;}
static void irq_exit(uint32_t v){irq=v;}
#define USB_TX_IRQ_ENTER() irq_enter()
#define USB_TX_IRQ_EXIT(v) irq_exit(v)
#define USB_DMA_STOPPED() stopped
#define RING_PUBLISH() __asm__ volatile("" ::: "memory")
#include "usb_source.inc"
#ifdef __wasm__
__attribute__((import_module("env"),import_name("emit"))) extern void out_emit(const void *,uint32_t);
#else
static void out_emit(const void *p,uint32_t n){fwrite(p,1,n,stdout);}
#endif
static uint8_t regs[8][256],ix,pending;
static uint32_t serial,fail_at,failed,apply_failed,sie_on=1,control_mode;
static uint32_t sends[5],violations,total_violations,stops,attaches,binds,off_requests,digest=2166136261u;
static void trace(uint32_t v){digest=(digest^v)*16777619u;}
static const uint8_t *roots[5];static uint8_t copies[5][184];static uint32_t lengths[5];
static uint32_t ota_now_ms(void){return fm1_ms++;}static void ota_idle(void){fm1_ms++;}
static void verify(void){for(uint32_t ep=0;ep<5;ep++)if(roots[ep])for(uint32_t i=0;i<lengths[ep];i++)if(roots[ep][i]!=copies[ep][i]){violations++;total_violations++;break;}}
static void retire(uint32_t ep){verify();roots[ep]=0;regs[ep][S_TXCSR1]=0;}
static void send(uint32_t ep,void *p,uint32_t n){verify();if(roots[ep]){violations++;total_violations++;}trace(200+ep);trace(n);for(uint32_t i=0;i<n;i++)trace(((uint8_t *)p)[i]);CHECK(n<=184);roots[ep]=p;lengths[ep]=n;memcpy(copies[ep],p,n);sends[ep]++;}
static void fm1_usb_reset(void){off_requests++;}
static void fm1_usb_off(void){off_requests++;}
static void fm1_usb_attach(void *p){(void)p;attaches++;}
static void fm1_usb_host_attach(void *p){(void)p;attaches++;}
static uint32_t fm1_usb_line(void){return 1;}
static uint32_t fm1_usb_sie_on(void){return sie_on;}
static void fm1_usb_sie_wr_start(uint32_t r,uint32_t v){
 trace(100);trace(r);trace(v);failed=++serial==fail_at;if(failed&&!apply_failed)return;
 if(r==S_INDEX)ix=(uint8_t)v;else regs[r<16?0:ix][r]=(uint8_t)v;
#ifdef USB_TEST_SIE_WRITE
 USB_TEST_SIE_WRITE(r,v);
#endif
 if(control_mode&&ix==0&&r==S_CSR0){
  if(v&0x20u){retire(0);regs[0][S_CSR0]=1;regs[0][S_COUNT0]=4;for(uint32_t i=0;i<4;i++)ep0buf[i]=(uint8_t)(i+41);}
  else if(v&2u)retire(0);
 }
}
static void fm1_usb_sie_rd_start(uint32_t r){trace(101);trace(r);failed=++serial==fail_at;
#ifdef USB_TEST_SIE_READ
 USB_TEST_SIE_READ(r);
#endif
 pending=regs[r<16?0:ix][r];trace(pending);}
static uint32_t fm1_usb_sie_done(void){return !failed;}
static uint32_t fm1_usb_sie_data(void){return pending;}
static void fm1_usb_ep0_buf(void *p){(void)p;binds++;}
static void fm1_usb_ep4_txbuf(void *p){(void)p;binds++;}
static void fm1_usb_ep_txbuf(uint32_t ep,void *p){(void)ep;(void)p;binds++;}
static void fm1_usb_ep_rxbuf(uint32_t ep,void *p){(void)ep;(void)p;binds++;}
static void fm1_usb_ep0_send(void *p,uint32_t n){send(0,p,n);}
static void fm1_usb_ep_send(uint32_t ep,void *p,uint32_t n){send(ep,p,n);}
static void fm1_usb_ep4_send(void *p,uint32_t n){send(4,p,n);}
static void fm1_usb_rx_sync(void){}
static void fm1_usb_ep_enable(uint32_t eps){(void)eps;}
static uint32_t fm1_usb_sof_take(void){return 0;}
static void fresh(void){
 for(uint32_t ep=0;ep<5;ep++)roots[ep]=0;
 memset(&usb,0,sizeof usb);memset(&uh,0,sizeof uh);memset(&uh_d,0,sizeof uh_d);memset(&cdc,0,sizeof cdc);memset(&uac,0,sizeof uac);
 memset(regs,0,sizeof regs);memset(sends,0,sizeof sends);memset(ep0buf,0xa5,sizeof ep0buf);
 serial=fail_at=failed=apply_failed=control_mode=0;sie_on=1;ix=0;stopped=0;attaches=binds=off_requests=0;violations=0;
 digest=2166136261u;usb.up=usb.config=1;mo_r=mo_w=so_r=so_w=co_r=co_w=mi_r=mi_w=0;ua_r=ua_w=0;ep4_k=0;fm1_ms=0;
}
static uint32_t lease(void){
#if EN_RUST_USB && !USB_LEASE_HEALTHY
 return usb.tx_lease;
#else
 return 0;
#endif
}
static void snapshot(uint32_t group,uint32_t arg){
 verify();const uint32_t values[]={group,arg,serial,violations,lease(),sends[0],sends[1],sends[3],sends[4],usb.up,usb.config,mo_r,mo_w,co_r,co_w,ua_r,uh.on,uh.want,uh.run,uh.fault,attaches,binds,off_requests,digest,uh_d.err,irq,total_violations};
 out_emit(values,sizeof values);cases++;
}
static void feed(uint32_t ep,uint32_t seed){
 if(ep==1)midi_out_q[mo_w++%MQ]=9u|(0x90u<<8)|((seed&127u)<<16)|(127u<<24);
 else if(ep==3)cdc_out[co_w++%CO_N]=(uint8_t)seed;
 else if(ep==4){uac.alt=1;uac.go=1;for(uint32_t i=0;i<64;i++)ua_ring[ua_w++%UA_N]=seed+i;}
}
static void service(uint32_t ep){if(ep==1)ep1_tx();else if(ep==3)ep3_tx();else if(ep==4)uac_service();else ep0_service();}
static void stop_fact(void){verify();for(uint32_t ep=0;ep<5;ep++)roots[ep]=0;stopped=1;stops++;}
static void device_cases(void){
 /* Every SIE operation in admission/arm/kick can fail, either accepted or not. */
 for(uint32_t ep=1;ep<=4;ep++)if(ep!=2)for(uint32_t point=1;point<=4;point++)for(uint32_t applied=0;applied<2;applied++){
  fresh();apply_failed=applied;fail_at=point;feed(ep,13);service(ep);snapshot(1,ep*100+point*2+applied);
  const uint32_t before=sends[ep];feed(ep,51);fail_at=0;failed=0;
  if(roots[ep]){ /* unknown kick + fresh zero may not release; accepted kick stays busy */
   const uint32_t uncertain=point>=3&&point<=(ep==4?3u:4u);
   regs[ep][S_TXCSR1]=uncertain?0:1;service(ep);snapshot(2,ep);
#if EN_RUST_USB
   CHECK(sends[ep]==before&&violations==0);
#endif
   regs[ep][S_TXCSR1]=1;service(ep);retire(ep);service(ep);snapshot(3,ep);
  }else{service(ep);snapshot(4,ep);}
#if EN_RUST_USB
  CHECK(violations==0);
#endif
 }
 /* After successful submission, off/unknown/failed INDEX must not grant a source. */
 for(uint32_t ep=1;ep<=4;ep++)if(ep!=2)for(uint32_t mode=0;mode<3;mode++){
  fresh();feed(ep,21);service(ep);const uint32_t before=sends[ep];feed(ep,22);
  if(mode==0)sie_on=0;else fail_at=serial+1u+((mode==2)?1u:0u);
  service(ep);snapshot(5,ep*4+mode);
#if EN_RUST_USB
  CHECK(sends[ep]==before&&violations==0);
#endif
  sie_on=1;fail_at=0;failed=0;retire(ep);service(ep);snapshot(6,ep);
 }
 /* Independent EP0 source: uncertain kick must not overwrite for the next chunk. */
 static uint8_t source[192];for(uint32_t i=0;i<sizeof source;i++)source[i]=(uint8_t)(i*13u);
 for(uint32_t point=1;point<=4;point++)for(uint32_t applied=0;applied<2;applied++){
  fresh();apply_failed=applied;fail_at=point;e0_send(source,192,192);snapshot(7,point*2+applied);
  uint32_t before=sends[0],had_root=roots[0]!=0;fail_at=0;failed=0;regs[0][S_CSR0]=0;ep0_service();snapshot(8,point);
#if EN_RUST_USB
  if(had_root)CHECK(sends[0]==before&&violations==0);
#endif
  if(roots[0]){regs[0][S_CSR0]=2;ep0_service();retire(0);ep0_service();}snapshot(9,point);
#if EN_RUST_USB
  CHECK(violations==0);
#endif
 }
}
static void host_cases(void){
 /* Whole-role switch preserves old roots/queue while the off request is uncertain. */
 for(uint32_t ep=1;ep<=4;ep++)if(ep!=2){
  fresh();feed(ep,43);service(ep);feed(ep,44);uint32_t before=mo_r;
  uh.want=1;uh_task(1000);snapshot(10,ep);
#if EN_RUST_USB
  CHECK(uh.on==0&&uh.want==1&&attaches==0&&mo_r==before&&violations==0);
#endif
  uh_task(2000);snapshot(11,ep);
#if EN_RUST_USB
  CHECK(uh.on==0&&attaches==0&&violations==0);
#endif
  stop_fact();uh_task(3000);snapshot(12,ep);
  CHECK(uh.on==1&&attaches>0);
 }
 /* The inverse role and RX-only ownership have the same whole-controller gate. */
 for(uint32_t out=0;out<2;out++){
  fresh();usb.up=0;uh.on=uh.want=1;
  uh_midi_t m={0};m.in_ep=2;m.in_type=2;m.in_mps=64;
  if(out){m.out_ep=3;m.out_type=2;m.out_mps=64;}
  uh_start_ep(&m);CHECK(uh.run);uint32_t old_binds=binds;
  if(out){feed(1,71);uh_poll();}
  uh.want=0;uh_task(1000);snapshot(24,out);
#if EN_RUST_USB
  CHECK(uh.on==1&&!uh.run&&!uh_out&&binds==old_binds&&!violations);
#endif
  uh_task(2000);snapshot(25,out);
#if EN_RUST_USB
  CHECK(uh.on==1&&binds==old_binds&&!violations);
#endif
  stop_fact();uh_task(3000);snapshot(26,out);CHECK(!uh.on&&!uh.want);
 }
 /* EP0 IN/OUT/status completion and unknown operations, with full real control. */
 for(uint32_t in=0;in<2;in++)for(uint32_t point=0;point<16;point++)for(uint32_t applied=0;applied<2;applied++){
  uint8_t output[8];fresh();usb.up=0;uh.on=uh.want=1;control_mode=1;fail_at=point;apply_failed=applied;
  int r=uh_ctl(1,in?0x80:0,6,0x100,0,in?4:0,output,8);snapshot(13,in*100+point*2+applied);
#if EN_RUST_USB
  if(r<0){uint32_t before=sends[0];CHECK((lease()&3u)==3u);fail_at=0;failed=0;CHECK(uh_ctl(1,0,9,1,0,0,0,8)<0&&sends[0]==before);}
  else {CHECK(!(lease()&3u));CHECK(!in||r==4);}
  CHECK(violations==0);
#else
  (void)r;
#endif
 }
 /* Host ACK path retains ring/source on every unknown CSR/INDEX or kick. */
 for(uint32_t point=1;point<=4;point++)for(uint32_t applied=0;applied<2;applied++){
  fresh();usb.up=0;uh.on=uh.want=uh.run=1;uh.out_mps=64;uh_out=1;ix=1;
  feed(1,61);feed(1,62);fail_at=point;apply_failed=applied;uh_poll();snapshot(14,point*2+applied);
  fail_at=0;failed=0;
  if(uh.fault){uint32_t before=mo_r;uh_task(1000);snapshot(15,point);
#if EN_RUST_USB
   CHECK(mo_r==before&&violations==0&&attaches==0);
#endif
   stop_fact();uh_task(2000);snapshot(16,point);
  }else{retire(1);uh_poll();snapshot(17,point);}
#if EN_RUST_USB
  CHECK(violations==0);
#endif
 }
 /* Reconfiguration may not use a logical reset or completed write as stop. */
 fresh();feed(1,77);service(1);usb.up=0;usb_start();snapshot(18,0);
#if EN_RUST_USB
 CHECK(!usb.up&&!attaches&&roots[1]);
#endif
 stop_fact();usb_start();snapshot(19,0);CHECK(usb.up&&attaches);
}
static void reset_flush_cases(void){
 static uint8_t source[192];for(uint32_t i=0;i<sizeof source;i++)source[i]=(uint8_t)(i*17u);
 static const uint8_t aborts[]={4,6,0x10,0x11,0x12,0x14,0x15};
 for(uint32_t k=0;k<sizeof aborts;k++){
  fresh();e0_send(source,192,192);uint32_t before=sends[0];CHECK(roots[0]);
  regs[0][S_CSR0]=aborts[k];ep0_service();snapshot(31,k);
  e0_send(source+64,64,64);regs[0][S_CSR0]=0;ep0_service();snapshot(32,k);
#if EN_RUST_USB
  CHECK(!usb.up&&!usb.config&&sends[0]==before&&!violations);
#endif
  stop_fact();usb_start();e0_send(source+64,64,64);snapshot(33,k);
#if EN_RUST_USB
  CHECK(sends[0]>before&&!violations);
#endif
 }
 for(uint32_t ep=0;ep<=4;ep++)if(ep!=2)for(uint32_t kind=0;kind<4;kind++){
  if((ep==0&&kind!=1)||(kind==2&&ep!=4)||(kind==3&&ep!=1&&ep!=3))continue;
  fresh();if(ep)feed(ep,81);else e0_send(source,192,192);
  if(ep)service(ep);CHECK(roots[ep]);const uint32_t before=sends[ep];
  if(kind==0)ep1_config();
  if(kind==1){
   /* Bus reset may clear status without proving old DMA has stopped. */
   regs[ep][S_TXCSR1]=0;regs[0][S_INTRUSB]=4;usb_poll();regs[0][S_INTRUSB]=0;
  }
  if(kind==2)uac_ep4_reset();
  if(kind==3){
   memset(ep0buf,0,8);ep0buf[0]=2;ep0buf[1]=1;ep0buf[4]=(uint8_t)(0x80u|ep);
   regs[0][S_CSR0]=1;ep0_service();
  }
  snapshot(27,ep*4+kind);usb.config=1;
  if(ep)feed(ep,83);else e0_send(source+64,64,64);
  service(ep);snapshot(28,ep);
#if EN_RUST_USB
  CHECK(!usb.up&&sends[ep]==before&&!violations);
#endif
  regs[ep][S_TXCSR1]=1;service(ep);regs[ep][S_TXCSR1]=0;service(ep);snapshot(29,ep);
#if EN_RUST_USB
  CHECK(sends[ep]==before&&!violations); /* reset invalidation requires stop, not busy/free */
#endif
  stop_fact();usb_start();usb.config=1;if(ep)feed(ep,87);else e0_send(source+64,64,64);
  service(ep);snapshot(30,ep);
#if EN_RUST_USB
  CHECK(sends[ep]>before&&!violations);
#endif
 }
}
static void healthy_cases(void){
 /* Exact same wire IO, payload, queue and diagnostics with observed success. */
 for(uint32_t ep=1;ep<=4;ep++)if(ep!=2)for(uint32_t depth=0;depth<=64;depth++){
  fresh();for(uint32_t i=0;i<depth;i++)feed(ep,i+7);
  for(uint32_t i=0;i<8;i++){service(ep);snapshot(20,ep*1000+depth*8+i);retire(ep);}
  CHECK(!violations);
 }
 for(uint32_t in=0;in<2;in++)for(uint32_t len=0;len<=64;len++){
  uint8_t output[64];fresh();usb.up=0;uh.on=uh.want=1;control_mode=1;
  int r=uh_ctl(1,in?0x80:0,6,0x100,0,in?(uint16_t)len:0,output,8);
  CHECK(r>=0);snapshot(21,in*100+len);CHECK(!violations);
 }
 static const uint8_t csrs[]={0,1,2,3,0x80,0x84};
 for(uint32_t depth=0;depth<=64;depth++)for(uint32_t mps=4;mps<=64;mps+=4){
  fresh();usb.up=0;uh.on=uh.want=uh.run=1;uh.out_mps=(uint8_t)mps;uh_out=1;ix=1;
  for(uint32_t i=0;i<depth;i++)feed(1,i);
  for(uint32_t turn=0;turn<8;turn++){
   uh_poll();snapshot(22,depth*100+mps);if(!roots[1])continue;
   for(uint32_t c=0;c<sizeof csrs;c++){
    regs[1][S_TXCSR1]=csrs[c];if(!csrs[c])retire(1);
    uh_poll();snapshot(23,c);CHECK(!violations);if(uh.fault)break;
   }
   retire(1);
  }
 }
}
#if defined(__wasm__) && !defined(USB_LEASE_EMBEDDED)
__attribute__((export_name("run")))
#endif
#ifndef USB_LEASE_EMBEDDED
int run(void){
#if USB_LEASE_HEALTHY
 healthy_cases();
#else
 device_cases();host_cases();reset_flush_cases();
#if !EN_RUST_USB
 CHECK(total_violations>0); /* retained frozen witness, never a tolerance */
#endif
#endif
 return (int)cases;
}
#ifndef __wasm__
int main(void){run();return 0;}
#endif
#endif
