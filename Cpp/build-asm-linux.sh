#!/usr/bin/env sh
set -eu
nasm -f elf64 ../ASM/income_sysv.asm -o income_asm.o
gcc -DCOC_USE_ASM -c engine.c -o engine.o
g++ -std=c++17 -O2 -DCOC_USE_ASM main.cpp engine.o income_asm.o -o city-of-catastrophes
