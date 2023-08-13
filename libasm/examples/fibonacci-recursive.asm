_start:
	; loadi a2, 10 ; uncomment this to run outside of testing framework
	jal ra, fib
	jal zero, exit

; fn fib(n: UGran) UGran
fib:
	;; push stack frame
	; sp.addr -= UGRAN_SIZE
	loadi t0, UGRAN_SIZE
	cgetaddr t1, sp
	sub t1, t1, t0
	csetaddr sp, t1
	; write return address to stack frame
	storec sp, ra
	; sp.addr -= UGRAN_SIZE
	loadi t0, UGRAN_SIZE
	cgetaddr t1, sp
	sub t1, t1, t0
	csetaddr sp, t1
	; write n to stack frame
	store64 sp, a2 ; @port
	; sp.addr -= UGRAN_SIZE
	loadi t0, UGRAN_SIZE
	cgetaddr t1, sp
	sub t1, t1, t0
	csetaddr sp, t1
	; write s0 to stack frame
	storec sp, s0
	; sp.addr -= UGRAN_SIZE
	loadi t0, UGRAN_SIZE
	cgetaddr t1, sp
	sub t1, t1, t0
	csetaddr sp, t1
	; write s1 to stack frame
	storec sp, s1

	; fib(0) = 0
	loadi t0, 0
	beq a2, t0, fib_ret_0

	; fib(0) = 1
	loadi t0, 1
	beq a2, t0, fib_ret_1

	;; fib(n) = fib(n - 1) + fib(n - 2)
	loadi t0, 1
	sub a2, a2, t0 ; n -= 1
	jal ra, fib ; fib(n - 1)
	cpy s0, a0

	loadi t0, 1
	sub a2, a2, t0 ; n -= 1
	jal ra, fib ; fib(n - 2)
	cpy s1, a0

	add a0, s0, s1 ; fib(n - 1) + fib(n - 2)
	jal zero, fib_ret

fib_ret_0:
	loadi a0, 0
	jal zero, fib_ret

fib_ret_1:
	loadi a0, 1
	jal zero, fib_ret

fib_ret:
	;; pop stack frame
	; read s1
	loadc s1, sp
	; sp.addr += UGRAN_SIZE
	cgetaddr t1, sp
	addi t1, t1, UGRAN_SIZE
	csetaddr sp, t1
	; read s0
	loadc s0, sp
	; sp.addr += UGRAN_SIZE
	cgetaddr t1, sp
	addi t1, t1, UGRAN_SIZE
	csetaddr sp, t1
	; read n
	loadu64 a2, sp ; @port
	; sp.addr += UGRAN_SIZE
	cgetaddr t1, sp
	addi t1, t1, UGRAN_SIZE
	csetaddr sp, t1
	; read return address
	loadc ra, sp
	; sp.addr += UGRAN_SIZE
	cgetaddr t1, sp
	addi t1, t1, UGRAN_SIZE
	csetaddr sp, t1
	; jump to return address
	cpy pc, ra

exit:
	loadi a2, SYS_EXIT
	syscall
