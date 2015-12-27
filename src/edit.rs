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

#[derive(Copy, Clone)]
pub enum ModeState {
    Emacs,
    Vi(ViMode, u32),
}

impl ModeState {
    pub fn new(mode: EditMode) -> Self {
        match mode {
            EditMode::Emacs => ModeState::Emacs,
            // vi mode should start in insert mode
            EditMode::Vi => ModeState::Vi(ViMode::Insert, 0),
        }
    }

    fn with_vi_mode(&self, vi_mode: ViMode) -> Self {
        if let ModeState::Vi(_, count) = *self {
            ModeState::Vi(vi_mode, count)
        }
        else {
            *self
        }
    }

    fn with_vi_count(&self, count: u32) -> Self {
        if let ModeState::Vi(mode, _) = *self {
            ModeState::Vi(mode, count)
        }
        else {
            *self
        }
    }

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
fn next_vi_mode(mode_state: ModeState) -> ModeState {
    match mode_state {
        ModeState::Vi(ViMode::Delete, _) => ModeState::Vi(ViMode::Normal, 0),
        ModeState::Vi(ViMode::Change, _) => ModeState::Vi(ViMode::Insert, 0),
        ModeState::Vi(ViMode::DeleteMoveChar(_), _) => ModeState::Vi(ViMode::Normal, 0),
        ModeState::Vi(ViMode::ChangeMoveChar(_), _) => ModeState::Vi(ViMode::Insert, 0),
        ModeState::Vi(_, _) => ModeState::Vi(ViMode::Normal, 0),
        // emacs mode is always emacs mode
        ModeState::Emacs => ModeState::Emacs,
    }
}

pub struct EditCtx<'a> {
    buf: Buffer,
    history_cursor: Cursor<'a>,
    prompt: &'a str,
    seq: Vec<u8>,
    enc: EncodingRef,
    mode_state: ModeState,
}

impl<'a> EditCtx<'a> {

    pub fn new(prompt: &'a str, history: &'a History, enc: EncodingRef, mode: EditMode) -> Self {
        EditCtx {
            buf: Buffer::new(),
            history_cursor: Cursor::new(history),
            prompt: prompt,
            seq: Vec::new(),
            enc: enc,
            mode_state: ModeState::new(mode),
        }
    }

    pub fn fill<I>(&mut self, it: I) where I: IntoIterator<Item=u8> {
        self.seq.extend(it)
    }

    /// Ignore one past the end of the line in vi normal mode.
    fn exclude_eol(&mut self) {
        if let ModeState::Vi(ViMode::Normal, _) = self.mode_state {
            self.buf.exclude_eol();
        }
    }
}

pub enum EditResult<C> {
    Cont(C),
    Halt(Result<String, Error>)
}

macro_rules! vi_repeat {
    ( $ctx:ident, $x:expr ) => {
        match $ctx.mode_state {
            ModeState::Emacs => { $x; }
            ModeState::Vi(mode, count) => {
                match count {
                    0 => { $x; }
                    _ => for _ in 0..count {
                        if !$x {
                            break;
                        }
                    },
                }
                $ctx.mode_state = ModeState::Vi(mode, 0);
            }
        }
    };
}

macro_rules! vi_delete {
    ( $ctx:ident with $dc:ident $x:expr ) => {
        vi_repeat!($ctx, $x);
        match $ctx.mode_state {
            ModeState::Vi(ViMode::Delete, _)
            | ModeState::Vi(ViMode::Change, _) => {
                $dc.delete();
            }
            _ => {}
        }
        $ctx.mode_state = next_vi_mode($ctx.mode_state);
    };
}

fn handle_common<'a>(ctx: &mut EditCtx<'a>, cinstr: instr::CommonInstr) -> EditResult<bool> {
    match cinstr {
        instr::CommonInstr::Done => EditResult::Halt(Ok(ctx.buf.drain())),
        instr::CommonInstr::Noop => EditResult::Cont(false),
        instr::CommonInstr::Cancel => EditResult::Halt(Err(Error::Cancel)),
        instr::CommonInstr::Clear => EditResult::Cont(true)
    }
}

fn handle_history<'a>(ctx: &mut EditCtx<'a>, hinstr: instr::HistoryInstr) -> EditResult<bool> {
    match hinstr {
        instr::HistoryInstr::Prev => {
            vi_repeat!(ctx, {
                let end = ctx.history_cursor.incr();
                if end {
                    ctx.buf.swap()
                }
                ctx.history_cursor.get().map(|s| ctx.buf.replace(s));
                end
            });
            EditResult::Cont(false)
        }
        instr::HistoryInstr::Next => {
            vi_repeat!(ctx, {
                let end = ctx.history_cursor.decr();
                if end {
                    ctx.buf.swap()
                }
                ctx.history_cursor.get().map(|s| ctx.buf.replace(s));
                end
            });
            EditResult::Cont(false)
        }
    }
}

fn handle_move_cursor<'a>(ctx: &mut EditCtx<'a>, mcinstr: instr::MoveCursorInstr) -> EditResult<bool> {
    match mcinstr {
        instr::MoveCursorInstr::Left => {
            let mut dc = ctx.buf.start_delete();
            vi_delete!(ctx with dc { dc.move_left() });
            EditResult::Cont(false)
        },
        instr::MoveCursorInstr::Right => {
            {
                let mut dc = ctx.buf.start_delete();
                vi_delete!(ctx with dc { dc.move_right() });
            }
            ctx.exclude_eol();
            EditResult::Cont(false)
        },
        instr::MoveCursorInstr::Start => {
            let mut dc = ctx.buf.start_delete();
            dc.move_start();
            match ctx.mode_state {
                ModeState::Vi(ViMode::Delete, _)
                | ModeState::Vi(ViMode::Change, _) => {
                    dc.delete();
                }
                _ => {}
            }
            ctx.mode_state = next_vi_mode(ctx.mode_state);
            EditResult::Cont(false)
        },
        instr::MoveCursorInstr::End => {
            {
                let mut dc = ctx.buf.start_delete();
                vi_delete!(ctx with dc { dc.move_end(); false });
            }
            ctx.exclude_eol();
            EditResult::Cont(false)
        }
    }
}

fn handle<'a>(ctx: &mut EditCtx<'a>, ins: instr::Instr) -> EditResult<bool> {
    use self::EditResult::*;

    match ins {
        instr::Instr::Common(cinstr) => handle_common(ctx, cinstr),
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
            ctx.mode_state = ModeState::Vi(ViMode::Normal, 0);
            Cont(false)
        }
        instr::Instr::DeleteToEnd => {
            {
                ctx.mode_state = ModeState::Vi(ViMode::Delete, 0);
                let mut dc = ctx.buf.start_delete();
                vi_delete!(ctx with dc { dc.move_end(); false });
            }
            ctx.exclude_eol();
            Cont(false)
        }
        instr::Instr::ChangeLine => {
            ctx.buf.drain();
            ctx.mode_state = ModeState::Vi(ViMode::Insert, 0);
            Cont(false)
        }
        instr::Instr::ChangeToEnd => {
            {
                ctx.mode_state = ModeState::Vi(ViMode::Change, 0);
                let mut dc = ctx.buf.start_delete();
                vi_delete!(ctx with dc { dc.move_end(); false });
            }
            ctx.exclude_eol();
            Cont(false)
        }
        instr::Instr::MoveCursor(mcinstr) => handle_move_cursor(ctx, mcinstr),
        instr::Instr::History(hinstr) => handle_history(ctx, hinstr),
        instr::Instr::NormalMode => {
            if let ModeState::Vi(ViMode::Insert, _) = ctx.mode_state {
                // cursor moves left when leaving insert mode
                ctx.buf.move_left();
            }
            ctx.mode_state = ModeState::Vi(ViMode::Normal, 0);
            Cont(false)
        }
        instr::Instr::ReplaceMode => {
            ctx.mode_state = ctx.mode_state.with_vi_mode(ViMode::Replace);
            Cont(false)
        }
        instr::Instr::MoveCharMode(mode) => {
            if let ModeState::Vi(vi_mode, _) = ctx.mode_state {
                let vi_mode = match vi_mode {
                    ViMode::Delete => ViMode::DeleteMoveChar(mode),
                    ViMode::Change => ViMode::ChangeMoveChar(mode),
                    _              => ViMode::MoveChar(mode),
                };
                ctx.mode_state = ctx.mode_state.with_vi_mode(vi_mode);
            }
            Cont(false)
        }
        instr::Instr::DeleteMode => {
            ctx.mode_state = ctx.mode_state.with_vi_mode(ViMode::Delete);
            Cont(false)
        }
        instr::Instr::ChangeMode => {
            ctx.mode_state = ctx.mode_state.with_vi_mode(ViMode::Change);
            Cont(false)
        }
        instr::Instr::Insert => {
            ctx.mode_state = ctx.mode_state.with_vi_mode(ViMode::Insert);
            Cont(false)
        }
        instr::Instr::InsertStart => {
            ctx.mode_state = ctx.mode_state.with_vi_mode(ViMode::Insert);
            ctx.buf.move_start();
            Cont(false)
        }
        instr::Instr::Append => {
            ctx.mode_state = ctx.mode_state.with_vi_mode(ViMode::Insert);
            ctx.buf.move_right();
            Cont(false)
        }
        instr::Instr::AppendEnd => {
            ctx.mode_state = ctx.mode_state.with_vi_mode(ViMode::Insert);
            ctx.buf.move_end();
            Cont(false)
        }
        instr::Instr::Digit(i) => {
            match (ctx.mode_state, i) {
                // if count is 0, then 0 moves to the start of a line
                (ModeState::Vi(_, 0), 0) => {
                    let mut dc = ctx.buf.start_delete();
                    vi_delete!(ctx with dc { dc.move_start(); false });
                }
                // otherwise add a digit to the count
                (ModeState::Vi(_, count), i) => {
                    if count <= (u32::MAX - i) / 10 {
                        ctx.mode_state = ctx.mode_state.with_vi_count(count * 10 + i);
                    }
                }
                (ModeState::Emacs, _) => {} // unreachable!()?
            }
            Cont(false)
        }
        instr::Instr::MoveEndOfWordRight => {
            {
                let mut dc = ctx.buf.start_delete();
                vi_repeat!(ctx, dc.move_to_end_of_word());
                match ctx.mode_state {
                    ModeState::Vi(ViMode::Delete, _)
                    | ModeState::Vi(ViMode::Change, _) => {
                        dc.move_right(); // vi deletes an extra character
                        dc.delete();
                        ctx.mode_state = next_vi_mode(ctx.mode_state);
                    }
                    _ => {}
                }
            }
            ctx.exclude_eol();
            Cont(false)
        }
        instr::Instr::MoveEndOfWordWsRight => {
            {
                let mut dc = ctx.buf.start_delete();
                vi_repeat!(ctx, dc.move_to_end_of_word_ws());
                match ctx.mode_state {
                    ModeState::Vi(ViMode::Delete, _)
                    | ModeState::Vi(ViMode::Change, _) => {
                        dc.move_right(); // vi deletes an extra character
                        dc.delete();
                        ctx.mode_state = next_vi_mode(ctx.mode_state);
                    }
                    _ => {}
                }
            }
            ctx.exclude_eol();
            Cont(false)
        }
        instr::Instr::MoveWordRight => {
            {
                let mut dc = ctx.buf.start_delete();
                vi_repeat!(ctx, dc.move_word());
                match ctx.mode_state {
                    ModeState::Vi(ViMode::Delete, _) => dc.delete(),
                    ModeState::Vi(ViMode::Change, _) => {
                        // move word right has special behavior in change mode
                        if !dc.started_on_whitespace() && dc.move_right() {
                            dc.move_to_end_of_word_back();
                            dc.move_right();
                        }
                        dc.delete();
                    }
                    _ => {}
                }
                ctx.mode_state = next_vi_mode(ctx.mode_state);
            }
            ctx.exclude_eol();
            Cont(false)
        }
        instr::Instr::MoveWordWsRight => {
            {
                let mut dc = ctx.buf.start_delete();
                vi_repeat!(ctx, dc.move_word_ws());
                match ctx.mode_state {
                    ModeState::Vi(ViMode::Delete, _) => dc.delete(),
                    ModeState::Vi(ViMode::Change, _) => {
                        // move word right has special behavior in change mode
                        if !dc.started_on_whitespace() && dc.move_right() {
                            dc.move_to_end_of_word_ws_back();
                            dc.move_right();
                        }
                        dc.delete();
                    }
                    _ => {}
                }
                ctx.mode_state = next_vi_mode(ctx.mode_state);
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
                if let ModeState::Vi(mode, count) = ctx.mode_state {
                    dc.move_to_char_right(c, match count {
                        0 => 1,
                        n => n,
                    });
                    match mode {
                        ViMode::DeleteMoveChar(_) | ViMode::ChangeMoveChar(_) => {
                            dc.move_right(); // make deletion inclusive
                            dc.delete();
                        }
                        _ => {},
                    }
                }
            }
            ctx.mode_state = next_vi_mode(ctx.mode_state);
            ctx.exclude_eol();
            Cont(false)
        }
        instr::Instr::MoveCharLeft(c) => {
            let mut dc = ctx.buf.start_delete();
            if let ModeState::Vi(mode, count) = ctx.mode_state {
                dc.move_to_char_left(c, match count {
                    0 => 1,
                    n => n,
                });
                match mode {
                    ViMode::DeleteMoveChar(_) | ViMode::ChangeMoveChar(_) => dc.delete(),
                    _ => {},
                }
            }
            ctx.mode_state = next_vi_mode(ctx.mode_state);
            Cont(false)
        }
        instr::Instr::MoveBeforeCharRight(c) => {
            if let ModeState::Vi(mode, count) = ctx.mode_state {
                let count = match count {
                    0 => 1,
                    n => n,
                };

                let mut dc = ctx.buf.start_delete();
                if dc.move_to_char_right(c, count) {
                    dc.move_left();
                    match mode {
                        ViMode::DeleteMoveChar(_) | ViMode::ChangeMoveChar(_) => {
                            dc.move_right(); // make deletion inclusive
                            dc.delete();
                        }
                        _ => {},
                    }
                }
            }
            ctx.mode_state = next_vi_mode(ctx.mode_state);
            Cont(false)
        }
        instr::Instr::MoveBeforeCharLeft(c) => {
            if let ModeState::Vi(mode, count) = ctx.mode_state {
                let count = match count {
                    0 => 1,
                    n => n,
                };

                let mut dc = ctx.buf.start_delete();
                if dc.move_to_char_left(c, count) {
                    dc.move_right();
                    match mode {
                        ViMode::DeleteMoveChar(_) | ViMode::ChangeMoveChar(_) => dc.delete(),
                        _ => {},
                    }
                }
            }
            ctx.mode_state = next_vi_mode(ctx.mode_state);
            Cont(false)
        }
        instr::Instr::Substitute => {
            vi_repeat!(ctx, ctx.buf.delete_char_right_of_cursor());
            ctx.mode_state = ctx.mode_state.with_vi_mode(ViMode::Insert);
            Cont(false)
        }
        instr::Instr::InsertAtCursor(text) => {
            ctx.buf.insert_chars_at_cursor(text);
            Cont(false)
        }
        instr::Instr::ReplaceAtCursor(text) => {
            vi_repeat!(ctx, {
                ctx.buf.replace_chars_at_cursor(text.clone());
                ctx.buf.move_right();
                ctx.buf.exclude_eol()
            });
            ctx.mode_state = ctx.mode_state.with_vi_mode(ViMode::Normal);
            Cont(false)
        }
    }
}

pub fn edit<'a>(ctx: &mut EditCtx<'a>) -> EditResult<Vec<u8>> {
    let res = match parse(&ctx.seq, ctx.enc) {
        Err(ParseError::Error(len)) => {
            for _ in (0..len) {
                ctx.seq.remove(0);
            };
            EditResult::Cont(false)
        },
        Err(ParseError::Incomplete) => EditResult::Cont(false),
        Ok(ParseSuccess(token, len)) => {
            let ins = instr::interpret_token(token, ctx.mode_state);
            let res = handle(ctx, ins);
            for _ in (0..len) {
                ctx.seq.remove(0);
            };
            res
        }
    };
    match res {
        EditResult::Cont(clear) => EditResult::Cont(ctx.buf.get_line(ctx.prompt, clear)),
        EditResult::Halt(res) => EditResult::Halt(res)
    }
}
