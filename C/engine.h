#ifndef ENGINE_H
#define ENGINE_H

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    int hp;
    int obrazenia;
    int zniszczony;
} WynikKatastrofy;

void engine_init(unsigned int seed);
int losuj_liczbe(int min, int max);
int oblicz_dochod(int dochod);
/* NOWE: dochod budynku zalezy od jego zdrowia (hp 0..100) */
int oblicz_dochod_hp(int dochod, int hp);
#ifdef COC_USE_ASM
int coc_asm_income_hp(int dochod, int hp);
#endif
WynikKatastrofy wykonaj_katastrofe(int hp, int obrazenia);
int oblicz_nowe_hp(int hp, int leczenie);

#ifdef __cplusplus
}
#endif

#endif
