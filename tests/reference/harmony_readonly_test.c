/* SPDX-License-Identifier: GPL-3.0-only */
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "amb.h"
#include "rust_harmony.h"
static uint32_t fake_irq,expected_irq,epoch,expected_epoch,pending;
static uint32_t calls,copies,deliveries,bad,math_calls,output_early;
static uint8_t *active_output;static unsigned output_bytes;
static void deliver(void){epoch++;deliveries++;pending=0;}
static void irq_write(uint32_t flags){fake_irq=flags;if((flags&0x100u)&&pending)deliver();}
#define FM1_IRQ_STATE_TEST 1
#define FM1_IRQ_STATE_READ(v) do {(v)=fake_irq;}while(0)
#define FM1_IRQ_STATE_WRITE(v) irq_write(v)
#define FM1_IRQ_STATE_FENCE() ((void)0)
#include "fm1_irq_state.h"
static uint8_t pattern(unsigned index,uint32_t version){return (uint8_t)((version*29u+index*17u)^(version>>8));}
static void harmony_read(fm1_harm_state_t *s,fm1_harm_config_t *c)
{
 unsigned i;
 for(i=0;i<sizeof *s+sizeof *c;i++){
  uint8_t *to=i<sizeof *s?(uint8_t*)s+i:(uint8_t*)c+i-sizeof *s;
  unsigned k;*to=pattern(i,epoch);copies++;
  for(k=0;k<output_bytes;k++)if(active_output[k]!=0xa5)output_early++;
  if(fake_irq&0x100u)deliver();else pending=1;
 }
}
#include "rust_harmony_snapshot.inc"
static void check(const fm1_harm_state_t*s,const fm1_harm_config_t*c)
{
 unsigned i;
 if(fake_irq!=expected_irq)bad++;
 for(i=0;i<sizeof *s+sizeof *c;i++){
  uint8_t value=i<sizeof *s?((const uint8_t*)s)[i]:((const uint8_t*)c)[i-sizeof *s];
  if(value!=pattern(i,expected_epoch))bad++;
 }
 math_calls++;
}
int32_t fm1_rust_harm_preview(const fm1_harm_state_t*s,const fm1_harm_config_t*c,int32_t d,int32_t k,int32_t*out)
{check(s,c);out[0]=d;out[1]=k;return 2;}
int32_t fm1_rust_harm_capture(const fm1_harm_state_t*s,const fm1_harm_config_t*c,uint8_t*out)
{check(s,c);memset(out,0x3c,EN_PRG_LEN);return 1;}
#include "harmony_readonly_wrappers.inc"
int main(void)
{
 unsigned n,nested,mode;uint32_t outer;
 for(n=0;n<65536;n++)for(nested=0;nested<2;nested++)for(mode=0;mode<2;mode++){
  int32_t pcs[4];uint8_t bytes[EN_PRG_LEN];
  fake_irq=0xa50f0000u|n;pending=0;epoch=n*7u+mode;
  outer=fake_irq;if(nested)outer=fm1_irq_state_save();
  expected_irq=fake_irq;expected_epoch=epoch;
  active_output=mode?bytes:(uint8_t*)pcs;output_bytes=mode?sizeof bytes:sizeof pcs;
  memset(active_output,0xa5,output_bytes);
  if(mode)harmony_capture(bytes);else if(harmony_preview(3,4,pcs)!=2)return 1;
  if(fake_irq!=expected_irq)return 2;
  if(nested)fm1_irq_state_restore(outer);
  if(fake_irq!=(0xa50f0000u|n))return 3;
  calls++;
 }
#ifdef FM1_HARM_NEGATIVE_CONTROL
 if(!bad)return 4;
#else
 if(bad||output_early||calls!=262144u||math_calls!=calls)return 5;
#endif
 printf("{\"calls\":%u,\"source_bytes\":%u,\"copies\":%u,\"deliveries\":%u,\"bad\":%u,\"early_output\":%u}\n",calls,(unsigned)(sizeof(fm1_harm_state_t)+sizeof(fm1_harm_config_t)),copies,deliveries,bad,output_early);
 return 0;
}
