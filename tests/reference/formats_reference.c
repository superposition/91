/* SPDX-License-Identifier: GPL-3.0-only */
/* Compile against verified immutable R01 C, never maintained Rust. */
#define EN_WEB 1
#include <stdint.h>
static void web_audio(const int32_t *block,uint32_t frames){(void)block;(void)frames;}
#define en_style_sounds formats_capture_style
#include "host/amb_host.c"
#undef en_style_sounds
#include "firmware/src/backup_proto.h"
#include "firmware/src/dx_proto.h"
#if EN_RUST_FORMATS
#include "formats_dx_current.inc"
#else
#include "formats_dx.inc"
#endif
static int16_t before_style[P_NPARAMS];
void en_style_sounds(int16_t *p);
void formats_capture_style(int16_t *p){memcpy(before_style,p,sizeof before_style);en_style_sounds(p);}
static void bytes(const uint8_t *b,unsigned n){unsigned i;putchar('[');for(i=0;i<n;i++)printf("%s%u",i?",":"",b[i]);putchar(']');}
static void signed_words(const int16_t *p,unsigned n){unsigned i;putchar('[');for(i=0;i<n;i++)printf("%s%d",i?",":"",p[i]);putchar(']');}
static uint32_t fmt_hash(uint32_t h,const uint8_t *b,unsigned n){while(n--)h=(h^*b++)*16777619u;return h;}
static uint32_t slot_fmt_hash(uint32_t h,const slot_t *s,int valid){
    unsigned p,l;uint8_t v=(uint8_t)valid;h=fmt_hash(h,&v,1);
    for(p=0;p<P_NPARAMS;p++){uint8_t b[2]={(uint8_t)s->par[p],(uint8_t)(s->par[p]>>8)};h=fmt_hash(h,b,2);}
    {uint8_t b[2]={(uint8_t)s->pool,(uint8_t)(s->pool>>8)};h=fmt_hash(h,b,2);}
    for(p=0;p<6;p++)for(l=0;l<6;l++){uint8_t b[2]={(uint8_t)s->pats[p][l],(uint8_t)(s->pats[p][l]>>8)};h=fmt_hash(h,b,2);}
    h=fmt_hash(h,&s->mod[0][0],96);h=fmt_hash(h,s->acl,18);return fmt_hash(h,s->cml,12);
}
int main(void){
    unsigned seed,i,j;uint8_t packed[128],voice[156],out[156],raw[640];slot_t slot,recipe_slot;
    en_init();memset(&recipe_slot,0,sizeof recipe_slot);en_piece_g(3,5,recipe_slot.par,&recipe_slot.pool);
    printf("{\"effects\":%u,\"settings_len\":%u,\"voices\":[",(unsigned)EN_NFXS,(unsigned)sizeof(set_t));
    for(seed=0;seed<256;seed++){
        for(i=0;i<128;i++)packed[i]=(uint8_t)(seed+i*37u);for(i=0;i<155;i++)voice[i]=(uint8_t)(seed+i*53u);
        en_voice_unpack(packed,out);if(seed)putchar(',');printf("[");bytes(out,155);putchar(',');en_voice_pack(voice,out);bytes(out,128);putchar(']');
    }
    printf("],\"voice_mutations\":[");
    for(i=0;i<283;i++){
        uint32_t h=2166136261u;for(j=0;j<256;j++){
            unsigned k;for(k=0;k<128;k++)packed[k]=(uint8_t)(k*37u+91u);for(k=0;k<155;k++)voice[k]=(uint8_t)(k*53u+91u);
            if(i<128){packed[i]=(uint8_t)j;en_voice_unpack(packed,out);h=fmt_hash(h,out,155);}else{voice[i-128]=(uint8_t)j;en_voice_pack(voice,out);h=fmt_hash(h,out,128);}
        }printf("%s%u",i?",":"",h);
    }
    printf("],\"records\":[");
    for(seed=0;seed<32;seed++){
        slot=recipe_slot;for(i=0;i<P_NPARAMS;i++)slot.par[i]=(int16_t)((int)seed*19-(int)i*3);
        slot.pool=0xab5;for(i=0;i<36;i++)slot.pats[i/6][i%6]=(uint16_t)(seed*907u+i*1637u);
        for(i=0;i<96;i++)slot.mod[i/16][i%16]=(uint8_t)(seed+i*37u);for(i=0;i<18;i++)slot.acl[i]=(uint8_t)(seed+i*13u);for(i=0;i<12;i++)slot.cml[i]=(uint8_t)(seed+i*17u);
        pack_slot(&slot,0);if(seed)putchar(',');bytes(image+IMG_HDR,IMG_REC);
    }
    printf("],\"record_mutations\":[");
    memset(image,0,sizeof image);for(i=0;i<376;i++)image[IMG_HDR+i]=(uint8_t)(91u+i*37u);wr16(image+IMG_HDR,0xab5);memset(image+IMG_HDR+REC_PAR,255,P_NPARAMS);
    for(i=0;i<376;i++){
        uint32_t h=2166136261u;uint8_t old=image[IMG_HDR+i];for(j=0;j<256;j++){int valid;image[IMG_HDR+i]=(uint8_t)j;slot=recipe_slot;valid=unpack_slot(0,P_NPARAMS,0,&slot);h=slot_fmt_hash(h,&slot,valid);}image[IMG_HDR+i]=old;printf("%s%u",i?",":"",h);
    }
    printf("],\"legacy\":[");
    {const unsigned counts[4]={1,76,98,154};unsigned row,revision,first=1;
     for(row=0;row<4;row++)for(revision=0;revision<4;revision++)for(seed=0;seed<8;seed++){
        unsigned count=counts[row],stride=count*2+74,n=20+10*stride;uint8_t *q=image+20;
        memset(image,0,sizeof image);memset(&proj,0,sizeof proj);wr32(image,BANK_MAGIC);wr32(image+4,1);wr16(image+8,count);wr16(image+10,1);wr16(image+18,revision);
        for(i=0;i<count;i++){const en_param_t *pi=en_param_info((int)i);int v=pi->lo+(int)((seed*19+i*17)%(unsigned)(pi->hi-pi->lo+1));if(i==P_STYLE)v=(int)(seed%5);wr16(q+i*2,(uint16_t)v);}
        if(seed>=4)wr16(q,seed&1?0x7fff:0x8000);wr16(q+count*2,seed>=6?0:0xab5);
        for(i=0;i<36;i++)wr16(q+count*2+2+i*2,seed*907+i*1637);
        old_bank_load((int)n);
        if(!first)putchar(',');first=0;printf("{\"count\":%u,\"revision\":%u,\"seed\":%u,\"input\":",count,revision,seed);bytes(q,stride);printf(",\"prefix\":");signed_words(before_style,P_NPARAMS);printf(",\"final\":");signed_words(proj.slot[0].par,P_NPARAMS);printf(",\"saved\":%u,\"pool\":%u,\"patterns\":[",saved,proj.slot[0].pool);for(i=0;i<36;i++)printf("%s%u",i?",":"",proj.slot[0].pats[i/6][i%6]);printf("]}");
     }}
    printf("],\"frames\":[");
    for(seed=0;seed<=64;seed++){for(i=0;i<seed;i++)packed[i]=(uint8_t)(seed+i*37u);j=bk_build_frame(raw,17,seed,3000,packed,seed);if(seed)putchar(',');bytes(raw,j);}
    printf("],\"dump\":");{int32_t vals[24];for(i=0;i<24;i++)vals[i]=(int32_t)i*9-100;vals[23]=8000;j=px_build_dump(raw,130,vals,24);bytes(raw,j);}
    printf(",\"editor_voice\":");for(i=0;i<128;i++)packed[i]=(uint8_t)(91u+i*37u);j=px_build_voice(raw,15,16,packed);bytes(raw,j);
    printf(",\"restore\":[");
    for(seed=0;seed<12;seed++){
        uint32_t obj=0,n=0,h=2166136261u,lh=2166136261u;uint8_t b[3842]={0};
        switch(seed){
        case 0:obj=0;n=IMG_LEN;wr32(b,IMG_MAGIC);b[4]=2;wr16(b+10,376);break;
        case 1:obj=0;n=20;wr32(b,BANK_MAGIC);break;
        case 2:obj=0;n=132;wr32(b,OLD_KIT_MAGIC);break;
        case 3:obj=4;n=sizeof(glob_t);wr32(b,GLOB_MAGIC);break;
        case 4:obj=4;n=12;wr32(b,GLOB_MAGIC);break;
        case 5:obj=23;n=4+EN_NGENRE*10*3;wr32(b,KND_MAGIC);break;
        case 6:obj=23;n=PX_SIZE;wr32(b,PX_MAGIC);b[4]=16;b[5]=19;break;
        case 7:obj=25;n=BS_SIZE;wr32(b,BS_MAGIC);b[4]=16;b[5]=19;break;
        case 8:obj=3;n=sizeof(vox_t);wr32(b,VOX_MAGIC);wr32(b+4,16);break;
        case 9:obj=2;n=sizeof(set_t);wr32(b,SET_MAGIC);wr16(b+4,EN_NFXS);wr16(b+6,12);break;
        case 10:obj=2;n=32;wr32(b,SET_MAGIC);wr16(b+4,2);wr16(b+6,12);break;
        case 11:obj=24;n=sizeof(mmap_t);wr32(b,MM_MAGIC2);b[4]=3;b[8]=0x90;b[9]=127;b[10]=175;b[11]=0;b[12]=60;b[13]=32;b[14]=0x50;b[15]=74;b[16]=64;break;
        }
        for(i=0;i<=n+2;i++){uint8_t valid=(uint8_t)ui_restore_valid(obj,b,i);lh=fmt_hash(lh,&valid,1);}
        for(i=0;i<n;i++){uint8_t old=b[i];for(j=0;j<256;j++){uint8_t valid;b[i]=(uint8_t)j;valid=(uint8_t)ui_restore_valid(obj,b,n);h=fmt_hash(h,&valid,1);}b[i]=old;}
        if(seed)putchar(',');printf("{\"object\":%u,\"bytes\":",obj);bytes(b,n);printf(",\"length_hash\":%u,\"mutation_hash\":%u}",lh,h);
    }
    printf("],\"dx\":[");
    for(seed=0;seed<4;seed++){
        unsigned n=0,offset=seed<2?5:4,count=seed==0?4096:seed==1?155:seed==2?128:0;uint32_t lh=2166136261u,mh=2166136261u;uint8_t original[4112];
        memset(dx_buf,0,sizeof dx_buf);
        if(seed<2){dx_buf[0]=0x43;dx_buf[1]=seed?15:0;dx_buf[2]=seed?0:9;dx_buf[3]=seed?1:32;dx_buf[4]=seed?27:0;}else{dx_buf[0]=0x7d;dx_buf[1]=0x50;dx_buf[2]=0x56;dx_buf[3]=seed==2?15:127;}
        for(i=0;i<count;i++)dx_buf[offset+i]=(uint8_t)((91+i*37)&127);
        if(count){dx_buf[offset+count]=(uint8_t)bk_sum(dx_buf+offset,count);n=offset+count+1;}else n=4;
        memcpy(original,dx_buf,sizeof original);
        for(i=0;i<=n+2;i++){uint8_t kind;dx_pos=i;dx_kind=0;dx_end();kind=dx_kind;lh=fmt_hash(lh,&kind,1);}
        dx_pos=n;for(i=0;i<8&&i<n;i++){uint8_t old=dx_buf[i];for(j=0;j<128;j++){uint8_t kind;dx_buf[i]=(uint8_t)j;dx_kind=0;dx_end();kind=dx_kind;mh=fmt_hash(mh,&kind,1);}dx_buf[i]=old;}
        if(seed)putchar(',');printf("{\"bytes\":[240,");for(i=0;i<n;i++)printf("%s%u",i?",":"",original[i]);printf(",247],\"kind\":%u,\"length_hash\":%u,\"header_hash\":%u}",seed+1,lh,mh);
    }
    printf("]}\n");
#if EN_RUST_FORMATS
    {extern uint32_t fm1_rust_jam_stat(uint32_t);fprintf(stderr,"{\"formats_native\":\"pass\",\"decodes\":%u,\"encodes\":%u,\"rejections\":%u,\"copied_bytes\":%u}\n",fm1_rust_jam_stat(36),fm1_rust_jam_stat(37),fm1_rust_jam_stat(38),fm1_rust_jam_stat(39));}
#endif
    return 0;
}
