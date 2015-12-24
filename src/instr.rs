use parser;
use edit::ModeState;
use edit::ViMode;

pub enum Instr {
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorStart,
    MoveCursorEnd,
    MoveEndOfWordRight,
    MoveEndOfWordWsRight,
    MoveWordRight,
    MoveWordWsRight,
    MoveWordLeft,
    MoveWordWsLeft,
    MoveCharRight(char),
    MoveCharLeft(char),
    MoveBeforeCharRight(char),
    MoveBeforeCharLeft(char),
    DeleteCharLeftOfCursor,
    DeleteCharRightOfCursor,
    DeleteCharRightOfCursorOrEOF,
    DeleteLine,
    DeleteToEnd,
    ChangeLine,
    ChangeToEnd,
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
    MoveCharMode(CharMoveType),
    DeleteMode,
    ChangeMode,
    Digit(u32),
    Done,
    DoneOrEof,
    Cancel,
    Clear,
    Noop
}

#[derive(Copy,Clone,PartialEq)]
pub enum CharMoveType {
    BeforeRight,
    BeforeLeft,
    Right,
    Left,
}

pub fn interpret_token(token: parser::Token, edit_mode_state: ModeState) -> Instr {
    match edit_mode_state {
        ModeState::Emacs => emacs_mode(token),
        ModeState::Vi(ViMode::Insert, _) => vi_insert_mode(token),
        ModeState::Vi(ViMode::Normal, _) => vi_normal_mode(token),
        ModeState::Vi(ViMode::Replace, _) => vi_replace_mode(token),
        ModeState::Vi(ViMode::MoveChar(move_type), _) => vi_move_char_mode(move_type, token),
        ModeState::Vi(ViMode::DeleteMoveChar(move_type), _) => vi_move_char_mode(move_type, token),
        ModeState::Vi(ViMode::ChangeMoveChar(move_type), _) => vi_move_char_mode(move_type, token),
        ModeState::Vi(ViMode::Delete, _) => vi_delete_mode(token),
        ModeState::Vi(ViMode::Change, _) => vi_change_mode(token),
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
        parser::Token::CtrlD        => Instr::DoneOrEof,
        parser::Token::Esc          => Instr::NormalMode,
        parser::Token::Backspace    => Instr::DeleteCharLeftOfCursor,
        parser::Token::EscBracket3T => Instr::DeleteCharRightOfCursor,
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
            "c"                     => Instr::ChangeMode,
            "C"                     => Instr::ChangeToEnd,
            "d"                     => Instr::DeleteMode,
            "D"                     => Instr::DeleteToEnd,

            "e"                     => Instr::MoveEndOfWordRight,
            "E"                     => Instr::MoveEndOfWordWsRight,
            "w"                     => Instr::MoveWordRight,
            "W"                     => Instr::MoveWordWsRight,
            "b"                     => Instr::MoveWordLeft,
            "B"                     => Instr::MoveWordWsLeft,
            "t"                     => Instr::MoveCharMode(CharMoveType::BeforeRight),
            "T"                     => Instr::MoveCharMode(CharMoveType::BeforeLeft),
            "f"                     => Instr::MoveCharMode(CharMoveType::Right),
            "F"                     => Instr::MoveCharMode(CharMoveType::Left),

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
fn vi_move_char_mode(move_type: CharMoveType, token: parser::Token) -> Instr {
    match token {
        parser::Token::Text(ref text) => match (move_type, text.chars().next()) {
            (CharMoveType::BeforeLeft, Some(c))  => Instr::MoveBeforeCharLeft(c),
            (CharMoveType::BeforeRight, Some(c)) => Instr::MoveBeforeCharRight(c),
            (CharMoveType::Left, Some(c))        => Instr::MoveCharLeft(c),
            (CharMoveType::Right, Some(c))       => Instr::MoveCharRight(c),
            (_, None)                            => Instr::NormalMode, // this is probably unreachable!()
        },
        _                           => Instr::NormalMode,
    }
}
fn vi_change_delete_common(token: &parser::Token) -> Instr {
    match *token {
        parser::Token::Text(ref text) => match text.as_ref() {
            "h"                     => Instr::MoveCursorLeft,
            "l"                     => Instr::MoveCursorRight,
            "0"                     => Instr::Digit(0),
            "$"                     => Instr::MoveCursorEnd,

            "e"                     => Instr::MoveEndOfWordRight,
            "E"                     => Instr::MoveEndOfWordWsRight,
            "w"                     => Instr::MoveWordRight,
            "W"                     => Instr::MoveWordWsRight,
            "b"                     => Instr::MoveWordLeft,
            "B"                     => Instr::MoveWordWsLeft,
            "t"                     => Instr::MoveCharMode(CharMoveType::BeforeRight),
            "T"                     => Instr::MoveCharMode(CharMoveType::BeforeLeft),
            "f"                     => Instr::MoveCharMode(CharMoveType::Right),
            "F"                     => Instr::MoveCharMode(CharMoveType::Left),

            "1"                     => Instr::Digit(1),
            "2"                     => Instr::Digit(2),
            "3"                     => Instr::Digit(3),
            "4"                     => Instr::Digit(4),
            "5"                     => Instr::Digit(5),
            "6"                     => Instr::Digit(6),
            "7"                     => Instr::Digit(7),
            "8"                     => Instr::Digit(8),
            "9"                     => Instr::Digit(9),
            _                       => Instr::NormalMode,
        },
        _                           => Instr::NormalMode,
    }
}
fn vi_change_mode(token: parser::Token) -> Instr {
    match token {
        parser::Token::Text(ref text) => match text.as_ref() {
            "c"                     => Instr::ChangeLine,
            _                       => vi_change_delete_common(&token),
        },
        _                           => Instr::NormalMode,
    }
}
fn vi_delete_mode(token: parser::Token) -> Instr {
    match token {
        parser::Token::Text(ref text) => match text.as_ref() {
            "d"                     => Instr::DeleteLine,
            _                       => vi_change_delete_common(&token),
        },
        _                           => Instr::NormalMode,
    }
}
