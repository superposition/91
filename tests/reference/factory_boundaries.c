/* SPDX-License-Identifier: GPL-3.0-only */
/* Valid-field oracle; undefined out-of-range C indexing is not executed. */
#include "amb.h"
#include <stdio.h>
#include <string.h>
int main(void){int kind,slot,value,first=1;printf("[\n");for(kind=0;kind<5;kind++)for(slot=0;slot<(kind<2||kind==4?8:1);slot++)for(value=0;value<(kind==0||kind==4?8:kind==3?2:4);value++){
 uint8_t b[6];int j,bit=kind==0||kind==4?slot*3:kind==1?24+slot*2:kind==2?41:40,width=kind==0||kind==4?3:kind==3?1:2;
 memset(b,0xa5,sizeof b);en_prg_put(b,bit,width,value);
 printf("%s[%d,%d,%d,[",first?"":",\n",kind,slot,value);first=0;for(j=0;j<6;j++)printf("%s%u",j?",":"",b[j]);printf("]]");
 }printf("\n]\n");return 0;}
