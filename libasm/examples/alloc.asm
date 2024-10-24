_start:
	; TODOO: support structs as immediate values
	; construct layout
	loadi a2, 64 ; size = 64
	loadi a3, 0 ; align = 1
	jal ra, layout_new

	; request allocation from parent allocator
	loadi a2, SYS_ALLOC_ALLOC
	cpy a3, z0
	cpy a4, a0 ; a4 set from previous call
	syscall

	; statting allocator
	loadi a2, SYS_ALLOC_STAT
	cpy a3, z0 ; not strictly necessary because z0 is already written to a3
	syscall

	jal zero, exit

layout_new: ; fn(size: UAddr, align: u8) -> Layout
	slli t0, a3, UADDR_BITS
	or a0, a2, t0
	cpy pc, ra

exit:
	loadi a2, SYS_EXIT
	syscall
