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
extern crate unicode_width;
extern crate encoding;
extern crate strcursor;

mod error;
mod buffer;
mod history;
mod parser;
mod instr;
mod term;

use std::os::unix::io::{RawFd, AsRawFd};

use encoding::types::Encoding;
use encoding::all::UTF_8;

pub use error::Error;
use history::History;
use buffer::Buffer;
use term::{Term, RawMode};

struct EditCtx<'a, E: 'a> {
    term: &'a mut Term,
    raw: &'a mut RawMode,
    history: &'a History,
    prompt: &'a str,
    enc: &'a E
}

fn edit<'a, E: Encoding>(ctx: EditCtx<'a, E>) -> Result<String, Error> {
    let mut buffer = Buffer::new();
    let mut seq: Vec<u8> = Vec::new();
    let mut history_cursor = history::Cursor::new(ctx.history);
    loop {
        try!(ctx.raw.write(&buffer.get_line(ctx.prompt)));
        let byte = try!(try!(ctx.term.read_byte()).ok_or(Error::EndOfFile));
        seq.push(byte);

        match parser::parse(&seq, ctx.enc) {
            parser::Result::Error => seq.clear(),
            parser::Result::Incomplete => (),
            parser::Result::Success(token, len) => {
                match instr::interpret_token(token) {
                    instr::Instr::Done                   => {
                        return Result::Ok(buffer.to_string());
                    },
                    instr::Instr::DeleteCharLeftOfCursor => {
                        buffer.delete_char_left_of_cursor();
                    },
                    instr::Instr::DeleteCharRightOfCursor => {
                        buffer.delete_char_right_of_cursor();
                    },
                    instr::Instr::DeleteCharRightOfCursorOrEOF => {
                        if !buffer.delete_char_right_of_cursor() {
                            return Err(Error::EndOfFile);
                        }
                    },
                    instr::Instr::MoveCursorLeft => {
                        buffer.move_left();
                    },
                    instr::Instr::MoveCursorRight => {
                        buffer.move_right();
                    },
                    instr::Instr::MoveCursorStart        => buffer.move_start(),
                    instr::Instr::MoveCursorEnd          => buffer.move_end(),
                    instr::Instr::HistoryPrev            => {
                        if history_cursor.incr() {
                            buffer.swap()
                        }
                        history_cursor.get().map(|s| buffer.replace(s));
                    },
                    instr::Instr::HistoryNext            => {
                        if history_cursor.decr() {
                            buffer.swap()
                        }
                        history_cursor.get().map(|s| buffer.replace(s));
                    },
                    instr::Instr::Noop                   => (),
                    instr::Instr::Cancel                 => return Err(Error::EndOfFile),
                    instr::Instr::Clear                  => try!(ctx.raw.clear()),
                    instr::Instr::InsertAtCursor(text)   => {
                        buffer.insert_chars_at_cursor(text)
                    }
                };
                for _ in (0..len) {
                    seq.remove(0);
                }
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
    pub fn read_line_with_enc<E: Encoding>(&mut self, prompt: &str, enc: &E) -> Result<String, Error> {
        if Term::is_unsupported_term() || !self.term.is_a_tty() {
            return Err(Error::UnsupportedTerm);
        }
        let result = self.term.acquire_raw_mode().and_then(|mut raw| {
            edit(EditCtx {
                term: &mut self.term,
                raw: &mut raw,
                history: &self.history,
                prompt: prompt,
                enc: enc
            })
        });
        println!("");
        result
    }

    /// Reads a utf8-encoded line from the input using the specified prompt.
    pub fn read_line(&mut self, prompt: &str) -> Result<String, Error> {
        self.read_line_with_enc(prompt, UTF_8)
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

    /// Clears the screen.
    pub fn clear_screen(&mut self) -> Result<(), Error> {
        let mut raw = try!(self.term.acquire_raw_mode());
        raw.clear().map_err(Error::ErrNo)
    }

}
