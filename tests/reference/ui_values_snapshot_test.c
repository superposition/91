/* SPDX-License-Identifier: GPL-3.0-only */
#include <stdint.h>
#include <stdio.h>
#include <string.h>
static uint32_t fake_irq,epoch,pending,deliveries,reads,bad;
static void deliver(void){epoch++;deliveries++;pending=0;}
static void irq_write(uint32_t value){fake_irq=value;if((fake_irq&256)&&pending)deliver();}
#define FM1_IRQ_STATE_TEST 1
#define FM1_IRQ_STATE_READ(v) do{(v)=fake_irq;}while(0)
#define FM1_IRQ_STATE_WRITE(v) irq_write(v)
#define FM1_IRQ_STATE_FENCE() ((void)0)
#include "fm1_irq_state.h"
static uint8_t pattern(unsigned i,uint32_t e){return (uint8_t)(i*17u+e*29u+(e>>9));}
static uint8_t sample_byte(unsigned i){uint8_t b=pattern(i,epoch);reads++;if(fake_irq&256)deliver();else pending=1;return b;}
static uint16_t sample_word(unsigned i){uint16_t b=(uint16_t)(pattern(i,epoch)|((uint16_t)pattern(i+1,epoch)<<8));reads++;if(fake_irq&256)deliver();else pending=1;return b;}
#define EN_PRG_LEN 6
enum{UIVR_PROGRESSION,UIVR_PREVIEW,UIVR_ACID,UIVR_KINDS};
static struct{uint8_t prg[6],prk[3];}proj;
static void en_prog_capture(uint8_t *out){memset(out,0,6);}
#include "ui_values_snapshot.inc"
int main(void){unsigned n,nested,i;uint32_t calls=0;
 for(n=0;n<65536;n++)for(nested=0;nested<2;nested++){
  uint8_t buffer[40];uint32_t saved,expected,version;
  fake_irq=0xa50f0000u|n;pending=0;epoch=n*7u;saved=fake_irq;
  if(nested)saved=fm1_irq_state_save();expected=fake_irq;version=epoch;
  memset(buffer,0xa5,sizeof buffer);
  if(ui_values_read(UIVR_ACID,buffer+1,38))return 1;
  if(fake_irq!=expected || buffer[0]!=0xa5 || buffer[39]!=0xa5)return 2;
  for(i=0;i<38;i++)if(buffer[1+i]!=pattern(i,version))bad++;
  if(nested)fm1_irq_state_restore(saved);
  if(fake_irq!=(0xa50f0000u|n))return 3;
  calls++;
 }
#ifdef EN_HOST
 if(!bad)return 4;
#else
 if(bad || calls!=131072 || reads!=calls*38)return 5;
#endif
 {uint8_t b[38];memset(b,0xa5,sizeof b);
  for(i=0;i<4;i++)if(ui_values_read(i,b,37)!=-1)return 6;
  if(ui_values_read(0xffffffffu,b,38)!=-1 || ui_values_read(UIVR_ACID,0,38)!=-1)return 7;
  for(i=0;i<38;i++)if(b[i]!=0xa5)return 8;
 }
 printf("{\"calls\":%u,\"reads\":%u,\"deliveries\":%u,\"incoherent_bytes\":%u}\n",calls,reads,deliveries,bad);
 return 0;
}
