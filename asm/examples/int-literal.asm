loadi t1, 23
loadi t2, 47
add t0, t1, t2
; 23+47 should be in t0

loadi a0, SYS_EXIT
syscall
