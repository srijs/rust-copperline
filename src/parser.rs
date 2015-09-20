use std::clone::Clone;

#[derive(Debug)]
pub enum Token {
    Null,
    CtrlA,
    CtrlB,
    CtrlC,
    CtrlD,
    CtrlE,
    CtrlF,
    CtrlG,
    CtrlH,
    Tab,
    CtrlJ,
    CtrlK,
    CtrlL,
    Enter,
    CtrlN,
    CtrlO,
    CtrlP,
    CtrlQ,
    CtrlR,
    CtrlS,
    CtrlT,
    CtrlU,
    CtrlV,
    CtrlW,
    CtrlX,
    CtrlY,
    CtrlZ,
    Esc,
    Backspace,
    EscBracket3T,
    EscBracketA,
    EscBracketB,
    EscBracketC,
    EscBracketD,
    EscBracketH,
    EscBracketF,
    Text
}

pub enum Result {
    Error,
    Incomplete,
    Success(Token)
}

fn match_head(i: &u8) -> Option<Token> {
    match *i {
        0   => Some(Token::Null),
        1   => Some(Token::CtrlA),
        2   => Some(Token::CtrlB),
        3   => Some(Token::CtrlC),
        4   => Some(Token::CtrlD),
        5   => Some(Token::CtrlE),
        6   => Some(Token::CtrlF),
        7   => Some(Token::CtrlG),
        8   => Some(Token::CtrlH),
        9   => Some(Token::Tab),
        10  => Some(Token::CtrlJ),
        11  => Some(Token::CtrlK),
        12  => Some(Token::CtrlL),
        13  => Some(Token::Enter),
        14  => Some(Token::CtrlN),
        15  => Some(Token::CtrlO),
        16  => Some(Token::CtrlP),
        17  => Some(Token::CtrlQ),
        18  => Some(Token::CtrlR),
        19  => Some(Token::CtrlS),
        20  => Some(Token::CtrlT),
        21  => Some(Token::CtrlU),
        22  => Some(Token::CtrlV),
        23  => Some(Token::CtrlW),
        24  => Some(Token::CtrlX),
        25  => Some(Token::CtrlY),
        26  => Some(Token::CtrlZ),
        27  => Some(Token::Esc),
        127 => Some(Token::Backspace),
        _   => None
    }
}

fn parse_esc_bracket(vec: &Vec<u8>) -> Result {
    match vec.get(2) {
        None => Result::Incomplete,
        Some(i) => {
            let i = i.clone();
            if i as char >= '0' && i as char <= '9' {
                /* Extended escape, read additional byte. */
                match vec.get(3) {
                    Option::None => Result::Incomplete,
                    Option::Some(j) => {
                        let j = j.clone();
                        match j as char {
                            '~' => match i as char {
                                '3' => Result::Success(Token::EscBracket3T),
                                _ => Result::Error
                            },
                            _ => Result::Error
                        }
                    }
                }
            } else {
                match i as char {
                    'A' => Result::Success(Token::EscBracketA),
                    'B' => Result::Success(Token::EscBracketB),
                    'C' => Result::Success(Token::EscBracketC),
                    'D' => Result::Success(Token::EscBracketD),
                    'F' => Result::Success(Token::EscBracketF),
                    'H' => Result::Success(Token::EscBracketH),
                    _ => Result::Error // TODO: implement more
                }
            }
        }
    }
}

fn parse_esc(vec: &Vec<u8>) -> Result {
    match vec.get(1) {
        None => Result::Incomplete,
        Some(i) => {
            let i = i.clone();
            if i as char == '[' {
                parse_esc_bracket(vec)
            } else if i as char == '0' {
                Result::Error // TODO: implement
            } else {
                Result::Error
            }
        }
    }
}

pub fn parse(vec: &Vec<u8>) -> Result {
    match vec.get(0) {
        None => Result::Incomplete,
        Some(i) => match match_head(i) {
            Some(Token::Esc) => parse_esc(vec),
            Some(t) => Result::Success(t),
            None => Result::Success(Token::Text)
        }
    }
}
