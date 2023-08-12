use fruticose_vm::int::UGran;

mod rt {
    use fruticose_vm::abi::Ty;
    use fruticose_vm::alloc;
    use fruticose_vm::capability::TaggedCapability;
    use fruticose_vm::exception::Exception;
    use fruticose_vm::mem::Memory;
    use fruticose_vm::registers::Register;

    #[test]
    fn invalidate_cap() -> Result<(), Exception> {
        let mut mem = Memory::new(32, 0, [].iter()).unwrap();
        let root_alloc = mem.regs.read(&mem.tags, Register::Z0 as _)?;
        let ation = alloc::alloc(root_alloc, TaggedCapability::LAYOUT, &mut mem)?;
        mem.write(ation, ation)?;
        let expanded = ation.set_bounds(ation.start(), ation.endb().add(1));
        mem.write(ation, expanded)?;
        let result: TaggedCapability = mem.read(ation)?;
        assert!(!result.is_valid());
        Ok(())
    }
}

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
    const FIB_REC: &str = include_str!("../../asm/examples/fibonacci-recursive.asm");
    const FIB_ITER: &str = include_str!("../../asm/examples/fibonacci-iter.asm");

    fn assemble(src: &str) -> Result<Vec<Op>, ParseErr> {
        let ops = Parser2::new(src)
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ops)
    }

    fn exec(mem: &mut Memory) -> Result<(), Exception> {
        loop {
            match mem.execute_next() {
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
        let mut mem = Memory::new(32, 0, ops.iter()).unwrap();
        drop(ops);
        exec(&mut mem).unwrap();
        expect_in_reg(&mut mem, Register::T1, TaggedCapability::from_ugran(23));
        expect_in_reg(&mut mem, Register::T2, TaggedCapability::from_ugran(47));
        expect_in_reg(&mut mem, Register::T0, TaggedCapability::from_ugran(71));
    }

    #[test]
    fn cmp() {
        let ops = assemble(CMP).unwrap();
        let mut mem = Memory::new(64, 0, ops.iter()).unwrap();
        drop(ops);
        exec(&mut mem).unwrap();
        expect_in_reg(&mut mem, Register::T1, TaggedCapability::from_ugran(47));
        expect_in_reg(&mut mem, Register::T2, TaggedCapability::from_ugran(48));
        expect_in_reg(&mut mem, Register::T0, TaggedCapability::from_ugran(1));
    }

    #[test]
    fn jmp_back() {
        let ops = assemble(JMP_BACK).unwrap();
        let mut mem = Memory::new(32, 0, ops.iter()).unwrap();
        drop(ops);
        exec(&mut mem).unwrap();
        expect_in_reg(&mut mem, Register::T0, TaggedCapability::from_ugran(53));
    }

    #[test]
    fn fibonacci_iter() -> Result<(), Exception> {
        let ops = assemble(FIB_ITER).unwrap();
        let mut mem = Memory::new(1024, 1024, ops.iter()).unwrap();
        let pc = mem.regs.read(&mem.tags, Register::Pc as _)?;
        for n in 0..94 {
            println!("fib(n = {n})");
            mem.regs.write(&mut mem.tags, Register::Pc as _, pc)?; // reset execution
            mem.regs.write_data(&mut mem.tags, Register::A2 as _, n)?;
            exec(&mut mem).unwrap();
            expect_in_reg(
                &mut mem,
                Register::A0,
                TaggedCapability::from_ugran(super::fib(n)),
            );
        }

        Ok(())
    }

    #[test]
    fn fibonacci_recursive() -> Result<(), Exception> {
        let ops = assemble(FIB_REC).unwrap();
        let mut mem = Memory::new(1024, 1024, ops.iter()).unwrap();
        let pc = mem.regs.read(&mem.tags, Register::Pc as _)?;
        for n in 0..10 {
            println!("fib(n = {n})");
            mem.regs.write(&mut mem.tags, Register::Pc as _, pc)?; // reset execution
            mem.regs.write_data(&mut mem.tags, Register::A2 as _, n)?;
            exec(&mut mem).unwrap();
            expect_in_reg(
                &mut mem,
                Register::A0,
                TaggedCapability::from_ugran(super::fib(n)),
            );
        }

        Ok(())
    }
}

fn fib(n: UGran) -> UGran {
    let mut f2 = 0;
    if n == 0 {
        return f2;
    };
    let mut f1 = 1;
    let mut f;
    for _ in 2..=n {
        f = f2
            .checked_add(f1)
            .expect("nth fibonacci mustn't overflow UGran");
        f2 = f1;
        f1 = f;
    }
    return f1;
}
