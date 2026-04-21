.intel_syntax noprefix

.global my_exit
my_exit:
    mov rax, 60
    syscall

.section .seso, "ax"
    .8byte 69
