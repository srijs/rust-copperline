use encoding::types::EncodingRef;

use error::Error;
use history::Cursor;
use buffer::Buffer;
use parser;
use instr;

pub struct EditCtx<'a> {
    pub buf: Buffer,
    pub seq: Vec<u8>,
    pub history_cursor: Cursor<'a>,
    pub enc: EncodingRef
}

pub enum EditResult {
    Continue,
    Clear,
    Halt,
    Err(Error)
}

pub fn edit<'a>(ctx: &mut EditCtx<'a>) -> EditResult {
    match parser::parse(&ctx.seq, ctx.enc) {
        parser::Result::Error(len) => {
            for _ in (0..len) {
                ctx.seq.remove(0);
            };
            EditResult::Continue
        },
        parser::Result::Incomplete => EditResult::Continue,
        parser::Result::Success(token, len) => {
            let r = match instr::interpret_token(token) {
                instr::Instr::Done => {
                    EditResult::Halt
                },
                instr::Instr::DeleteCharLeftOfCursor => {
                    ctx.buf.delete_char_left_of_cursor();
                    EditResult::Continue
                },
                instr::Instr::DeleteCharRightOfCursor => {
                    ctx.buf.delete_char_right_of_cursor();
                    EditResult::Continue
                },
                instr::Instr::DeleteCharRightOfCursorOrEOF => {
                    if !ctx.buf.delete_char_right_of_cursor() {
                        EditResult::Err(Error::EndOfFile)
                    } else {
                        EditResult::Continue
                    }
                },
                instr::Instr::MoveCursorLeft => {
                    ctx.buf.move_left();
                    EditResult::Continue
                },
                instr::Instr::MoveCursorRight => {
                    ctx.buf.move_right();
                    EditResult::Continue
                },
                instr::Instr::MoveCursorStart => {
                    ctx.buf.move_start();
                    EditResult::Continue
                },
                instr::Instr::MoveCursorEnd => {
                    ctx.buf.move_end();
                    EditResult::Continue
                },
                instr::Instr::HistoryPrev => {
                    if ctx.history_cursor.incr() {
                        ctx.buf.swap()
                    }
                    ctx.history_cursor.get().map(|s| ctx.buf.replace(s));
                    EditResult::Continue
                },
                instr::Instr::HistoryNext => {
                    if ctx.history_cursor.decr() {
                        ctx.buf.swap()
                    }
                    ctx.history_cursor.get().map(|s| ctx.buf.replace(s));
                    EditResult::Continue
                },
                instr::Instr::Noop => EditResult::Continue,
                instr::Instr::Cancel => EditResult::Err(Error::Cancel),
                instr::Instr::Clear => EditResult::Clear,
                instr::Instr::InsertAtCursor(text) => {
                    ctx.buf.insert_chars_at_cursor(text);
                    EditResult::Continue
                }
            };
            for _ in (0..len) {
                ctx.seq.remove(0);
            };
            r
        }
    }
}
