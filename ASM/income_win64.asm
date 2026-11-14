; City of Catastrophes - NASM x86-64 Windows ABI
; int coc_asm_income_hp(int income, int hp)
; Args: ECX=income, EDX=hp. Return: EAX.
bits 64
default rel
section .text
global coc_asm_income_hp
coc_asm_income_hp:
    test ecx, ecx
    jle .zero
    test edx, edx
    jle .zero
    cmp edx, 100
    jle .hp_ok
    mov edx, 100
.hp_ok:
    movsxd rax, ecx
    movsxd r8, edx
    imul rax, r8
    add rax, 50
    cqo
    mov r8, 100
    idiv r8
    cmp rax, 2147483647
    jle .done
    mov eax, 2147483647
.done:
    ret
.zero:
    xor eax, eax
    ret
