loadi t1, 23
loadi t2, 47
add t0, t1, t2
; t0 contains 70
addi t0, t0, 1
; t0 contains 71

loadi a2, SYS_EXIT
syscall
