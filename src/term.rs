use std::os::unix::io::RawFd;

use std;
use libc;
use nix;
use nix::errno::Errno;
use nix::unistd::{read, write};
use nix::sys::termios;
use nix::sys::termios::{BRKINT, ICRNL, INPCK, ISTRIP, IXON, OPOST, CS8, ECHO, ICANON, IEXTEN, ISIG, VMIN, VTIME};

use error::Error;

pub struct Term {
    in_fd: RawFd,
    out_fd: RawFd,
    original_termios: Option<termios::Termios>
}

static UNSUPPORTED_TERM: [&'static str; 3] = ["dumb","cons25","emacs"];

impl Term {

    pub fn new(in_fd: RawFd, out_fd: RawFd) -> Term {
        Term {
            in_fd: in_fd,
            out_fd: out_fd,
            original_termios: None
        }
    }

    pub fn is_unsupported_term() -> bool {
        match std::env::var("TERM") {
            Ok(term) => {
                let mut unsupported = false;
                for iter in &UNSUPPORTED_TERM {
                    unsupported = term == *iter
                }
                unsupported
            }
            Err(_) => false
        }
    }

    pub fn is_a_tty(&self) -> bool {
        unsafe { libc::isatty(self.in_fd) != 0 && libc::isatty(self.out_fd) != 0 }
    }

    fn enable_raw_mode(&mut self) -> Result<(), Error> {

        if !self.is_a_tty() {
            return Err(Error::from(nix::Error::from_errno(Errno::ENOTTY)));
        }

        if self.original_termios.is_some() {
            return Ok(());
        }

        let original_termios = try!(termios::tcgetattr(self.out_fd));

        let mut raw = original_termios;
        raw.c_iflag = raw.c_iflag & !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        raw.c_oflag = raw.c_oflag & !(OPOST);
        raw.c_cflag = raw.c_cflag | (CS8);
        raw.c_lflag = raw.c_lflag & !(ECHO | ICANON | IEXTEN | ISIG);
        raw.c_cc[VMIN] = 1;
        raw.c_cc[VTIME] = 0;

        try!(termios::tcsetattr(self.out_fd, termios::TCSAFLUSH, &raw));

        self.original_termios = Some(original_termios);

        Ok(())

    }

    fn disable_raw_mode(&mut self) -> Result<(), Error> {
        let original_termios = self.original_termios.take();
        if original_termios.is_some() {
            try!(termios::tcsetattr(self.out_fd, termios::TCSAFLUSH, &original_termios.unwrap()));
        }
        Ok(())
    }

    pub fn with_raw_mode<A, F>(&mut self, f: F) -> Result<A, Error>
        where F: FnOnce(&mut Term) -> Result<A, Error> {
        try!(self.enable_raw_mode());
        let result = f(self);
        try!(self.disable_raw_mode());
        println!("");
        result
    }

    pub fn read_byte(&mut self) -> Result<Option<u8>, nix::Error> {
        let mut input: [u8; 1] = [0; 1];
        let n = try!(read(self.in_fd, &mut input));
        if n == 0 {
            return Ok(None);
        }
        Ok(Some(input[0]))
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<usize, nix::Error> {
        write(self.out_fd, bytes)
    }

}
