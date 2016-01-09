// TODO: Make an example, instead.

fn main() {
    { extern crate ansi_interpreter as ai; ai::intercept_stdio(); }
    print!("\x1b]2;Hello, World! ☺\x07");
    println!("\x1b[0;43;91mH\x1b[22mello\x1b[39m, \x1b[92mW\x1b[22morld\x1b[94m!\x1b[m");
    std::thread::sleep(std::time::Duration::from_millis(1000));
}
