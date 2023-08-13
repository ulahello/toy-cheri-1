jal zero, _start

back:
	loadi t0, 53
	loadi a2, SYS_EXIT
	syscall

_start:
	jal zero, back
