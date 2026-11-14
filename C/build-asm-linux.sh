#!/usr/bin/env sh
set -eu
nasm -f elf64 ../ASM/income_sysv.asm -o income_asm.o
gcc -std=c11 -O2 -DCOC_USE_ASM main.c engine.c income_asm.o -o city-of-catastrophes
