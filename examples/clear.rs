fn main() {
    { extern crate ansi_interpreter as ai; ai::intercept_stdio(); }

    print!("abcdef\x1b[3D");

    let s = std::env::args().skip(1).next();
    match s.as_ref().map(|s| &**s) {
        Some("down") => { print!("\x1b[0J"); },
        Some("up") => { print!("\x1b[1J"); },
        Some("all") | None => { print!("\x1b[2J"); },
        Some("right") => { print!("\x1b[0K"); },
        Some("left") => { print!("\x1b[1K"); },
        Some("line") => { print!("\x1b[2K"); },
        Some(_) => {
            println!("\x1b[3DUsage: clear [up|down|all|left|right|line]");
            flush();
            return;
        }
    }

    print!("\x1b[3C");
    flush();
}

fn flush() {
    ::std::io::Write::flush(&mut ::std::io::stdout()).unwrap();
    ::std::thread::sleep(::std::time::Duration::from_millis(10));
}
