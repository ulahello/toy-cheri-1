use anyhow::Context;
use argh::FromArgs;
use tracing::{span, Level};

use std::io::stderr;
use std::process::ExitCode;

use fruticose_vm::capability::{Capability, TaggedCapability};
use fruticose_vm::exception::Exception;
use fruticose_vm::int::UAddr;
use fruticose_vm::mem::Memory;
use fruticose_vm::op::Op;
use fruticose_vm::registers::Register;
use fruticose_vm::syscall::SyscallKind;

/// Fruticose virtual machine
#[derive(FromArgs)]
struct Args {
    /// granules of physical memory to use
    #[argh(option, short = 'g')]
    granules: UAddr,
}

fn main() -> ExitCode {
    tracing_subscriber::fmt::fmt()
        .with_writer(stderr)
        .with_max_level(Level::TRACE)
        .with_timer(tracing_subscriber::fmt::time::uptime())
        .pretty()
        .init();

    let args: Args = argh::from_env();

    if let Err(err) = try_main(&args) {
        eprintln!("fatal error: {err}");
        let chain = err.chain().skip(1);
        if chain.len() != 0 {
            eprintln!("context:");
            for err in chain {
                eprintln!("{padding}{err}", padding = " ".repeat(2));
            }
        }
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn try_main(args: &Args) -> anyhow::Result<()> {
    let span = span!(Level::TRACE, "main", granules = args.granules);
    let _guard = span.enter();

    let init: &[Op] = &[
        Op::nop(),
        Op::loadi(
            Register::A0 as _,
            TaggedCapability::new(Capability::from_ugran(SyscallKind::Exit as _), false),
        ),
        Op::syscall(),
    ];
    let mut mem = Memory::new(args.granules, init).context("failed to instantiate memory")?;
    tracing::info!("execution start");
    loop {
        match mem.execute_op() {
            Ok(()) => (),
            Err(Exception::ProcessExit) => break,
            other => other?,
        }
    }
    tracing::info!("execution halted");
    Ok(())
}
