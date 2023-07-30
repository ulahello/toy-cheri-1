loadi a2, SYS_ALLOC_ALLOC
loadi a3, allocator ; TODO
loadi a4, layout ; TODO
syscall

exit:
	loadi a2, SYS_EXIT
	syscall
