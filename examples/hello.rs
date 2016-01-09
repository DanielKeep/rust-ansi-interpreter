use std::iter::repeat;

fn main() {
    { extern crate ansi_interpreter as ai; ai::intercept_stdio(); }
    print!("\x1b]2;Hello, World! ☺ ℌ𝔬𝔬𝔯𝔞𝔶 𝔘𝔫𝔦𝔠𝔬𝔡𝔢!\x07");
    println!("Secret text!");
    print!("\x1b[A\x1b[1B\x1b[2A\x1b[B");
    println!("\x1b[0;43;91mH\x1b[22mello\x1b[2D\x1b[C\x1b[1D\x1b[2C\x1b[39m, \x1b[92mW\x1b[22morld\x1b[94m!\x1b[m");
    for i in 0..100 {
        let chs = (i + 1) / 2;
        let s: String = repeat('#').take(chs).chain(repeat(' ')).take(50).collect();
        print!("\x1b[s\x1b[2;3H[{}]\x1b[u", s);
        flush();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn flush() {
    ::std::io::Write::flush(&mut ::std::io::stdout()).unwrap()
}
