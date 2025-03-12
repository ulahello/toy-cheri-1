# CHERI learning experiment (v1)
A few years ago I grew really interested in memory safe hardware and
how a system like CHERI affects (and in some ways simplifies)
operating system design. This is not my expertise **at all**, I'm just
here to learn! This project is an artifact that process.

## What is this specifically?
This is an emulator for a nonexistent/nonstandard ISA heavily based on
CHERI for RISC-V.

### `/libvm`
The heart of the emulator. This contains the implementation of
capabilities, permissions, a few syscalls, memory allocation, and the
actual instructions.

### `/libasm`
An assembler library --- it converts structured text into a sequence
of instructions that can be directly passed to the emulator.

There is no bytecode format! The emulator just loads the text file.

### `/vm`
User-facing program that executes the given assembly source.

There's also an interactive debugger that lets you play with the
registers. There's no breakpoints yet, you just have to save the
program counter into a saved register and hope that the code you're
testing does not violate the calling convention (!!!).

```console
$ cargo build
$ cargo run -- --help
Usage: fruticose [-g <granules>] [-s <stack-size>] [-d <debug>] -i <init>

Fruticose virtual machine

Options:
  -g, --granules    granules of physical memory to use
  -s, --stack-size  stack size in bytes for init program
  -d, --debug       choose if/how to run the debugger
  -i, --init        path to init program assembly
  --help            display usage information
```

## v1?
I've been working on a Zig rewrite that is more complete and better
distinguishes the emulator layer from user programs and kernels. I'm
also trying to properly implement interrupts (partially complete, but
doesn't integrate with CHERI security yet).

## Further reading
- [CHERI homepage](https://www.cl.cam.ac.uk/research/security/ctsrd/cheri/)
- [Introduction to CHERI](https://www.cl.cam.ac.uk/techreports/UCAM-CL-TR-941.pdf)
- [ISA spec](https://www.cl.cam.ac.uk/techreports/UCAM-CL-TR-987.pdf)
- [Deeply cool paper about OS design from ~first principles with CHERI](https://www.cl.cam.ac.uk/techreports/UCAM-CL-TR-961.pdf)
