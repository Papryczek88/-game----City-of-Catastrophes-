@echo off
setlocal
nasm -f win64 ..\ASM\income_win64.asm -o income_asm.obj || exit /b 1
gcc -std=c11 -O2 -DCOC_USE_ASM main.c engine.c income_asm.obj -o CityOfCatastrophes.exe
