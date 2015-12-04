use std::mem::swap;
use std::ops::Deref;

use unicode_width::UnicodeWidthStr;

use strcursor::StrCursor;

use builder::Builder;

pub struct Buffer {
    front_buf: String,
    back_buf: String,
    pos: usize
}

impl Buffer {

    pub fn new() -> Buffer {
        Buffer {
            front_buf: String::new(),
            back_buf: String::new(),
            pos: 0
        }
    }

    pub fn swap(&mut self) {
        swap(&mut self.front_buf, &mut self.back_buf);
        self.pos = self.front_buf.len();
    }

    pub fn replace(&mut self, s: &str) {
        self.front_buf.clear();
        self.front_buf.extend(s.chars());
        self.pos = s.len();
    }

    pub fn insert_char_at_cursor(&mut self, c: char) {
        self.front_buf.insert(self.pos, c);
        self.pos += c.len_utf8();
    }

    pub fn insert_chars_at_cursor(&mut self, s: String) {
        for c in s.chars() {
            self.insert_char_at_cursor(c);
        }
    }

    pub fn replace_chars_at_cursor(&mut self, s: String) {
        self.delete_char_right_of_cursor();
        let insert_len = s.chars().count();
        self.insert_chars_at_cursor(s);
        self.pos -= insert_len;
    }

    pub fn delete_char_left_of_cursor(&mut self) {
        if self.move_left() {
            self.front_buf.remove(self.pos);
        }
    }

    pub fn delete_char_right_of_cursor(&mut self) -> bool {
        if self.pos < self.front_buf.len() {
            self.front_buf.remove(self.pos);
            return true;
        } else {
            return false;
        }
    }

    fn cursor(&self) -> StrCursor {
        StrCursor::new_at_left_of_byte_pos(self.front_buf.deref(), self.pos)
    }

    fn prev_pos(&self) -> Option<usize> {
        self.cursor().at_prev().map(|c| c.byte_pos())
    }

    pub fn move_left(&mut self) -> bool {
        match self.prev_pos() {
            Some(pos) => {
                self.pos = pos;
                true
            },
            None => false
        }
    }

    fn next_pos(&self) -> Option<usize> {
        self.cursor().at_next().map(|c| c.byte_pos())
    }

    pub fn move_right(&mut self) -> bool {
        match self.next_pos() {
            Some(pos) => {
                self.pos = pos;
                true
            },
            None => false
        }
    }

    /// If the cursor is one past the end of the line, move one character to the left.
    pub fn exclude_eol(&mut self) {
        self.move_right();
        self.move_left();
    }

    pub fn move_start(&mut self) {
        self.pos = 0;
    }

    pub fn move_word(&mut self) {
        self.vi_move_word(ViMoveMode::Keyword);
    }

    pub fn move_word_ws(&mut self) {
        self.vi_move_word(ViMoveMode::Whitespace);
    }

    fn vi_move_word(&mut self, move_mode: ViMoveMode) {
        enum State {
            Whitespace,
            Keyword,
            NonKeyword,
        };

        let mut state = match self.cursor().cp_after() {
            None => return,
            Some(c) => match c {
                c if c.is_whitespace() => State::Whitespace,
                c if is_vi_keyword(c) => State::Keyword,
                _ => State::NonKeyword,
            },
        };

        while self.move_right() {
            let c = match self.cursor().cp_after() {
                Some(c) => c,
                _ => return,
            };

            match state {
                State::Whitespace => match c {
                    c if c.is_whitespace() => {},
                    _ => return,
                },
                State::Keyword => match c {
                    c if c.is_whitespace() => state = State::Whitespace,
                    c if move_mode == ViMoveMode::Keyword
                        && !is_vi_keyword(c)
                    => return,
                    _ => {}
                },
                State::NonKeyword => match c {
                    c if c.is_whitespace() => state = State::Whitespace,
                    c if move_mode == ViMoveMode::Keyword
                        && is_vi_keyword(c)
                    => return,
                    _ => {}
                },
            }
        }
    }

    pub fn move_end(&mut self) {
        self.pos = self.front_buf.len();
    }

    pub fn move_to_end_of_word(&mut self) {
        self.vi_move_word_end(ViMoveMode::Keyword);
    }

    pub fn move_to_end_of_word_ws(&mut self) {
        self.vi_move_word_end(ViMoveMode::Whitespace);
    }

    fn vi_move_word_end(&mut self, move_mode: ViMoveMode) {
        enum State {
            Whitespace,
            EndOnWord,
            EndOnOther,
            EndOnWhitespace,
        };

        let mut state = State::Whitespace;

        while self.move_right() {

            // XXX maybe use for self.cursor().slice_after().char_indicies()
            // XXX should we use self.cursor().after()?
            let c = match self.cursor().cp_after() {
                Some(c) => c,
                _ => return,
            };

            match state {
                State::Whitespace => match c {
                    // skip initial whitespace
                    c if c.is_whitespace() => {},
                    // if we are in keyword mode and found a keyword, stop on word
                    c if move_mode == ViMoveMode::Keyword
                        && is_vi_keyword(c) =>
                    {
                        state = State::EndOnWord;
                    },
                    // not in keyword mode, stop on whitespace
                    _ if move_mode == ViMoveMode::Whitespace => {
                        state = State::EndOnWhitespace;
                    }
                    // in keyword mode, found non-whitespace non-keyword, stop on anything
                    _ => {
                        state = State::EndOnOther;
                    }
                },
                State::EndOnWord if !is_vi_keyword(c) => {
                    self.move_left();
                    return;
                },
                State::EndOnWhitespace if c.is_whitespace() => {
                    self.move_left();
                    return;
                },
                State::EndOnOther if c.is_whitespace() || is_vi_keyword(c) => {
                    self.move_left();
                    return;
                },
                _ => {},
            }
        }
    }

    fn char_pos(&self) -> usize {
         UnicodeWidthStr::width(self.cursor().slice_before())
    }

    fn move_to_pos(&mut self, pos: usize) {
        if pos > self.front_buf.len() {
            self.move_end();
        }
        else {
            self.pos = pos;
        }
    }

    pub fn get_line(&self, prompt: &str, clear: bool) -> Vec<u8> {
        let mut line = Builder::new();
        if clear {
            line.clear_screen();
        }
        line.carriage_return();
        line.append(prompt);
        line.append(&self.front_buf);
        line.erase_to_right();
        line.set_cursor_pos(prompt.len() + self.char_pos());
        line.build()
    }

    pub fn to_string(self) -> String {
        self.front_buf
    }

    pub fn drain(&mut self) -> String {
        let mut s = String::new();
        swap(&mut s, &mut self.front_buf);
        self.pos = 0;
        s
    }
}

#[derive(PartialEq)]
enum ViMoveMode {
    Keyword,
    Whitespace,
}

/// All alphanumeric characters and _ are considered valid for keywords in vi by default.
fn is_vi_keyword(c: char) -> bool {
    match c {
        '_' => true,
        c if c.is_alphanumeric() => true,
        _ => false,
    }
}

#[test]
fn move_and_insert_ascii() {
    let mut buf = Buffer::new();
    buf.insert_char_at_cursor('a');
    buf.move_left();
    buf.insert_char_at_cursor('x');
    buf.move_left();
    buf.move_right();
    buf.move_right();
    buf.insert_char_at_cursor('b');
    buf.move_start();
    buf.insert_char_at_cursor('w');
    buf.move_end();
    buf.insert_char_at_cursor('c');
    buf.move_left();
    buf.move_left();
    buf.delete_char_left_of_cursor();
    assert_eq!(buf.to_string(), "wxbc".to_string());
}

#[test]
fn move_and_insert_cyrillic() {
    let mut buf = Buffer::new();
    buf.insert_char_at_cursor('Й');
    buf.move_left();
    buf.insert_char_at_cursor('ч');
    buf.move_left();
    buf.move_right();
    buf.move_right();
    buf.insert_char_at_cursor('Њ');
    buf.move_start();
    buf.insert_char_at_cursor('Ѿ');
    buf.move_end();
    buf.insert_char_at_cursor('Җ');
    buf.move_left();
    buf.move_left();
    buf.delete_char_left_of_cursor();
    assert_eq!(buf.to_string(), "ѾчЊҖ".to_string());
}

#[test]
fn move_and_insert_cjk() {
    let mut buf = Buffer::new();
    buf.insert_char_at_cursor('䩖');
    buf.move_left();
    buf.insert_char_at_cursor('䨻');
    buf.move_left();
    buf.move_right();
    buf.move_right();
    buf.insert_char_at_cursor('䦴');
    buf.move_start();
    buf.insert_char_at_cursor('乫');
    buf.move_end();
    buf.insert_char_at_cursor('憛');
    buf.move_left();
    buf.move_left();
    buf.delete_char_left_of_cursor();
    assert_eq!(buf.to_string(), "乫䨻䦴憛".to_string());
}

#[test]
fn move_to_pos() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("pos".to_string());
    let pos = buf.char_pos();
    buf.insert_chars_at_cursor("pos".to_string());

    assert!(buf.char_pos() != pos);
    buf.move_to_pos(pos);
    assert_eq!(buf.char_pos(), pos);
}

#[test]
fn move_to_pos_past_end() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("pos".to_string());
    let end_pos = buf.char_pos();

    assert_eq!(buf.char_pos(), end_pos);
    buf.move_to_pos(10_000);
    assert_eq!(buf.char_pos(), end_pos);
}

#[test]
fn move_to_end_of_word_simple() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here are".to_string());
    let start_pos = buf.char_pos();
    buf.insert_chars_at_cursor(" som".to_string());
    let end_pos = buf.char_pos();
    buf.insert_chars_at_cursor("e words".to_string());
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word();
    assert_eq!(buf.char_pos(), end_pos);
}

#[test]
fn move_to_end_of_word_comma() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here ar".to_string());
    let start_pos = buf.char_pos();
    buf.insert_char_at_cursor('e');
    let end_pos1 = buf.char_pos();
    buf.insert_chars_at_cursor(", som".to_string());
    let end_pos2 = buf.char_pos();
    buf.insert_chars_at_cursor("e words".to_string());
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word();
    assert_eq!(buf.char_pos(), end_pos1);
    buf.move_to_end_of_word();
    assert_eq!(buf.char_pos(), end_pos2);
}

#[test]
fn move_to_end_of_word_nonkeywords() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here ar".to_string());
    let start_pos = buf.char_pos();
    buf.insert_chars_at_cursor("e,,,".to_string());
    let end_pos1 = buf.char_pos();
    buf.insert_chars_at_cursor(",som".to_string());
    let end_pos2 = buf.char_pos();
    buf.insert_chars_at_cursor("e words".to_string());
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word();
    assert_eq!(buf.char_pos(), end_pos1);
    buf.move_to_end_of_word();
    assert_eq!(buf.char_pos(), end_pos2);
}

#[test]
fn move_to_end_of_word_whitespace() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here are".to_string());
    let start_pos = buf.char_pos();
    buf.insert_chars_at_cursor("      som".to_string());
    let end_pos = buf.char_pos();
    buf.insert_chars_at_cursor("e words".to_string());
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word();
    assert_eq!(buf.char_pos(), end_pos);
}

#[test]
fn move_to_end_of_word_whitespace_nonkeywords() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here ar".to_string());
    let start_pos = buf.char_pos();
    buf.insert_chars_at_cursor("e   ,,,".to_string());
    let end_pos1 = buf.char_pos();
    buf.insert_chars_at_cursor(", som".to_string());
    let end_pos2 = buf.char_pos();
    buf.insert_chars_at_cursor("e words".to_string());
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word();
    assert_eq!(buf.char_pos(), end_pos1);
    buf.move_to_end_of_word();
    assert_eq!(buf.char_pos(), end_pos2);
}

#[test]
fn replace_chars_at_cursor() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("text".to_string());
    let pos = buf.char_pos();
    buf.insert_chars_at_cursor(" string".to_string());
    for _ in 0..buf.char_pos() - pos {
        buf.move_left();
    }

    // replace should not move the cursor
    assert_eq!(buf.char_pos(), pos);
    buf.replace_chars_at_cursor("_".to_string());
    assert_eq!(buf.to_string(), "text_string".to_string());
}

#[test]
fn exclude_eol() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("text".to_string());
    let end = buf.char_pos();
    buf.move_left();

    let target = buf.char_pos();
    buf.move_right();
    buf.move_right();

    // should be at the end of the string
    assert_eq!(buf.char_pos(), end);

    // a call to exclude_eol() should move the cursor
    buf.exclude_eol();
    assert_eq!(buf.char_pos(), target);

    // further calls should not move the cursor
    buf.exclude_eol();
    assert_eq!(buf.char_pos(), target);
}

#[test]
fn move_to_end_of_word_ws_simple() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here are".to_string());
    let start_pos = buf.char_pos();
    buf.insert_chars_at_cursor(" som".to_string());
    let end_pos = buf.char_pos();
    buf.insert_chars_at_cursor("e words".to_string());
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word_ws();
    assert_eq!(buf.char_pos(), end_pos);
}

#[test]
fn move_to_end_of_word_ws_comma() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here ar".to_string());
    let start_pos = buf.char_pos();
    buf.insert_char_at_cursor('e');
    let end_pos1 = buf.char_pos();
    buf.insert_chars_at_cursor(", som".to_string());
    let end_pos2 = buf.char_pos();
    buf.insert_chars_at_cursor("e words".to_string());
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word_ws();
    assert_eq!(buf.char_pos(), end_pos1);
    buf.move_to_end_of_word_ws();
    assert_eq!(buf.char_pos(), end_pos2);
}

#[test]
fn move_to_end_of_word_ws_nonkeywords() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here ar".to_string());
    let start_pos = buf.char_pos();
    buf.insert_chars_at_cursor("e,,,,som".to_string());
    let end_pos = buf.char_pos();
    buf.insert_chars_at_cursor("e words".to_string());
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word_ws();
    assert_eq!(buf.char_pos(), end_pos);
}

#[test]
fn move_to_end_of_word_ws_whitespace() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here are".to_string());
    let start_pos = buf.char_pos();
    buf.insert_chars_at_cursor("      som".to_string());
    let end_pos = buf.char_pos();
    buf.insert_chars_at_cursor("e words".to_string());
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word_ws();
    assert_eq!(buf.char_pos(), end_pos);
}

#[test]
fn move_to_end_of_word_ws_whitespace_nonkeywords() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here ar".to_string());
    let start_pos = buf.char_pos();
    buf.insert_chars_at_cursor("e   ,,,".to_string());
    let end_pos1 = buf.char_pos();
    buf.insert_chars_at_cursor(", som".to_string());
    let end_pos2 = buf.char_pos();
    buf.insert_chars_at_cursor("e words".to_string());
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word_ws();
    assert_eq!(buf.char_pos(), end_pos1);
    buf.move_to_end_of_word_ws();
    assert_eq!(buf.char_pos(), end_pos2);
}

#[test]
fn move_word_simple() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here ".to_string());
    let pos1 = buf.char_pos();
    buf.insert_chars_at_cursor("are ".to_string());
    let pos2 = buf.char_pos();
    buf.insert_chars_at_cursor("some words".to_string());
    buf.move_start();

    buf.move_word();
    assert_eq!(buf.char_pos(), pos1);
    buf.move_word();
    assert_eq!(buf.char_pos(), pos2);

    buf.move_start();
    buf.move_word_ws();
    assert_eq!(buf.char_pos(), pos1);
    buf.move_word_ws();
    assert_eq!(buf.char_pos(), pos2);
}

#[test]
fn move_word_whitespace() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("   ".to_string());
    let pos1 = buf.char_pos();
    buf.insert_chars_at_cursor("word".to_string());
    let pos2 = buf.char_pos();
    buf.move_start();

    buf.move_word();
    assert_eq!(buf.char_pos(), pos1);
    buf.move_word();
    assert_eq!(buf.char_pos(), pos2);

    buf.move_start();
    buf.move_word_ws();
    assert_eq!(buf.char_pos(), pos1);
    buf.move_word_ws();
    assert_eq!(buf.char_pos(), pos2);
}

#[test]
fn move_word_nonkeywords() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("...".to_string());
    let pos1 = buf.char_pos();
    buf.insert_chars_at_cursor("word".to_string());
    let pos2 = buf.char_pos();
    buf.move_start();

    buf.move_word();
    assert_eq!(buf.char_pos(), pos1);
    buf.move_word();
    assert_eq!(buf.char_pos(), pos2);

    buf.move_start();
    buf.move_word_ws();
    assert_eq!(buf.char_pos(), pos2);
}

#[test]
fn move_word_whitespace_nonkeywords() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("...   ".to_string());
    let pos1 = buf.char_pos();
    buf.insert_chars_at_cursor("...".to_string());
    let pos2 = buf.char_pos();
    buf.insert_chars_at_cursor("word".to_string());
    let pos3 = buf.char_pos();
    buf.move_start();

    buf.move_word();
    assert_eq!(buf.char_pos(), pos1);
    buf.move_word();
    assert_eq!(buf.char_pos(), pos2);

    buf.move_start();
    buf.move_word_ws();
    assert_eq!(buf.char_pos(), pos1);
    buf.move_word_ws();
    assert_eq!(buf.char_pos(), pos3);
}
