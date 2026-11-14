# City of Catastrophes 10.0 — Terminal Edition

V10 rozwija projekt V9 i dodaje opcjonalny moduł NASM x86-64. ASM przyspiesza obliczanie dochodu budynku zależnego od HP. Pozostała logika gry nadal pozostaje w C, C++ i Rust.

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
Funkcja ASM `coc_asm_income_hp(income, hp)` realizuje tę samą regułę co silnik: dochód <= 0 lub HP <= 0 daje 0; HP powyżej 100 jest ograniczane do 100; wynik to `(income * HP + 50) / 100`. Obliczenie pośrednie jest 64-bitowe.

## Ważne
Domyślna kompilacja C/C++ i Rust pozostaje przenośna i nie wymaga NASM. ASM jest włączany jawnie. Wersje C, C++ i Rust zachowują dotychczasowe zasady rozgrywki.
