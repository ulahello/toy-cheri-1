jal zero, _start

back:
	loadi t0, 53
	loadi a0, SYS_EXIT
	syscall

_start:
	jal zero, back
