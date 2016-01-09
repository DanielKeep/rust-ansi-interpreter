/*!
This module is concerned with how to interpose an interpreter between Rust and the standard handles.

This is a *tremendous* pain in the ass.  Here's how it goes:
*/
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use self::mlw::*;

pub fn intercept_stdio() {
    try_intercept_stdio().unwrap()
}

fn try_intercept_stdio() -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;

    // Get the current stdout handle.
    let conout = try!(try!(get_std_handle(StdHandle::Output))
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "no stdout handle available for this process")));
    // let conerr = try!(try!(get_std_handle(StdHandle::Error))
    //     .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "no stderr handle available for this process")));

    // Create the pipe we'll use to capture stdout output.
    let (orp, owp) = try!(create_pipe());
    let (erp, ewp) = try!(create_pipe());

    let conout_hand = conout.as_raw_handle();

    let interp = super::ConsoleInterpreter::new(conout, conout_hand);
    let interc = ::AnsiIntercept::new(interp);
    let interc = Arc::new(Mutex::new(interc));

    // Spin up the interpreter threads.
    let _ = try!(thread::Builder::new()
        .name(String::from("ansi_interpreter.stdout"))
        .spawn({ let interc = interc.clone(); move || {
            let mut orp = orp;
            let mut buf = [0; 4096];

            loop {
                let bytes = match orp.read(&mut buf) {
                    Ok(0) => {
                        // *Probably* EOF.
                        return;
                    },

                    Ok(bytes) => bytes,

                    Err(err) => {
                        panic!("error while reading from stdout pipe: {}", err);
                    }
                };

                // Push those bytes through the interceptor.
                let mut buf = &buf[..bytes];
                while buf.len() > 0 {
                    match interc.lock().unwrap().write(buf) {
                        Ok(0) => {
                            // *Probably* cannot write any more.
                            return;
                        }

                        Ok(b) => {
                            buf = &buf[b..];
                        },

                        Err(err) => {
                            panic!("error while writing to stdout: {}", err);
                        }
                    }
                }
            }
        } }));

    let _ = try!(thread::Builder::new()
        .name(String::from("ansi_interpreter.stderr"))
        .spawn({ let interc = interc.clone(); move || {
            let mut erp = erp;
            let mut buf = [0; 4096];

            loop {
                let bytes = match erp.read(&mut buf) {
                    Ok(0) => {
                        // *Probably* EOF.
                        return;
                    },

                    Ok(bytes) => bytes,

                    Err(err) => {
                        panic!("error while reading from stderr pipe: {}", err);
                    }
                };

                // Push those bytes through the interceptor.
                let mut buf = &buf[..bytes];
                while buf.len() > 0 {
                    match interc.lock().unwrap().write(buf) {
                        Ok(0) => {
                            // *Probably* cannot write any more.
                            return;
                        }

                        Ok(b) => {
                            buf = &buf[b..];
                        },

                        Err(err) => {
                            panic!("error while writing to stderr: {}", err);
                        }
                    }
                }
            }
        } }));

    // Redirect the process handle.
    try!(set_std_handle(StdHandle::Output, owp));
    try!(set_std_handle(StdHandle::Error, ewp));

    Ok(())
}

mod mlw {
    extern crate kernel32;
    extern crate winapi;

    use std::fs::File;
    use std::io;
    use std::mem::zeroed;
    use std::os::windows::io::{FromRawHandle, IntoRawHandle};
    use std::ptr;
    use self::winapi::{DWORD, INVALID_HANDLE_VALUE};

    const DEFAULT_BUFFER_SIZE: DWORD = 0;

    #[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
    pub enum StdHandle {
        Input,
        Output,
        Error,
    }

    impl StdHandle {
        pub fn into_handle(self) -> DWORD {
            use self::StdHandle::*;
            match self {
                Input => winapi::STD_INPUT_HANDLE,
                Output => winapi::STD_OUTPUT_HANDLE,
                Error => winapi::STD_ERROR_HANDLE,
            }
        }
    }

    pub fn create_pipe() -> io::Result<(File, File)> {
        unsafe {
            let mut read_pipe = zeroed();
            let mut write_pipe = zeroed();
            if kernel32::CreatePipe(&mut read_pipe, &mut write_pipe, ptr::null_mut(), DEFAULT_BUFFER_SIZE) == 0 {
                return Err(io::Error::last_os_error());
            }
            let read_file = File::from_raw_handle(read_pipe);
            let write_file = File::from_raw_handle(write_pipe);
            Ok((read_file, write_file))
        }
    }

    pub fn get_std_handle(std_handle: StdHandle) -> io::Result<Option<File>> {
        unsafe {
            match kernel32::GetStdHandle(std_handle.into_handle()) {
                h if h == INVALID_HANDLE_VALUE => Err(io::Error::last_os_error()),
                h if h.is_null() => Ok(None),
                h => Ok(Some(File::from_raw_handle(h)))
            }
        }
    }

    pub fn set_std_handle<H>(std_handle: StdHandle, handle: H) -> io::Result<()>
    where H: IntoRawHandle {
        unsafe {
            match kernel32::SetStdHandle(std_handle.into_handle(), handle.into_raw_handle()) {
                0 => Err(io::Error::last_os_error()),
                _ => Ok(())
            }
        }
    }
}
