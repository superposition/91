/* SPDX-License-Identifier: GPL-3.0-only */
/* Source-level oracle: invoke the complete real policy consumers, with bounded
 * published CPU0 clock/section observations injected between completed renders. */
static uint64_t automatic_seen,automatic_required;
static void automatic_case(unsigned n,uint32_t*random,int32_t*pcm)
{
 unsigned i,index=n%28,kind=n%5;uint32_t r=random_word(random),meta[6];
 held_keys &= ~(1u<<11); /* no undefined simultaneous implicit tap */
#if EN_RUST_APP
 for(i=0;i<5;i++)if(fm1_rust_ui_auto_metadata(index,i,&meta[i]))overflow=1;
 if(fm1_rust_ui_auto_metadata(n%27,5,&meta[5]))overflow=1;
#else
 meta[0]=AFX[index].fx;meta[1]=AFX[index].lvl;meta[2]=AFX[index].sec;
 meta[3]=AFX[index].len;meta[4]=AFX[index].fav;meta[5]=(uint32_t)(int32_t)MIXK[n%27];
#endif
 for(i=0;i<6;i++)word(meta[i]);
 if(meta[0]>=64)overflow=1;else automatic_required|=1ull<<meta[0];
 en_jam_set(JAM_ON,0);en_play((n/5)%4!=0);en_render(pcm,256,2048);
 /* These public observations are byte/word/float scalars, never private worker
  * sample spans. Injection is test-only; source ownership is foreground here. */
 en_bars=((r>>8)%512)*16u+(n&15u);en_sec=(uint8_t)((n/20)%3);
 en_sec_req=(uint8_t)((n/15)%3);en_sec_bars=(uint8_t)((n/10)%5);
 en_drum_pos=(uint8_t)((n/5)%16);en_intro_k=(n&64u)?1.0f:0.5f;
 en_drift_bars=(float)((n/5)%5)*0.75f;
 proj.genre=(uint8_t)((n/5)%EN_NGENRE);
 ui.dj_on=(uint8_t)((n/5)%2);ui.dj_len=(uint8_t)((n/10)%5);
 ui.dj_piece=(uint8_t)((n/15)%2);ui.dj_break=(uint8_t)((n/10)%3);
 ui.dj_rounds=(uint8_t)((n/20)%4);ui.dj_round=(uint8_t)(r>>24);
 ui.dj_ramp=(uint8_t)((n/15)%5);ui.dj_phase=(uint8_t)((n/35)%3);
 ui.dj_n=(uint16_t)r;ui.dj_fillon=(uint8_t)((n/25)%2);
 ui.dj_tlast=(n&16u)?proj.par[P_TEMPO]:(int16_t)(70+r%121);
 ui.scene_q=(uint8_t)((n/5)%5);ui.intro=(uint8_t)((n/5)%5);
 ui.midi_map=(uint8_t)((n/5)%2);ui.usb_host=(uint8_t)((n/10)%2);
 ui.pat_rev=(uint8_t)((n/5)%7);ui.pat_up=(uint8_t)((n/10)%2);ui.pat_held=en_pat_now;
 ui.af_rng=(n&128u)?r:0;ui.af_sec=(uint8_t)((n/25)%3);
 ui.af_fx=(uint8_t)(r%EN_NFX);ui.af_last[0]=(uint8_t)((r>>8)%EN_NFX);ui.af_last[1]=(uint8_t)((r>>16)%EN_NFX);
 ui.af_bar=0xffffffffu;ui.af_quiet=(n&32u)?0:(en_bars>>4)+2;
 ui.af_start=en_bars-1;ui.af_end=(n&16u)?0:en_bars+16;
 ui.af_user=(uint8_t)((n/45)%2);ui.af_said=(uint8_t)((n/35)%2);
 ui.fxs[EN_FXS_AF_AMT]=(uint8_t)((n/5)%4);ui.fxs[EN_FXS_AF_LEN]=(uint8_t)((n/10)%4);
 ui.fxs[EN_FXS_AF_EVERY]=(uint8_t)((n/20)%4);ui.fxs[EN_FXS_AF_PICK]=(uint8_t)((n/25)%2);
 if(kind==4 && (n&512u)) {
  unsigned length=4u<<(ui.fxs[EN_FXS_AF_LEN]&3),count=length>16?2:1;
  en_playing=1;en_intro_k=1.0f;ui.fx_menu=0;ui.af_user=0;ui.af_end=0;ui.af_quiet=0;
  ui.fxs[EN_FXS_AF_AMT]=(uint8_t)(1+(n/5)%3);
  en_bars=(((r>>8)%16)*32u+32u-count)*16u+(n&15u);
 }
 /* Two independently chosen seeds supplement the random trace's rare choices.
  * Expected IDs still come from the real frozen/current policies and tables. */
 if(n==514 || n==519) {
  en_playing=1;en_intro_k=1.0f;en_sec=0;en_bars=48;
  ui.fx_menu=0;ui.af_user=0;ui.af_end=0;ui.af_quiet=0;ui.af_bar=0xffffffffu;ui.af_sec=0;
  ui.dj_on=0;ui.af_last[0]=ui.af_last[1]=0;
  ui.fxs[EN_FXS_AF_AMT]=3;ui.fxs[EN_FXS_AF_LEN]=0;
  ui.fxs[EN_FXS_AF_EVERY]=0;ui.fxs[EN_FXS_AF_PICK]=1;
  ui.af_rng=n==514?19:5;
 }
 switch(kind){
 case 0:ui.tap_n=0;mix_key((int)((n/5)%27));break;
 case 1:dj_next();break;
 case 2:dj_bar();break;
 case 3:dj_tick();break;
 case 4:af_tick();break;
 }
 if(kind==4 && (n&512u) && ui.af_end && ui.af_fx<64)automatic_seen|=1ull<<ui.af_fx;
 word(ui.af_fx);word(ui.af_end);
}
