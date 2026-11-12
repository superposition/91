/* SPDX-License-Identifier: GPL-3.0-only */
/* Test adapter: hex SysEx lines over stdio, real app/audio and Jam handler. */
#define main host_script_main
#include "../../host/amb_host.c"
#undef main
#include "../../firmware/src/app/jam_usb.h"

int main(void)
{
    static uint8_t scratch[JP_MAX], reply[148];
    char line[2048];
    ui_splash_ms=0;boot();run_ms(100);
    while(fgets(line,sizeof line,stdin)) {
        unsigned n=0,i,len;uint8_t message[540];char *p=line;
        if(!strcmp(line,"reboot\n")) {
            memset(&ju,0,sizeof ju);boot();run_ms(100);
            puts("ok");fflush(stdout);continue;
        }
        while(p[0] && p[0]!='\n' && p[0]!='\r') {
            unsigned v;int used=0;
            if(n>=sizeof message || sscanf(p,"%2x%n",&v,&used)!=1 || used!=2) return 2;
            message[n++]=(uint8_t)v;p+=used;
        }
        /* Status polling advances simulated time, with no wall-clock sleep. */
        run_ms(n>3 && message[3]==8 ? 1000:20);
        len=ju_request(message,n,scratch,0,reply);
        for(i=0;i<len;i++) printf("%02x",reply[i]);
        putchar('\n');fflush(stdout);
    }
    return 0;
}
