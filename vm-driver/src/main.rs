mod debug;

#[cfg(test)]
mod tests;

use anyhow::Context;
use argh::FromArgs;
use nu_ansi_term::{Color, Style};
use tracing::{span, Level};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use core::num::IntErrorKind;
use std::io::{stderr, stdout, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{fs, io};

use fruticose_asm::lex::{LexErrTyp, TokenTyp};
use fruticose_asm::parse1::{ParseErr, ParseErrTyp, TokenClass};
use fruticose_asm::parse2::Parser2;
use fruticose_asm::Span;
use fruticose_vm::exception::Exception;
use fruticose_vm::int::UAddr;
use fruticose_vm::mem::Memory;
use fruticose_vm::op::Op;

use crate::debug::DebugMode;

/// Fruticose virtual machine
#[derive(FromArgs)]
struct Args {
    /// granules of physical memory to use
    #[argh(option, short = 'g', default = "4096")]
    granules: UAddr,

    /// stack size in bytes for init program
    #[argh(option, short = 's', default = "1024")]
    stack_size: UAddr,

    /// choose if/how to run the debugger
    #[argh(option, short = 'd', default = "DebugMode::Never")]
    debug: DebugMode,

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

    if let Err(err) = try_main(args) {
        _ = pretty_print_main_err(BufWriter::new(stderr()), err);
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn try_main(args: Args) -> anyhow::Result<()> {
    let span = span!(
        Level::TRACE,
        "main",
        granules = args.granules,
        stack_size = args.stack_size,
        debug_mode = format_args!("{:?}", args.debug),
    );
    let _guard = span.enter();

    let mut stdout = BufWriter::new(stdout());

    let mut mem = {
        let init: Vec<Op> = assemble_init(&args.init).context("failed to load init program")?;
        Memory::new(args.granules, args.stack_size, init.iter())
            .context("failed to instantiate memory")?
    };

    if args.debug == DebugMode::Always {
        tracing::info!("launching debugger before execution start");
        args.debug.launch(&mut mem, None, &mut stdout)?;
        tracing::info!("debugger yielded, terminating fruticose");
        return Ok(());
    }

    tracing::info!("execution start");
    loop {
        match mem.execute_op() {
            Ok(()) => (),
            Err(Exception::ProcessExit) => break,
            Err(raised) => {
                if args.debug == DebugMode::Error {
                    tracing::info!("launching debugger, exception raised");
                    args.debug.launch(&mut mem, Some(raised), &mut stdout)?;
                    tracing::info!("debugger yielded, resuming exception handling");
                }
                return Err(raised.into());
            }
        }
    }
    tracing::info!("execution halted");

    Ok(())
}

fn assemble_init(init: &Path) -> anyhow::Result<Vec<Op>> {
    let span = span!(
        Level::TRACE,
        "load_init",
        path = format_args!("{}", init.display())
    );
    let _guard = span.enter();

    tracing::debug!("loading init program");

    tracing::trace!("reading init program");
    let init_src = fs::read_to_string(init).context("failed to read init program source")?;

    tracing::trace!("assembling init program");
    let parser = Parser2::new(&init_src);
    let mut ops = Vec::new();
    let mut err_count: usize = 0;
    let mut err_out = BufWriter::new(stderr());
    for try_op in parser {
        match try_op {
            Ok(op) => ops.push(op),
            Err(err) => {
                err_count += 1;
                pretty_print_parse_err(&mut err_out, init, err)?;
                writeln!(err_out)?;
            }
        }
    }
    err_out.flush()?;
    if err_count > 0 {
        anyhow::bail!(
            "failed to assemble init program due to {err_count} previous error{}",
            if err_count == 1 { "" } else { "s" }
        );
    }
    Ok(ops)
}

fn pretty_print_main_err<W: Write>(mut f: W, err: anyhow::Error) -> anyhow::Result<()> {
    let err_title = Color::Red.bold();
    let err_body = Color::LightRed.bold();
    let context_title = Style::new().bold();
    let context_body = Style::new();

    writeln!(
        f,
        "{}fatal error: {}{err}",
        err_title.prefix(),
        err_title.infix(err_body)
    )?;

    let chain = err.chain().skip(1);
    if chain.len() != 0 {
        writeln!(
            f,
            "{}context:{}",
            err_body.infix(context_title),
            context_title.infix(context_body)
        )?;
        for err in chain {
            writeln!(
                f,
                " {}^{} {err}",
                context_body.infix(context_title),
                context_title.infix(context_body),
            )?;
        }
        write!(f, "{}", context_body.suffix())?;
    }

    f.flush()?;
    Ok(())
}

fn pretty_print_parse_err<W: Write>(
    mut f: W,
    src_path: &Path,
    err: ParseErr<'_>,
) -> anyhow::Result<()> {
    let err_title = Color::LightRed.bold();
    let err_underline = err_title;
    let err_body = Style::new().bold();
    let text = Style::new();
    let symbols = Color::LightBlue.bold();
    let err_span = text;

    let span = err.span;

    write!(
        f,
        "{}assembler error:{} ",
        err_title.prefix(),
        err_title.infix(err_body)
    )?;

    match err.typ {
        ParseErrTyp::Lex(err) => match err {
            LexErrTyp::UnknownIdent => write!(f, "unknown identifier")?,
            LexErrTyp::InvalidUnsignedInt(err) => match err.kind() {
                IntErrorKind::PosOverflow => {
                    write!(f, "unsigned integer literal overflows granule")?;
                }
                _ => write!(f, "unsigned integer literal is invalid ({err})")?,
            },
        },
        ParseErrTyp::ExpectedTyp { expected, found } => {
            // TODO: show operand count if missing comma
            write!(f, "expected {expected}, but found {found}")?;
        }
        ParseErrTyp::ExpectedClass { expected, found } => {
            write!(f, "expected {expected}, but found ")?;
            if let Some(class) = found.classify() {
                write!(f, "{class}")?;
            } else {
                write!(f, "{found}")?;
            }
        }
        ParseErrTyp::InvalidOperand { found } => {
            write!(f, "expected operand, but found '{found}'")?
        }
        ParseErrTyp::OperandTypeMismatch { expected, found } => {
            write!(
                f,
                "operand mismatch: expected {expected}, but found {found}"
            )?;
        }
        ParseErrTyp::InvalidStmtStart { found } => write!(
            f,
            "expected statement start '{}' or '{}', but found '{found}'",
            TokenClass::Op,
            TokenTyp::Identifier
        )?,
        ParseErrTyp::LabelRedef { first_def: _ } => write!(f, "labels cannot be redefined")?, // TODO: show where first defined
        ParseErrTyp::LabelUndef => write!(f, "undefined label")?,
        ParseErrTyp::LabelOffsetOverflow => {
            write!(f, "overflow occured while computing label offset")?;
        }
    }
    writeln!(f)?;

    let diag = Diagnostic::new(
        span,
        src_path,
        text,
        symbols,
        err_span,
        err_body,
        err_underline,
    );
    diag.write(&mut f)?;

    f.flush()?;
    Ok(())
}

struct Diagnostic<'s, 'p> {
    span: Span<'s>,
    src_path: &'p Path,
    text: Style,
    symbols: Style,
    err_span: Style,
    err_body: Style,
    err_underline: Style,
}

impl<'s, 'p> Diagnostic<'s, 'p> {
    pub fn new(
        span: Span<'s>,
        src_path: &'p Path,
        text: Style,
        symbols: Style,
        err_span: Style,
        err_body: Style,
        err_underline: Style,
    ) -> Self {
        Self {
            span,
            src_path,
            text,
            symbols,
            err_span,
            err_body,
            err_underline,
        }
    }

    pub fn write<W: Write>(&self, mut f: W) -> io::Result<()> {
        let span = self.span;
        let text = self.text;
        let symbols = self.symbols;
        let err_span = self.err_span;
        let err_body = self.err_body;
        let err_underline = self.err_underline;

        let pre_span = &span.get_line()[..span.col_idx];
        let mut in_span = span.get();
        let mut post_span = &span.get_line()[span.col_idx..][span.len..];
        if post_span.is_empty() {
            in_span = in_span.trim_end();
        } else {
            post_span = post_span.trim_end();
        }
        let pre_span_len = UnicodeWidthStr::width(pre_span);
        let in_span_len = UnicodeWidthStr::width(in_span).max(1);

        let line = span.line + 1;
        let col = {
            let graphs = || UnicodeSegmentation::grapheme_indices(span.get_line(), true);
            if let Some(col) = graphs().position(|(idx, _)| idx == span.col_idx) {
                col + 1
            } else {
                // eof isnt a real character! but its still loved
                graphs().map(|(idx, _)| idx).last().unwrap_or(0)
            }
        };

        let line_fmt_width = line.ilog10() as usize + 1;
        let side_pad = 1;
        let line_pad = side_pad + line_fmt_width + side_pad;
        // TODO: allocating strings here is silly
        let line_padding = " ".repeat(line_pad);
        let side_padding = " ".repeat(side_pad);

        writeln!(
            f,
            "{side_padding}{}@{} {src_path}:{line}:{col}",
            err_body.infix(symbols),
            symbols.infix(text),
            src_path = self.src_path.display(),
        )?;

        writeln!(
            f,
            "{line_padding}{}|{}",
            text.infix(symbols),
            symbols.infix(text)
        )?;
        writeln!(
        f,
        "{side_padding}{}{line}{}{side_padding}{}|{}{side_padding}{pre_span}{}{in_span}{}{post_span}",
        text.infix(symbols),
        symbols.infix(text),
        text.infix(symbols),
        symbols.infix(text),
        text.infix(err_span),
        err_span.infix(text),
    )?;
        writeln!(
            f,
            "{line_padding}{}|{}{side_padding}{skip_pre}{}{fake_underline}{}",
            text.infix(symbols),
            symbols.infix(text),
            text.infix(err_underline),
            err_underline.suffix(),
            skip_pre = " ".repeat(pre_span_len),
            fake_underline = "^".repeat(in_span_len),
        )?;

        Ok(())
    }
}
