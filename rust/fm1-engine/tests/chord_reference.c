/* SPDX-License-Identifier: GPL-3.0-only */
/* Explicit oracle generator, never run by cargo test. The checked fixture and
 * hashes were recorded from citypop-voices-20261113/source's qualified C test.
 * To compare a reference checkout, compile with
 * -DCHORD_REFERENCE_SOURCE='"/absolute/reference/tests/host/chord_plan_test.c"'
 * and run from the repository root. Do not replace goldens from Rust output. */
/* Recorded SHA256: jam.h 0cce8edc1e94e11b08e627b35977d86208aa4987191dd36255008993ba4149f1
 * chord_plan.inc 9e36ef32c7c3f16d7666af02b21161653e0f0311ca1547dedf94945a58fb73a6
 * literal-chords.jmp dfa1d0f2be70a9531188379c44f13f9d6f99a774ca318f646791b4fdef4ab8f5 */
#ifndef CHORD_REFERENCE_SOURCE
#define CHORD_REFERENCE_SOURCE "../../../tests/host/chord_plan_test.c"
#endif
#define main qualified_chord_test_main
#include CHORD_REFERENCE_SOURCE
#undef main
int main(void) {
    uint8_t p[JP_MAX]; literal_fixture(p);
    FILE *f=fopen("rust/fm1-engine/tests/packs/literal-chords.jmp","wb");assert(f);
    assert(fwrite(p,1,JP2_META,f)==JP2_META);fclose(f);
    printf("closed: %08x\n",legacy_hash(p));
    p[JP2_HEAD+21]=1;printf("drop2: %08x\n",legacy_hash(p));
    return 0;
}
