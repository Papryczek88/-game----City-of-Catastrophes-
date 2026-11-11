#include <iostream>
#include <vector>
#include <string>
#include <sstream>
#include <ctime>
#include <limits>

#ifdef _WIN32
#include <windows.h>
#include <conio.h>
#else
#include <termios.h>
#include <unistd.h>
#include <sys/select.h>
#endif

#include "engine.h"

using namespace std;

struct Budynek {
    string nazwa;
    char znak;
    int hp;
    int cena;
    int mieszkancy;
    int dochod;
    int atrakcja;
    int czasBudowy;
    bool fabryka;
    int miejsce;
};

struct Budowa {
    Budynek budynek;
    int dni;
    int miejsce;
};

const string RESET  = "\033[0m";
const string CYAN   = "\033[36m";
const string ZIEL   = "\033[32m";
const string ZOLTY  = "\033[33m";
const string CZERW  = "\033[31m";
const string BOLD   = "\033[1m";

const int ROZMIAR_MAPY = 16;
const int KLAWISZ_GORA = 1;
const int KLAWISZ_DOL = 2;
const int KLAWISZ_LEWO = 3;
const int KLAWISZ_PRAWO = 4;
const int KLAWISZ_ENTER = 5;
const int KLAWISZ_ESC = 6;

void wlaczKoloryWindows() {
#ifdef _WIN32
    HANDLE uchwyt = GetStdHandle(STD_OUTPUT_HANDLE);
    if (uchwyt == INVALID_HANDLE_VALUE) return;

    DWORD tryb = 0;
    if (GetConsoleMode(uchwyt, &tryb)) {
        SetConsoleMode(uchwyt, tryb | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
    }
#endif
}

void wyczyscEkran() {
    cout << "\033[2J\033[H";
}

bool wczytajLiczbe(const string& komunikat, int& liczba) {
    string linia;
    while (true) {
        cout << komunikat;
        if (!getline(cin, linia)) return false;

        istringstream wejscie(linia);
        string nadmiar;
        if ((wejscie >> liczba) && !(wejscie >> nadmiar)) return true;

        cout << CZERW << "Enter a valid number.\n" << RESET;
    }
}

int wczytajKlawisz() {
#ifdef _WIN32
    int znak = _getch();

    if (znak == 0 || znak == 224) {
        znak = _getch();
        if (znak == 72) return KLAWISZ_GORA;
        if (znak == 80) return KLAWISZ_DOL;
        if (znak == 75) return KLAWISZ_LEWO;
        if (znak == 77) return KLAWISZ_PRAWO;
        return 0;
    }

    if (znak == 13) return KLAWISZ_ENTER;
    if (znak == 27) return KLAWISZ_ESC;
    return 0;
#else
    termios stareUstawienia;
    if (tcgetattr(STDIN_FILENO, &stareUstawienia) != 0) return 0;

    termios noweUstawienia = stareUstawienia;
    noweUstawienia.c_lflag &= static_cast<tcflag_t>(~(ICANON | ECHO));
    noweUstawienia.c_cc[VMIN] = 1;
    noweUstawienia.c_cc[VTIME] = 0;

    if (tcsetattr(STDIN_FILENO, TCSANOW, &noweUstawienia) != 0) return 0;

    auto przywrocTerminal = [&]() {
        tcsetattr(STDIN_FILENO, TCSANOW, &stareUstawienia);
    };

    auto czytajZLimitemCzasu = [](unsigned char& znak) -> bool {
        fd_set zestaw;
        FD_ZERO(&zestaw);
        FD_SET(STDIN_FILENO, &zestaw);

        timeval limit;
        limit.tv_sec = 0;
        limit.tv_usec = 150000;

        int gotowe = select(STDIN_FILENO + 1, &zestaw, nullptr, nullptr, &limit);
        if (gotowe <= 0) return false;
        return read(STDIN_FILENO, &znak, 1) == 1;
    };

    unsigned char znak = 0;
    if (read(STDIN_FILENO, &znak, 1) != 1) {
        przywrocTerminal();
        return 0;
    }

    if (znak == '\n' || znak == '\r') {
        przywrocTerminal();
        return KLAWISZ_ENTER;
    }

    if (znak != 27) {
        przywrocTerminal();
        return 0;
    }

    unsigned char drugiZnak = 0, trzeciZnak = 0;
    if (!czytajZLimitemCzasu(drugiZnak)) {
        przywrocTerminal();
        return KLAWISZ_ESC;
    }

    if ((drugiZnak == '[' || drugiZnak == 'O') &&
        czytajZLimitemCzasu(trzeciZnak)) {
        przywrocTerminal();
        if (trzeciZnak == 'A') return KLAWISZ_GORA;
        if (trzeciZnak == 'B') return KLAWISZ_DOL;
        if (trzeciZnak == 'D') return KLAWISZ_LEWO;
        if (trzeciZnak == 'C') return KLAWISZ_PRAWO;
        return 0;
    }

    przywrocTerminal();
    return KLAWISZ_ESC;
#endif
}

void dodajKomunikat(string& komunikat, const string& nowyTekst) {
    if (nowyTekst.empty()) return;
    if (!komunikat.empty()) komunikat += " ";
    komunikat += nowyTekst;
}

bool miejsceZajete(int miejsce, const vector<Budynek>& miasto,
                   const vector<Budowa>& budowy) {
    for (const Budynek& budynek : miasto)
        if (budynek.miejsce == miejsce) return true;

    for (const Budowa& budowa : budowy)
        if (budowa.miejsce == miejsce) return true;

    return false;
}

char znakNaMiejscu(int miejsce, const vector<Budynek>& miasto,
                   const vector<Budowa>& budowy) {
    for (const Budynek& budynek : miasto)
        if (budynek.miejsce == miejsce) return budynek.znak;

    for (const Budowa& budowa : budowy)
        if (budowa.miejsce == miejsce) return '#';

    return '.';
}

void pokazMape(const vector<Budynek>& miasto, const vector<Budowa>& budowy,
               int zaznaczoneMiejsce = -1) {
    cout << BOLD << CYAN << "\n  CITY MAP\n" << RESET;

    for (int y = 0; y < 4; y++) {
        cout << "  ";
        for (int x = 0; x < 4; x++) {
            int miejsce = y * 4 + x;
            char znak = znakNaMiejscu(miejsce, miasto, budowy);

            if (miejsce == zaznaczoneMiejsce) {
                cout << "<" << ZOLTY << (znak == '.' ? ' ' : znak)
                     << RESET << "> ";
            } else if (znak == '#') {
                cout << "[" << ZOLTY << "#" << RESET << "] ";
            } else if (znak == '.') {
                cout << "[ ] ";
            } else {
                cout << "[" << ZIEL << znak << RESET << "] ";
            }
        }
        cout << "\n";
    }

    cout << "  H-House  B-Block  A-Apartment  P-Park\n";
    cout << "  R-Restaurant  F-Farm  I-Industrial plant  #-construction\n";
}

void pokazBudynki(const vector<Budynek>& miasto) {
    cout << BOLD << "\n  YOUR BUILDINGS\n" << RESET;
    if (miasto.empty()) {
        cout << "  No buildings.\n";
        return;
    }

    for (size_t i = 0; i < miasto.size(); i++) {
        cout << "  " << i + 1 << ". " << miasto[i].nazwa
             << "  HP: " << miasto[i].hp
             << "  Dochod: " << miasto[i].dochod << "\n";
    }
}

void pokazBudowy(const vector<Budowa>& budowy) {
    cout << BOLD << "\n  UNDER CONSTRUCTION\n" << RESET;
    if (budowy.empty()) {
        cout << "  No construction projects.\n";
        return;
    }

    for (const Budowa& budowa : budowy) {
        cout << "  " << budowa.budynek.nazwa
             << " - days remaining: " << budowa.dni << "\n";
    }
}

void pokazEkran(const vector<Budynek>& miasto, const vector<Budowa>& budowy,
                int pieniadze, int ludzie, int atrakcja, int punkty,
                int dzien, int niwelatory, const string& komunikat) {
    wyczyscEkran();

    cout << BOLD << CYAN;
    cout << "====================================================\n";
    cout << "                 CITY OF CATASTROPHES 7.0\n";
    cout << "====================================================\n" << RESET;

    cout << "  Day: " << ZOLTY << dzien << RESET
         << "     Money: " << ZIEL << pieniadze << RESET
         << "     Residents: " << ludzie << "\n";
    cout << "  Attractiveness: " << atrakcja
         << "     Disaster shields: " << niwelatory
         << "     Points: " << punkty << "\n";

    pokazMape(miasto, budowy);
    pokazBudynki(miasto);
    pokazBudowy(budowy);

    cout << BOLD << "\n  MENU\n" << RESET;
    cout << "  1. Buy a building\n";
    cout << "  2. Buy a disaster shield\n";
    cout << "  3. Repair a building\n";
    cout << "  4. Next day\n";
    cout << "  5. Disaster\n";
    cout << "  6. Exit\n";

    if (!komunikat.empty())
        cout << "\n  " << ZOLTY << komunikat << RESET << "\n";

    cout << "\n  Empty tile: blank, construction in progress: #.\n";
}

int wybierzMiejsce(const vector<Budynek>& miasto,
                   const vector<Budowa>& budowy) {
    int zaznaczone = 0;
    string blad;

    while (true) {
        wyczyscEkran();
        cout << BOLD << CYAN << "          CHOOSE A CONSTRUCTION SITE\n" << RESET;
        cout << "  Arrow keys: move | Enter: confirm | Esc: cancel\n";
        pokazMape(miasto, budowy, zaznaczone);

        if (!blad.empty())
            cout << "\n  " << CZERW << blad << RESET << "\n";

        int klawisz = wczytajKlawisz();
        blad.clear();

        if (klawisz == KLAWISZ_GORA && zaznaczone >= 4)
            zaznaczone -= 4;
        else if (klawisz == KLAWISZ_DOL && zaznaczone < 12)
            zaznaczone += 4;
        else if (klawisz == KLAWISZ_LEWO && zaznaczone % 4 > 0)
            zaznaczone--;
        else if (klawisz == KLAWISZ_PRAWO && zaznaczone % 4 < 3)
            zaznaczone++;
        else if (klawisz == KLAWISZ_ENTER) {
            if (!miejsceZajete(zaznaczone, miasto, budowy))
                return zaznaczone;
            blad = "This tile is already occupied. Choose another.";
        } else if (klawisz == KLAWISZ_ESC) {
            return -1;
        }
    }
}

void zakonczBudowy(vector<Budynek>& miasto, vector<Budowa>& budowy,
                   int& ludzie, int& atrakcja, string& komunikat) {
    for (size_t i = 0; i < budowy.size();) {
        budowy[i].dni--;

        if (budowy[i].dni <= 0) {
            Budynek gotowy = budowy[i].budynek;
            gotowy.miejsce = budowy[i].miejsce;
            miasto.push_back(gotowy);
            ludzie += gotowy.mieszkancy;
            atrakcja += gotowy.atrakcja;

            dodajKomunikat(komunikat, "Construction completed: " + gotowy.nazwa + ".");
            budowy.erase(budowy.begin() + static_cast<vector<Budowa>::difference_type>(i));
        } else {
            i++;
        }
    }
}

void przejdzDni(int liczbaDni, int& dzien, int& pieniadze,
                int& ludzie, int& atrakcja, vector<Budynek>& miasto,
                vector<Budowa>& budowy, string& komunikat) {
    for (int i = 0; i < liczbaDni; i++) {
        dzien++;
        for (const Budynek& budynek : miasto)
            pieniadze += oblicz_dochod(budynek.dochod);

        zakonczBudowy(miasto, budowy, ludzie, atrakcja, komunikat);
    }
}

int main() {
    wlaczKoloryWindows();
    engine_init(static_cast<unsigned int>(time(nullptr)));

    vector<Budynek> miasto;
    vector<Budowa> budowy;

    const vector<Budynek> sklep = {
        {"House",         'D', 100,  50,  5,  0,  0, 1, false, -1},
        {"Apartment block",        'B', 100, 120, 20,  1,  0, 3, false, -1},
        {"Luxury apartment",  'A', 100, 200, 40,  2, 10, 5, false, -1},
        {"Park",        'P', 100,  80,  0,  1, 15, 2, false, -1},
        {"Restaurant", 'R', 100, 150,  5,  5, 20, 3, false, -1},
        {"Farm",        'O', 100,  60,  0,  1,  0, 2, false, -1},
        {"Factory",     'F', 100, 300, 10, 10, -5, 7, true,  -1}
    };

    int pieniadze = 500;
    int ludzie = 0;
    int atrakcja = 0;
    int punkty = 0;
    int dzien = 1;
    int niwelatory = 0;
    string komunikat = "Welcome! Expand your city and watch out for disasters.";

    while (true) {
        pokazEkran(miasto, budowy, pieniadze, ludzie, atrakcja,
                   punkty, dzien, niwelatory, komunikat);
        komunikat.clear();

        int wybor;
        if (!wczytajLiczbe("\n  Wybor: ", wybor)) break;

        if (wybor == 1) {
            cout << "\n" << BOLD << "  BUILDING SHOP\n" << RESET;
            for (size_t i = 0; i < sklep.size(); i++) {
                cout << "  " << i + 1 << ". " << sklep[i].nazwa
                     << " | price: " << sklep[i].cena
                     << " | construction: " << sklep[i].czasBudowy << " days\n";
            }

            int kup;
            if (!wczytajLiczbe("  Building number: ", kup)) break;

            if (kup < 1 || kup > static_cast<int>(sklep.size())) {
                komunikat = "That building does not exist.";
            } else if (miasto.size() + budowy.size() >= ROZMIAR_MAPY) {
                komunikat = "The city is full.";
            } else if (pieniadze < sklep[kup - 1].cena) {
                komunikat = "You do not have enough money.";
            } else {
                int miejsce = wybierzMiejsce(miasto, budowy);
                if (miejsce == -1) {
                    komunikat = "Site selection cancelled.";
                } else {
                    pieniadze -= sklep[kup - 1].cena;
                    budowy.push_back({sklep[kup - 1],
                                      sklep[kup - 1].czasBudowy, miejsce});
                    komunikat = "Construction started. The tile is marked with #.";
                }
            }
        } else if (wybor == 2) {
            if (pieniadze >= 200) {
                pieniadze -= 200;
                niwelatory++;
                komunikat = "Disaster shield purchased.";
            } else {
                komunikat = "You do not have enough money for a disaster shield.";
            }
        } else if (wybor == 3) {
            if (miasto.empty()) {
                komunikat = "You do not have any buildings yet.";
            } else {
                pokazBudynki(miasto);
                int numer;
                if (!wczytajLiczbe("  Building number to repair: ", numer)) break;

                if (numer < 1 || numer > static_cast<int>(miasto.size())) {
                    komunikat = "There is no building with that number.";
                } else if (miasto[numer - 1].hp >= 100) {
                    komunikat = "This building is undamaged.";
                } else if (pieniadze < 50) {
                    komunikat = "You do not have enough money for repairs.";
                } else {
                    miasto[numer - 1].hp =
                        oblicz_nowe_hp(miasto[numer - 1].hp, 25);
                    pieniadze -= 50;
                    komunikat = "Building repaired.";
                }
            }
        } else if (wybor == 4) {
            przejdzDni(1, dzien, pieniadze, ludzie, atrakcja,
                       miasto, budowy, komunikat);
            dodajKomunikat(komunikat,
                           "Day " + to_string(dzien) + ".");
        } else if (wybor == 5) {
            const string katastrofy[] = {
                "Flood", "Earthquake", "Meteor",
                "Hurricane", "Fire", "Factory accident"
            };

            bool jestFactory = false;
            for (const Budynek& budynek : miasto) {
                if (budynek.fabryka) {
                    jestFactory = true;
                    break;
                }
            }

            int los = (jestFactory && losuj_liczbe(0, 1) == 0)
                         ? 5 : losuj_liczbe(0, 4);

            int obrazenia = (los == 5) ? 70 : losuj_liczbe(20, 59);

            if (niwelatory > 0) {
                obrazenia /= 2;
                niwelatory--;
                dodajKomunikat(komunikat,
                               "The disaster shield reduced the damage.");
            }

            int utrataDni;
            if (los == 0) utrataDni = 1;
            else if (los == 1) utrataDni = 3;
            else if (los == 2) utrataDni = 5;
            else if (los == 3) utrataDni = 2;
            else utrataDni = 4;

            dodajKomunikat(komunikat,
                katastrofy[los] + "! Damage: -" +
                to_string(obrazenia) + " HP.");

            for (size_t i = 0; i < miasto.size();) {
                WynikKatastrofy wynik =
                    wykonaj_katastrofe(miasto[i].hp, obrazenia);
                miasto[i].hp = wynik.hp;

                if (wynik.zniszczony) {
                    dodajKomunikat(komunikat,
                                   "Destroyed: " + miasto[i].nazwa + ".");
                    ludzie -= miasto[i].mieszkancy;
                    atrakcja -= miasto[i].atrakcja;
                    miasto.erase(miasto.begin() +
                        static_cast<vector<Budynek>::difference_type>(i));
                } else {
                    i++;
                }
            }

            if (ludzie < 0) ludzie = 0;
            if (atrakcja < 0) atrakcja = 0;

            przejdzDni(utrataDni, dzien, pieniadze, ludzie,
                       atrakcja, miasto, budowy, komunikat);

            dodajKomunikat(komunikat,
                "The disaster cost " + to_string(utrataDni) + " days.");
            punkty += 30;
        } else if (wybor == 6) {
            wyczyscEkran();
            cout << BOLD << CYAN << "GAME OVER\n" << RESET;
            cout << "Dni: " << dzien << "\n";
            cout << "Points: " << punkty << "\n";
            cout << "Residents: " << ludzie << "\n";
            cout << "Money: " << pieniadze << "\n";
            cout << "Attractiveness: " << atrakcja << "\n";
            break;
        } else {
            komunikat = "Invalid option.";
        }

        if (ludzie <= 0 && dzien > 5) {
            pokazEkran(miasto, budowy, pieniadze, ludzie, atrakcja,
                       punkty, dzien, niwelatory,
                       "The city has collapsed - no residents remain.");
            cout << "\n  Game over.\n";
            break;
        }
    }

    return 0;
}
