mod exception;

use anyhow::Context;
use argh::FromArgs;
use tracing::{span, Level};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use std::fs;
use std::io::{stderr, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fruticose_asm::lex::LexErrTyp;
use fruticose_asm::parse::{ParseErr, ParseErrTyp, Parser};
use fruticose_vm::exception::Exception;
use fruticose_vm::int::UAddr;
use fruticose_vm::mem::Memory;
use fruticose_vm::op::Op;

use crate::exception::VmException;

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
                let op = match try_op {
                    Ok(op) => op,
                    Err(err) => {
                        let mut err_out = stderr();
                        pretty_print_parse_err(&mut err_out, &args.init, err)?;
                        writeln!(err_out)?;
                        anyhow::bail!(VmException::AssembleInit);
                    }
                };
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

fn pretty_print_parse_err<W: Write>(
    mut f: W,
    src_path: &Path,
    err: ParseErr<'_>,
) -> anyhow::Result<()> {
    let span = err.span;

    write!(f, "assembler error: ")?;

    match err.typ {
        ParseErrTyp::Lex(err) => match err {
            LexErrTyp::UnknownIdent => write!(f, "unknown identifier")?,
        },
        ParseErrTyp::ExpectedTyp { expected, found } => {
            write!(f, "expected {expected} but found {found}")?;
        }
        ParseErrTyp::ExpectedClass { expected, found } => {
            write!(f, "expected {expected} but found ")?;
            if let Some(class) = found.classify() {
                write!(f, "{class}")?;
            } else {
                write!(f, "{found}")?;
            }
        }
        ParseErrTyp::InvalidOperand => write!(f, "invalid operand")?,
    }
    writeln!(f)?;

    let pre_span = &span.get_line()[..span.col_idx];
    let in_span = span.get();
    let post_span = &span.get_line()[span.col_idx..][span.len..];

    let line = span.line + 1;
    let col = UnicodeSegmentation::grapheme_indices(span.get_line(), true)
        .position(|(idx, _)| idx == span.col_idx)
        .unwrap()
        + 1;

    let line_fmt_width = line.ilog10() as usize + 1;
    let side_pad = 1;
    let line_pad = side_pad + line_fmt_width + side_pad;
    // TODO: allocating strings here is silly
    let line_padding = " ".repeat(line_pad);
    let side_padding = " ".repeat(side_pad);

    writeln!(
        f,
        "{side_padding}@ {src_path}:{line}:{col}",
        src_path = src_path.display(),
    )?;

    writeln!(f, "{line_padding}|")?;
    writeln!(
        f,
        "{side_padding}{line}{side_padding}|{side_padding}{pre_span}{in_span}{post_span}",
    )?;
    writeln!(
        f,
        "{line_padding}|{side_padding}{skip_pre}{fake_underline}",
        skip_pre = " ".repeat(UnicodeWidthStr::width(pre_span)),
        fake_underline = "^".repeat(UnicodeWidthStr::width(in_span)),
    )?;

    f.flush()?;
    Ok(())
}
