.intel_syntax noprefix

.global _start
_start:
    mov rdi, 42
    call my_exit

.section .seso, "awx"
    .8byte 42
