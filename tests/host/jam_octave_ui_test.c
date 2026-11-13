/* SPDX-License-Identifier: GPL-3.0-only */
#include <assert.h>
#define main host_script_main
#include "../../host/amb_host.c"
#undef main
static void turn_octave(int view,int delta)
{
    set_view(view);run_ms(60);enc_acc[EN_K2]=delta;accel_off=1;run_ms(60);accel_off=0;
}
int main(void)
{
    unsigned n;const uint8_t *p;
    ui_splash_ms=0;boot();run_ms(100);
    p=en_jam_data(&n);assert(jp_v3(p));
    assert(en_jam_option(JAM_KEYS_OCTAVE)==1 && en_jam_option(JAM_BASS_OCTAVE)==1);
    assert(jam_ui_field(JAM_TOUCH)==JAM_KEYS_OCTAVE && jam_ui_field(JAM_BASS_ATTACK)==JAM_BASS_OCTAVE);
    turn_octave(V_JAMKEYS,-3);assert(en_jam_option(JAM_KEYS_OCTAVE)==-2);
    assert(en_jam_option(JAM_BASS_OCTAVE)==1);
    turn_octave(V_JAMBASS,2);assert(en_jam_option(JAM_BASS_OCTAVE)==2);
    assert(en_jam_option(JAM_KEYS_OCTAVE)==-2);
    turn_octave(V_JAMKEYS,3);assert(en_jam_option(JAM_KEYS_OCTAVE)==1);
    turn_octave(V_JAMKEYS,20);assert(en_jam_option(JAM_KEYS_OCTAVE)==2);
    assert(en_jam_option(JAM_TOUCH)==50 && en_jam_option(JAM_BASS_ATTACK)==0);
    puts("UI: DX Keys/Bass knob2 controls independent bounded signed octaves; defaults +1/+1; native Touch/Attack unchanged OK");
    return 0;
}
