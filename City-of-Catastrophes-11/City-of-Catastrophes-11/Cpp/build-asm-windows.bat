@echo off
setlocal
nasm -f win64 ..\ASM\income_win64.asm -o income_asm.obj || exit /b 1
gcc -DCOC_USE_ASM -c engine.c -o engine.o || exit /b 1
g++ -std=c++17 -O2 -DCOC_USE_ASM main.cpp engine.o income_asm.obj -o CityOfCatastrophes.exe
