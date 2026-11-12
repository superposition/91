/* SPDX-License-Identifier: GPL-3.0-only
 * Copyright (C) 2026 Leo Kuroshita (@kurogedelic), Hügelton Instruments */
/* Small-canvas renderer (no full framebuffer). Draw text/lines into
 * an off-screen strip, then blit it in one DMA transfer. Pixels are stored
 * byte-swapped (the panel takes RGB565 big-endian). */
typedef struct {               /* proportional, see tools/gen_font.py */
    uint8_t h;
    uint8_t pad;               /* bitmap starts this many pixels left of the pen */
    uint8_t first, last;
    const uint8_t *adv;        /* advance per glyph */
    const uint8_t *bw;         /* bitmap width per glyph (starts FONT_PAD left of the pen) */
    const uint16_t *off;       /* byte offset of each glyph */
    const uint8_t *data;
} felucca_font_t;
#include "felucca_font.h"

#define CV_MAX (240u * 144u)      /* X0X: the main area, both bands, is 240 x 144 */
static uint16_t cv_px[CV_MAX] __attribute__((section(".pool")));
static uint32_t cv_w, cv_h;
static int32_t cv_oy;            /* y offset for graph drawing */
static int32_t cv_y0, cv_y1;     /* X0X: the rows drawing may touch (cv_band) */
static uint8_t cv_scan;          /* odd screen rows a quarter darker */
#if EN_RUST_APP
#include "dsp/rust_app.h"
#ifndef LCD_READY
#if (defined(EN_HOST) && EN_HOST) || (defined(EN_WEB) && EN_WEB) || defined(FM1_APP_TEST)
#define LCD_READY() (lcd_sync(),1) /* only synchronous/non-DMA software backends */
#else
#error "Rust graphics requires confirmed LCD_READY; void lcd_sync is not a completion proof"
#endif
#endif
#endif
#if EN_RUST_APP || defined(LCD_READY)
#define CV_RETAINED_LCD 1
static uint8_t cv_dma_pending;
static int cv_ready(void)
{
    if(LCD_READY()!=1) {cv_dma_pending=2;return 0;} /* retained/unknown: quarantine */
    cv_dma_pending=0;return 1;
}
static int cv_mutable(void)
{
    return cv_dma_pending!=2 && (!cv_dma_pending || cv_ready());
}
#endif
#if EN_RUST_APP
static fm1_canvas_font cv_font_view(const felucca_font_t *f)
{
    fm1_canvas_font out={f->h,f->pad,f->first,f->last,f->adv,f->bw,f->off,f->data,0};
    uint32_t i,n=(uint32_t)f->last-f->first+1u;
    for(i=0;i<n;i++) {uint32_t end=f->off[i]+(uint32_t)f->h*((f->bw[i]+1u)/2u);if(end>out.length)out.length=end;}
    return out;
}
static uint32_t cv_string_length(const char *s)
{
    uint32_t n=0;
    if(s) while(n<=256u && s[n]) n++;
    return n;
}
static int32_t cv_rust(uint32_t op,const int32_t args[6],const felucca_font_t *font,const char *text)
{
    fm1_canvas_state state={cv_w,cv_h,cv_oy,cv_y0,cv_y1,cv_scan};
    fm1_canvas_font view={0};
    int32_t result;
    if(cv_dma_pending==2) return -1; /* only a fresh successful begin may reclaim */
    if(font) view=cv_font_view(font);
    /* The previous transfer can retain cv_px after lcd_blit returns. Never
     * form a mutable Rust pixel slice until that hardware read has ended. */
    if(op!=1u && (op!=5u || cv_scan) && cv_dma_pending && !cv_ready()) return -1;
    result=fm1_rust_canvas(op,cv_px,CV_MAX,&state,args,font?&view:0,(const uint8_t*)text,cv_string_length(text));
    cv_w=state.width;cv_h=state.height;cv_oy=state.origin;cv_y0=state.top;cv_y1=state.bottom;cv_scan=(uint8_t)state.scan;
    return result;
}
#endif

#define RGB(r, g, b) ((uint16_t)((((r) >> 3) << 11) | (((g) >> 2) << 5) | ((b) >> 3)))
#define C_BLACK 0x0000u
#define C_WHITE 0xFFFFu              /* accent only: what is being touched / where we are */
/* The screen's five steps, darkest to brightest, plus white; picked as THEME in GLOBAL. */
typedef struct {
    const char *name;
    uint16_t c[5];
} palette_t;
/* X0X: the first three steps are neutral greys in every theme (structure: rules, empty cells,
 * labels); only the two brightest carry the theme's hue (values, highlights). Colour on the
 * screen then means something: a part, a value, an alarm. */
#define NEUTRALS RGB(34, 34, 40), RGB(74, 74, 82), RGB(150, 150, 158)
static const palette_t PALETTES[] = {
    {"GREEN", {NEUTRALS, RGB(80, 220, 120), RGB(150, 255, 175)}},
    {"AMBER", {NEUTRALS, RGB(232, 136, 30), RGB(255, 184, 70)}},
    {"CYAN", {NEUTRALS, RGB(60, 180, 235), RGB(150, 225, 255)}},
    {"RED", {NEUTRALS, RGB(232, 76, 60), RGB(255, 132, 112)}},
    {"MONO", {NEUTRALS, RGB(196, 196, 202), RGB(236, 236, 240)}},
};
#define NPALETTES (sizeof(PALETTES) / sizeof(PALETTES[0]))
static uint16_t pal[5];
#define C_LINE pal[0]                /* 1 rules, separators */
#define C_DIM pal[1]                 /* 2 inactive, empty steps, units */
#define C_GRAY pal[2]                /* 3 labels */
#define C_AMB pal[3]                 /* 4 secondary text */
#define C_HI pal[4]                  /* 5 values, curves */

static void palette_set(uint32_t i)
{
#if EN_RUST_APP
    fm1_rust_palette(i,pal);return;
#endif

    uint32_t k;
    for (k = 0; k < 5u; k++)
        pal[k] = PALETTES[i % NPALETTES].c[k];
}

static inline uint16_t swap16(uint32_t c) { return (uint16_t)(((c >> 8) & 0xFFu) | ((c & 0xFFu) << 8)); }

static void cv_begin(uint32_t w, uint32_t h, uint16_t bg)
{
#if EN_RUST_APP
    { const int32_t a[6]={(int32_t)w,(int32_t)h,bg,0,0,0};if(!cv_ready())return;(void)cv_rust(0,a,0,0);return; }
#endif

    uint32_t i, n;
    if (w * h > CV_MAX)
        h = CV_MAX / w;
#if CV_RETAINED_LCD
    if(!cv_ready()) return;
#else
    lcd_sync();                     /* the last blit may still read cv_px */
#endif
    cv_w = w;
    cv_h = h;
    cv_oy = cv_y0 = 0;
    cv_y1 = (int32_t)h;
    n = w * h;
    for (i = 0; i < n; i++)
        cv_px[i] = swap16(bg);
}

/* X0X: draw the next things as if the canvas were rows y0 .. y0+h-1 alone (one band of a
 * canvas that holds several, sent in one transfer so the bands change on screen together) */
static void cv_band(int32_t y0, int32_t h)
{
#if EN_RUST_APP
    { const int32_t a[6]={y0,h,0,0,0,0};(void)cv_rust(1,a,0,0);return; }
#endif

    cv_oy = cv_y0 = y0;
    cv_y1 = y0 + h;
}

/* Scanlines are declared with the strip state above. */
static void cv_scanlines(uint32_t y, uint32_t r0)
{
#if EN_RUST_APP
    { const int32_t a[6]={(int32_t)y,(int32_t)r0,0,0,0,0};(void)cv_rust(5,a,0,0);return; }
#endif

    uint32_t r;
    if (!cv_scan)
        return;
#if CV_RETAINED_LCD
    if(!cv_mutable()) return;
#endif
    for (r = r0 + ((y + r0) & 1u ? 0u : 1u); r < cv_h; r += 2u) {   /* the odd screen rows */
        uint16_t *p = cv_px + r * cv_w, *e = p + cv_w;
        for (; p < e; p++) {
            const uint32_t c = swap16(*p);
            *p = swap16(c - ((c >> 2) & 0x39E7u));
        }
    }
}
static void cv_blit(uint32_t x, uint32_t y)
{
#if CV_RETAINED_LCD
    if(cv_dma_pending==2) return;
#endif
    cv_scanlines(y, 0);
#if CV_RETAINED_LCD
    if(cv_dma_pending==2) return;
#endif
    lcd_blit(x, y, cv_w, cv_h, cv_px);
#if CV_RETAINED_LCD
    cv_dma_pending=1;
#endif
}

/* canvas rows r0 .. cv_h-1 only, to screen row y + r0 */
static void cv_blit_from(uint32_t x, uint32_t y, uint32_t r0)
{
#if CV_RETAINED_LCD
    if(cv_dma_pending==2) return;
#endif
    cv_scanlines(y, r0);
#if CV_RETAINED_LCD
    if(cv_dma_pending==2) return;
#endif
    if (r0 < cv_h) {
        lcd_blit(x, y + r0, cv_w, cv_h - r0, cv_px + r0 * cv_w);
#if CV_RETAINED_LCD
        cv_dma_pending=1;
#endif
    }
}

static inline void cv_pset(int32_t x, int32_t y, uint16_t c)
{
#if EN_RUST_APP
    { const int32_t a[6]={x,y,c,0,0,0};(void)cv_rust(2,a,0,0);return; }
#endif

#if CV_RETAINED_LCD
    if(!cv_mutable()) return;
#endif
    y += cv_oy;
    if ((uint32_t)x < cv_w && y >= cv_y0 && y < cv_y1)
        cv_px[(uint32_t)y * cv_w + (uint32_t)x] = swap16(c);
}

static void cv_rect(int32_t x, int32_t y, int32_t w, int32_t h, uint16_t c)
{
#if EN_RUST_APP
    { const int32_t a[6]={x,y,w,h,c,0};(void)cv_rust(3,a,0,0);return; }
#endif

#if CV_RETAINED_LCD
    if(!cv_mutable()) return;
#endif
    int32_t x1 = x + w, y1 = y + h + cv_oy, i;
    uint16_t sc = swap16(c);
    y += cv_oy;                             /* clipped once, then filled row by row */
    if (x < 0)
        x = 0;
    if (y < cv_y0)
        y = cv_y0;
    if (x1 > (int32_t)cv_w)
        x1 = (int32_t)cv_w;
    if (y1 > cv_y1)
        y1 = cv_y1;
    for (; y < y1; y++)
        for (i = x; i < x1; i++)
            cv_px[(uint32_t)y * cv_w + (uint32_t)i] = sc;
}

static void cv_line(int32_t x0, int32_t y0, int32_t x1, int32_t y1, uint16_t c)
{
#if EN_RUST_APP
    { const int32_t a[6]={x0,y0,x1,y1,c,0};(void)cv_rust(4,a,0,0);return; }
#endif

    int32_t dx = x1 > x0 ? x1 - x0 : x0 - x1, sx = x0 < x1 ? 1 : -1;
    int32_t dy = y1 > y0 ? y0 - y1 : y1 - y0, sy = y0 < y1 ? 1 : -1;
    int32_t err = dx + dy, guard = 2000;
    while (guard--) {                       /* bounded: a line is never longer than 480 px */
        int32_t e2 = 2 * err;               /* both tests use the same error value */
        cv_pset(x0, y0, c);
        if (x0 == x1 && y0 == y1)
            break;
        if (e2 >= dy) {
            err += dy;
            x0 += sx;
        }
        if (e2 <= dx) {
            err += dx;
            y0 += sy;
        }
    }
}

/* glyph index of a character: lower case folds to upper case when the font
 * has none, anything missing (and C1 controls) draws as '?' */
static uint32_t glyph(const felucca_font_t *f, uint32_t ch)
{
    if (ch >= 'a' && ch <= 'z' && f->last < 'a')
        ch -= 32u;
    if (ch < f->first || ch > f->last || (ch >= 127u && ch < 160u))
        ch = '?';
    return ch - f->first;
}

/* text in colour c, each pixel's coverage blended over what is already on the canvas (an
 * anti-aliased face on a coloured box gets no dark fringe); returns the end x */
static int32_t cv_text(int32_t x, int32_t y, const felucca_font_t *f, const char *s, uint16_t c)
{
#if EN_RUST_APP
    { const int32_t a[6]={x,y,c,0,0,0};return cv_rust(6,a,f,s); }
#endif

#if CV_RETAINED_LCD
    if(!cv_mutable()) return -1;
#endif
    uint32_t r = c >> 11, g = (c >> 5) & 63u, b = c & 31u;
    for (; *s; s++) {
        uint32_t gi = glyph(f, (uint8_t)*s), gx, gy, w, bpr;
        const uint8_t *gd;
        w = f->bw[gi];
        bpr = (w + 1u) / 2u;
        gd = f->data + f->off[gi];
        for (gy = 0; gy < f->h; gy++)
            for (gx = 0; gx < w; gx++) {
                uint32_t a = gd[gy * bpr + gx / 2u];
                int32_t px = x - f->pad + (int32_t)gx, py = y + (int32_t)gy + cv_oy;
                a = (gx & 1u) ? (a & 15u) : (a >> 4);
                if (!a || (uint32_t)px >= cv_w || py < cv_y0 || py >= cv_y1)
                    continue;
                if (a == 15u) {
                    cv_px[(uint32_t)py * cv_w + (uint32_t)px] = swap16(c);
                } else {
                    uint32_t bg = swap16(cv_px[(uint32_t)py * cv_w + (uint32_t)px]);
                    uint32_t br = bg >> 11, bgg = (bg >> 5) & 63u, bb = bg & 31u;
                    uint32_t nr = (r * a + br * (15u - a)) / 15u, ng = (g * a + bgg * (15u - a)) / 15u,
                             nb = (b * a + bb * (15u - a)) / 15u;
                    cv_px[(uint32_t)py * cv_w + (uint32_t)px] = swap16((nr << 11) | (ng << 5) | nb);
                }
            }
        x += f->adv[gi];
    }
    return x;
}

static int32_t text_w(const felucca_font_t *f, const char *s)
{
#if EN_RUST_APP
    { const fm1_canvas_font view=cv_font_view(f);return fm1_rust_text_width(&view,(const uint8_t*)s,cv_string_length(s)); }
#endif

    int32_t w = 0;
    for (; *s; s++)
        w += f->adv[glyph(f, (uint8_t)*s)];
    return w;
}

/* one-shot: text in a box, cleared to black, blitted */
static void draw_text_box(uint32_t x, uint32_t y, uint32_t w, const felucca_font_t *f, const char *s,
                          uint16_t c, int align)
{
    int32_t tw = text_w(f, s), tx = 0;
    cv_begin(w, f->h, C_BLACK);
    if (align == 1)
        tx = ((int32_t)w - tw) / 2;
    else if (align == 2)
        tx = (int32_t)w - tw;
    cv_text(tx, 0, f, s, c);
    cv_blit(x, y);
    lcd_sync();                     /* one-shots (boot, crash, UBOOT, update) finish here */
}
