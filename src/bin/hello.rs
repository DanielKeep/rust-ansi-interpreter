// TODO: Make an example, instead.

fn main() {
    { extern crate ansi_interpreter as ai; ai::intercept_stdio(); }
    println!("\x1b[0;91mH\x1b[22mello\x1b[m, \x1b[92mW\x1b[22morld\x1b[94m!\x1b[m");
    std::thread::sleep(std::time::Duration::from_millis(10));
}
