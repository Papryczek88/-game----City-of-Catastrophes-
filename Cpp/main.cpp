// City of Catastrophes 8.0 - wersja C++
// Cale sterowanie: strzalki (lub W/A/S/D) + Enter + Esc.
#include <algorithm>
#include <array>
#include <cstdlib>
#include <ctime>
#include <iomanip>
#include <iostream>
#include <string>
#include <vector>

#ifdef _WIN32
#include <conio.h>
#include <windows.h>
#else
#include <sys/select.h>
#include <termios.h>
#include <unistd.h>
#endif

#include "engine.h"

using namespace std;

namespace {

const char* const RESET = "\033[0m";
const char* const CYAN  = "\033[36m";
const char* const ZIEL  = "\033[32m";
const char* const ZOLTY = "\033[33m";
const char* const CZERW = "\033[31m";
const char* const BOLD  = "\033[1m";
const char* const REV   = "\033[44;97m";  // ciemnoniebieskie tlo, bialy tekst
const char* const EOL   = "\033[K\n";

constexpr int SZER = 4;
constexpr int POLA = SZER * SZER;
constexpr int CENA_TARCZY = 200;
constexpr int CENA_NAPRAWY = 50;
constexpr int LECZENIE = 25;
constexpr int SZER_LEWEJ = 24;

struct Typ {
    const char* nazwa;
    char znak;
    int cena, mieszkancy, dochod, atrakcja, czasBudowy;
    bool fabryka;
};

const Typ TYPY[] = {
    {"House",            'H',  50,  5,  0,   0, 1, false},
    {"Apartment block",  'B', 120, 20,  1,   0, 3, false},
    {"Luxury apartment", 'A', 200, 40,  2,  10, 5, false},
    {"Park",             'P',  80,  0,  1,  15, 2, false},
    {"Restaurant",       'R', 150,  5,  5,  20, 3, false},
    {"Farm",             'F',  60,  0,  1,   0, 2, false},
    {"Factory",          'I', 300, 10, 10,  -5, 7, true},
};
constexpr int LICZBA_TYPOW = static_cast<int>(sizeof(TYPY) / sizeof(TYPY[0]));

enum Stan { PUSTE, BUDOWA, GOTOWE };
enum Tryb { T_BUDOWA, T_NAPRAWA };
enum Klawisz { K_BRAK, K_GORA, K_DOL, K_LEWO, K_PRAWO, K_ENTER, K_ESC };

struct Pole {
    int stan = PUSTE;
    int typ = 0;
    int hp = 0;
    int dni = 0;
};

struct Gra {
    array<Pole, POLA> mapa{};
    int pieniadze = 500, ludzie = 0, atrakcja = 0;
    int punkty = 0, dzien = 1, tarcze = 0;
    string komunikat = "Welcome! Expand your city and watch out for disasters.";
};

// ---------------------------------------------------------------- terminal

#ifndef _WIN32
termios g_stare;
bool g_raw = false;
#endif
bool g_terminalWlaczony = false;

void terminalKoniec() {
    if (!g_terminalWlaczony) return;
    g_terminalWlaczony = false;
    cout << RESET << "\033[?25h\033[?1049l" << flush;  // kursor + ekran normalny
#ifndef _WIN32
    if (g_raw) tcsetattr(STDIN_FILENO, TCSANOW, &g_stare);
#endif
}

void terminalStart() {
#ifdef _WIN32
    HANDLE h = GetStdHandle(STD_OUTPUT_HANDLE);
    DWORD tryb = 0;
    if (h != INVALID_HANDLE_VALUE && GetConsoleMode(h, &tryb))
        SetConsoleMode(h, tryb | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
#else
    if (tcgetattr(STDIN_FILENO, &g_stare) == 0) {
        termios nowe = g_stare;
        nowe.c_lflag &= static_cast<tcflag_t>(~(ICANON | ECHO | ISIG));
        nowe.c_cc[VMIN] = 1;
        nowe.c_cc[VTIME] = 0;
        if (tcsetattr(STDIN_FILENO, TCSANOW, &nowe) == 0) g_raw = true;
    }
#endif
    g_terminalWlaczony = true;
    cout << "\033[?1049h\033[?25l\033[2J" << flush;  // osobny ekran, ukryj kursor
    atexit(terminalKoniec);
}

int klawiszZeZnaku(int c) {
    if (c == 'w' || c == 'W') return K_GORA;
    if (c == 's' || c == 'S') return K_DOL;
    if (c == 'a' || c == 'A') return K_LEWO;
    if (c == 'd' || c == 'D') return K_PRAWO;
    if (c == '\r' || c == '\n' || c == ' ') return K_ENTER;
    if (c == 3) return K_ESC;  // Ctrl+C
    return K_BRAK;
}

#ifndef _WIN32
bool czekajNaZnak(int ms) {
    fd_set zestaw;
    FD_ZERO(&zestaw);
    FD_SET(STDIN_FILENO, &zestaw);
    timeval limit;
    limit.tv_sec = 0;
    limit.tv_usec = ms * 1000;
    return select(STDIN_FILENO + 1, &zestaw, nullptr, nullptr, &limit) > 0;
}

bool czytajZnak(unsigned char& c) {
    return czekajNaZnak(60) && read(STDIN_FILENO, &c, 1) == 1;
}
#endif

int wczytajKlawisz() {
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
        int k = klawiszZeZnaku(z);
        if (k != K_BRAK) return k;
    }
#else
    for (;;) {
        unsigned char c = 0;
        if (read(STDIN_FILENO, &c, 1) != 1) exit(0);  // koniec wejscia
        if (c != 27) {
            int k = klawiszZeZnaku(c);
            if (k != K_BRAK) return k;
            continue;
        }
        unsigned char a = 0, b = 0;
        if (!czytajZnak(a) || (a != '[' && a != 'O')) return K_ESC;
        if (!czytajZnak(b)) return K_ESC;
        while ((b >= '0' && b <= '9') || b == ';')  // np. ESC [ 1 ; 5 A
            if (!czytajZnak(b)) return K_ESC;
        if (b == 'A') return K_GORA;
        if (b == 'B') return K_DOL;
        if (b == 'D') return K_LEWO;
        if (b == 'C') return K_PRAWO;
    }
#endif
}

// ---------------------------------------------------------------- rysowanie

void ln(const string& s = "") { cout << s << EOL; }

string kol(const char* kolor, const string& s) {
    return string(kolor) + s + RESET;
}

string pad(const string& s, size_t szer) {
    return s.size() >= szer ? s : s + string(szer - s.size(), ' ');
}

const char* kolorHp(int hp) { return hp > 60 ? ZIEL : (hp > 30 ? ZOLTY : CZERW); }

string pasekHp(int hp) {
    int n = (hp + 9) / 10;
    return kol(kolorHp(hp), "[" + string(n, '#') + string(10 - n, '-') + "] " +
                                to_string(hp) + " HP");
}

string wierszMapy(const Gra& g, int y, int kursor) {
    string s = "  ";
    for (int x = 0; x < SZER; x++) {
        int i = y * SZER + x;
        const Pole& p = g.mapa[i];
        const char* k = RESET;
        char znak = ' ';
        if (p.stan == BUDOWA) { k = ZOLTY; znak = '#'; }
        else if (p.stan == GOTOWE) { k = kolorHp(p.hp); znak = TYPY[p.typ].znak; }

        if (i == kursor)
            s += string(REV) + "[" + znak + "]" + RESET;
        else
            s += "[" + string(k) + znak + RESET + "]";
        s += " ";
    }
    return s;
}

bool budowaTrwa(const Gra& g) {
    for (const Pole& p : g.mapa)
        if (p.stan == BUDOWA) return true;
    return false;
}

// Caly ekran: naglowek, mapa po lewej, kolumna "prawa" obok, linie "dol" pod spodem.
void ekran(const Gra& g, int kursor, const vector<string>& prawa,
           const vector<string>& dol) {
    cout << "\033[H";
    ln(string(BOLD) + CYAN + "  ===== CITY OF CATASTROPHES 8.0 =====" + RESET);
    ln("  Day: " + kol(ZOLTY, to_string(g.dzien)) + "   Money: " +
       kol(ZIEL, to_string(g.pieniadze)) + "   Residents: " + to_string(g.ludzie));
    ln("  Attractiveness: " + to_string(g.atrakcja) + "   Shields: " +
       to_string(g.tarcze) + "   Points: " + to_string(g.punkty));
    ln();

    size_t wiersze = max<size_t>(6, prawa.size());
    for (size_t r = 0; r < wiersze; r++) {
        string lewa;
        if (r == 0)
            lewa = string(BOLD) + "  CITY MAP" + RESET + string(SZER_LEWEJ - 10, ' ');
        else if (r <= SZER)
            lewa = wierszMapy(g, static_cast<int>(r) - 1, kursor) +
                   string(SZER_LEWEJ - 18, ' ');
        else
            lewa = string(SZER_LEWEJ, ' ');
        ln(lewa + (r < prawa.size() ? prawa[r] : ""));
    }

    ln("  H House  B Block  A Apartment  P Park");
    ln("  R Restaurant  F Farm  I Industry  # Under construction");

    string budowy;
    for (const Pole& p : g.mapa) {
        if (p.stan != BUDOWA) continue;
        if (!budowy.empty()) budowy += ", ";
        budowy += string(TYPY[p.typ].nazwa) + " (" + to_string(p.dni) + "d)";
    }
    ln("  Under construction: " + (budowy.empty() ? string("none") : budowy));

    for (const string& s : dol) ln(s);
    cout << "\033[J" << flush;
}

// ---------------------------------------------------------------- logika

void dodajKomunikat(Gra& g, const string& tekst) {
    if (tekst.empty()) return;
    if (!g.komunikat.empty()) g.komunikat += " ";
    g.komunikat += tekst;
}

void przejdzDni(Gra& g, int liczbaDni) {
    for (int d = 0; d < liczbaDni; d++) {
        g.dzien++;
        for (Pole& p : g.mapa) {
            if (p.stan == GOTOWE) {
                g.pieniadze += oblicz_dochod_hp(oblicz_dochod(TYPY[p.typ].dochod), p.hp);
            } else if (p.stan == BUDOWA && --p.dni <= 0) {
                p.stan = GOTOWE;
                g.ludzie += TYPY[p.typ].mieszkancy;
                g.atrakcja += TYPY[p.typ].atrakcja;
                g.punkty += 10;
                dodajKomunikat(g, string("Completed: ") + TYPY[p.typ].nazwa + ".");
            }
        }
    }
}

void katastrofa(Gra& g) {
    static const char* const NAZWY[] = {"Flood", "Earthquake", "Meteor",
                                        "Hurricane", "Fire", "Factory accident"};
    static const int DNI[] = {1, 3, 5, 2, 4, 4};

    bool jestFabryka = false;
    for (const Pole& p : g.mapa)
        if (p.stan == GOTOWE && TYPY[p.typ].fabryka) jestFabryka = true;

    int los = (jestFabryka && losuj_liczbe(0, 1) == 0) ? 5 : losuj_liczbe(0, 4);
    int obrazenia = (los == 5) ? 70 : losuj_liczbe(20, 59);

    if (g.tarcze > 0) {
        obrazenia /= 2;
        g.tarcze--;
        dodajKomunikat(g, "The shield halved the damage.");
    }
    dodajKomunikat(g, string(NAZWY[los]) + "! Damage: -" + to_string(obrazenia) + " HP.");

    for (Pole& p : g.mapa) {
        if (p.stan != GOTOWE) continue;
        WynikKatastrofy w = wykonaj_katastrofe(p.hp, obrazenia);
        p.hp = w.hp;
        if (w.zniszczony) {
            dodajKomunikat(g, string("Destroyed: ") + TYPY[p.typ].nazwa + ".");
            g.ludzie -= TYPY[p.typ].mieszkancy;
            g.atrakcja -= TYPY[p.typ].atrakcja;
            p = Pole{};
        }
    }
    if (g.ludzie < 0) g.ludzie = 0;
    if (g.atrakcja < 0) g.atrakcja = 0;

    przejdzDni(g, DNI[los]);
    dodajKomunikat(g, "The disaster cost " + to_string(DNI[los]) + " days.");
    g.punkty += 30;
}

// ---------------------------------------------------------------- menu

string hint(const string& tekst) { return "  " + string(CYAN) + tekst + RESET; }

int menuGlowne(const Gra& g, int& wybrane) {
    static const char* const POZYCJE[] = {
        "Buy a building", "Buy disaster shield (200)", "Repair a building (50)",
        "Next day", "Cause a disaster", "Exit"};
    const int N = 6;

    for (;;) {
        vector<string> prawa{kol(BOLD, "MENU")};
        for (int i = 0; i < N; i++) {
            string t = string(i == wybrane ? "> " : "  ") + pad(POZYCJE[i], 28);
            prawa.push_back(i == wybrane ? string(REV) + t + RESET : t);
        }
        vector<string> dol{""};
        if (!g.komunikat.empty()) dol.push_back("  " + kol(ZOLTY, g.komunikat));
        dol.push_back("");
        dol.push_back(hint("Up/Down: choose | Enter: confirm | Esc: exit"));
        ekran(g, -1, prawa, dol);

        int k = wczytajKlawisz();
        if (k == K_GORA) wybrane = (wybrane + N - 1) % N;
        else if (k == K_DOL) wybrane = (wybrane + 1) % N;
        else if (k == K_ENTER) return wybrane;
        else if (k == K_ESC) return N - 1;
    }
}

bool potwierdz(const Gra& g, const string& pytanie) {
    bool tak = false;
    for (;;) {
        string opcje = tak ? string("  No    ") + REV + " > Yes " + RESET
                           : string(REV) + " > No " + RESET + "    Yes";
        ekran(g, -1, {kol(BOLD, "CONFIRM")},
              {"", "  " + kol(ZOLTY, pytanie), "  " + opcje, "",
               hint("Left/Right: choose | Enter: confirm | Esc: cancel")});
        int k = wczytajKlawisz();
        if (k == K_LEWO || k == K_PRAWO || k == K_GORA || k == K_DOL) tak = !tak;
        else if (k == K_ENTER) return tak;
        else if (k == K_ESC) return false;
    }
}

int wybierzBudynek(const Gra& g) {
    int wybrane = 0;
    string blad;
    for (;;) {
        vector<string> prawa{kol(BOLD, "BUILDING SHOP")};
        for (int i = 0; i < LICZBA_TYPOW; i++) {
            const Typ& t = TYPY[i];
            string tekst = string(i == wybrane ? "> " : "  ") + pad(t.nazwa, 17) +
                           pad(to_string(t.cena), 5) + to_string(t.czasBudowy) + "d";
            string przod = string(i == wybrane ? REV : (t.cena > g.pieniadze ? CZERW : ""));
            prawa.push_back(przod + tekst + RESET);
        }
        const Typ& t = TYPY[wybrane];
        vector<string> dol{
            "",
            "  Residents: " + to_string(t.mieszkancy) + "   Income: " + to_string(t.dochod) +
                "/day   Attractiveness: " + (t.atrakcja > 0 ? "+" : "") + to_string(t.atrakcja),
            "  " + kol(CZERW, blad),
            hint("Up/Down: choose | Enter: buy | Esc: back")};
        ekran(g, -1, prawa, dol);

        int k = wczytajKlawisz();
        blad.clear();
        if (k == K_GORA) wybrane = (wybrane + LICZBA_TYPOW - 1) % LICZBA_TYPOW;
        else if (k == K_DOL) wybrane = (wybrane + 1) % LICZBA_TYPOW;
        else if (k == K_ESC) return -1;
        else if (k == K_ENTER) {
            if (TYPY[wybrane].cena > g.pieniadze) blad = "You do not have enough money.";
            else return wybrane;
        }
    }
}

string sprawdzPole(const Gra& g, int i, Tryb tryb) {
    const Pole& p = g.mapa[i];
    if (tryb == T_BUDOWA)
        return p.stan == PUSTE ? "" : "This tile is already occupied.";
    if (p.stan != GOTOWE) return "There is no finished building here.";
    if (p.hp >= 100) return "This building is undamaged.";
    return "";
}

int wybierzPole(const Gra& g, Tryb tryb, int typ) {
    int kursor = 0;
    for (int i = 0; i < POLA; i++)
        if (sprawdzPole(g, i, tryb).empty()) { kursor = i; break; }
    string blad;

    for (;;) {
        const Pole& p = g.mapa[kursor];
        vector<string> prawa{kol(BOLD, tryb == T_BUDOWA ? "CHOOSE A CONSTRUCTION SITE"
                                                         : "CHOOSE A BUILDING TO REPAIR")};
        if (tryb == T_BUDOWA) prawa.push_back(string("Placing: ") + TYPY[typ].nazwa);
        if (p.stan == PUSTE) {
            prawa.push_back("Empty tile");
        } else if (p.stan == BUDOWA) {
            prawa.push_back(string(TYPY[p.typ].nazwa) + " (under construction)");
            prawa.push_back("Days remaining: " + to_string(p.dni));
        } else {
            const Typ& t = TYPY[p.typ];
            prawa.push_back(t.nazwa);
            prawa.push_back(pasekHp(p.hp));
            prawa.push_back("Income: " + to_string(oblicz_dochod_hp(t.dochod, p.hp)) +
                            "/day (max " + to_string(t.dochod) + ")");
            prawa.push_back("Residents: " + to_string(t.mieszkancy));
        }
        ekran(g, kursor, prawa,
              {"", "  " + kol(CZERW, blad),
               hint("Arrows: move | Enter: confirm | Esc: cancel")});

        int k = wczytajKlawisz();
        blad.clear();
        if (k == K_GORA && kursor >= SZER) kursor -= SZER;
        else if (k == K_DOL && kursor < POLA - SZER) kursor += SZER;
        else if (k == K_LEWO && kursor % SZER > 0) kursor--;
        else if (k == K_PRAWO && kursor % SZER < SZER - 1) kursor++;
        else if (k == K_ESC) return -1;
        else if (k == K_ENTER) {
            blad = sprawdzPole(g, kursor, tryb);
            if (blad.empty()) return kursor;
        }
    }
}

void podsumowanie(const Gra& g) {
    cout << "  Days: " << g.dzien << "\n  Points: " << g.punkty
         << "\n  Residents: " << g.ludzie << "\n  Money: " << g.pieniadze
         << "\n  Attractiveness: " << g.atrakcja << "\n";
}

}  // namespace

int main() {
    engine_init(static_cast<unsigned int>(time(nullptr)));
    terminalStart();

    Gra g;
    int wybrane = 0;
    bool upadek = false;

    for (;;) {
        int wybor = menuGlowne(g, wybrane);
        g.komunikat.clear();

        if (wybor == 0) {
            int zajete = static_cast<int>(count_if(g.mapa.begin(), g.mapa.end(),
                [](const Pole& p) { return p.stan != PUSTE; }));
            if (zajete >= POLA) g.komunikat = "The city is full.";
            if (g.komunikat.empty()) {
                int typ = wybierzBudynek(g);
                if (typ >= 0) {
                    int miejsce = wybierzPole(g, T_BUDOWA, typ);
                    if (miejsce >= 0) {
                        g.pieniadze -= TYPY[typ].cena;
                        g.mapa[miejsce] = Pole{BUDOWA, typ, 100, TYPY[typ].czasBudowy};
                        g.komunikat = string("Construction started: ") + TYPY[typ].nazwa + ".";
                    } else {
                        g.komunikat = "Cancelled.";
                    }
                }
            }
        } else if (wybor == 1) {
            if (g.pieniadze >= CENA_TARCZY) {
                g.pieniadze -= CENA_TARCZY;
                g.tarcze++;
                g.komunikat = "Disaster shield purchased.";
            } else {
                g.komunikat = "You do not have enough money for a shield.";
            }
        } else if (wybor == 2) {
            bool uszkodzone = false;
            for (int i = 0; i < POLA; i++)
                if (sprawdzPole(g, i, T_NAPRAWA).empty()) uszkodzone = true;

            if (!uszkodzone) g.komunikat = "Nothing to repair.";
            else if (g.pieniadze < CENA_NAPRAWY) g.komunikat = "You do not have enough money for repairs.";
            else {
                int miejsce = wybierzPole(g, T_NAPRAWA, -1);
                if (miejsce >= 0) {
                    Pole& p = g.mapa[miejsce];
                    p.hp = oblicz_nowe_hp(p.hp, LECZENIE);
                    g.pieniadze -= CENA_NAPRAWY;
                    g.komunikat = string("Repaired: ") + TYPY[p.typ].nazwa + ".";
                }
            }
        } else if (wybor == 3) {
            przejdzDni(g, 1);
            dodajKomunikat(g, "Day " + to_string(g.dzien) + ".");
        } else if (wybor == 4) {
            if (potwierdz(g, "Cause a disaster? Buildings will be damaged (+30 points)."))
                katastrofa(g);
        } else if (wybor == 5) {
            if (potwierdz(g, "Exit the game?")) break;
        }

        if (g.ludzie <= 0 && g.dzien > 5 && !budowaTrwa(g)) {
            upadek = true;
            break;
        }
    }

    // ekran koncowy
    cout << "\033[2J\033[H" << BOLD << CYAN
         << (upadek ? "  The city has collapsed - no residents remain.\n" : "")
         << "  GAME OVER\n" << RESET;
    podsumowanie(g);
    cout << "\n  Press any key..." << flush;
    wczytajKlawisz();

    terminalKoniec();
    cout << (upadek ? "The city has collapsed.\n" : "") << "GAME OVER\n";
    podsumowanie(g);
    return 0;
}
