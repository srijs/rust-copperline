use std::os::unix::io::RawFd;

use std;
use libc;
use nix;
use nix::errno::Errno;
use nix::unistd::{read, write};
use nix::fcntl::{flock, FlockArg};
use nix::sys::termios;
use nix::sys::termios::{BRKINT, ICRNL, INPCK, ISTRIP, IXON, OPOST, CS8, ECHO, ICANON, IEXTEN, ISIG, VMIN, VTIME};

use error::Error;

pub struct RawMode {
    fd: RawFd,
    original_termios: termios::Termios
}

impl RawMode {

    fn acquire(fd: RawFd) -> Result<RawMode, nix::Error> {
        try!(flock(fd, FlockArg::LockExclusive));

        let original_termios = try!(termios::tcgetattr(fd));

        let mut raw = original_termios;
        raw.c_iflag = raw.c_iflag & !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        raw.c_oflag = raw.c_oflag & !(OPOST);
        raw.c_cflag = raw.c_cflag | (CS8);
        raw.c_lflag = raw.c_lflag & !(ECHO | ICANON | IEXTEN | ISIG);
        raw.c_cc[VMIN] = 1;
        raw.c_cc[VTIME] = 0;

        try!(termios::tcsetattr(fd, termios::TCSAFLUSH, &raw));

        Ok(RawMode{
            fd: fd,
            original_termios: original_termios
        })
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<usize, nix::Error> {
        write(self.fd, bytes)
    }

    pub fn clear(&mut self) -> Result<(), nix::Error> {
        self.write(b"\x1b[H\x1b[2J").map(|_| ())
    }

}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = termios::tcsetattr(self.fd, termios::TCSAFLUSH, &self.original_termios);
        let _ = flock(self.fd, FlockArg::Unlock);
    }
}

pub struct Term {
    in_fd: RawFd,
    out_fd: RawFd
}

static UNSUPPORTED_TERM: [&'static str; 3] = ["dumb","cons25","emacs"];

impl Term {

    pub fn new(in_fd: RawFd, out_fd: RawFd) -> Term {
        Term {
            in_fd: in_fd,
            out_fd: out_fd
        }
    }

    pub fn is_unsupported_term() -> bool {
        match std::env::var("TERM") {
            Ok(term) => UNSUPPORTED_TERM.iter().any(|x| &&term == x),
            Err(_) => false
        }
    }

    pub fn is_a_tty(&self) -> bool {
        unsafe { libc::isatty(self.out_fd) != 0 }
    }

    pub fn acquire_raw_mode(&self) -> Result<RawMode, Error> {
        if !self.is_a_tty() {
            return Err(Error::from(nix::Error::from_errno(Errno::ENOTTY)));
        }
        RawMode::acquire(self.out_fd).map_err(Error::from)
    }

    pub fn read_byte(&mut self) -> Result<Option<u8>, nix::Error> {
        let mut input: [u8; 1] = [0; 1];
        let n = try!(read(self.in_fd, &mut input));
        if n == 0 {
            return Ok(None);
        }
        Ok(Some(input[0]))
    }

}
