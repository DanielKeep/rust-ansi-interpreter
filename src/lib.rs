extern crate conv;
extern crate num;
extern crate smallvec;

pub use ansi::{AnsiIntercept, EraseDisplay, EraseLine, AnsiInterpret};

// TODO: reconsider exporting this.
pub use win32::intercept_stdio;

#[macro_use] mod macros;

mod ansi;
mod util;
mod win32;
