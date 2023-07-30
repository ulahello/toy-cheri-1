loadi t1, 47
loadi t2, 48
bne t1, t2, cmp_true
jal zero, cmp_false

cmp_true:
	loadi t0, 1
	jal zero, exit

cmp_false:
	loadi t0, 0
	jal zero, exit

exit:
	loadi a2, SYS_EXIT
	syscall
