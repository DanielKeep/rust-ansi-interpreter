#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(unused_variables)]

use std::cmp::min;
use std::error::Error;
use std::ops::{Add, Mul};
use std::io::{self, Write};
use conv::{TryFrom, TryInto, UnwrapOk, ValueFrom, ValueInto};
use num::Zero;
use smallvec::{Array, SmallVec};
use util::drop_front;

// Should be two pointers worth to get the most out of SmallVec.
#[cfg(target_pointer_width = "32")]
const MIN_BUFFER_SIZE: usize = 8;
#[cfg(target_pointer_width = "64")]
const MIN_BUFFER_SIZE: usize = 16;

// How long will we let a sequence get before we give up and assume someone's trying to crash us?
const MAX_SEQ_SIZE: usize = 256;

#[test]
fn test_max_seq_size() {
    assert!(MIN_BUFFER_SIZE < MAX_SEQ_SIZE);
}

// How much stack space should we use for buffering sequences during parsing?  This has to be a number supported by `smallvec`.
const SEQ_BUFFER_SIZE: usize = 32;

const ESC: u8 = 0x1b;

marker_error! {
    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    pub struct MalformedSeq
    impl {
        desc {"malformed escape sequence"}
    }
}

pub struct AnsiIntercept<I>
where I: AnsiInterpret {
    /// Buffer for incomplete escape sequences.
    buffer: SmallVec<[u8; MIN_BUFFER_SIZE]>,

    /// Interpreter instance.
    interp: I,
}

impl<I> AnsiIntercept<I>
where I: AnsiInterpret {
    pub fn new(interp: I) -> Self {
        AnsiIntercept {
            buffer: SmallVec::new(),
            interp: interp,
        }
    }
}

impl<I> Write for AnsiIntercept<I>
where I: AnsiInterpret {
    fn write(&mut self, mut buf: &[u8]) -> io::Result<usize> {
        /*
        Fast path: no partial escape sequence being buffered, so if we can find a run of bytes with no escape sequences, we can just dump everything up to that point.
        */
        if self.buffer.len() == 0 {
            let run_len = buf.iter().cloned().enumerate()
                .filter(|&(_, b)| is_escape_start(b))
                .map(|(i, _)| i)
                .next()
                .unwrap_or(buf.len());
            if run_len > 0 {
                let run = &buf[0..run_len];
                return self.interp.write_text(run);
            }
        }

        /*
        If the input is empty, stop now.
        */
        if buf.len() == 0 {
            return Ok(0);
        }

        /*
        The way we handle escape codes is thus: *either* the buffered bytes plus the input forms a complete sequence (plus a tail) *or* it doesn't.

        In the latter case, we tack `buf` on to the end of the buffer and return that we've consumed those bytes.

        In the former, we clear the buffer and chop off the bytes of `buf` that were used and return having consumed *only* those bytes.

        It might be worth, in the future, distinguishing cases where we don't violate the "each `write` represents a single attempt to write to the underlying object" and just writing the next run of text after an escape sequence.
        */
        match {
            let bytes = self.buffer.iter().cloned()
                .chain(buf.iter().cloned());

            extract_sequence(bytes, &mut self.interp)
        } {
            Ok(EscSeqParse::IncompleteSeq) => {
                // If the buffer is getting suspiciously long, give up and dump up to `MAX_SEQ_SIZE` bytes.  This is so that spurious escape bytes don't cause large chunks of output to disappear.
                if self.buffer.len() + buf.len() > MAX_SEQ_SIZE {
                    let limit = MAX_SEQ_SIZE - self.buffer.len();
                    try!(self.interp.write_text(&self.buffer));
                    self.buffer = SmallVec::new();

                    let limit = min(limit, buf.len());
                    try!(self.interp.write_text(&buf[..limit]));
                    Ok(limit)
                } else {
                    self.buffer.extend(buf.iter().cloned());
                    Ok(buf.len())
                }
            },

            Ok(EscSeqParse::UsedBytes(n)) => {
                let mut n = n.value_into().unwrap_ok();
                let drop_n = min(n, self.buffer.len());
                n -= drop_n;

                drop_front(&mut self.buffer, drop_n);

                // Say that we've consumed the relevant bytes from `buf`.
                Ok(n)
            },

            Err(err) => {
                Err(io::Error::new(io::ErrorKind::InvalidData, err))
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.interp.flush()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum EraseDisplay {
    CursorToBottom,
    TopToCursor,
    All,
}

marker_error! {
    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    pub struct InvalidEraseDisplayArg
    impl {
        desc {"invalid erase display arg"}
    }
}

impl TryFrom<Option<u8>> for EraseDisplay {
    type Err = InvalidEraseDisplayArg;
    fn try_from(v: Option<u8>) -> Result<EraseDisplay, Self::Err> {
        use self::EraseDisplay::*;
        match v {
            Some(0) | None => Ok(CursorToBottom),
            Some(1) => Ok(TopToCursor),
            Some(2) => Ok(All),
            _ => Err(InvalidEraseDisplayArg)
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum EraseLine {
    CursorToEnd,
    StartToCursor,
    All,
}

marker_error! {
    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    pub struct InvalidEraseLineArg
    impl {
        desc {"invalid erase display arg"}
    }
}

impl TryFrom<Option<u8>> for EraseLine {
    type Err = InvalidEraseLineArg;
    fn try_from(v: Option<u8>) -> Result<EraseLine, Self::Err> {
        use self::EraseLine::*;
        match v {
            Some(0) | None => Ok(CursorToEnd),
            Some(1) => Ok(StartToCursor),
            Some(2) => Ok(All),
            _ => Err(InvalidEraseLineArg)
        }
    }
}

pub trait AnsiInterpret {
    fn write_text(&mut self, buf: &[u8]) -> io::Result<usize>;
    fn flush(&mut self) -> io::Result<()> { Ok(()) }

    fn cuu_seq(&mut self, r: u16) -> Result<(), GenError> { Ok(()) }
    fn cud_seq(&mut self, r: u16) -> Result<(), GenError> { Ok(()) }
    fn cuf_seq(&mut self, c: u16) -> Result<(), GenError> { Ok(()) }
    fn cub_seq(&mut self, c: u16) -> Result<(), GenError> { Ok(()) }
    fn cup_seq(&mut self, r: u16, c: u16) -> Result<(), GenError> { Ok(()) }
    fn ed_seq(&mut self, n: EraseDisplay) -> Result<(), GenError> { Ok(()) }
    fn el_seq(&mut self, n: EraseLine) -> Result<(), GenError> { Ok(()) }
    fn sgr_seq(&mut self, ns: &[u8]) -> Result<(), GenError> { Ok(()) }
    fn dsr_seq(&mut self) -> Result<(), GenError> { Ok(()) }
    fn scp_seq(&mut self) -> Result<(), GenError> { Ok(()) }
    fn rcp_seq(&mut self) -> Result<(), GenError> { Ok(()) }

    fn osc_txt_seq(&mut self, n: u16, txt: &str) -> Result<(), GenError> { Ok(()) }

    fn hvp_seq(&mut self, r: u16, c: u16) -> Result<(), GenError> {
        self.cup_seq(r, c)
    }

    fn other_seq(&mut self, bytes: &[u8]) -> Result<(), GenError> {
        Ok(())
    }
}

enum EscSeqParse {
    IncompleteSeq,
    UsedBytes(u16),
}

#[test]
fn test_esc_seq_parse_used_bytes_size() {
    use self::EscSeqParse::*;
    let dummy = match UsedBytes(0) { UsedBytes(v) => v, _ => panic!("wat") };
    let max_size = (1 << 8*::std::mem::size_of_val(&dummy)) - 1;
    assert!(max_size >= MAX_SEQ_SIZE);
}

type GenError = Box<Error + Send + Sync>;
type ParseResult = Result<EscSeqParse, GenError>;

/**
This function's job is to extract a complete escape sequence from the input, then pass *that* to the function that will parse it (sans opening `ESC`).
*/
fn extract_sequence<B, I>(bytes: B, interp: &mut I) -> ParseResult
where
    B: Iterator<Item=u8> + Clone,
    I: AnsiInterpret,
{
    use self::EscSeqParse::*;

    let bytes_start = bytes.clone();
    let mut bytes = bytes;
    assert_eq!(bytes.next(), Some(ESC));

    let mut state = ExtractState::Start;
    let seq_len = { bytes.take(MAX_SEQ_SIZE).take_while(|&b| state.push(b)).count() };
    if state != ExtractState::End {
        return Ok(IncompleteSeq);
    }

    let seq_bytes = bytes_start.take(1 + seq_len).skip(1);
    let seq_bytes: SmallVec<[_; SEQ_BUFFER_SIZE]> = seq_bytes.collect();
    match parse_sequence(&seq_bytes, interp) {
        // Don't forget that we dropped the leading `ESC`.
        Ok(UsedBytes(bs)) => Ok(UsedBytes(bs + 1)),
        other => rethrow!(other)
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum ExtractState {
    Start,
    CsiStart,
    CsiBody,
    CsiTail,
    Osc,
    OscEsc,
    End
}

impl ExtractState {
    fn push(&mut self, b: u8) -> bool {
        use self::ExtractState::*;

        *self = match (*self, b) {
            (Start, b'[') => CsiStart,
            (Start, b']') => Osc,
            (Start, _) => End,

            (CsiStart, 0x3c...0x3f) => CsiStart,
            (CsiStart, 0x30...0x39) | (CsiStart, 0x3b) => CsiBody,
            (CsiStart, 0x20...0x2f) => CsiTail,
            (CsiStart, 0x40...0x7e) => End,
            (CsiStart, _) => End,

            (CsiBody, 0x30...0x39) | (CsiBody, 0x3b) => CsiBody,
            (CsiBody, 0x20...0x2f) => CsiTail,
            (CsiBody, 0x40...0x7e) => End,
            (CsiBody, _) => End,

            (CsiTail, 0x20...0x2f) => CsiTail,
            (CsiTail, 0x40...0x7e) => End,
            (CsiTail, _) => End,

            (Osc, 0x07) => End,
            (Osc, 0x1b) => OscEsc,
            (Osc, _) => Osc,

            (OscEsc, b'\\') => End,
            (OscEsc, _) => Osc,

            (End, _) => return false
        };

        true
    }
}

/**
Parse the extracted escape sequence, and call the appropriate trait method.
*/
fn parse_sequence<I>(bytes: &[u8], interp: &mut I) -> ParseResult
where I: AnsiInterpret {
    use self::EscSeqParse::*;

    /*
    One somewhat frustrating aspect of how ANSI codes are structured is that the terminal letter is what decides *which* code you're talking about.  This makes doing any sort of pre-emptive parsing a bit dicey.
    */
    let ok_result = {let l = bytes.len() as u16; move |()| UsedBytes(l)};

    if let Some(&b'[') = bytes.first() {
        let tail_bytes = &bytes[1..];
        let term = match tail_bytes.last().map(|&b| b) {
            Some(b) => b,
            None => throw!(MalformedSeq)
        };
        let arg_bytes = &tail_bytes[..tail_bytes.len()-1];
        match term {
            b'A' => {
                let r = try!(parse_1n(arg_bytes));
                let r = r.unwrap_or(1);
                rethrow!(interp.cuu_seq(r).map(ok_result))
            },
            b'B' => {
                let r = try!(parse_1n(arg_bytes));
                let r = r.unwrap_or(1);
                rethrow!(interp.cud_seq(r).map(ok_result))
            },
            b'C' => {
                let c = try!(parse_1n(arg_bytes));
                let c = c.unwrap_or(1);
                rethrow!(interp.cuf_seq(c).map(ok_result))
            },
            b'D' => {
                let c = try!(parse_1n(arg_bytes));
                let c = c.unwrap_or(1);
                rethrow!(interp.cub_seq(c).map(ok_result))
            },
            b'H' => {
                let (r, c) = try!(parse_2n(arg_bytes));
                let r = r.unwrap_or(1);
                let c = c.unwrap_or(1);
                rethrow!(interp.cup_seq(r, c).map(ok_result))
            },
            b'J' => {
                let n = try!(parse_1n(arg_bytes));
                let n = try!(n.try_into());
                rethrow!(interp.ed_seq(n)
                    .map(ok_result))
            },
            b'K' => {
                let n = try!(parse_1n(arg_bytes));
                let n = try!(n.try_into());
                rethrow!(interp.el_seq(n)
                    .map(ok_result))
            },
            b'f' => {
                let (r, c) = try!(parse_2n(arg_bytes));
                let r = r.unwrap_or(1);
                let c = c.unwrap_or(1);
                rethrow!(interp.hvp_seq(r, c).map(ok_result))
            },
            b'm' => {
                let mut ns = try!(parse_ns::<[_; 2], _>(arg_bytes));
                if ns.len() == 0 {
                    ns.push(0);
                }
                rethrow!(interp.sgr_seq(&ns).map(ok_result))
            },
            b'n' => {
                let n = try!(parse_1n(arg_bytes));
                let n = n.unwrap_or(0);
                // n = 6 is the only meaningful parameter for us.
                if n == 6 {
                    rethrow!(interp.dsr_seq().map(ok_result))
                } else {
                    rethrow!(interp.other_seq(&bytes).map(ok_result))
                }
            },
            b's' => {
                try!(parse_0n(arg_bytes));
                rethrow!(interp.scp_seq().map(ok_result))
            },
            b'u' => {
                try!(parse_0n(arg_bytes));
                rethrow!(interp.rcp_seq().map(ok_result))
            },
            _ => rethrow!(interp.other_seq(&bytes).map(ok_result))
        }
    } else if let Some(&b']') = bytes.first() {
        // Grab leading number.
        let tail_bytes = &bytes[1..];
        let (tail_bytes, n) = try!(parse_num(tail_bytes));
        let n = match n { Some(n) => n, None => throw!(MalformedSeq) };

        // Strip ;
        match tail_bytes.first() {
            Some(&b';') => (),
            _ => return rethrow!(interp.other_seq(&bytes).map(ok_result))
        }
        let tail_bytes = &tail_bytes[1..];

        // Strip ending ST
        let drop_end = match tail_bytes.last() {
            Some(&7) => 1,
            Some(&b'\\') => 2,
            _ => throw!(MalformedSeq)
        };

        // Pull out text
        let txt = &tail_bytes[..tail_bytes.len() - drop_end];
        let txt = ::std::str::from_utf8(txt).expect("non-ASCII in OSC txt");

        rethrow!(interp.osc_txt_seq(n, txt).map(ok_result))
    } else {
        rethrow!(interp.other_seq(&bytes).map(ok_result))
    }
}

trait ParseNum: Zero + ValueFrom<u64> + Add<Self, Output=Self> + Mul<Self, Output=Self> {}
impl<T> ParseNum for T
where T: Zero + ValueFrom<u64> + Add<T, Output=T> + Mul<T, Output=T> {}

fn parse_0n(mut bytes: &[u8]) -> Result<(), MalformedSeq> {
    if bytes != b"" {
        Err(MalformedSeq)
    } else {
        Ok(())
    }
}

#[test]
fn test_parse_0n() {
    assert_eq!(parse_0n(b""), Ok(()));
    assert_eq!(parse_0n(b";"), Err(MalformedSeq));
    assert_eq!(parse_0n(b"0"), Err(MalformedSeq));
    assert_eq!(parse_0n(b"0m"), Err(MalformedSeq));
    assert_eq!(parse_0n(b"0;0"), Err(MalformedSeq));
    assert_eq!(parse_0n(b"0;0m"), Err(MalformedSeq));
}

fn parse_1n<N>(mut bytes: &[u8]) -> Result<Option<N>, MalformedSeq>
where N: ParseNum {
    let (bytes, n) = try!(parse_num(bytes));
    if bytes != b"" {
        Err(MalformedSeq)
    } else {
        Ok(n)
    }
}

#[test]
fn test_parse_1n() {
    assert_eq!(parse_1n::<u8>(b""), Ok(None));
    assert_eq!(parse_1n::<u8>(b"0"), Ok(Some(0)));
    assert_eq!(parse_1n::<u8>(b";"), Err(MalformedSeq));
    assert_eq!(parse_1n::<u8>(b"m"), Err(MalformedSeq));
    assert_eq!(parse_1n::<u8>(b"0m"), Err(MalformedSeq));
    assert_eq!(parse_1n::<u8>(b";0"), Err(MalformedSeq));
    assert_eq!(parse_1n::<u8>(b"0;0"), Err(MalformedSeq));
    assert_eq!(parse_1n::<u8>(b"0;0m"), Err(MalformedSeq));
}

fn parse_2n<N>(mut bytes: &[u8]) -> Result<(Option<N>, Option<N>), MalformedSeq>
where N: ParseNum {
    let (bytes, n1) = try!(parse_num(bytes));
    match bytes.first().cloned() {
        None => Ok((n1, None)),
        Some(b';') => {
            let (bytes, n2) = try!(parse_num(&bytes[1..]));
            if bytes == b"" {
                Ok((n1, n2))
            } else {
                Err(MalformedSeq)
            }
        },
        _ => Err(MalformedSeq)
    }
}

#[test]
fn test_parse_2n() {
    assert_eq!(parse_2n::<u8>(b""), Ok((None, None)));
    assert_eq!(parse_2n::<u8>(b"0"), Ok((Some(0), None)));
    assert_eq!(parse_2n::<u8>(b";"), Ok((None, None)));
    assert_eq!(parse_2n::<u8>(b"m"), Err(MalformedSeq));
    assert_eq!(parse_2n::<u8>(b"0m"), Err(MalformedSeq));
    assert_eq!(parse_2n::<u8>(b";1"), Ok((None, Some(1))));
    assert_eq!(parse_2n::<u8>(b"0;1"), Ok((Some(0), Some(1))));
    assert_eq!(parse_2n::<u8>(b"0;1m"), Err(MalformedSeq));
}

fn parse_ns<A, N>(mut bytes: &[u8]) -> Result<SmallVec<A>, MalformedSeq>
where
    A: Array<Item=N>,
    N: ParseNum,
{
    let mut ns = SmallVec::new();

    while bytes != b"" {
        let (tail, n) = try!(parse_num(bytes));
        match tail.first().cloned() {
            Some(b';') => bytes = &tail[1..],
            Some(_) => return Err(MalformedSeq),
            None => bytes = tail,
        }
        if let Some(n) = n {
            ns.push(n);
        }
    }

    Ok(ns)
}

#[test]
fn test_parse_ns() {
    macro_rules! check_ns {
        (@result Ok($($elems:expr),*)) => {
            Ok(vec![$($elems),*])
        };
        (@result Err($err:expr)) => {
            Err($err)
        };
        ($bs:expr, $ok_or_err:ident($($res:tt)*)) => {
            {
                let r = parse_ns::<[u8; 2], u8>($bs)
                    .map(|a| a.iter().cloned().collect::<Vec<_>>());
                assert_eq!(r, check_ns!(@result $ok_or_err($($res)*)));
            }
        };
    }

    check_ns!(b"", Ok());
    check_ns!(b"0", Ok(0));
    check_ns!(b";", Ok());
    check_ns!(b"m", Err(MalformedSeq));
    check_ns!(b"0m", Err(MalformedSeq));
    check_ns!(b";1", Ok(1));
    check_ns!(b"0;1", Ok(0, 1));
    check_ns!(b"0;1m", Err(MalformedSeq));
}

fn parse_num<N>(mut bytes: &[u8]) -> Result<(&[u8], Option<N>), MalformedSeq>
where N: ParseNum {
    let mut v = Zero::zero();
    let mut default = true;
    while let Some(&b) = bytes.first() {
        match b {
            b'0'...b'9' => {
                let dig = try!(((b - b'0') as u64).value_into()
                    .map_err(|_| MalformedSeq));
                v = (v * try!(10.value_into().map_err(|_| MalformedSeq))) + dig;
                default = false;
                bytes = {&bytes[1..]};
            },
            b';' => {
                let v = if default { None } else { Some(v) };
                return Ok((bytes, v))
            },
            _ => return Err(MalformedSeq)
        }
    }
    let v = if default { None } else { Some(v) };
    Ok((&bytes[0..0], v))
}

#[test]
fn test_parse_num() {
    fn bs(b: &[u8]) -> &[u8] { b }
    assert_eq!(parse_num::<i32>(b""), Ok((bs(b""), None)));
    assert_eq!(parse_num(b"0"),       Ok((bs(b""), Some(0))));
    assert_eq!(parse_num(b"0;"),      Ok((bs(b";"), Some(0))));
    assert_eq!(parse_num(b"0;1"),     Ok((bs(b";1"), Some(0))));
    assert_eq!(parse_num(b"1"),       Ok((bs(b""), Some(1))));
    assert_eq!(parse_num(b"1;2"),     Ok((bs(b";2"), Some(1))));
    assert_eq!(parse_num(b"12"),      Ok((bs(b""), Some(12))));
    assert_eq!(parse_num(b"12;3"),    Ok((bs(b";3"), Some(12))));

    assert_eq!(parse_num::<i32>(b"m"),    Err(MalformedSeq));
    assert_eq!(parse_num::<i32>(b"0m"),   Err(MalformedSeq));
}

fn is_escape_start(b: u8) -> bool {
    b == ESC
}

fn is_escape_end(b: u8) -> bool {
    0x40 <= b && b <= 0x7e
}
