/*

# Todo

## Track the window

Most terminals work by having a fixed-size window, where lines pushed off the top go into the scrollback buffer.  Windows has a giant buffer with a sliding window.

It would produce more "expected" behaviour if the interpreter kept track of where the window was "up to", so that even if you scroll away, it can work out how to interpret window positions relative to the buffer.

*/
#![cfg(windows)]
extern crate kernel32;
extern crate winapi;
extern crate wio;

pub use self::intercept::intercept_stdio;

mod intercept;

use std::cmp::{max, min};
use std::io::{self, Write};
use self::winapi::{
    DWORD, HANDLE, WORD,
    CONSOLE_SCREEN_BUFFER_INFO, COORD,
};
use self::wio::wide::ToWide;
use ansi::{AnsiIntercept, EraseDisplay, EraseLine, AnsiInterpret};
use conv::{ConvUtil, UnwrapOrSaturate};

type GenError = Box<::std::error::Error + Send + Sync>;

const COLOR_ALL: WORD = FOREGROUND_WHITE | FOREGROUND_INTENSITY | BACKGROUND_WHITE | BACKGROUND_INTENSITY;

const FOREGROUND_SHIFT: usize = 0;
const FOREGROUND_BLUE: WORD = winapi::FOREGROUND_BLUE as WORD;
const FOREGROUND_GREEN: WORD = winapi::FOREGROUND_GREEN as WORD;
const FOREGROUND_RED: WORD = winapi::FOREGROUND_RED as WORD;
const FOREGROUND_INTENSITY: WORD = winapi::FOREGROUND_INTENSITY as WORD;
const FOREGROUND_WHITE: WORD = FOREGROUND_RED | FOREGROUND_GREEN | FOREGROUND_BLUE;

const BACKGROUND_SHIFT: usize = 4;
const BACKGROUND_BLUE: WORD = winapi::BACKGROUND_BLUE as WORD;
const BACKGROUND_GREEN: WORD = winapi::BACKGROUND_GREEN as WORD;
const BACKGROUND_RED: WORD = winapi::BACKGROUND_RED as WORD;
const BACKGROUND_INTENSITY: WORD = winapi::BACKGROUND_INTENSITY as WORD;
const BACKGROUND_WHITE: WORD = BACKGROUND_RED | BACKGROUND_GREEN | BACKGROUND_BLUE;

// TODO: reconsider this
pub fn wrap_stdout() -> Result<AnsiIntercept<io::Stdout, ConsoleInterpreter>, io::Error> {
    let stdout = io::stdout();

    let console = unsafe {
        match kernel32::GetStdHandle(winapi::STD_OUTPUT_HANDLE) {
            h if h == winapi::INVALID_HANDLE_VALUE => return Err(io::Error::last_os_error()),
            h if h.is_null() => return Err(io::Error::new(io::ErrorKind::NotFound, "stdout not connected")),
            h => h,
        }
    };

    let ci = ConsoleInterpreter::new(console);

    Ok(AnsiIntercept::new(stdout, ci))
}

pub struct ConsoleInterpreter {
    console: HANDLE,
    scp: COORD,
}

impl ConsoleInterpreter {
    pub fn new(console: HANDLE) -> Self {
        ConsoleInterpreter {
            console: console,
            scp: COORD {
                X: 0,
                Y: 0,
            }
        }
    }

    fn mut_text_attrs<F, R>(&self, f: F) -> Result<R, io::Error>
    where F: FnOnce(&mut WORD) -> R {
        unsafe {
            let mut info = ::std::mem::zeroed();
            if kernel32::GetConsoleScreenBufferInfo(self.console, &mut info) == 0 {
                return Err(io::Error::last_os_error());
            }
            let mut attrs = info.wAttributes;
            let r = f(&mut attrs);
            if kernel32::SetConsoleTextAttribute(self.console, attrs) == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(r)
        }
    }
}

impl AnsiInterpret for ConsoleInterpreter {
    fn cuu_seq<W: Write>(&mut self, _: &mut W, r: u16) -> Result<(), GenError> {
        if r == 0 { return Ok(()); }

        let csbi = try!(get_console_screen_buffer_info(self.console));

        let abs_y = csbi.dwCursorPosition.Y;
        let abs_x = csbi.dwCursorPosition.X;

        let abs_y = max(0, abs_y.saturating_sub(r.value_as::<i16>().unwrap_or_saturate()));

        let abs_pos = COORD {
            X: abs_x,
            Y: abs_y,
        };

        try!(set_console_cursor_position(self.console, abs_pos));
        Ok(())
    }

    fn cud_seq<W: Write>(&mut self, _: &mut W, r: u16) -> Result<(), GenError> {
        if r == 0 { return Ok(()); }

        let csbi = try!(get_console_screen_buffer_info(self.console));

        let abs_y = csbi.dwCursorPosition.Y;
        let abs_x = csbi.dwCursorPosition.X;

        let abs_y = min(csbi.dwSize.Y - 1, abs_y.saturating_add(r.value_as::<i16>().unwrap_or_saturate()));

        let abs_pos = COORD {
            X: abs_x,
            Y: abs_y,
        };

        try!(set_console_cursor_position(self.console, abs_pos));
        Ok(())
    }

    fn cuf_seq<W: Write>(&mut self, _: &mut W, c: u16) -> Result<(), GenError> {
        if c == 0 { return Ok(()); }

        let csbi = try!(get_console_screen_buffer_info(self.console));

        let abs_y = csbi.dwCursorPosition.Y;
        let abs_x = csbi.dwCursorPosition.X;

        let abs_x = min(csbi.dwSize.X - 1, abs_x.saturating_add(c.value_as::<i16>().unwrap_or_saturate()));

        let abs_pos = COORD {
            X: abs_x,
            Y: abs_y,
        };

        try!(set_console_cursor_position(self.console, abs_pos));
        Ok(())
    }

    fn cub_seq<W: Write>(&mut self, _: &mut W, c: u16) -> Result<(), GenError> {
        if c == 0 { return Ok(()); }

        let csbi = try!(get_console_screen_buffer_info(self.console));

        let abs_y = csbi.dwCursorPosition.Y;
        let abs_x = csbi.dwCursorPosition.X;

        let abs_x = max(0, abs_x.saturating_sub(c.value_as::<i16>().unwrap_or_saturate()));

        let abs_pos = COORD {
            X: abs_x,
            Y: abs_y,
        };

        try!(set_console_cursor_position(self.console, abs_pos));
        Ok(())
    }

    fn cup_seq<W: Write>(&mut self, _: &mut W, r: u16, c: u16) -> Result<(), GenError> {
        let x = c.saturating_sub(1);
        let y = r.saturating_sub(1);

        let csbi = try!(get_console_screen_buffer_info(self.console));

        let x = min(x, csbi.dwSize.X.value_as::<u16>().unwrap_or_saturate() - 1);
        let y = min(y, csbi.dwSize.Y.value_as::<u16>().unwrap_or_saturate() - 1);

        let abs_x = x + csbi.srWindow.Left.value_as::<u16>().unwrap_or_saturate();
        let abs_y = y + csbi.srWindow.Top.value_as::<u16>().unwrap_or_saturate();

        let abs_pos = COORD {
            X: abs_x.value_as::<i16>().unwrap_or_saturate(),
            Y: abs_y.value_as::<i16>().unwrap_or_saturate(),
        };

        try!(set_console_cursor_position(self.console, abs_pos));
        Ok(())
    }

    fn ed_seq<W: Write>(&mut self, _: &mut W, n: EraseDisplay) -> Result<(), GenError> {
        use ansi::EraseDisplay::*;
        unsafe {
            let csbi = try!(get_console_screen_buffer_info(self.console));

            let (start, len) = match n {
                TopToCursor => {
                    let start = COORD {
                        X: 0,
                        Y: csbi.srWindow.Top,
                    };
                    let lines = (csbi.dwCursorPosition.Y - start.Y) + 1;
                    let lines = lines.value_as::<DWORD>().unwrap_or_saturate();
                    let len = lines * csbi.dwSize.X.value_as::<DWORD>().unwrap_or_saturate();
                    (start, len)
                },
                CursorToBottom => {
                    let start = COORD {
                        X: 0,
                        Y: csbi.dwCursorPosition.Y,
                    };
                    let lines = (csbi.srWindow.Bottom - start.Y) + 1;
                    let lines = lines.value_as::<DWORD>().unwrap_or_saturate();
                    let len = lines * csbi.dwSize.X.value_as::<DWORD>().unwrap_or_saturate();
                    (start, len)
                },
                All => {
                    let start = COORD {
                        X: 0,
                        Y: csbi.srWindow.Top,
                    };
                    let lines = (csbi.srWindow.Bottom - start.Y) + 1;
                    let lines = lines.value_as::<DWORD>().unwrap_or_saturate();
                    let len = lines * csbi.dwSize.X.value_as::<DWORD>().unwrap_or_saturate();
                    (start, len)
                },
            };

            let mut dummy = 0;
            if kernel32::FillConsoleOutputAttribute(self.console, csbi.wAttributes, len, start, &mut dummy) == 0 {
                throw!(io::Error::last_os_error());
            }
            if kernel32::FillConsoleOutputCharacterW(self.console, 0x20, len, start, &mut dummy) == 0 {
                throw!(io::Error::last_os_error());
            }

            Ok(())
        }
    }

    fn el_seq<W: Write>(&mut self, sink: &mut W, n: EraseLine) -> Result<(), GenError> {
        rethrow!(write!(sink, "[EL:{}]", n as u8))
    }

    fn sgr_seq<W: Write>(&mut self, sink: &mut W, ns: &[u8]) -> Result<(), GenError> {
        try!(sink.flush());
        for &n in ns {
            match n {
                0 => try!(self.mut_text_attrs(|attrs| {
                    // Reset.
                    *attrs = (*attrs & !COLOR_ALL) | FOREGROUND_WHITE;
                })),
                1 => try!(self.mut_text_attrs(|attrs| {
                    // Bold.
                    *attrs = *attrs | FOREGROUND_INTENSITY;
                })),
                22 => try!(self.mut_text_attrs(|attrs| {
                    // Not-bold.
                    *attrs = *attrs & !FOREGROUND_INTENSITY;
                })),
                n @ 30...37 => try!(self.mut_text_attrs(|attrs| {
                    // Foreground.
                    if let Some(c) = sgr_color_to_fg(n) {
                        *attrs = (*attrs & !FOREGROUND_WHITE) | c;
                    }
                })),
                39 => try!(self.mut_text_attrs(|attrs| {
                    // Default-foreground.
                    *attrs = (*attrs & !FOREGROUND_INTENSITY) | FOREGROUND_WHITE;
                })),
                _n @ 40...47 => try!(self.mut_text_attrs(|attrs| {
                    // Background.
                    if let Some(c) = sgr_color_to_bg(n) {
                        *attrs = (*attrs & !BACKGROUND_WHITE) | c;
                    }
                })),
                49 => try!(self.mut_text_attrs(|attrs| {
                    // Default-background.
                    *attrs = (*attrs & !BACKGROUND_INTENSITY) | BACKGROUND_WHITE;
                })),
                n @ 90...97 => try!(self.mut_text_attrs(|attrs| {
                    // Bold-foreground.
                    if let Some(c) = sgr_color_to_fg(n) {
                        *attrs = (*attrs & !FOREGROUND_WHITE) | c;
                    }
                })),
                _n @ 100...107 => try!(self.mut_text_attrs(|attrs| {
                    // Bold-background.
                    if let Some(c) = sgr_color_to_bg(n) {
                        *attrs = (*attrs & !BACKGROUND_WHITE) | c;
                    }
                })),
                _ => {
                    // Do nothing.
                }
            }
        }
        Ok(())
    }

    fn dsr_seq<W: Write>(&mut self, sink: &mut W) -> Result<(), GenError> {
        rethrow!(sink.write_all(b"[DSR]"))
    }

    fn scp_seq<W: Write>(&mut self, _: &mut W) -> Result<(), GenError> {
        let info = try!(get_console_screen_buffer_info(self.console));
        self.scp = info.dwCursorPosition;
        Ok(())
    }

    fn rcp_seq<W: Write>(&mut self, _: &mut W) -> Result<(), GenError> {
        try!(set_console_cursor_position(self.console, self.scp));
        Ok(())
    }

    fn osc_txt_seq<W: Write>(&mut self, _: &mut W, n: u16, txt: &str) -> Result<(), GenError> {
        unsafe {
            match n {
                0 | 2 => {
                    let wtxt = txt.to_wide_null();
                    if kernel32::SetConsoleTitleW(wtxt.as_ptr()) == 0 {
                        throw!(io::Error::last_os_error())
                    }
                    Ok(())
                },
                _ => Ok(())
            }
        }
    }

    fn hvp_seq<W: Write>(&mut self, sink: &mut W, r: u16, c: u16) -> Result<(), GenError> {
        self.cup_seq(sink, r, c)
    }

    fn other_seq<W: Write>(&mut self, sink: &mut W, bytes: &[u8]) -> Result<(), GenError> {
        let mut bs = String::new();
        for b in bytes {
            use std::fmt::Write;
            write!(bs, "{:02x}", b).unwrap();
        }
        rethrow!(write!(sink, "[UNK:{}]", bs))
    }
}

#[test]
fn test_winapi_consts() {
    use self::FOREGROUND_RED as FR;
    use self::FOREGROUND_GREEN as FG;
    use self::FOREGROUND_BLUE as FB;
    use self::FOREGROUND_SHIFT as FS;

    use self::BACKGROUND_RED as BR;
    use self::BACKGROUND_GREEN as BG;
    use self::BACKGROUND_BLUE as BB;
    use self::BACKGROUND_SHIFT as BS;

    assert_eq!(1 << FS, FB);
    assert_eq!(2 << FS, FG);
    assert_eq!(4 << FS, FR);

    assert_eq!(1 << BS, BB);
    assert_eq!(2 << BS, BG);
    assert_eq!(4 << BS, BR);
}

fn sgr_color_to_fg(n: u8) -> Option<WORD> {
    use self::FOREGROUND_INTENSITY as FI;
    use self::FOREGROUND_SHIFT as FS;

    fn split_bits(c: u8) -> WORD {
        (((c & 1) << 2)
            | (c & 2)
            | ((c & 4) >> 2)) as WORD
    }

    Some(match n {
        n @ 30...37 => split_bits(n - 30) << FS,
        n @ 90...97 => (split_bits(n - 90) << FS) | FI,
        39 => 0,

        _ => return None
    })
}

fn sgr_color_to_bg(n: u8) -> Option<WORD> {
    use self::BACKGROUND_INTENSITY as BI;
    use self::BACKGROUND_SHIFT as BS;

    fn split_bits(c: u8) -> WORD {
        (((c & 1) << 2)
            | (c & 2)
            | ((c & 4) >> 2)) as WORD
    }

    Some(match n {
        n @ 40...47 => split_bits(n - 40) << BS,
        n @ 100...107 => (split_bits(n - 100) << BS) | BI,
        49 => 0,

        _ => return None
    })
}

fn get_console_screen_buffer_info(console: HANDLE) -> io::Result<CONSOLE_SCREEN_BUFFER_INFO> {
    unsafe {
        let mut info = ::std::mem::zeroed();
        if kernel32::GetConsoleScreenBufferInfo(console, &mut info) == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(info)
        }
    }
}

fn set_console_cursor_position(console: HANDLE, pos: COORD) -> io::Result<()> {
    unsafe {
        if kernel32::SetConsoleCursorPosition(console, pos) == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}
