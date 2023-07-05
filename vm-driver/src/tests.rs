mod exec {
    use fruticose_asm::parse::{ParseErr, Parser};
    use fruticose_vm::capability::TaggedCapability;
    use fruticose_vm::exception::Exception;
    use fruticose_vm::mem::Memory;
    use fruticose_vm::op::Op;
    use fruticose_vm::registers::Register;

    const ADD: &str = include_str!("../../asm/examples/add.asm");

    fn assemble(src: &str) -> Result<Vec<Op>, ParseErr> {
        let ops = Parser::new(src)
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
}
