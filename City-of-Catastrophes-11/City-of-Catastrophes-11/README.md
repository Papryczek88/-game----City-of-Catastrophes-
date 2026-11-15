# City of Catastrophes 11.0 — Terminal Edition

V10 rozwija projekt V9 i dodaje opcjonalny moduł NASM x86-64. ASM przyspiesza obliczanie dochodu budynku zależnego od HP. Pozostała logika gry nadal pozostaje w C, C++ i Rust.

## Nowości w 11.0 (wersja Rust)

- **Języki**: angielski (domyślny), polski, niemiecki, portugalski (Portugalia). Nazwy języków są pokazane w swoim języku. Zmiana: menu → *Settings*. Wybór zapisuje się w `settings.txt`; przy pierwszym uruchomieniu gra pyta o język.
- **Ustawienia**: język oraz włączanie/wyłączanie podpowiedzi.
- **Lepsze menu**: opis zaznaczonej pozycji, niedostępne opcje są wyszarzone, w sklepie widać zużycie/produkcję prądu.
- **Większa mapa**: 6 × 6 (36 pól), start na bloku 2 × 2 na środku.
- **Nowe budynki**: Panel słoneczny (50, +10 prądu), Elektrownia węglowa (200, +50 prądu, −10 atrakcyjności), Turbina wiatrowa (10, losowo 0–10 prądu dziennie).
- **Prąd**: wszystkie budynki zużywają prąd (Dom 2, Blok 6, Luksusowy apartament 12, Park 1, Restauracja 8, Farma 3, Fabryka 25). Przy niedoborze dochód spada proporcjonalnie. Wartości są w tabeli `TYPY` na górze `Rust/src/main.rs`.
- Cena działki rośnie teraz o 25 (a nie 50) przy większej mapie.
- Tłumaczenia są w `Rust/src/lang.rs`.
- Wersje `C/` i `Cpp/` pozostają na starszych zasadach (v8, bez działek, kredytów i prądu).

## Zasady gry (z wersji 10)

- Mapa (w wersji Rust 11.0) ma rozmiar 6 × 6; w starszych wersjach 4 × 4.
- Na start są dostępne 4 działki; reszta musi zostać kupiona.
- Działki można nabywać w menu za rosnącą cenę.
- Można wziąć kredyt w wysokości 500, który wymaga spłaty z 10% odsetkiem.
- Budynki, naprawy, katastrofy i budżet działają zgodnie z głównym mechanizmem gry.

## Struktura
- `C/` — wersja C
- `Cpp/` — wersja C++ z istniejącym silnikiem C
- `Rust/` — wersja Rust; ASM można włączyć funkcją Cargo `asm`
- `ASM/` — implementacje NASM dla Linux/WSL (System V ABI) i Windows x64 (Win64 ABI)

## Wymagania
- GCC/G++ lub kompatybilny kompilator
- NASM 2.15+ dla kompilacji z ASM
- Rust/Cargo dla wersji Rust

## C i C++ — Linux / Debian WSL
Z katalogu `C`:
```sh
chmod +x build-asm-linux.sh
./build-asm-linux.sh
./city-of-catastrophes
```
Z katalogu `Cpp` analogicznie:
```sh
chmod +x build-asm-linux.sh
./build-asm-linux.sh
./city-of-catastrophes
```

## C i C++ — Windows (MinGW-w64)
W folderze `C` lub `Cpp` uruchom `build-asm-windows.bat`. Wymagane `nasm`, `gcc` i `g++` w PATH.

## Rust
W katalogu `Rust`:
```sh
cargo run --release
```
Wersja przenośna bez ASM. Aby włączyć moduł NASM:
```sh
cargo run --release --features asm
```
Dla `asm` NASM musi być dostępny w PATH. `build.rs` dobiera format obiektu do targetu Rust. Na Windows MSVC wymagane jest również `lib.exe` z Visual Studio Build Tools.

## Zgodność
Funkcja ASM `coc_asm_income_hp(income, hp)` realizuje tę samą regułę co silnik: dochód <= 0 lub HP <= 0 daje 0; HP powyżej 100 jest ograniczane do 100; wynik to `(income * HP + 50) / 100`. Oprócz tego game logic pozostaje zgodna z mechaniką wydawania pieniędzy, napraw, kredytu i kupna działek.

## Ważne
Domyślna kompilacja C/C++ i Rust pozostaje przenośna i nie wymaga NASM. ASM jest włączany jawnie. Wersje C, C++ i Rust zachowują dotychczasowe zasady rozgrywki, a teraz także kupno działek i kredyty.
