/* SPDX-License-Identifier: GPL-3.0-only */
/* Invoke actual app helpers, retaining the complete C0 app and engine. */
static void performance_case(unsigned n,uint32_t*random)
{
 unsigned i,s=(n/12)%4,kind=n%12;
 uint32_t r=random_word(random);
 /* Explicit tap cases seed their history. Exclude an additional simultaneous
  * key tap: the frozen C helper divides by zero for that undefined input. */
 held_keys &= ~(1u<<11);
 ui.fx_lock=(uint64_t)r|((uint64_t)(r&255u)<<32);
 ui.trk_m=(uint8_t)r;ui.trk_s=(uint8_t)(r>>8);
 ui.undo_m=(uint8_t)(r>>16);ui.undo_s=(uint8_t)(r>>24);
 ui.undo_lock=(uint64_t)(r^0x89abcdefu);ui.undo_ok=(n/12)%3!=0;
 ui.scene_quant=(n/12)&1u;ui.scene_set=(uint8_t)(r&15u);
 for(i=0;i<4;i++){
  unsigned lane;uint32_t x=random_word(random);
  ui.fxk[i]=(int8_t)((int)(x%201)-100);
  ui.undo_k[i]=(int8_t)((int)((x>>8)%201)-100);
  ui.scene_lock[i]=((uint64_t)x<<8)|i;
  ui.scene_m[i]=(uint8_t)x;ui.scene_s[i]=(uint8_t)(x>>8);
  ui.scene_t[i]=now_ms-(n%3==0?599u:n%3==1?600u:601u);
  for(lane=0;lane<4;lane++)ui.scene_k[i][lane]=(int8_t)((int)((x>>(lane*4))%201)-100);
  for(lane=0;lane<6;lane++){
   ui.scene_g[i][lane]=(uint8_t)((x+lane)%101);
   ui.scene_g[i][6+lane]=(uint8_t)((x+lane)%7);
  }
  ui.scene_g[i][12]=(uint8_t)(x%EN_NPAT);
 }
 switch(kind){
 case 0:undo_save();break;
 case 1:undo_swap();break;
 case 2:
  ui.tap_n=(uint8_t)(n/12);for(i=0;i<4;i++)ui.tap_t[i]=now_ms-(1u+(r%1100u))*(4u-i);
  tap_tempo();break;
 case 3:scene_store((int)s);break;
 case 4:scene_apply((int)s);break;
 case 5:scene_recall((int)s);break;
 case 6:scene_reset();break;
 case 7:ui.scene_pend=(uint8_t)r;scene_tick();break;
 case 8:glob.sl_rsv[1]=(uint8_t)(n/12);mix_start((int)(70+r%121));break;
 case 9:
  ui.mx_bars=(uint8_t)((n/12)%33);ui.mx_n=(uint8_t)((n/12)%40);
  ui.mx_from=(int16_t)(70+r%121);ui.mx_to=(int16_t)(70+(r>>8)%121);
  ui.mx_last=(n&16u)?proj.par[P_TEMPO]:(int16_t)(70+(r>>16)%121);
  mix_tick();break;
 case 10:
  ui.key_set=(uint8_t)(n/12);ui.key_on=(uint8_t)((n/12)&1u);ui.sec_last=(uint8_t)((n/24)%3);
  key_tick();break;
 case 11:set_view((int)((n/12)%NVIEWS));break;
 }
}
