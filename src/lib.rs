extern crate conv;
extern crate num;
extern crate smallvec;

pub use ansi::{AnsiIntercept, EraseDisplay, EraseLine, AnsiInterpret};

#[macro_use] mod macros;

mod ansi;
mod util;
mod win32;
