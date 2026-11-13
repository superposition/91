/* SPDX-License-Identifier: GPL-3.0-only */
#include <assert.h>
#define main host_script_main
#include "../../host/amb_host.c"
#undef main
static void turn_row(int knob,int delta)
{
    enc_acc[EN_K1+knob]=delta;accel_off=1;run_ms(60);accel_off=0;
}
static int saw_off(unsigned wanted)
{
    char line[96];unsigned ms,st,n,v;int found=0;
    fflush(midi_log);rewind(midi_log);
    while(fgets(line,sizeof line,midi_log))if(sscanf(line,"%u ms: %x %x %x",&ms,&st,&n,&v)==4 && st==0x80 && n==wanted)found=1;
    fseek(midi_log,0,SEEK_END);return found;
}
int main(void)
{
    unsigned n,view;uint8_t before[JP_MAX];const uint8_t *p;uint16_t pool;
    ui_splash_ms=0;boot();en_play(0);run_ms(100);set_view(V_JAMSWELL);run_ms(60);
    p=en_jam_data(&n);memcpy(before,p,n);pool=proj.pool;
    turn_row(0,-20);assert(en_jam_option(JAM_SYNTH)==0);
    turn_row(0,1);assert(en_jam_option(JAM_SYNTH)==1);
    turn_row(0,20);assert(en_jam_option(JAM_SYNTH)==1);
    p=en_jam_data(&n);before[JP2_HEAD+25]=1;assert(!memcmp(before,p,n));
    assert(jam_ui_row_field(3,0,JAM_SWELL_VOICE)==JAM_SYNTH);
    assert(jam_ui_row_field(3,1,JAM_SWELL)==JAM_KEYS_PROGRAM);
    assert(jam_ui_row_field(3,2,JAM_VARIATION)==JAM_KEYS_OCTAVE);
    assert(jam_ui_row_field(3,3,JAM_MOVE)==JAM_SPACE);
    for(view=0;view<NVIEWS;view++) {
        set_view((int)view);run_ms(40);held_keys=1;run_ms(60);
        assert((last_out&0xffff00u)==0x904100u); /* F3 + octave1 = F4, everywhere. */
        held_keys=0;run_ms(60);assert((last_out&0xffff00u)==0x804100u);assert(proj.pool==pool);
    }
    ui.fx_menu=1;ui.fx_page=1;held_keys=1;run_ms(60);assert((last_out&0xffff00u)==0x904100u);
    held_keys=0;run_ms(60);assert((last_out&0xffff00u)==0x804100u);ui.fx_menu=ui.fx_page=0;
    set_view(V_JAMSWELL);run_ms(40);turn_row(2,-3);assert(en_jam_option(JAM_KEYS_OCTAVE)==-2);
    min_q[mi_w++%MQ]=9u|0x90u<<8|60u<<16|98u<<24;run_ms(60);
    assert((last_out&0xffff00u)==0x903c00u); /* Incoming MIDI remains raw C4. */
    min_q[mi_w++%MQ]=8u|0x80u<<8|60u<<16;run_ms(60);assert((last_out&0xffff00u)==0x803c00u);
    turn_row(3,10);assert(en_jam_option(JAM_SPACE)==48);
    held_keys=1;run_ms(60);assert((last_out&0xffff00u)==0x901d00u);
    ui.fx_menu=1;ui.fx_page=1;held_btn=1u<<B_HOME;run_ms(50);held_btn=0;run_ms(80);
    assert(ui.view==V_JAMHELP && !en_jam_option(JAM_SYNTH) && !ui.fx_menu && !ui.fx_page);assert((last_out&0xffff00u)==0x801d00u);
    held_keys=0;run_ms(60);
    ui.fx_menu=1;ui.fx_page=1;held_btn=1u<<B_HOME;run_ms(50);held_btn=0;run_ms(80);
    assert(ui.fx_menu==1 && ui.fx_page==1); /* Band retains its pre-Synth HOME behavior. */
    ui.fx_menu=ui.fx_page=0;en_jam_set(JAM_SYNTH,1);run_ms(60);
    midi_log=tmpfile();assert(midi_log);
    min_q[mi_w++%MQ]=9u|0x90u<<8|61u<<16|98u<<24;run_ms(60);
    for(view=K_FXP;view<K_FXP+NFXP*4;view++)if(FXP_UI(view) && FXP_CTL(view).kind!=KN_PAR && FXP_CTL(view).idx==19)break;
    assert(view<K_FXP+NFXP*4);fxp_set((int)view,1);run_ms(60);assert(saw_off(61));fxp_set((int)view,0);run_ms(40);
    en_jam_key(190,62,100);run_ms(60);runner_start();run_ms(60);assert(saw_off(62));runner_stop();run_ms(60);
    en_jam_key(191,63,100);run_ms(60);rm_enter();run_ms(60);assert(saw_off(63));rm_leave();run_ms(60);
    min_q[mi_w++%MQ]=9u|0x90u<<8|64u<<16|98u<<24;run_ms(60);
    ui.saver_on=1;min_q[mi_w++%MQ]=8u|0x80u<<8|64u<<16;input();run_ms(60);assert(saw_off(64));ui.saver_on=0;
    fclose(midi_log);midi_log=0;
    puts("Synth UI: four direct rows, all43 pages and FX chromatic keys, raw MIDI, Space, HOME release/Band return; MIDI channel/remote/game takeovers and screensaver releases; no pool edits OK");
    return 0;
}
