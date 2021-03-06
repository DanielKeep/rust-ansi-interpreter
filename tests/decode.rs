extern crate ansi_interpreter as ai;
extern crate itertools;

macro_rules! rethrow {
    ($e:expr) => {
        match $e {
            ::std::result::Result::Ok(v) => ::std::result::Result::Ok(v),
            ::std::result::Result::Err(err) => {
                let err = ::std::convert::From::from(err);
                ::std::result::Result::Err(err)
            }
        }
    };
}

use std::io::{self, Write};
use itertools::Itertools;

type GenError = Box<std::error::Error + Send + Sync>;

struct Dump<W: Write>(W);

impl<W: Write> ai::AnsiInterpret for Dump<W> {
    fn write_text(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn cuu_seq(&mut self, r: u16) -> Result<(), GenError> {
        rethrow!(write!(self.0, "[CUU:{}]", r))
    }
    fn cud_seq(&mut self, r: u16) -> Result<(), GenError> {
        rethrow!(write!(self.0, "[CUD:{}]", r))
    }
    fn cuf_seq(&mut self, c: u16) -> Result<(), GenError> {
        rethrow!(write!(self.0, "[CUF:{}]", c))
    }
    fn cub_seq(&mut self, c: u16) -> Result<(), GenError> {
        rethrow!(write!(self.0, "[CUF:{}]", c))
    }
    fn cup_seq(&mut self, r: u16, c: u16) -> Result<(), GenError> {
        rethrow!(write!(self.0, "[CUP:{},{}]", r, c))
    }
    fn ed_seq(&mut self, n: ai::EraseDisplay) -> Result<(), GenError> {
        rethrow!(write!(self.0, "[ED:{}]", n as u8))
    }
    fn el_seq(&mut self, n: ai::EraseLine) -> Result<(), GenError> {
        rethrow!(write!(self.0, "[EL:{}]", n as u8))
    }
    fn sgr_seq(&mut self, ns: &[u8]) -> Result<(), GenError> {
        let ns = ns.iter().join(",");
        rethrow!(write!(self.0, "[SGR:{}]", ns))
    }
    fn dsr_seq(&mut self) -> Result<(), GenError> {
        rethrow!(self.0.write_all(b"[DSR]"))
    }
    fn scp_seq(&mut self) -> Result<(), GenError> {
        rethrow!(self.0.write_all(b"[SCP]"))
    }
    fn rcp_seq(&mut self) -> Result<(), GenError> {
        rethrow!(self.0.write_all(b"[RCP]"))
    }
    fn hvp_seq(&mut self, r: u16, c: u16) -> Result<(), GenError> {
        rethrow!(write!(self.0, "[HVP:{},{}]", r, c))
    }

    fn osc_txt_seq(&mut self, n: u16, txt: &str) -> Result<(), GenError> {
        rethrow!(write!(self.0, "[OSC:{},{:?}]", n, txt))
    }

    fn other_seq(&mut self, bytes: &[u8]) -> Result<(), GenError> {
        let mut bs = String::new();
        for b in bytes {
            use std::fmt::Write;
            try!(write!(bs, "{:02x}", b)
                .map_err(|_| io::Error::new(
                    io::ErrorKind::Other,
                    "formatting error")));
        }
        rethrow!(write!(self.0, "[UNK:{}]", bs))
    }
}

#[test]
fn test_decode() {
    println!("");
    let mut s = vec![];
    {
        let mut intercept = ai::AnsiIntercept::new(Dump(&mut s));
        write!(intercept,
"
Cursor up four: \x1b[4A = \x1b[2A\x1b[2A.
Cursor down two: \x1b[2B = \x1b[1B\x1b[B.
Two steps \x1b[2C, one step \x1b[D.
Cursor at \x1b[4;12H.
Erase the display \x1b[1J and \x1b[J and \x1b[2J.
Erase the line \x1b[1K and \x1b[K and \x1b[2K.
Cursor at \x1b[12;4f.

Roses are \x1b[31m, backgrounds are \x1b[40m.
Your text is now \x1b[6ming, what'cha think about that?

\x1b[6n\x1b[s\x1b[u\x1b[7x

An unreasonably long, invalid sequence:
\x1b[34567890123456123456789012345612345678901234561234567890123456\
1234567890123456123456789012345612345678901234561234567890123456\
1234567890123456123456789012345612345678901234561234567890123456\
1234567890123456123456789012345612345678901234561234567890123456\
1234567890123456123456789012345612345678901234561234567890123456.

Terminal title: \x1b]2;Final Destination (terminal, geddit?)\x07.
"
        )
    }.expect(&format!("could not write to interceptor; got {:?}", ::std::str::from_utf8(&s).unwrap_or("{invalid}")));

    assert_eq!(&*String::from_utf8(s).unwrap(),
"
Cursor up four: [CUU:4] = [CUU:2][CUU:2].
Cursor down two: [CUD:2] = [CUD:1][CUD:1].
Two steps [CUF:2], one step [CUF:1].
Cursor at [CUP:4,12].
Erase the display [ED:1] and [ED:0] and [ED:2].
Erase the line [EL:1] and [EL:0] and [EL:2].
Cursor at [HVP:12,4].

Roses are [SGR:31], backgrounds are [SGR:40].
Your text is now [SGR:6]ing, what'cha think about that?

[DSR][SCP][RCP][UNK:5b3778]

An unreasonably long, invalid sequence:
\x1b[34567890123456123456789012345612345678901234561234567890123456\
1234567890123456123456789012345612345678901234561234567890123456\
1234567890123456123456789012345612345678901234561234567890123456\
1234567890123456123456789012345612345678901234561234567890123456\
1234567890123456123456789012345612345678901234561234567890123456.

Terminal title: [OSC:2,\"Final Destination (terminal, geddit?)\"].
"
    );
}
