use parser;

pub enum Instr {
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorStart,
    MoveCursorEnd,
    DeleteCharLeftOfCursor,
    DeleteCharRightOfCursor,
    DeleteCharRightOfCursorOrEOF,
    InsertAtCursor,
    HistoryNext,
    HistoryPrev,
    Done,
    Cancel,
    Clear,
    Noop
}

pub fn interpret_token(token: parser::Token) -> Instr {
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
        parser::Token::Text         => Instr::InsertAtCursor,
        parser::Token::CtrlC        => Instr::Cancel,
        parser::Token::CtrlL        => Instr::Clear,
        _                           => Instr::Noop
    }
}
