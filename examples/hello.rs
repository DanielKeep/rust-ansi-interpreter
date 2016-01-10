use std::iter::repeat;

fn main() {
    { extern crate ansi_interpreter as ai; ai::intercept_stdio(); }
    print!("\x1b]2;Hello, World! ☺ ℌ𝔬𝔬𝔯𝔞𝔶 𝔘𝔫𝔦𝔠𝔬𝔡𝔢!\x07");
    println!("Secret text!");
    print!("\x1b[A\x1b[1B\x1b[2A\x1b[B");
    println!("\x1b[0;43;91mH\x1b[22mello\x1b[2D\x1b[C\x1b[1D\x1b[2C\x1b[39m, \x1b[92mW\x1b[22morld\x1b[94m!\x1b[m");
    println!("\x1b[0;31;1mBlarp! Blarp!\x1b[m: error text!");
    print!("\x1b[6n");
    flush();
    match read_cpr() {
        Some((r, c)) => {
            println!("\x1b[32;1mCPR\x1b[m: (\x1b[1m{}\x1b[m, \x1b[1m{}\x1b[m)", r, c);
        },
        None => {
            println!("\x1b[32;1mCPR\x1b[m: \x1b[31;1mFAILED\x1b[m");
        }
    }
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

fn read_cpr() -> Option<(u16, u16)> {
    use std::io::Read;

    let mut stdin = std::io::stdin().bytes().peekable();
    let mut stdin = &mut stdin;

    match stdin.next() {
        Some(Ok(0x1b)) => (),
        _ => return None
    }
    match stdin.next() {
        Some(Ok(b'[')) => (),
        _ => return None
    }

    fn is_digit<E>(b: &Result<u8, E>) -> bool {
        b.as_ref().map(|&b| b'0' <= b && b <= b'9').unwrap_or(false)
    }

    let r_bs: Vec<_> = stdin.take_while(is_digit).map(Result::unwrap).collect();
    let c_bs: Vec<_> = stdin.take_while(is_digit).map(Result::unwrap).collect();

    let r = std::str::from_utf8(&r_bs).unwrap_or("").parse().unwrap_or(1);
    let c = std::str::from_utf8(&c_bs).unwrap_or("").parse().unwrap_or(1);

    Some((r, c))
}
