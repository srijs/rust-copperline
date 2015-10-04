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

mod enc;
mod error;
mod builder;
mod buffer;
mod history;
mod parser;
mod instr;
mod edit;
mod term;

use std::os::unix::io::{RawFd, AsRawFd};

use encoding::types::EncodingRef;
use encoding::all::{ASCII, UTF_8};

pub use enc::Encoding;
pub use error::Error;
use history::History;
use buffer::Buffer;
use term::{Term, RawMode};
use edit::{EditCtx, EditResult, edit};

struct RunCtx<'a> {
    term: &'a mut Term,
    raw: &'a mut RawMode,
    history: &'a History,
    prompt: &'a str,
    enc: EncodingRef
}

fn run<'a>(ctx: RunCtx<'a>) -> Result<String, Error> {
    let mut edit_ctx = EditCtx {
        buf: Buffer::new(),
        seq: Vec::new(),
        history_cursor: history::Cursor::new(ctx.history),
        enc: ctx.enc
    };
    let mut clear = false;
    loop {
        try!(ctx.raw.write(&edit_ctx.buf.get_line(ctx.prompt, clear)));
        let byte = try!(try!(ctx.term.read_byte()).ok_or(Error::EndOfFile));
        edit_ctx.seq.push(byte);
        let res = edit(&mut edit_ctx);
        match res {
            EditResult::Continue => {
                clear = false;
            },
            EditResult::Clear => {
                clear = true;
            },
            EditResult::Halt => {
                return Ok(edit_ctx.buf.to_string());
            },
            EditResult::Err(err) => {
                return Err(err);
            }
        }
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
    fn read_line_with_enc(&mut self, prompt: &str, enc: EncodingRef) -> Result<String, Error> {
        if Term::is_unsupported_term() || !self.term.is_a_tty() {
            return Err(Error::UnsupportedTerm);
        }
        let result = self.term.acquire_raw_mode().and_then(|mut raw| {
            run(RunCtx {
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

    /// Reads a line from the input using the specified prompt and encoding.
    pub fn read_line(&mut self, prompt: &str, encoding: Encoding) -> Result<String, Error> {
        self.read_line_with_enc(prompt, enc::to_encoding_ref(encoding))
    }

    /// Reads a ASCII encoded line from the input using the specified prompt.
    pub fn read_line_ascii(&mut self, prompt: &str) -> Result<String, Error> {
        self.read_line_with_enc(prompt, ASCII)
    }

    /// Reads a UTF-8 encoded line from the input using the specified prompt.
    pub fn read_line_utf8(&mut self, prompt: &str) -> Result<String, Error> {
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
        let mut line = builder::Builder::new();
        line.clear_screen();
        raw.write(&line.build()).map(|_| ()).map_err(Error::ErrNo)
    }

}
