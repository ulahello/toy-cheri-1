_start:
	; loadi a2, 10 ; uncomment this to run outside of testing framework
	jal ra, fib
	jal zero, exit

; fn fib(n: UGran) UGran
fib:
	loadi t2, 0 ; f2 = 0
	beq a2, t2, fib_ret_0 ; if (n == 0) return f2
	loadi t1, 1 ; f1 = 1
fib_loop_start:
	loadi t0, 2 ; idx = 2
fib_loop:
	bltu a2, t0, fib_end ; if (n < idx) break

	add t3, t2, t1 ; fn = f2 + f1
	cpy t2, t1 ; f2 = f1
	cpy t1, t3 ; f1 = fn

	addi t0, t0, 1 ; idx += 1
	jal zero, fib_loop ; continue

fib_end:
	; return f1
	cpy a0, t1
	jal zero, fib_ret

fib_ret_0:
	; return f2
	cpy a0, t2
	jal zero, fib_ret

fib_ret:
	cgetaddr t0, ra
___zero: ; HACK: need to support label OR immediate
	jalr zero, t0, ___zero ; jump to the return address

exit:
	loadi a2, SYS_EXIT
	syscall
