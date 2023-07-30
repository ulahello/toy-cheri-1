; request allocation from parent allocator
loadi a2, SYS_ALLOC_ALLOC
loadc a3, z0
loadi a4, 18446744073709551680 ; 64, 0, 0, 0, 0, 0, 0, 0 | 1, 0, 0, 0, 0, 0, 0
syscall

; statting allocator
loadi  a2, SYS_ALLOC_STAT
syscall

exit:
	loadi a2, SYS_EXIT
	syscall
