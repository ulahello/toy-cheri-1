use anyhow::Context;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::reload;

use core::str::FromStr;
use core::sync::atomic::{AtomicBool, Ordering};
use std::io::{self, BufRead, Write};

use fruticose_vm::exception::Exception;
use fruticose_vm::mem::Memory;
use fruticose_vm::registers::Register;

#[derive(Debug, PartialEq, Eq)]
pub enum DebugMode {
    Never,
    Error,
    Always,
}

impl FromStr for DebugMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("never") {
            Ok(Self::Never)
        } else if s.eq_ignore_ascii_case("error") {
            Ok(Self::Error)
        } else if s.eq_ignore_ascii_case("always") {
            Ok(Self::Always)
        } else {
            Err("unrecognized debug mode")
        }
    }
}

impl DebugMode {
    pub fn launch<W: Write>(
        self,
        mem: &mut Memory,
        already_raised: Option<Exception>,
        log_handle: reload::Handle<LevelFilter, tracing_subscriber::Registry>,
        out: W,
    ) -> anyhow::Result<()> {
        debug_assert_ne!(self, DebugMode::Never);
        launch_inner(mem, already_raised, log_handle, out)
    }
}

fn launch_inner<W: Write>(
    mem: &mut Memory,
    already_raised: Option<Exception>,
    log_handle: reload::Handle<LevelFilter, tracing_subscriber::Registry>,
    mut out: W,
) -> anyhow::Result<()> {
    let stop_exec: &'static AtomicBool = Box::leak(Box::new(AtomicBool::new(false)));
    ctrlc::set_handler(|| {
        stop_exec.store(true, Ordering::Relaxed);
    })
    .context("failed to set Ctrl-C handler")?;

    splash(&mut out)?;
    loop {
        let input = readln(&mut out, "> ")?;
        let mut cmd = input.trim().split_ascii_whitespace();
        if let Some(next) = cmd.next() {
            match next {
                "quit" | "q" => break,

                "help" | "h" => {
                    writeln!(out, "quit. quit.")?;
                    writeln!(out, "help. list commands.")?;
                    writeln!(out, "log [on | off]. toggle logs.")?;
                    writeln!(out, "step [<count> | while]. execute the next Op(s).")?;
                    writeln!(out, "print <location>. print value at location.")?;
                    writeln!(out, "do <operation>. execute operation.")?;
                }

                "log" | "l" => {
                    let want: bool = if let Some(state) = cmd.next() {
                        match state {
                            "on" | "true" | "t" => true,
                            "off" | "false" | "f" => false,
                            _ => {
                                writeln!(out, "error: invalid logging state")?;
                                continue;
                            }
                        }
                    } else {
                        let cur: bool = log_handle
                            .with_current(|filter| *filter > LevelFilter::INFO)
                            .context("Subscriber is gone??")?;
                        !cur
                    };
                    let want_level = if want {
                        LevelFilter::TRACE
                    } else {
                        LevelFilter::INFO
                    };
                    log_handle
                        .modify(|filter| *filter = want_level)
                        .context("Subscriber is gone??")?;

                    writeln!(out, "{} logging", if want { "enabled" } else { "disabled" })?;
                }

                "step" | "s" => {
                    let mut remain = Some(1);
                    if let Some(qual) = cmd.next() {
                        if qual == "while" {
                            remain = None;
                        } else {
                            match qual.parse::<u128>() {
                                Ok(n) => remain = Some(n),
                                Err(err) => {
                                    writeln!(out, "error: invalid count: {err}")?;
                                    continue;
                                }
                            }
                        }
                    }

                    if let Some(raised) = already_raised {
                        writeln!(
                            out,
                            "warning: the following exception has already been raised"
                        )?;
                        pretty_println_exception(&mut out, raised)?;
                    }

                    let mut count: Option<u128> = Some(0); // none indicates overflow
                    let mut raised = None;
                    while remain != Some(0) {
                        if stop_exec.swap(false, Ordering::Relaxed) {
                            writeln!(out, "ctrl-c pressed, aborting step")?;
                            break;
                        }
                        if let Err(except) = mem.execute_next() {
                            raised = Some(except);
                            break;
                        }
                        if let Some(n) = count {
                            count = n.checked_add(1);
                        }
                        if let Some(ref mut n) = remain {
                            *n -= 1;
                        }
                    }

                    if let Some(n) = count {
                        write!(
                            out,
                            "executed {n} op{s}... ",
                            s = if n == 1 { "" } else { "s" }
                        )?;
                    }
                    if let Some(except) = raised {
                        writeln!(out, "exception raised")?;
                        pretty_println_exception(&mut out, except)?;
                    } else {
                        writeln!(out, "OK")?;
                    }
                }

                "print" | "p" => match cmd.next() {
                    Some(loc) => {
                        if let Some(reg) = Register::from_str(loc) {
                            let val = mem.regs.read(&mem.tags, reg as _)?;
                            writeln!(out, "{val:#?}")?;
                        } else {
                            writeln!(out, "error: unknown location '{loc}'")?;
                        }
                    }

                    None => writeln!(out, "error: missing argument <location>")?,
                },

                "do" | "d" => {
                    let src = if let Some(s) = cmd.remainder() {
                        s
                    } else {
                        writeln!(out, "error: missing argument <operation>")?;
                        continue;
                    };
                    // HACK: assembler api doesn't let me expect the contents of a line, excluding the newline
                    let src = format!("{src}\n");
                    if let Ok(ops) = super::assemble_src(&src, None) {
                        debug_assert!(ops.len() <= 1);
                        for op in ops {
                            if let Err(raised) = mem.execute_op(op, None, false) {
                                pretty_println_exception(&mut out, raised)?;
                            }
                        }
                    }
                }

                unk => writeln!(out, "error: unknown command '{unk}'")?,
            }
        }
    }
    out.flush()?;
    Ok(())
}

fn splash<W: Write>(mut f: W) -> io::Result<()> {
    let line1 = " fruticose debugger ";
    let line2 = "type 'h' or 'help' for help.";
    writeln!(
        f,
        "{line1:=^len$}",
        len = line2.len() /* assuming line2 is ascii */
    )?;
    writeln!(f, "{line2}")?;
    Ok(())
}

fn pretty_println_exception<W: Write>(mut f: W, except: Exception) -> io::Result<()> {
    writeln!(f, ":: {except}")?;
    Ok(())
}

fn readln<W: Write>(mut out: W, prompt: &str) -> io::Result<String> {
    write!(out, "{prompt}")?;
    out.flush()?;
    let mut stdin = io::stdin().lock();
    let mut input = String::new();
    stdin.read_line(&mut input)?;
    Ok(input)
}
