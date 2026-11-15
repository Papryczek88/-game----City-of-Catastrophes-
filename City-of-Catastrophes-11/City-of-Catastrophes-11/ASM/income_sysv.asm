; City of Catastrophes - NASM x86-64 System V ABI (Linux/WSL)
; int coc_asm_income_hp(int income, int hp)
; Args: EDI=income, ESI=hp. Return: EAX.
bits 64
default rel
section .text
global coc_asm_income_hp
coc_asm_income_hp:
    test edi, edi
    jle .zero
    test esi, esi
    jle .zero
    cmp esi, 100
    jle .hp_ok
    mov esi, 100
.hp_ok:
    movsxd rax, edi
    movsxd rcx, esi
    imul rax, rcx
    add rax, 50
    cqo
    mov ecx, 100
    idiv rcx
    ; clamp to INT_MAX to keep the C ABI return value defined
    cmp rax, 2147483647
    jle .done
    mov eax, 2147483647
.done:
    ret
.zero:
    xor eax, eax
    ret
