/* City of Catastrophes 8.0 - wersja C
 * Cale sterowanie: strzalki (lub W/A/S/D) + Enter + Esc. */
#define _POSIX_C_SOURCE 200809L

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#ifdef _WIN32
#include <conio.h>
#include <windows.h>
#else
#include <sys/select.h>
#include <termios.h>
#include <unistd.h>
#endif

#include "engine.h"

#define RESET "\033[0m"
#define CYAN  "\033[36m"
#define ZIEL  "\033[32m"
#define ZOLTY "\033[33m"
#define CZERW "\033[31m"
#define BOLD  "\033[1m"
#define REV   "\033[44;97m" /* ciemnoniebieskie tlo, bialy tekst */
#define EOL   "\033[K\n"

#define SZER 4
#define POLA (SZER * SZER)
#define CENA_TARCZY 200
#define CENA_NAPRAWY 50
#define LECZENIE 25
#define SZER_LEWEJ 24

typedef struct {
    const char* nazwa;
    char znak;
    int cena, mieszkancy, dochod, atrakcja, czasBudowy;
    int fabryka;
} Typ;

static const Typ TYPY[] = {
    {"House",            'H',  50,  5,  0,   0, 1, 0},
    {"Apartment block",  'B', 120, 20,  1,   0, 3, 0},
    {"Luxury apartment", 'A', 200, 40,  2,  10, 5, 0},
    {"Park",             'P',  80,  0,  1,  15, 2, 0},
    {"Restaurant",       'R', 150,  5,  5,  20, 3, 0},
    {"Farm",             'F',  60,  0,  1,   0, 2, 0},
    {"Factory",          'I', 300, 10, 10,  -5, 7, 1},
};
#define LICZBA_TYPOW ((int)(sizeof(TYPY) / sizeof(TYPY[0])))

enum { PUSTE, BUDOWA, GOTOWE };
enum { T_BUDOWA, T_NAPRAWA };
enum { K_BRAK, K_GORA, K_DOL, K_LEWO, K_PRAWO, K_ENTER, K_ESC };

typedef struct {
    int stan, typ, hp, dni;
} Pole;

typedef struct {
    Pole mapa[POLA];
    int pieniadze, ludzie, atrakcja, punkty, dzien, tarcze;
    char komunikat[512];
} Gra;

/* Kolumna tekstu (np. menu po prawej, linie na dole ekranu). */
#define MAX_L 12
typedef struct {
    char t[MAX_L][240];
    int n;
} Linie;

/* ---------------------------------------------------------------- terminal */

#ifndef _WIN32
static struct termios g_stare;
static int g_raw = 0;
#endif
static int g_terminalWlaczony = 0;

static void terminalKoniec(void) {
    if (!g_terminalWlaczony) return;
    g_terminalWlaczony = 0;
    fputs(RESET "\033[?25h\033[?1049l", stdout); /* kursor + ekran normalny */
    fflush(stdout);
#ifndef _WIN32
    if (g_raw) tcsetattr(STDIN_FILENO, TCSANOW, &g_stare);
#endif
}

static void terminalStart(void) {
#ifdef _WIN32
    HANDLE h = GetStdHandle(STD_OUTPUT_HANDLE);
    DWORD tryb = 0;
    if (h != INVALID_HANDLE_VALUE && GetConsoleMode(h, &tryb))
        SetConsoleMode(h, tryb | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
#else
    if (tcgetattr(STDIN_FILENO, &g_stare) == 0) {
        struct termios nowe = g_stare;
        nowe.c_lflag &= (tcflag_t)~(ICANON | ECHO | ISIG);
        nowe.c_cc[VMIN] = 1;
        nowe.c_cc[VTIME] = 0;
        if (tcsetattr(STDIN_FILENO, TCSANOW, &nowe) == 0) g_raw = 1;
    }
#endif
    g_terminalWlaczony = 1;
    fputs("\033[?1049h\033[?25l\033[2J", stdout); /* osobny ekran, ukryj kursor */
    fflush(stdout);
    atexit(terminalKoniec);
}

static int klawiszZeZnaku(int c) {
    if (c == 'w' || c == 'W') return K_GORA;
    if (c == 's' || c == 'S') return K_DOL;
    if (c == 'a' || c == 'A') return K_LEWO;
    if (c == 'd' || c == 'D') return K_PRAWO;
    if (c == '\r' || c == '\n' || c == ' ') return K_ENTER;
    if (c == 3) return K_ESC; /* Ctrl+C */
    return K_BRAK;
}

#ifndef _WIN32
static int czytajZnak(unsigned char* c) {
    fd_set zestaw;
    struct timeval limit;
    FD_ZERO(&zestaw);
    FD_SET(STDIN_FILENO, &zestaw);
    limit.tv_sec = 0;
    limit.tv_usec = 60000;
    if (select(STDIN_FILENO + 1, &zestaw, NULL, NULL, &limit) <= 0) return 0;
    return read(STDIN_FILENO, c, 1) == 1;
}
#endif

static int wczytajKlawisz(void) {
#ifdef _WIN32
    for (;;) {
        int z = _getch();
        if (z == 0 || z == 224) {
            z = _getch();
            if (z == 72) return K_GORA;
            if (z == 80) return K_DOL;
            if (z == 75) return K_LEWO;
            if (z == 77) return K_PRAWO;
            continue;
        }
        if (z == 27) return K_ESC;
        {
            int k = klawiszZeZnaku(z);
            if (k != K_BRAK) return k;
        }
    }
#else
    for (;;) {
        unsigned char c = 0, a = 0, b = 0;
        if (read(STDIN_FILENO, &c, 1) != 1) exit(0); /* koniec wejscia */
        if (c != 27) {
            int k = klawiszZeZnaku(c);
            if (k != K_BRAK) return k;
            continue;
        }
        if (!czytajZnak(&a) || (a != '[' && a != 'O')) return K_ESC;
        if (!czytajZnak(&b)) return K_ESC;
        while ((b >= '0' && b <= '9') || b == ';') /* np. ESC [ 1 ; 5 A */
            if (!czytajZnak(&b)) return K_ESC;
        if (b == 'A') return K_GORA;
        if (b == 'B') return K_DOL;
        if (b == 'D') return K_LEWO;
        if (b == 'C') return K_PRAWO;
    }
#endif
}

/* ---------------------------------------------------------------- rysowanie */

static void ln(const char* fmt, ...) {
    va_list a;
    va_start(a, fmt);
    vprintf(fmt, a);
    va_end(a);
    fputs(EOL, stdout);
}

static void dodaj(Linie* l, const char* fmt, ...) {
    va_list a;
    if (l->n >= MAX_L) return;
    va_start(a, fmt);
    vsnprintf(l->t[l->n], sizeof l->t[0], fmt, a);
    va_end(a);
    l->n++;
}

static void app(char* bufor, size_t pojemnosc, const char* fmt, ...) {
    size_t dl = strlen(bufor);
    va_list a;
    va_start(a, fmt);
    vsnprintf(bufor + dl, pojemnosc - dl, fmt, a);
    va_end(a);
}

static const char* kolorHp(int hp) { return hp > 60 ? ZIEL : (hp > 30 ? ZOLTY : CZERW); }

static void pasekHp(char* out, size_t n, int hp) {
    char kreski[16], puste[16];
    int ile = (hp + 9) / 10, i;
    for (i = 0; i < ile; i++) kreski[i] = '#';
    kreski[ile] = '\0';
    for (i = 0; i < 10 - ile; i++) puste[i] = '-';
    puste[10 - ile] = '\0';
    snprintf(out, n, "%s[%s%s] %d HP" RESET, kolorHp(hp), kreski, puste, hp);
}

static void wierszMapy(char* out, size_t n, const Gra* g, int y, int kursor) {
    int x;
    snprintf(out, n, "  ");
    for (x = 0; x < SZER; x++) {
        int i = y * SZER + x;
        const Pole* p = &g->mapa[i];
        const char* k = RESET;
        char znak = ' ';
        if (p->stan == BUDOWA) { k = ZOLTY; znak = '#'; }
        else if (p->stan == GOTOWE) { k = kolorHp(p->hp); znak = TYPY[p->typ].znak; }

        if (i == kursor) app(out, n, REV "[%c]" RESET " ", znak);
        else app(out, n, "[%s%c" RESET "] ", k, znak);
    }
}

static int budowaTrwa(const Gra* g) {
    int i;
    for (i = 0; i < POLA; i++)
        if (g->mapa[i].stan == BUDOWA) return 1;
    return 0;
}

/* Caly ekran: naglowek, mapa po lewej, kolumna "prawa" obok, linie "dol" pod spodem. */
static void ekran(const Gra* g, int kursor, const Linie* prawa, const Linie* dol) {
    char budowy[400] = "";
    char lewa[300];
    int wiersze = prawa->n > 6 ? prawa->n : 6;
    int r, i;

    fputs("\033[H", stdout);
    ln(BOLD CYAN "  ===== CITY OF CATASTROPHES 8.0 =====" RESET);
    ln("  Day: " ZOLTY "%d" RESET "   Money: " ZIEL "%d" RESET "   Residents: %d",
       g->dzien, g->pieniadze, g->ludzie);
    ln("  Attractiveness: %d   Shields: %d   Points: %d", g->atrakcja, g->tarcze, g->punkty);
    ln("%s", "");

    for (r = 0; r < wiersze; r++) {
        if (r == 0) snprintf(lewa, sizeof lewa, BOLD "  CITY MAP" RESET "%*s", SZER_LEWEJ - 10, "");
        else if (r <= SZER) {
            wierszMapy(lewa, sizeof lewa, g, r - 1, kursor);
            app(lewa, sizeof lewa, "%*s", SZER_LEWEJ - 18, "");
        } else snprintf(lewa, sizeof lewa, "%*s", SZER_LEWEJ, "");
        ln("%s%s", lewa, r < prawa->n ? prawa->t[r] : "");
    }

    ln("  H House  B Block  A Apartment  P Park");
    ln("  R Restaurant  F Farm  I Industry  # Under construction");

    for (i = 0; i < POLA; i++) {
        const Pole* p = &g->mapa[i];
        if (p->stan != BUDOWA) continue;
        app(budowy, sizeof budowy, "%s%s (%dd)", budowy[0] ? ", " : "", TYPY[p->typ].nazwa, p->dni);
    }
    ln("  Under construction: %s", budowy[0] ? budowy : "none");

    for (i = 0; i < dol->n; i++) ln("%s", dol->t[i]);
    fputs("\033[J", stdout);
    fflush(stdout);
}

/* ---------------------------------------------------------------- logika */

static void dodajKomunikat(Gra* g, const char* fmt, ...) {
    char nowy[256];
    va_list a;
    size_t dl = strlen(g->komunikat);
    va_start(a, fmt);
    vsnprintf(nowy, sizeof nowy, fmt, a);
    va_end(a);
    if (!nowy[0]) return;
    app(g->komunikat, sizeof g->komunikat, "%s%s", dl ? " " : "", nowy);
}

static void przejdzDni(Gra* g, int liczbaDni) {
    int d, i;
    for (d = 0; d < liczbaDni; d++) {
        g->dzien++;
        for (i = 0; i < POLA; i++) {
            Pole* p = &g->mapa[i];
            if (p->stan == GOTOWE) {
                g->pieniadze += oblicz_dochod_hp(oblicz_dochod(TYPY[p->typ].dochod), p->hp);
            } else if (p->stan == BUDOWA && --p->dni <= 0) {
                p->stan = GOTOWE;
                g->ludzie += TYPY[p->typ].mieszkancy;
                g->atrakcja += TYPY[p->typ].atrakcja;
                g->punkty += 10;
                dodajKomunikat(g, "Completed: %s.", TYPY[p->typ].nazwa);
            }
        }
    }
}

static void katastrofa(Gra* g) {
    static const char* const NAZWY[] = {"Flood", "Earthquake", "Meteor",
                                        "Hurricane", "Fire", "Factory accident"};
    static const int DNI[] = {1, 3, 5, 2, 4, 4};
    int jestFabryka = 0, los, obrazenia, i;

    for (i = 0; i < POLA; i++)
        if (g->mapa[i].stan == GOTOWE && TYPY[g->mapa[i].typ].fabryka) jestFabryka = 1;

    los = (jestFabryka && losuj_liczbe(0, 1) == 0) ? 5 : losuj_liczbe(0, 4);
    obrazenia = (los == 5) ? 70 : losuj_liczbe(20, 59);

    if (g->tarcze > 0) {
        obrazenia /= 2;
        g->tarcze--;
        dodajKomunikat(g, "The shield halved the damage.");
    }
    dodajKomunikat(g, "%s! Damage: -%d HP.", NAZWY[los], obrazenia);

    for (i = 0; i < POLA; i++) {
        Pole* p = &g->mapa[i];
        WynikKatastrofy w;
        if (p->stan != GOTOWE) continue;
        w = wykonaj_katastrofe(p->hp, obrazenia);
        p->hp = w.hp;
        if (w.zniszczony) {
            dodajKomunikat(g, "Destroyed: %s.", TYPY[p->typ].nazwa);
            g->ludzie -= TYPY[p->typ].mieszkancy;
            g->atrakcja -= TYPY[p->typ].atrakcja;
            memset(p, 0, sizeof *p);
        }
    }
    if (g->ludzie < 0) g->ludzie = 0;
    if (g->atrakcja < 0) g->atrakcja = 0;

    przejdzDni(g, DNI[los]);
    dodajKomunikat(g, "The disaster cost %d days.", DNI[los]);
    g->punkty += 30;
}

/* ---------------------------------------------------------------- menu */

static void hint(Linie* l, const char* tekst) { dodaj(l, "  " CYAN "%s" RESET, tekst); }

static int menuGlowne(const Gra* g, int* wybrane) {
    static const char* const POZYCJE[] = {
        "Buy a building", "Buy disaster shield (200)", "Repair a building (50)",
        "Next day", "Cause a disaster", "Exit"};
    const int N = 6;

    for (;;) {
        Linie prawa = {0}, dol = {0};
        int i, k;
        dodaj(&prawa, BOLD "MENU" RESET);
        for (i = 0; i < N; i++) {
            if (i == *wybrane) dodaj(&prawa, REV "> %-28s" RESET, POZYCJE[i]);
            else dodaj(&prawa, "  %-28s", POZYCJE[i]);
        }
        dodaj(&dol, "%s", "");
        if (g->komunikat[0]) dodaj(&dol, "  " ZOLTY "%s" RESET, g->komunikat);
        dodaj(&dol, "%s", "");
        hint(&dol, "Up/Down: choose | Enter: confirm | Esc: exit");
        ekran(g, -1, &prawa, &dol);

        k = wczytajKlawisz();
        if (k == K_GORA) *wybrane = (*wybrane + N - 1) % N;
        else if (k == K_DOL) *wybrane = (*wybrane + 1) % N;
        else if (k == K_ENTER) return *wybrane;
        else if (k == K_ESC) return N - 1;
    }
}

static int potwierdz(const Gra* g, const char* pytanie) {
    int tak = 0;
    for (;;) {
        Linie prawa = {0}, dol = {0};
        int k;
        dodaj(&prawa, BOLD "CONFIRM" RESET);
        dodaj(&dol, "%s", "");
        dodaj(&dol, "  " ZOLTY "%s" RESET, pytanie);
        if (tak) dodaj(&dol, "  No    " REV " > Yes " RESET);
        else dodaj(&dol, "  " REV " > No " RESET "    Yes");
        dodaj(&dol, "%s", "");
        hint(&dol, "Left/Right: choose | Enter: confirm | Esc: cancel");
        ekran(g, -1, &prawa, &dol);

        k = wczytajKlawisz();
        if (k == K_LEWO || k == K_PRAWO || k == K_GORA || k == K_DOL) tak = !tak;
        else if (k == K_ENTER) return tak;
        else if (k == K_ESC) return 0;
    }
}

static int wybierzBudynek(const Gra* g) {
    int wybrane = 0;
    char blad[64] = "";
    for (;;) {
        Linie prawa = {0}, dol = {0};
        const Typ* t = &TYPY[wybrane];
        int i, k;
        dodaj(&prawa, BOLD "BUILDING SHOP" RESET);
        for (i = 0; i < LICZBA_TYPOW; i++) {
            dodaj(&prawa, "%s%s%-17s%-5d%dd" RESET,
                  i == wybrane ? REV : (TYPY[i].cena > g->pieniadze ? CZERW : ""),
                  i == wybrane ? "> " : "  ", TYPY[i].nazwa, TYPY[i].cena, TYPY[i].czasBudowy);
        }
        dodaj(&dol, "%s", "");
        dodaj(&dol, "  Residents: %d   Income: %d/day   Attractiveness: %s%d", t->mieszkancy,
              t->dochod, t->atrakcja > 0 ? "+" : "", t->atrakcja);
        dodaj(&dol, "  " CZERW "%s" RESET, blad);
        hint(&dol, "Up/Down: choose | Enter: buy | Esc: back");
        ekran(g, -1, &prawa, &dol);

        k = wczytajKlawisz();
        blad[0] = '\0';
        if (k == K_GORA) wybrane = (wybrane + LICZBA_TYPOW - 1) % LICZBA_TYPOW;
        else if (k == K_DOL) wybrane = (wybrane + 1) % LICZBA_TYPOW;
        else if (k == K_ESC) return -1;
        else if (k == K_ENTER) {
            if (TYPY[wybrane].cena > g->pieniadze) snprintf(blad, sizeof blad, "You do not have enough money.");
            else return wybrane;
        }
    }
}

/* Zwraca NULL gdy pole jest poprawne, inaczej powod odmowy. */
static const char* sprawdzPole(const Gra* g, int i, int tryb) {
    const Pole* p = &g->mapa[i];
    if (tryb == T_BUDOWA) return p->stan == PUSTE ? NULL : "This tile is already occupied.";
    if (p->stan != GOTOWE) return "There is no finished building here.";
    if (p->hp >= 100) return "This building is undamaged.";
    return NULL;
}

static int wybierzPole(const Gra* g, int tryb, int typ) {
    int kursor = 0, i;
    const char* blad = "";
    for (i = 0; i < POLA; i++)
        if (!sprawdzPole(g, i, tryb)) { kursor = i; break; }

    for (;;) {
        Linie prawa = {0}, dol = {0};
        const Pole* p = &g->mapa[kursor];
        int k;
        dodaj(&prawa, BOLD "%s" RESET, tryb == T_BUDOWA ? "CHOOSE A CONSTRUCTION SITE"
                                                          : "CHOOSE A BUILDING TO REPAIR");
        if (tryb == T_BUDOWA) dodaj(&prawa, "Placing: %s", TYPY[typ].nazwa);
        if (p->stan == PUSTE) {
            dodaj(&prawa, "Empty tile");
        } else if (p->stan == BUDOWA) {
            dodaj(&prawa, "%s (under construction)", TYPY[p->typ].nazwa);
            dodaj(&prawa, "Days remaining: %d", p->dni);
        } else {
            const Typ* t = &TYPY[p->typ];
            char pasek[100];
            pasekHp(pasek, sizeof pasek, p->hp);
            dodaj(&prawa, "%s", t->nazwa);
            dodaj(&prawa, "%s", pasek);
            dodaj(&prawa, "Income: %d/day (max %d)", oblicz_dochod_hp(t->dochod, p->hp), t->dochod);
            dodaj(&prawa, "Residents: %d", t->mieszkancy);
        }
        dodaj(&dol, "%s", "");
        dodaj(&dol, "  " CZERW "%s" RESET, blad);
        hint(&dol, "Arrows: move | Enter: confirm | Esc: cancel");
        ekran(g, kursor, &prawa, &dol);

        k = wczytajKlawisz();
        blad = "";
        if (k == K_GORA && kursor >= SZER) kursor -= SZER;
        else if (k == K_DOL && kursor < POLA - SZER) kursor += SZER;
        else if (k == K_LEWO && kursor % SZER > 0) kursor--;
        else if (k == K_PRAWO && kursor % SZER < SZER - 1) kursor++;
        else if (k == K_ESC) return -1;
        else if (k == K_ENTER) {
            const char* powod = sprawdzPole(g, kursor, tryb);
            if (!powod) return kursor;
            blad = powod;
        }
    }
}

static void podsumowanie(const Gra* g) {
    printf("  Days: %d\n  Points: %d\n  Residents: %d\n  Money: %d\n  Attractiveness: %d\n",
           g->dzien, g->punkty, g->ludzie, g->pieniadze, g->atrakcja);
}

int main(void) {
    Gra g;
    int wybrane = 0, upadek = 0;

    memset(&g, 0, sizeof g);
    g.pieniadze = 500;
    g.dzien = 1;
    snprintf(g.komunikat, sizeof g.komunikat, "Welcome! Expand your city and watch out for disasters.");

    setvbuf(stdout, NULL, _IOFBF, 1 << 16);
    engine_init((unsigned int)time(NULL));
    terminalStart();

    for (;;) {
        int wybor = menuGlowne(&g, &wybrane);
        g.komunikat[0] = '\0';

        if (wybor == 0) {
            int zajete = 0, i;
            for (i = 0; i < POLA; i++)
                if (g.mapa[i].stan != PUSTE) zajete++;

            if (zajete >= POLA) {
                snprintf(g.komunikat, sizeof g.komunikat, "The city is full.");
            } else {
                int typ = wybierzBudynek(&g);
                if (typ >= 0) {
                    int miejsce = wybierzPole(&g, T_BUDOWA, typ);
                    if (miejsce >= 0) {
                        g.pieniadze -= TYPY[typ].cena;
                        g.mapa[miejsce].stan = BUDOWA;
                        g.mapa[miejsce].typ = typ;
                        g.mapa[miejsce].hp = 100;
                        g.mapa[miejsce].dni = TYPY[typ].czasBudowy;
                        snprintf(g.komunikat, sizeof g.komunikat, "Construction started: %s.", TYPY[typ].nazwa);
                    } else {
                        snprintf(g.komunikat, sizeof g.komunikat, "Cancelled.");
                    }
                }
            }
        } else if (wybor == 1) {
            if (g.pieniadze >= CENA_TARCZY) {
                g.pieniadze -= CENA_TARCZY;
                g.tarcze++;
                snprintf(g.komunikat, sizeof g.komunikat, "Disaster shield purchased.");
            } else {
                snprintf(g.komunikat, sizeof g.komunikat, "You do not have enough money for a shield.");
            }
        } else if (wybor == 2) {
            int uszkodzone = 0, i;
            for (i = 0; i < POLA; i++)
                if (!sprawdzPole(&g, i, T_NAPRAWA)) uszkodzone = 1;

            if (!uszkodzone) {
                snprintf(g.komunikat, sizeof g.komunikat, "Nothing to repair.");
            } else if (g.pieniadze < CENA_NAPRAWY) {
                snprintf(g.komunikat, sizeof g.komunikat, "You do not have enough money for repairs.");
            } else {
                int miejsce = wybierzPole(&g, T_NAPRAWA, -1);
                if (miejsce >= 0) {
                    Pole* p = &g.mapa[miejsce];
                    p->hp = oblicz_nowe_hp(p->hp, LECZENIE);
                    g.pieniadze -= CENA_NAPRAWY;
                    snprintf(g.komunikat, sizeof g.komunikat, "Repaired: %s.", TYPY[p->typ].nazwa);
                }
            }
        } else if (wybor == 3) {
            przejdzDni(&g, 1);
            dodajKomunikat(&g, "Day %d.", g.dzien);
        } else if (wybor == 4) {
            if (potwierdz(&g, "Cause a disaster? Buildings will be damaged (+30 points)."))
                katastrofa(&g);
        } else if (wybor == 5) {
            if (potwierdz(&g, "Exit the game?")) break;
        }

        if (g.ludzie <= 0 && g.dzien > 5 && !budowaTrwa(&g)) {
            upadek = 1;
            break;
        }
    }

    /* ekran koncowy */
    fputs("\033[2J\033[H" BOLD CYAN, stdout);
    if (upadek) fputs("  The city has collapsed - no residents remain.\n", stdout);
    fputs("  GAME OVER\n" RESET, stdout);
    podsumowanie(&g);
    fputs("\n  Press any key...", stdout);
    fflush(stdout);
    wczytajKlawisz();

    terminalKoniec();
    if (upadek) puts("The city has collapsed.");
    puts("GAME OVER");
    podsumowanie(&g);
    return 0;
}
