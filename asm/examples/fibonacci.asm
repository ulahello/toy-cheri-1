_start:
	; TODO: why does it raise invalid operation when n is too high?
	; loadi a2, 20 ; uncomment this to run outside of testing framework
	jal ra, fib
	jal zero, exit

; fn fib(n: UGran) UGran
fib:
	;; push stack frame
	; sp.addr -= UADDR_SIZE
	loadi t0, UADDR_SIZE
	cgetaddr t1, sp
	sub t1, t1, t0
	csetaddr sp, t1
	; write return address to stack frame
	cgetaddr t2, ra
	store64 sp, t2 ; @port
	; sp.addr -= UGRAN_SIZE
	loadi t0, UGRAN_SIZE
	cgetaddr t1, sp
	sub t1, t1, t0
	csetaddr sp, t1
	; write n to stack frame
	store128 sp, a2 ; @port
	; sp.addr -= UGRAN_SIZE
	loadi t0, UGRAN_SIZE
	cgetaddr t1, sp
	sub t1, t1, t0
	csetaddr sp, t1
	; write s0 to stack frame
	store128 sp, s0 ; @port
	; sp.addr -= UGRAN_SIZE
	loadi t0, UGRAN_SIZE
	cgetaddr t1, sp
	sub t1, t1, t0
	csetaddr sp, t1
	; write s1 to stack frame
	store128 sp, s1 ; @port

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
	loadu128 s1, sp ; @port
	; sp.addr += UGRAN_SIZE
	cgetaddr t1, sp
	addi t1, t1, UGRAN_SIZE
	csetaddr sp, t1
	; read s0
	loadu128 s0, sp ; @port
	; sp.addr += UGRAN_SIZE
	cgetaddr t1, sp
	addi t1, t1, UGRAN_SIZE
	csetaddr sp, t1
	; read n
	loadu128 a2, sp ; @port
	; sp.addr += UGRAN_SIZE
	cgetaddr t1, sp
	addi t1, t1, UGRAN_SIZE
	csetaddr sp, t1
	; read return address
	loadu64 t0, sp
	csetaddr ra, t0 ; set return address
	; sp.addr += UADDR_SIZE
	cgetaddr t1, sp
	addi t1, t1, UADDR_SIZE
	csetaddr sp, t1
	; jump to return address
___zero: ; HACK: need to support label OR immediate
	jalr zero, t0, ___zero ; jump to the return address

exit:
	loadi a2, SYS_EXIT
	syscall
