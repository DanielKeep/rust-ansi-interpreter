// TODO: Make an example, instead.
extern crate ansi_interpreter as ai;

use std::io::Write;

fn main() {
    let mut out = ai::wrap_stdout().unwrap();
    writeln!(out, "\x1b[0;91mH\x1b[22mello\x1b[m, \x1b[92mW\x1b[22morld\x1b[94m!\x1b[m").unwrap();
}
