use std::clone::Clone;

use encoding::types::{EncodingRef, RawDecoder};

#[derive(Debug, PartialEq)]
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
    Text(String)
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Error(usize),
    Incomplete
}

#[derive(Debug, PartialEq)]
pub struct ParseSuccess<T>(pub T, pub usize);

pub type ParseResult<T> = Result<ParseSuccess<T>, ParseError>;

fn map_and_filter_result<T, U, F: FnOnce(T) -> Option<U>>(r: ParseResult<T>, f: F) -> ParseResult<U> {
    r.and_then(|ParseSuccess(t, n)| {
        match f(t) {
            Some(u) => Ok(ParseSuccess(u, n)),
            None => Err(ParseError::Error(n))
        }
    })
}

fn filter_result<T, F: FnOnce(T) -> bool>(r: ParseResult<T>, f: F) -> ParseResult<()> {
    map_and_filter_result(r, |t| {
        if f(t) {
            Some(())
        } else {
            None
        }
    })
}

fn match_head(i: u8) -> Option<Token> {
    match i {
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

fn parse_char(vec: &[u8], off: usize) -> ParseResult<u8> {
    match vec.get(off) {
        None => Err(ParseError::Incomplete),
        Some(i) => {
            Ok(ParseSuccess(i.clone(), off + 1))
        }
    }
}

fn parse_esc_bracket(vec: &[u8]) -> ParseResult<Token> {
    let c = try!(parse_char(vec, 2)).0 as char;
    if c >= '0' && c <= '9' {
        /* Extended escape, read additional byte. */
        let d = try!(parse_char(vec, 3)).0 as char;
        match (c, d) {
            ('3', '~') => Ok(ParseSuccess(Token::EscBracket3T, 4)),
            _ => Err(ParseError::Error(4))
        }
    } else {
        match c {
            'A' => Ok(ParseSuccess(Token::EscBracketA, 3)),
            'B' => Ok(ParseSuccess(Token::EscBracketB, 3)),
            'C' => Ok(ParseSuccess(Token::EscBracketC, 3)),
            'D' => Ok(ParseSuccess(Token::EscBracketD, 3)),
            'F' => Ok(ParseSuccess(Token::EscBracketF, 3)),
            'H' => Ok(ParseSuccess(Token::EscBracketH, 3)),
            _ => Err(ParseError::Error(2)) // TODO: implement more
        }
    }
}

fn parse_esc(vec: &[u8]) -> ParseResult<Token> {
    let c = try!(parse_char(vec, 1)).0 as char;
    if c == '[' {
        parse_esc_bracket(vec)
    } else if c == '0' {
        Err(ParseError::Error(2)) // TODO: implement
    } else {
        Err(ParseError::Error(2))
    }
}

pub fn parse(vec: &[u8], enc: EncodingRef) -> ParseResult<Token> {
    let i = try!(parse_char(vec, 0)).0;
    match match_head(i) {
        Some(Token::Esc) => parse_esc(vec),
        Some(t) => Ok(ParseSuccess(t, 1)),
        None => {
            let mut dec = enc.raw_decoder();
            let mut text = String::new();
            match dec.raw_feed(vec, &mut text) {
                (offset, None) => Ok(ParseSuccess(Token::Text(text), offset)),
                (offset, Some(_)) => Err(ParseError::Error(offset))
            }
        }
    }
}
