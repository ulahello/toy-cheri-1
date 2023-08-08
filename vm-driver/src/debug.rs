use core::str::FromStr;
use std::io::{self, BufRead, Read, Write};

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
        out: W,
    ) -> anyhow::Result<()> {
        debug_assert_ne!(self, DebugMode::Never);
        launch_inner(mem, already_raised, out)
    }
}

fn launch_inner<W: Write>(
    mem: &mut Memory,
    already_raised: Option<Exception>,
    mut out: W,
) -> anyhow::Result<()> {
    loop {
        let input = readln(&mut out, "> ")?;
        let mut cmd = input.trim().split_ascii_whitespace();
        if let Some(next) = cmd.next() {
            match next {
                "quit" | "q" => break,

                "help" | "h" => {
                    writeln!(out, "quit. quit.")?;
                    writeln!(out, "help. list commands.")?;
                    writeln!(out, "step [<count> | while]. execute the next Op(s).")?;
                    writeln!(out, "read <location>. read value at location.")?;
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
                                }
                            }
                        }
                    }

                    if let Some(raised) = already_raised {
                        writeln!(
                            out,
                            "warning: the following exception has already been raised"
                        )?;
                        writeln!(out, ":: {raised}")?;
                    }

                    let mut count: Option<u128> = Some(0); // none indicates overflow
                    let mut raised = None;
                    while remain != Some(0) {
                        if let Err(except) = mem.execute_op() {
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
                        writeln!(out, ":: {except}")?;
                    } else {
                        writeln!(out, "OK")?;
                    }
                }

                "read" | "r" => match cmd.next() {
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

                unk => writeln!(out, "error: unknown command '{unk}'")?,
            }
        }
    }
    out.flush()?;
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
