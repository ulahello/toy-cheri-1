mod exec {
    use fruticose_asm::parse1::ParseErr;
    use fruticose_asm::parse2::Parser2;
    use fruticose_vm::capability::TaggedCapability;
    use fruticose_vm::exception::Exception;
    use fruticose_vm::mem::Memory;
    use fruticose_vm::op::Op;
    use fruticose_vm::registers::Register;

    const ADD: &str = include_str!("../../asm/examples/add.asm");
    const CMP: &str = include_str!("../../asm/examples/cmp.asm");
    const JMP_BACK: &str = include_str!("../../asm/examples/jmp-back.asm");

    fn assemble(src: &str) -> Result<Vec<Op>, ParseErr> {
        let ops = Parser2::new(src)
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ops)
    }

    fn exec(mem: &mut Memory) -> Result<(), Exception> {
        loop {
            match mem.execute_op() {
                Ok(()) => (),
                Err(err) => match err {
                    Exception::ProcessExit => break Ok(()),
                    _ => return Err(err),
                },
            }
        }
    }

    #[track_caller]
    fn expect_in_reg(mem: &mut Memory, reg: Register, tcap: TaggedCapability) {
        let val = mem.regs.read(&mut mem.tags, reg as _).unwrap();
        assert_eq!(val, tcap);
    }

    #[test]
    fn add() {
        let ops = assemble(ADD).unwrap();
        let mut mem = Memory::new(32, ops.iter()).unwrap();
        drop(ops);
        exec(&mut mem).unwrap();
        expect_in_reg(&mut mem, Register::T1, TaggedCapability::from_ugran(23));
        expect_in_reg(&mut mem, Register::T2, TaggedCapability::from_ugran(47));
        expect_in_reg(&mut mem, Register::T0, TaggedCapability::from_ugran(71));
    }

    #[test]
    fn cmp() {
        let ops = assemble(CMP).unwrap();
        let mut mem = Memory::new(64, ops.iter()).unwrap();
        drop(ops);
        exec(&mut mem).unwrap();
        expect_in_reg(&mut mem, Register::T1, TaggedCapability::from_ugran(47));
        expect_in_reg(&mut mem, Register::T2, TaggedCapability::from_ugran(48));
        expect_in_reg(&mut mem, Register::T0, TaggedCapability::from_ugran(1));
    }

    #[test]
    fn jmp_back() {
        let ops = assemble(JMP_BACK).unwrap();
        let mut mem = Memory::new(32, ops.iter()).unwrap();
        drop(ops);
        exec(&mut mem).unwrap();
        expect_in_reg(&mut mem, Register::T0, TaggedCapability::from_ugran(53));
    }
}
