use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use smallvec::{Array, SmallVec};

// FIXME: This sucks.
pub fn drop_front<A, T>(sv: &mut SmallVec<A>, n: usize)
where
    A: Array<Item=T>,
    T: Clone,
{
    assert!(n <= sv.len());

    let tmp = sv.iter().skip(n).cloned().collect();
    ::std::mem::replace(sv, tmp);
}

pub struct SharedWrite<W>(Arc<Mutex<W>>) where W: 'static + Write;

impl<W> SharedWrite<W> where W: 'static + Write {
    pub fn new(w: W) -> Self {
        SharedWrite(Arc::new(Mutex::new(w)))
    }
}

impl<W> Clone for SharedWrite<W> where W: 'static + Write {
    fn clone(&self) -> Self {
        SharedWrite(self.0.clone())
    }
}

impl<W> Write for SharedWrite<W> where W: 'static + Write {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.lock().unwrap().flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.0.lock().unwrap().write_all(buf)
    }

    fn write_fmt(&mut self, fmt: ::std::fmt::Arguments) -> io::Result<()> {
        self.0.lock().unwrap().write_fmt(fmt)
    }
}
