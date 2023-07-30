; request allocation from parent allocator
loadi a2, SYS_ALLOC_ALLOC
loadc a3, z0
loadi a4, 18446744073709551680 ; 64, 0, 0, 0, 0, 0, 0, 0 | 1, 0, 0, 0, 0, 0, 0
syscall

; statting allocator
loadi a2, SYS_ALLOC_STAT
loadc a3, z0 ; not strictly necessary because z0 is already written to a3
syscall

exit:
	loadi a2, SYS_EXIT
	syscall
