extern crate libc;
extern crate nix;

pub mod error;
mod buffer;
mod parser;
mod instr;
mod term;

use std::os::unix::io::RawFd;

use error::Error;
use buffer::Buffer;
use term::Term;

fn readline_edit(term: &mut Term, prompt: &str) -> Result<String, Error> {
    let mut buffer = Buffer::new();
    let mut seq: Vec<u8> = Vec::new();
    loop {
        try!(term.write(&buffer.get_line(prompt)));
        let byte = try!(try!(term.read_byte()).ok_or(Error::EndOfFile));
        seq.push(byte);
        match parser::parse(&seq) {
            parser::Result::Error => return Err(Error::InvalidUTF8),
            parser::Result::Incomplete => continue,
            parser::Result::Success(token) => {
                match instr::interpret_token(token) {
                    instr::Instr::Done                   => {
                        return Ok(buffer.to_string());
                    },
                    instr::Instr::DeleteCharLeftOfCursor => buffer.delete_char_left_of_cursor(),
                    instr::Instr::MoveCursorLeft         => buffer.move_left(),
                    instr::Instr::MoveCursorRight        => buffer.move_right(),
                    instr::Instr::MoveCursorStart        => buffer.move_start(),
                    instr::Instr::MoveCursorEnd          => buffer.move_end(),
                    instr::Instr::HistoryPrev            => (),
                    instr::Instr::HistoryNext            => (),
                    instr::Instr::Noop                   => (),
                    instr::Instr::Cancel                 => return Err(Error::EndOfFile),
                    instr::Instr::InsertStringAtCursor   => try!(buffer.insert_string_at_cursor(&seq))
                };
                seq.clear();
            }
        };
    }
}

fn readline_raw(term: &mut Term, prompt: &str) -> Result<String, Error> {
    if Term::is_unsupported_term() || !term.is_a_tty() {
        return Err(Error::UnsupportedTerm);
    }
    term.with_raw_mode(|term| {
        readline_edit(term, prompt)
    })
}


pub struct Copperline {
    term: Term
}

impl Copperline {

    pub fn new() -> Copperline {
        Copperline::from_raw_fd(libc::STDIN_FILENO, libc::STDOUT_FILENO)
    }

    fn from_raw_fd(ifd: RawFd, ofd: RawFd) -> Copperline {
        Copperline{term: Term::new(ifd, ofd)}
    }

    pub fn readline(&mut self, prompt: &str) -> Result<String, Error> {
        readline_raw(&mut self.term, prompt)
    }

}
