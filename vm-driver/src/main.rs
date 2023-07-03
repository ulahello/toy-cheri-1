use anyhow::Context;
use argh::FromArgs;
use fruticose_asm::parse::Parser;
use tracing::{span, Level};

use std::fs;
use std::io::stderr;
use std::path::PathBuf;
use std::process::ExitCode;

use fruticose_vm::exception::Exception;
use fruticose_vm::int::UAddr;
use fruticose_vm::mem::Memory;
use fruticose_vm::op::Op;

/// Fruticose virtual machine
#[derive(FromArgs)]
struct Args {
    /// granules of physical memory to use
    #[argh(option, short = 'g')]
    granules: UAddr,

    /// path to init program assembly
    #[argh(option, short = 'i')]
    init: PathBuf,
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
    let span1 = span!(Level::TRACE, "main", granules = args.granules);
    let _guard1 = span1.enter();

    let mut mem = {
        let init: Vec<Op> = {
            let span2 = span!(
                Level::TRACE,
                "load_init",
                path = format_args!("{}", args.init.display())
            );
            let _guard2 = span2.enter();

            tracing::debug!("loading init program");

            tracing::trace!("reading init program");
            let init_src =
                fs::read_to_string(&args.init).context("failed to read init program source")?;

            tracing::trace!("assembling init program");
            let parser = Parser::new(&init_src);
            let mut ops = Vec::new();
            for try_op in parser {
                let op = try_op.expect("TODOO: handle asm parse error");
                ops.push(op);
            }
            ops
        };

        Memory::new(args.granules, init.iter()).context("failed to instantiate memory")?
    };

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
