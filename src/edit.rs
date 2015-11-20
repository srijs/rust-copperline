use encoding::types::EncodingRef;

use error::Error;
use history::{Cursor, History};
use buffer::Buffer;
use parser::{parse, ParseError, ParseSuccess};
use instr;

#[derive(Copy,Clone)]
pub enum EditMode {
    Emacs,
    Vi,
}

#[derive(Copy, Clone, PartialEq)]
pub enum ViMode {
    Insert,
    Normal,
    Replace,
}

pub struct EditCtx<'a> {
    buf: Buffer,
    history_cursor: Cursor<'a>,
    prompt: &'a str,
    seq: Vec<u8>,
    enc: EncodingRef,
    mode: EditMode,
    vi_mode: ViMode,
    vi_count: u32,
}

impl<'a> EditCtx<'a> {

    pub fn new(prompt: &'a str, history: &'a History, enc: EncodingRef, mode: EditMode) -> Self {
        EditCtx {
            buf: Buffer::new(),
            history_cursor: Cursor::new(history),
            prompt: prompt,
            seq: Vec::new(),
            enc: enc,
            mode: mode,
            // always start in insert mode
            vi_mode: ViMode::Insert,
            vi_count: 0,
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

macro_rules! vi_repeat {
    ( $ctx:ident, $x:expr ) => {
        match $ctx.vi_count {
            0 => { $x; }
            _ => for _ in 0..$ctx.vi_count { $x; }
        }
        $ctx.vi_count = 0;
    };
    ( $ctx:ident, $operation:expr, $step:expr ) => {
        match $ctx.vi_count {
            0 => { $operation; }
            _ => {
                $operation;
                for _ in 1..$ctx.vi_count { $step; $operation; }
            }
        }
        $ctx.vi_count = 0;
    };
}

pub fn edit<'a>(ctx: &mut EditCtx<'a>) -> EditResult<Vec<u8>> {
    use self::EditResult::*;

    let res = match parse(&ctx.seq, ctx.enc) {
        Err(ParseError::Error(len)) => {
            for _ in (0..len) {
                ctx.seq.remove(0);
            };
            Cont(false)
        },
        Err(ParseError::Incomplete) => Cont(false),
        Ok(ParseSuccess(token, len)) => {
            let res = match instr::interpret_token(token, ctx.mode, ctx.vi_mode) {
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
                instr::Instr::NormalMode => {
                    if ctx.vi_mode == ViMode::Insert {
                        // cursor moves left when leaving insert mode
                        ctx.buf.move_left();
                    }
                    ctx.vi_mode = ViMode::Normal;
                    ctx.vi_count = 0;
                    Cont(false)
                }
                instr::Instr::ReplaceMode => {
                    ctx.vi_mode = ViMode::Replace;
                    Cont(false)
                }
                instr::Instr::Insert => {
                    ctx.vi_mode = ViMode::Insert;
                    Cont(false)
                }
                instr::Instr::InsertStart => {
                    ctx.vi_mode = ViMode::Insert;
                    ctx.buf.move_start();
                    Cont(false)
                }
                instr::Instr::Append => {
                    ctx.vi_mode = ViMode::Insert;
                    ctx.buf.move_right();
                    Cont(false)
                }
                instr::Instr::AppendEnd => {
                    ctx.vi_mode = ViMode::Insert;
                    ctx.buf.move_end();
                    Cont(false)
                }
                instr::Instr::Digit(i) => {
                    match (ctx.vi_count, i) {
                        // if count is 0, then 0 moves to the start of a line
                        (0, 0) => ctx.buf.move_start(),
                        // otherwise add a digit to the count
                        (_, i) => ctx.vi_count = ctx.vi_count * 10 + i,
                    }
                    Cont(false)
                }
                instr::Instr::MoveEndOfWordRight => {
                    vi_repeat!(ctx, ctx.buf.move_to_end_of_word());
                    Cont(false)
                }
                instr::Instr::Substitute => {
                    ctx.buf.delete_char_right_of_cursor();
                    ctx.vi_mode = ViMode::Insert;
                    Cont(false)
                }
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
                instr::Instr::ReplaceAtCursor(text) => {
                    vi_repeat!(
                        ctx,
                        ctx.buf.replace_chars_at_cursor(text.clone()),
                        ctx.buf.move_right()
                    );
                    ctx.vi_mode = ViMode::Normal;
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
