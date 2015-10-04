use encoding::types::EncodingRef;

use error::Error;
use history::{Cursor, History};
use buffer::Buffer;
use parser;
use instr;

pub struct EditCtx<'a> {
    buf: Buffer,
    history_cursor: Cursor<'a>,
    prompt: &'a str,
    seq: Vec<u8>,
    enc: EncodingRef
}

impl<'a> EditCtx<'a> {

    pub fn new(prompt: &'a str, history: &'a History, enc: EncodingRef) -> Self {
        EditCtx {
            buf: Buffer::new(),
            history_cursor: Cursor::new(history),
            prompt: prompt,
            seq: Vec::new(),
            enc: enc
        }
    }

    pub fn fill(&mut self, byte: u8) {
        self.seq.push(byte)
    }

}

pub enum EditResult<C> {
    Cont(C),
    Halt(Result<String, Error>)
}

pub fn edit<'a>(ctx: &mut EditCtx<'a>) -> EditResult<Vec<u8>> {
    use self::EditResult::*;

    let res = match parser::parse(&ctx.seq, ctx.enc) {
        parser::Result::Error(len) => {
            for _ in (0..len) {
                ctx.seq.remove(0);
            };
            Cont(false)
        },
        parser::Result::Incomplete => Cont(false),
        parser::Result::Success(token, len) => {
            let res = match instr::interpret_token(token) {
                instr::Instr::Done => {
                    Halt(Ok(ctx.buf.drain()))
                },
                instr::Instr::DeleteCharLeftOfCursor => {
                    ctx.buf.delete_char_left_of_cursor();
                    Cont(false)
                },
                instr::Instr::DeleteCharRightOfCursor => {
                    ctx.buf.delete_char_right_of_cursor();
                    Cont(false)
                },
                instr::Instr::DeleteCharRightOfCursorOrEOF => {
                    if !ctx.buf.delete_char_right_of_cursor() {
                        Halt(Err(Error::EndOfFile))
                    } else {
                        Cont(false)
                    }
                },
                instr::Instr::MoveCursorLeft => {
                    ctx.buf.move_left();
                    Cont(false)
                },
                instr::Instr::MoveCursorRight => {
                    ctx.buf.move_right();
                    Cont(false)
                },
                instr::Instr::MoveCursorStart => {
                    ctx.buf.move_start();
                    Cont(false)
                },
                instr::Instr::MoveCursorEnd => {
                    ctx.buf.move_end();
                    Cont(false)
                },
                instr::Instr::HistoryPrev => {
                    if ctx.history_cursor.incr() {
                        ctx.buf.swap()
                    }
                    ctx.history_cursor.get().map(|s| ctx.buf.replace(s));
                    Cont(false)
                },
                instr::Instr::HistoryNext => {
                    if ctx.history_cursor.decr() {
                        ctx.buf.swap()
                    }
                    ctx.history_cursor.get().map(|s| ctx.buf.replace(s));
                    Cont(false)
                },
                instr::Instr::Noop => {
                    Cont(false)
                },
                instr::Instr::Cancel => Halt(Err(Error::Cancel)),
                instr::Instr::Clear => {
                    Cont(true)
                },
                instr::Instr::InsertAtCursor(text) => {
                    ctx.buf.insert_chars_at_cursor(text);
                    Cont(false)
                }
            };
            for _ in (0..len) {
                ctx.seq.remove(0);
            };
            res
        }
    };
    match res {
        Cont(clear) => EditResult::Cont(ctx.buf.get_line(ctx.prompt, clear)),
        Halt(res) => EditResult::Halt(res)
    }
}
