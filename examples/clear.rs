fn main() {
    { extern crate ansi_interpreter as ai; ai::intercept_stdio(); }

    let s = std::env::args().skip(1).next();
    match s.as_ref().map(|s| &**s) {
        Some("down") => { print!("\x1b[0J"); },
        Some("up") => { print!("\x1b[1J"); },
        Some("all") | None => { print!("\x1b[2J"); },
        Some(_) => {
            println!("Usage: clear [up|down|all]");
        }
    }

    flush();
}

fn flush() {
    ::std::io::Write::flush(&mut ::std::io::stdout()).unwrap();
    ::std::thread::sleep(::std::time::Duration::from_millis(10));
}
