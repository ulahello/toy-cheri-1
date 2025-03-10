#![feature(int_roundings, const_option)]
#![deny(elided_lifetimes_in_paths)]

pub mod abi;
pub mod access;
pub mod alloc;
pub mod capability;
pub mod exception;
pub mod int;
pub mod mem;
pub mod op;
pub mod process;
pub mod registers;
pub mod revoke;
pub mod syscall;

#[cfg(test)]
pub mod tests;
