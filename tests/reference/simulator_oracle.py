#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Retain the whole original host as an independent test-only oracle.

Only wall-clock CPU telemetry is replaced with deterministic zero, and one
final platform-state sink is added. Scenario, cadence and sink code is unchanged.
"""
from pathlib import Path
import sys

source=Path(sys.argv[1]).resolve()
out=Path(sys.argv[2])
text=(source/'host/amb_host.c').read_text(encoding='utf-8')
text=text.replace('#include "../firmware/', '#include "'+str(source/'firmware').replace('\\','/')+'/')
start=text.index('static double now_ns(void)')
end=text.index('\n}',start)+2
text=text[:start]+'static double now_ns(void) { return 0.0; }'+text[end:]
text=text.replace('static void run_ms(uint32_t ms)','static FILE *sim_oracle_pcm;\nstatic void run_ms(uint32_t ms)')
text=text.replace('            en_render(blk, 256, master);','            en_render(blk, 256, master);\n            if(sim_oracle_pcm)fwrite(blk,4,512,sim_oracle_pcm);')
text=text.replace('    ui_splash_ms = 0u;', '    {const char *p=getenv("FM1_SIM_PCM");if(p)sim_oracle_pcm=fopen(p,"wb");}\n    ui_splash_ms = 0u;')
trace=r'''
    {const char *path=getenv("FM1_SIM_TRACE");if(path){FILE *f=fopen(path,"wb");
#define W(v) do {uint32_t w_=(uint32_t)(v);fwrite(&w_,4,1,f);} while(0)
    unsigned k;W(now_ms);W(held_btn);W(held_keys);W(master);for(k=0;k<NE;k++)W(enc_acc[k]);
    W(host_batt);W(host_usb);W(host_seamless);W(host_seed);W(lit_btn);W(lit_keys);W(keys_dim);W(dim_keys);W(glow_on);W(mi_r);W(mi_w);W(midi_out_count);W(clock_out_count);
    for(k=0;k<128;k++)W(cc_out_n[k]);W(last_out);W(last_vel);W(host_uh_want);W(host_uh_msg);W(rs_valid);W(store_lock);W(blits);W(store_writes);
    for(k=0;k<OBJ_NOBJ;k++)W(store_len[k]);for(k=0;k<MQ;k++)W(min_q[k]);fwrite(store,1,sizeof store,f);fwrite(fb,1,sizeof fb,f);
    fwrite(&audio_due_ms,8,1,f);W(ui_due);fwrite(&peak_out,4,1,f);fclose(f);
#undef W
    }}
'''
needle='    printf("host: %u ms simulated'
assert text.count(needle)==1
text=text.replace(needle,trace+needle)
out.write_text(text,encoding='utf-8')
