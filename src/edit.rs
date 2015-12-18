use encoding::types::EncodingRef;

use std::u32;
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
    MoveChar(instr::CharMoveType),
    DeleteMoveChar(instr::CharMoveType),
    ChangeMoveChar(instr::CharMoveType),
    Delete,
    Change,
}

/// Set a new vi mode based on the current vi mode.
///
/// There are several common mode transitions that we handle here.
macro_rules! set_next_vi_mode {
    ( $ctx:ident ) => {
        $ctx.vi_mode = match $ctx.vi_mode {
            ViMode::Delete => ViMode::Normal,
            ViMode::Change => ViMode::Insert,
            ViMode::DeleteMoveChar(_) => ViMode::Normal,
            ViMode::ChangeMoveChar(_) => ViMode::Insert,
            _ => ViMode::Normal,
        }
    }
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

    /// Ignore one past the end of the line in vi normal mode.
    fn exclude_eol(&mut self) {
        if self.vi_mode == ViMode::Normal {
            self.buf.exclude_eol();
        }
    }

    fn set_next_vi_mode(&mut self) {
        set_next_vi_mode!(self);
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
            _ => for _ in 0..$ctx.vi_count {
                if !$x {
                    break;
                }
            },
        }
        $ctx.vi_count = 0;
    };
    ( $ctx:ident, $operation:expr, $step:expr ) => {
        match $ctx.vi_count {
            0 => { $operation; }
            _ => {
                if $operation {
                    for _ in 1..$ctx.vi_count {
                        if !$step {
                            break;
                        }
                        if !$operation {
                            break;
                        }
                    }
                }
            }
        }
        $ctx.vi_count = 0;
    };
}

macro_rules! vi_delete {
    ( $ctx:ident with $dc:ident $x:expr ) => {
        vi_repeat!($ctx, $x);
        match $ctx.vi_mode {
            ViMode::Delete | ViMode::Change => {
                $dc.delete();
            }
            _ => {}
        }
        set_next_vi_mode!($ctx);
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
                instr::Instr::DoneOrEof => {
                    if ctx.buf.is_empty() {
                        Halt(Err(Error::EndOfFile))
                    }
                    else {
                        Halt(Ok(ctx.buf.drain()))
                    }
                }
                instr::Instr::DeleteCharLeftOfCursor => {
                    vi_repeat!(ctx, ctx.buf.delete_char_left_of_cursor());
                    Cont(false)
                },
                instr::Instr::DeleteCharRightOfCursor => {
                    vi_repeat!(ctx, ctx.buf.delete_char_right_of_cursor());
                    ctx.exclude_eol();
                    Cont(false)
                },
                instr::Instr::DeleteCharRightOfCursorOrEOF => {
                    if !ctx.buf.delete_char_right_of_cursor() {
                        Halt(Err(Error::EndOfFile))
                    } else {
                        Cont(false)
                    }
                },
                instr::Instr::DeleteLine => {
                    ctx.buf.drain();
                    ctx.vi_mode = ViMode::Normal;
                    ctx.vi_count = 0;
                    Cont(false)
                }
                instr::Instr::DeleteToEnd => {
                    ctx.vi_count = 0;
                    {
                        ctx.vi_mode = ViMode::Delete;
                        let mut dc = ctx.buf.start_delete();
                        vi_delete!(ctx with dc { dc.move_end(); false });
                    }
                    ctx.exclude_eol();
                    Cont(false)
                }
                instr::Instr::ChangeLine => {
                    ctx.buf.drain();
                    ctx.vi_mode = ViMode::Insert;
                    ctx.vi_count = 0;
                    Cont(false)
                }
                instr::Instr::ChangeToEnd => {
                    ctx.vi_count = 0;
                    {
                        ctx.vi_mode = ViMode::Change;
                        let mut dc = ctx.buf.start_delete();
                        vi_delete!(ctx with dc { dc.move_end(); false });
                    }
                    ctx.exclude_eol();
                    Cont(false)
                }
                instr::Instr::MoveCursorLeft => {
                    let mut dc = ctx.buf.start_delete();
                    vi_delete!(ctx with dc { dc.move_left() });
                    Cont(false)
                },
                instr::Instr::MoveCursorRight => {
                    {
                        let mut dc = ctx.buf.start_delete();
                        vi_delete!(ctx with dc { dc.move_right() });
                    }
                    ctx.exclude_eol();
                    Cont(false)
                },
                instr::Instr::MoveCursorStart => {
                    ctx.vi_count = 0;
                    let mut dc = ctx.buf.start_delete();
                    dc.move_start();
                    if ctx.vi_mode == ViMode::Delete || ctx.vi_mode == ViMode::Change {
                        dc.delete();
                        set_next_vi_mode!(ctx);
                    }
                    Cont(false)
                },
                instr::Instr::MoveCursorEnd => {
                    ctx.vi_count = 0;
                    {
                        let mut dc = ctx.buf.start_delete();
                        vi_delete!(ctx with dc { dc.move_end(); false });
                    }
                    ctx.exclude_eol();
                    Cont(false)
                },
                instr::Instr::HistoryPrev => {
                    vi_repeat!(ctx, {
                        let end = ctx.history_cursor.incr();
                        if end {
                            ctx.buf.swap()
                        }
                        ctx.history_cursor.get().map(|s| ctx.buf.replace(s));
                        end
                    });
                    Cont(false)
                },
                instr::Instr::HistoryNext => {
                    vi_repeat!(ctx, {
                        let end = ctx.history_cursor.decr();
                        if end {
                            ctx.buf.swap()
                        }
                        ctx.history_cursor.get().map(|s| ctx.buf.replace(s));
                        end
                    });
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
                instr::Instr::MoveCharMode(mode) => {
                    ctx.vi_mode = match ctx.vi_mode {
                        ViMode::Delete => ViMode::DeleteMoveChar(mode),
                        ViMode::Change => ViMode::ChangeMoveChar(mode),
                        _              => ViMode::MoveChar(mode),
                    };
                    Cont(false)
                }
                instr::Instr::DeleteMode => {
                    ctx.vi_mode = ViMode::Delete;
                    Cont(false)
                }
                instr::Instr::ChangeMode => {
                    ctx.vi_mode = ViMode::Change;
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
                        (0, 0) => {
                            let mut dc = ctx.buf.start_delete();
                            vi_delete!(ctx with dc { dc.move_start(); false });
                        }
                        // otherwise add a digit to the count
                        (_, i) => {
                            if ctx.vi_count <= (u32::MAX - i) / 10 {
                                ctx.vi_count = ctx.vi_count * 10 + i
                            }
                        }
                    }
                    Cont(false)
                }
                instr::Instr::MoveEndOfWordRight => {
                    {
                        let mut dc = ctx.buf.start_delete();
                        vi_repeat!(ctx, dc.move_to_end_of_word());
                        if ctx.vi_mode == ViMode::Delete || ctx.vi_mode == ViMode::Change {
                            dc.move_right(); // vi deletes an extra character
                            dc.delete();
                            set_next_vi_mode!(ctx);
                        }
                    }
                    ctx.exclude_eol();
                    Cont(false)
                }
                instr::Instr::MoveEndOfWordWsRight => {
                    {
                        let mut dc = ctx.buf.start_delete();
                        vi_repeat!(ctx, dc.move_to_end_of_word_ws());
                        if ctx.vi_mode == ViMode::Delete || ctx.vi_mode == ViMode::Change {
                            dc.move_right(); // vi deletes an extra character
                            dc.delete();
                            set_next_vi_mode!(ctx);
                        }
                    }
                    ctx.exclude_eol();
                    Cont(false)
                }
                instr::Instr::MoveWordRight => {
                    {
                        let mut dc = ctx.buf.start_delete();
                        vi_repeat!(ctx, dc.move_word());
                        match ctx.vi_mode {
                            ViMode::Delete => dc.delete(),
                            ViMode::Change => {
                                // move word right has special behavior in change mode
                                if !dc.started_on_whitespace() && dc.move_right() {
                                    dc.move_to_end_of_word_back();
                                    dc.move_right();
                                }
                                dc.delete();
                            }
                            _ => {}
                        }
                        set_next_vi_mode!(ctx);
                    }
                    ctx.exclude_eol();
                    Cont(false)
                }
                instr::Instr::MoveWordWsRight => {
                    {
                        let mut dc = ctx.buf.start_delete();
                        vi_repeat!(ctx, dc.move_word_ws());
                        match ctx.vi_mode {
                            ViMode::Delete => dc.delete(),
                            ViMode::Change => {
                                // move word right has special behavior in change mode
                                if !dc.started_on_whitespace() && dc.move_right() {
                                    dc.move_to_end_of_word_ws_back();
                                    dc.move_right();
                                }
                                dc.delete();
                            }
                            _ => {}
                        }
                        set_next_vi_mode!(ctx);
                    }
                    ctx.exclude_eol();
                    Cont(false)
                }
                instr::Instr::MoveWordLeft => {
                    let mut dc = ctx.buf.start_delete();
                    vi_delete!(ctx with dc { dc.move_word_back() });
                    Cont(false)
                }
                instr::Instr::MoveWordWsLeft => {
                    let mut dc = ctx.buf.start_delete();
                    vi_delete!(ctx with dc { dc.move_word_ws_back() });
                    Cont(false)
                }
                instr::Instr::MoveCharRight(c) => {
                    {
                        let mut dc = ctx.buf.start_delete();
                        dc.move_to_char_right(c, match ctx.vi_count {
                            0 => 1,
                            n => n,
                        });
                        match ctx.vi_mode {
                            ViMode::DeleteMoveChar(_) | ViMode::ChangeMoveChar(_) => {
                                dc.move_right(); // make deletion inclusive
                                dc.delete();
                            }
                            _ => {},
                        }
                    }
                    ctx.vi_count = 0;
                    ctx.set_next_vi_mode();
                    ctx.exclude_eol();
                    Cont(false)
                }
                instr::Instr::MoveCharLeft(c) => {
                    let mut dc = ctx.buf.start_delete();
                    dc.move_to_char_left(c, match ctx.vi_count {
                        0 => 1,
                        n => n,
                    });
                    match ctx.vi_mode {
                        ViMode::DeleteMoveChar(_) | ViMode::ChangeMoveChar(_) => dc.delete(),
                        _ => {},
                    }
                    ctx.vi_count = 0;
                    set_next_vi_mode!(ctx);
                    Cont(false)
                }
                instr::Instr::MoveBeforeCharRight(c) => {
                    let count = match ctx.vi_count {
                        0 => 1,
                        n => n,
                    };

                    let mut dc = ctx.buf.start_delete();
                    if dc.move_to_char_right(c, count) {
                        dc.move_left();
                        match ctx.vi_mode {
                            ViMode::DeleteMoveChar(_) | ViMode::ChangeMoveChar(_) => {
                                dc.move_right(); // make deletion inclusive
                                dc.delete();
                            }
                            _ => {},
                        }
                    }
                    ctx.vi_count = 0;
                    set_next_vi_mode!(ctx);
                    Cont(false)
                }
                instr::Instr::MoveBeforeCharLeft(c) => {
                    let count = match ctx.vi_count {
                        0 => 1,
                        n => n,
                    };

                    let mut dc = ctx.buf.start_delete();
                    if dc.move_to_char_left(c, count) {
                        dc.move_right();
                        match ctx.vi_mode {
                            ViMode::DeleteMoveChar(_) | ViMode::ChangeMoveChar(_) => dc.delete(),
                            _ => {},
                        }
                    }
                    ctx.vi_count = 0;
                    set_next_vi_mode!(ctx);
                    Cont(false)
                }
                instr::Instr::Substitute => {
                    vi_repeat!(ctx, ctx.buf.delete_char_right_of_cursor());
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
                        {
                            ctx.buf.replace_chars_at_cursor(text.clone());
                            true
                        },
                        {
                            ctx.buf.move_right();
                            ctx.buf.exclude_eol()
                        }
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
