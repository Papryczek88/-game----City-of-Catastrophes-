#include "engine.h"
#include <stdlib.h>

void engine_init(unsigned int seed) {
    srand(seed);
}

int losuj_liczbe(int min, int max) {
    if (min > max) {
        int temp = min;
        min = max;
        max = temp;
    }
    return min + rand() % (max - min + 1);
}

int oblicz_dochod(int dochod) {
    return dochod > 0 ? dochod : 0;
}

int oblicz_dochod_hp(int dochod, int hp) {
#ifdef COC_USE_ASM
    return coc_asm_income_hp(dochod, hp);
#else
    if (dochod <= 0 || hp <= 0) return 0;
    if (hp > 100) hp = 100;
    return (int)(((long long)dochod * hp + 50) / 100);
#endif
}

WynikKatastrofy wykonaj_katastrofe(int hp, int obrazenia) {
    WynikKatastrofy wynik;
    if (obrazenia < 0) obrazenia = 0;

    wynik.hp = hp - obrazenia;
    if (wynik.hp < 0) wynik.hp = 0;

    wynik.obrazenia = obrazenia;
    wynik.zniszczony = (wynik.hp == 0);
    return wynik;
}

int oblicz_nowe_hp(int hp, int leczenie) {
    hp += leczenie;
    if (hp > 100) hp = 100;
    if (hp < 0) hp = 0;
    return hp;
}
