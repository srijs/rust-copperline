use parser;
use edit::EditMode;
use edit::ViMode;

pub enum Instr {
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorStart,
    MoveCursorEnd,
    MoveEndOfWordRight,
    DeleteCharLeftOfCursor,
    DeleteCharRightOfCursor,
    DeleteCharRightOfCursorOrEOF,
    Substitute,
    InsertAtCursor(String),
    ReplaceAtCursor(String),
    HistoryNext,
    HistoryPrev,
    Insert,
    InsertStart,
    Append,
    AppendEnd,
    NormalMode,
    ReplaceMode,
    Digit(u32),
    Done,
    Cancel,
    Clear,
    Noop
}

pub fn interpret_token(token: parser::Token, edit_mode: EditMode, vi_mode: ViMode) -> Instr {
    match edit_mode {
        EditMode::Emacs => emacs_mode(token),
        EditMode::Vi => match vi_mode {
            ViMode::Insert => vi_insert_mode(token),
            ViMode::Normal => vi_normal_mode(token),
            ViMode::Replace => vi_replace_mode(token),
        },
    }
}

fn emacs_mode(token: parser::Token) -> Instr {
    match token {
        parser::Token::Enter        => Instr::Done,
        parser::Token::Backspace    => Instr::DeleteCharLeftOfCursor,
        parser::Token::CtrlH        => Instr::DeleteCharLeftOfCursor,
        parser::Token::EscBracket3T => Instr::DeleteCharRightOfCursor,
        parser::Token::CtrlD        => Instr::DeleteCharRightOfCursorOrEOF,
        parser::Token::EscBracketA  => Instr::HistoryPrev,
        parser::Token::CtrlP        => Instr::HistoryPrev,
        parser::Token::EscBracketB  => Instr::HistoryNext,
        parser::Token::CtrlN        => Instr::HistoryNext,
        parser::Token::EscBracketC  => Instr::MoveCursorRight,
        parser::Token::CtrlF        => Instr::MoveCursorRight,
        parser::Token::EscBracketD  => Instr::MoveCursorLeft,
        parser::Token::CtrlB        => Instr::MoveCursorLeft,
        parser::Token::CtrlA        => Instr::MoveCursorStart,
        parser::Token::EscBracketH  => Instr::MoveCursorStart,
        parser::Token::CtrlE        => Instr::MoveCursorEnd,
        parser::Token::EscBracketF  => Instr::MoveCursorEnd,
        parser::Token::Text(text)   => Instr::InsertAtCursor(text),
        parser::Token::CtrlC        => Instr::Cancel,
        parser::Token::CtrlL        => Instr::Clear,
        _                           => Instr::Noop
    }
}

fn vi_common(token: &parser::Token) -> Instr {
    match *token {
        parser::Token::Enter        => Instr::Done,
        parser::Token::Esc          => Instr::NormalMode,
        parser::Token::Backspace    => Instr::DeleteCharLeftOfCursor,
        parser::Token::EscBracket3T => Instr::DeleteCharRightOfCursor,
        // XXX EOF
        parser::Token::CtrlD        => Instr::DeleteCharRightOfCursorOrEOF,
        // movement keys
        parser::Token::EscBracketA  => Instr::HistoryPrev,
        parser::Token::EscBracketB  => Instr::HistoryNext,
        parser::Token::EscBracketC  => Instr::MoveCursorRight,
        parser::Token::EscBracketD  => Instr::MoveCursorLeft,
        // home
        parser::Token::EscBracketH  => Instr::MoveCursorStart,
        // end
        parser::Token::EscBracketF  => Instr::MoveCursorEnd,
        parser::Token::CtrlC        => Instr::Cancel,
        parser::Token::CtrlL        => Instr::Clear,
        _                           => Instr::Noop,
    }
}

fn vi_insert_mode(token: parser::Token) -> Instr {
    match token {
        parser::Token::Text(text)   => Instr::InsertAtCursor(text),
        _                           => vi_common(&token),
    }
}
fn vi_normal_mode(token: parser::Token) -> Instr {
    match token {
        parser::Token::Text(text)   => match text.as_ref() {
            "h"                     => Instr::MoveCursorLeft,
            "j"                     => Instr::HistoryNext,
            "k"                     => Instr::HistoryPrev,
            "l"                     => Instr::MoveCursorRight,
            "0"                     => Instr::Digit(0),
            "$"                     => Instr::MoveCursorEnd,

            "x"                     => Instr::DeleteCharRightOfCursor,
            "s"                     => Instr::Substitute,
            "r"                     => Instr::ReplaceMode,

            "e"                     => Instr::MoveEndOfWordRight,

            "a"                     => Instr::Append,
            "A"                     => Instr::AppendEnd,
            "i"                     => Instr::Insert,
            "I"                     => Instr::InsertStart,

            "1"                     => Instr::Digit(1),
            "2"                     => Instr::Digit(2),
            "3"                     => Instr::Digit(3),
            "4"                     => Instr::Digit(4),
            "5"                     => Instr::Digit(5),
            "6"                     => Instr::Digit(6),
            "7"                     => Instr::Digit(7),
            "8"                     => Instr::Digit(8),
            "9"                     => Instr::Digit(9),

            _                       => Instr::Noop,
        },
        _                           => vi_common(&token),
    }
}
fn vi_replace_mode(token: parser::Token) -> Instr {
    match token {
        parser::Token::Text(text)   => Instr::ReplaceAtCursor(text),
        _                           => Instr::NormalMode,
    }
}
