//! # A low-level terminal line editing library
//!
//! Copperline is a line editing library written from scratch in Rust,
//! born from the authors frustration with the interface of existing
//! bindings to C-libraries like readline, libedit or linenoise.
//!
//! Features that are currently supported:
//!
//! - Cursor movement and text insertion
//! - Jumps (via `C-a` and `C-e`)
//! - History browsing (via `C-p` and `C-n`)
//!
//! It has a clean, hackable codebase, which I hope will foster
//! contributions so that the Rust ecosystem will soon be able to utilise
//! a mature, pure-Rust line editing library.

extern crate libc;
extern crate nix;

mod error;
mod buffer;
mod history;
mod parser;
mod instr;
mod term;

use std::os::unix::io::{RawFd, AsRawFd};

pub use error::Error;
use history::History;
use buffer::Buffer;
use term::{Term, RawMode};

fn readline_edit(term: &mut Term, raw: &mut RawMode, history: &History, prompt: &str) -> Result<String, Error> {
    let mut buffer = Buffer::new();
    let mut seq: Vec<u8> = Vec::new();
    let mut history_cursor = history::Cursor::new(history);
    loop {
        try!(raw.write(&buffer.get_line(prompt)));
        let byte = try!(try!(term.read_byte()).ok_or(Error::EndOfFile));
        seq.push(byte);

        match parser::parse(&seq) {
            parser::Result::Error => return Err(Error::InvalidUTF8),
            parser::Result::Incomplete => continue,
            parser::Result::Success(token) => {
                match instr::interpret_token(token) {
                    instr::Instr::Done                   => {
                        return buffer.to_string();
                    },
                    instr::Instr::DeleteCharLeftOfCursor => buffer.delete_byte_left_of_cursor(),
                    instr::Instr::DeleteCharRightOfCursor => {
                        if !buffer.delete_byte_right_of_cursor() {
                            return Err(Error::EndOfFile);
                        }
                    },
                    instr::Instr::MoveCursorLeft         => buffer.move_left(),
                    instr::Instr::MoveCursorRight        => buffer.move_right(),
                    instr::Instr::MoveCursorStart        => buffer.move_start(),
                    instr::Instr::MoveCursorEnd          => buffer.move_end(),
                    instr::Instr::HistoryPrev            => {
                        if history_cursor.incr() {
                            buffer.swap()
                        }
                        history_cursor.get().map(|s| buffer.replace(s.as_bytes()));
                    },
                    instr::Instr::HistoryNext            => {
                        if history_cursor.decr() {
                            buffer.swap()
                        }
                        history_cursor.get().map(|s| buffer.replace(s.as_bytes()));
                    },
                    instr::Instr::Noop                   => (),
                    instr::Instr::Cancel                 => return Err(Error::EndOfFile),
                    instr::Instr::Clear                  => {
                        try!(raw.write(b"\x1b[H\x1b[2J"));
                    },
                    instr::Instr::InsertAtCursor         => buffer.insert_bytes_at_cursor(&seq)
                };
                seq.clear();
            }
        };
    }
}

pub struct Copperline {
    term: Term,
    history: History
}

impl Copperline {

    /// Constructs a new Copperline from stdin to stdout.
    pub fn new() -> Copperline {
        Copperline::new_from_raw_fds(libc::STDIN_FILENO, libc::STDOUT_FILENO)
    }

    /// Constructs a new Copperline from the specified resources.
    pub fn new_from_io<I: AsRawFd, O: AsRawFd>(i: &I, o: &O) -> Copperline {
        Copperline::new_from_raw_fds(i.as_raw_fd(), o.as_raw_fd())
    }

    /// Constructs a new Copperline from the specified file descriptors.
    pub fn new_from_raw_fds(ifd: RawFd, ofd: RawFd) -> Copperline {
        Copperline {
            term: Term::new(ifd, ofd),
            history: History::new()
        }
    }

    /// Reads a line from the input using the specified prompt.
    pub fn read_line(&mut self, prompt: &str) -> Result<String, Error> {
        if Term::is_unsupported_term() || !self.term.is_a_tty() {
            return Err(Error::UnsupportedTerm);
        }
        let result = self.term.acquire_raw_mode().and_then(|mut raw| {
            readline_edit(&mut self.term, &mut raw, &self.history, prompt)
        });
        println!("");
        result
    }

    /// Returns the current length of the history.
    pub fn get_current_history_length(&self) -> usize {
        self.history.len()
    }

    /// Adds a line to the history.
    pub fn add_history(&mut self, line: String) {
        self.history.push(line)
    }

    /// Retrieves a line from the history by index.
    pub fn get_history_item(&self, idx: usize) -> Option<&String> {
        self.history.get(idx)
    }

    /// Removes an item from the history by index and returns it.
    pub fn remove_history_item(&mut self, idx: usize) -> Option<String> {
        self.history.remove(idx)
    }

    /// Clears the current history.
    pub fn clear_history(&mut self) {
        self.history.clear()
    }


    /// Clears the screen
    pub fn clear_screen(&mut self) -> Result<usize, Error> {
        self.term.acquire_raw_mode().and_then(|mut raw| raw.write(b"\x1b[H\x1b[2J").map_err(|e| Error::ErrNo(e)))
    }

}
