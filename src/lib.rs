extern crate conv;
extern crate num;
extern crate smallvec;

pub use export::*;

#[macro_use] mod macros;

mod ansi;
mod util;
mod win32;

mod export {
    pub use ansi::{AnsiIntercept, EraseDisplay, EraseLine, AnsiInterpret};

    #[cfg(windows)]
    pub use win32::intercept_stdio;

    #[cfg(not(windows))]
    pub fn intercept_stdio() {}
}
