use std::mem::swap;
use std::ops::Deref;
use std::cmp::Ordering;

use unicode_width::UnicodeWidthStr;
use unicode_segmentation::UnicodeSegmentation;

use builder::Builder;

#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub struct Position {
    byte_pos: usize,
    char_pos: usize
}

impl Position {

    pub fn set_to_end_of_str(&mut self, buf: &str) {
        self.byte_pos = buf.len();
        self.char_pos = UnicodeWidthStr::width(buf);
    }

    pub fn increase_by_char(&mut self, c: char) {
        self.byte_pos += c.len_utf8();
        self.char_pos += 1;
    }

    pub fn decrease_by_str(&mut self, buf: &str) {
        self.byte_pos -= buf.len();
        self.char_pos -= UnicodeWidthStr::width(buf);
    }

}

pub struct Buffer {
    front_buf: String,
    back_buf: String,
    pos: Position
}

impl Buffer {

    pub fn new() -> Buffer {
        Buffer {
            front_buf: String::new(),
            back_buf: String::new(),
            pos: Position {
                byte_pos: 0,
                char_pos: 0
            }
        }
    }

    pub fn swap(&mut self) {
        swap(&mut self.front_buf, &mut self.back_buf);
        self.pos.set_to_end_of_str(self.front_buf.as_str());
    }

    pub fn replace(&mut self, s: &str) {
        self.front_buf.clear();
        self.front_buf.extend(s.chars());
        self.pos.set_to_end_of_str(s);
    }

    pub fn insert_char_at_cursor(&mut self, c: char) {
        self.front_buf.insert(self.pos.byte_pos, c);
        self.pos.increase_by_char(c);
    }

    pub fn insert_chars_at_cursor(&mut self, s: &str) {
        for c in s.chars() {
            self.insert_char_at_cursor(c);
        }
    }

    pub fn replace_chars_at_cursor(&mut self, s: &str) {
        self.delete_char_right_of_cursor();
        self.insert_chars_at_cursor(s);
        self.pos.decrease_by_str(s);
    }

    pub fn delete_char_left_of_cursor(&mut self) -> bool {
        if self.move_left() {
            self.front_buf.remove(self.pos.byte_pos);
            true
        }
        else {
            false
        }
    }

    pub fn delete_char_right_of_cursor(&mut self) -> bool {
        if self.pos.byte_pos < self.front_buf.len() {
            self.front_buf.remove(self.pos.byte_pos);
            return true;
        } else {
            return false;
        }
    }

    fn prev_pos(&self) -> Option<Position> {
        if self.pos.char_pos == 0 {
            None
        } else {
            match UnicodeSegmentation::graphemes(self.front_buf.as_str(), true).nth(self.pos.char_pos - 1) {
                Some(prev) => Some(Position {
                    byte_pos: self.pos.byte_pos - prev.len(),
                    char_pos: self.pos.char_pos - 1
                }),
                None => None
            }
        }
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

    fn next_pos(&self) -> Option<Position> {
        match UnicodeSegmentation::graphemes(self.front_buf.as_str(), true).nth(self.pos.char_pos) {
            Some(next) => Some(Position {
                byte_pos: self.pos.byte_pos + next.len(),
                char_pos: self.pos.char_pos + 1
            }),
            None => None
        }
    }

    fn cp_after(&self) -> Option<char> {
        match UnicodeSegmentation::graphemes(self.front_buf.as_str(), true).nth(self.pos.char_pos) {
            Some(next) => next.chars().next(),
            None => None
        }
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
    pub fn exclude_eol(&mut self) -> bool {
        let end = self.move_right();
        self.move_left();
        end
    }

    pub fn move_start(&mut self) {
        self.pos.byte_pos = 0;
        self.pos.char_pos = 0;
    }

    pub fn move_word(&mut self) -> bool {
        self.vi_move_word(ViMoveMode::Keyword, ViMoveDir::Right)
    }

    pub fn move_word_ws(&mut self) -> bool {
        self.vi_move_word(ViMoveMode::Whitespace, ViMoveDir::Right)
    }

    pub fn move_to_end_of_word_back(&mut self) -> bool {
        self.vi_move_word(ViMoveMode::Keyword, ViMoveDir::Left)
    }

    pub fn move_to_end_of_word_ws_back(&mut self) -> bool {
        self.vi_move_word(ViMoveMode::Whitespace, ViMoveDir::Left)
    }

    fn vi_move_word(&mut self, move_mode: ViMoveMode, direction: ViMoveDir) -> bool {
        enum State {
            Whitespace,
            Keyword,
            NonKeyword,
        };

        let mut state = match self.cp_after() {
            None => return false,
            Some(c) => match c {
                c if c.is_whitespace() => State::Whitespace,
                c if is_vi_keyword(c) => State::Keyword,
                _ => State::NonKeyword,
            },
        };

        let advance = |buf: &mut Self| {
            match direction {
                ViMoveDir::Right => buf.move_right(),
                ViMoveDir::Left => buf.move_left(),
            }
        };

        while advance(self) {
            let c = match self.cp_after() {
                Some(c) => c,
                _ => return false,
            };

            match state {
                State::Whitespace => match c {
                    c if c.is_whitespace() => {},
                    _ => return true,
                },
                State::Keyword => match c {
                    c if c.is_whitespace() => state = State::Whitespace,
                    c if move_mode == ViMoveMode::Keyword
                        && !is_vi_keyword(c)
                    => return true,
                    _ => {}
                },
                State::NonKeyword => match c {
                    c if c.is_whitespace() => state = State::Whitespace,
                    c if move_mode == ViMoveMode::Keyword
                        && is_vi_keyword(c)
                    => return true,
                    _ => {}
                },
            }
        }
        return false;
    }

    pub fn move_end(&mut self) {
        self.pos.set_to_end_of_str(self.front_buf.as_str());
    }

    pub fn move_to_end_of_word(&mut self) -> bool {
        self.vi_move_word_end(ViMoveMode::Keyword, ViMoveDir::Right)
    }

    pub fn move_to_end_of_word_ws(&mut self) -> bool {
        self.vi_move_word_end(ViMoveMode::Whitespace, ViMoveDir::Right)
    }

    pub fn move_word_back(&mut self) -> bool {
        self.vi_move_word_end(ViMoveMode::Keyword, ViMoveDir::Left)
    }

    pub fn move_word_ws_back(&mut self) -> bool {
        self.vi_move_word_end(ViMoveMode::Whitespace, ViMoveDir::Left)
    }

    fn vi_move_word_end(&mut self, move_mode: ViMoveMode, direction: ViMoveDir) -> bool {
        enum State {
            Whitespace,
            EndOnWord,
            EndOnOther,
            EndOnWhitespace,
        };

        let mut state = State::Whitespace;

        let advance = |buf: &mut Self| {
            match direction {
                ViMoveDir::Right => buf.move_right(),
                ViMoveDir::Left => buf.move_left(),
            }
        };

        let go_back = |buf: &mut Self| {
            match direction {
                ViMoveDir::Right => buf.move_left(),
                ViMoveDir::Left => buf.move_right(),
            }
        };

        while advance(self) {

            // XXX maybe use for self.cursor().slice_after().char_indicies()
            // XXX should we use self.cursor().after()?
            let c = match self.cp_after() {
                Some(c) => c,
                _ => return false,
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
                    go_back(self);
                    return true;
                },
                State::EndOnWhitespace if c.is_whitespace() => {
                    go_back(self);
                    return true;
                },
                State::EndOnOther if c.is_whitespace() || is_vi_keyword(c) => {
                    go_back(self);
                    return true;
                },
                _ => {},
            }
        }
        return false;
    }

    /// Move count characters to the right.
    ///
    /// If count characters are not found, the position will not be changed.
    pub fn move_to_char_right(&mut self, target_c: char, count: u32) -> bool {
        let pos = self.pos;
        for _ in 0..count {
            if !self.move_to_char(target_c, ViMoveDir::Right) {
                self.move_to_pos(pos);
                return false;
            }
        }
        return true;
    }

    /// Move count characters to the left.
    ///
    /// If count characters are not found, the position will not be changed.
    pub fn move_to_char_left(&mut self, target_c: char, count: u32) -> bool {
        let pos = self.pos;
        for _ in 0..count {
            if !self.move_to_char(target_c, ViMoveDir::Left) {
                self.move_to_pos(pos);
                return false;
            }
        }
        return true;
    }

    fn move_to_char(&mut self, target_c: char, direction: ViMoveDir) -> bool {
        // XXX this code is very similar to code in move_word_end(), should be replaced with some
        // sort of iterator over the internal buffer starting at the current position
        let advance = |buf: &mut Self| {
            match direction {
                ViMoveDir::Right => buf.move_right(),
                ViMoveDir::Left => buf.move_left(),
            }
        };

        while advance(self) {
            match self.cp_after() {
                Some(c) if c == target_c => return true,
                Some(_) => {}
                None => return false,
            }
        }
        return false;
    }

    fn char_pos(&self) -> usize {
        self.pos.char_pos
    }

    fn byte_pos(&self) -> usize {
        self.pos.byte_pos
    }

    fn move_to_pos(&mut self, pos: Position) -> bool {
        if pos.byte_pos > self.front_buf.len() {
            self.move_end();
            false
        }
        else {
            self.pos = pos;
            true
        }
    }

    fn delete_to_pos(&mut self, pos: Position) {
        // the idea here is to start at the right most position and delete moving to the left until
        // the left most position
        let (start_pos, end_pos) = match self.pos.cmp(&pos) {
            // char_pos() is less than pos, start at pos and delete back to char_pos()
            Ordering::Less => (pos, self.pos),
            // char_pos() and pos are the same, nothing to do
            Ordering::Equal => return,
            // char_pos() is greater than pos, start at char_pos() and delete back to pos
            Ordering::Greater => (self.pos, pos),
        };

        self.move_to_pos(start_pos);
        while self.pos > end_pos {
            self.delete_char_left_of_cursor();
        }
    }

    pub fn start_delete(&mut self) -> DeleteContext {
        DeleteContext::new(self)
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

    pub fn as_str(&self) -> &str {
        self.front_buf.as_str()
    }

    pub fn drain(&mut self) -> String {
        let mut s = String::new();
        swap(&mut s, &mut self.front_buf);
        self.pos.byte_pos = 0;
        self.pos.char_pos = 0;
        s
    }

    pub fn is_empty(&self) -> bool {
        self.front_buf.is_empty()
    }
}

#[must_use]
pub struct DeleteContext<'a> {
    was_on_whitespace: bool,
    start_pos: Position,
    buf: &'a mut Buffer,
}

impl<'a> DeleteContext<'a> {
    fn new(b: &'a mut Buffer) -> Self {
        DeleteContext {
            was_on_whitespace: match b.cp_after() {
                Some(c) if c.is_whitespace() => true,
                _ => false,
            },
            start_pos: b.pos,
            buf: b,
        }
    }

    pub fn started_on_whitespace(&self) -> bool {
        self.was_on_whitespace
    }

    pub fn delete(mut self) {
        self.buf.delete_to_pos(self.start_pos)
    }
}

impl<'a> Deref for DeleteContext<'a> {
    type Target = Buffer;

    fn deref(&self) -> &Self::Target {
        &self.buf
    }
}

use std::ops::DerefMut;
impl<'a> DerefMut for DeleteContext<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buf
    }
}

#[derive(PartialEq)]
enum ViMoveMode {
    Keyword,
    Whitespace,
}

enum ViMoveDir {
    Left,
    Right,
}


/// All alphanumeric characters and _ are considered valid for keywords in vi by default.
fn is_vi_keyword(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
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
    assert_eq!(buf.as_str(), "䩖");
    buf.move_left();
    buf.insert_char_at_cursor('䨻');
    assert_eq!(buf.as_str(), "䨻䩖");
    buf.move_left();
    buf.move_right();
    buf.move_right();
    buf.insert_char_at_cursor('䦴');
    assert_eq!(buf.as_str(), "䨻䩖䦴");
    buf.move_start();
    buf.insert_char_at_cursor('乫');
    assert_eq!(buf.as_str(), "乫䨻䩖䦴");
    buf.move_end();
    buf.insert_char_at_cursor('憛');
    assert_eq!(buf.as_str(), "乫䨻䩖䦴憛");
    buf.move_left();
    buf.move_left();
    buf.delete_char_left_of_cursor();
    assert_eq!(buf.as_str(), "乫䨻䦴憛");
}

#[test]
fn move_to_pos() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("pos");
    let pos = buf.pos;
    buf.insert_chars_at_cursor("pos");

    assert!(buf.pos != pos);
    buf.move_to_pos(pos);
    assert_eq!(buf.pos, pos);
}

#[test]
fn move_to_end_of_word_simple() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here are");
    let start_pos = buf.pos;
    buf.insert_chars_at_cursor(" som");
    let end_pos = buf.pos;
    buf.insert_chars_at_cursor("e words");
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word();
    assert_eq!(buf.pos, end_pos);
}

#[test]
fn move_to_end_of_word_comma() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here ar");
    let start_pos = buf.pos;
    buf.insert_char_at_cursor('e');
    let end_pos1 = buf.pos;
    buf.insert_chars_at_cursor(", som");
    let end_pos2 = buf.pos;
    buf.insert_chars_at_cursor("e words");
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word();
    assert_eq!(buf.pos, end_pos1);
    buf.move_to_end_of_word();
    assert_eq!(buf.pos, end_pos2);
}

#[test]
fn move_to_end_of_word_nonkeywords() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here ar");
    let start_pos = buf.pos;
    buf.insert_chars_at_cursor("e,,,");
    let end_pos1 = buf.pos;
    buf.insert_chars_at_cursor(",som");
    let end_pos2 = buf.pos;
    buf.insert_chars_at_cursor("e words");
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word();
    assert_eq!(buf.pos, end_pos1);
    buf.move_to_end_of_word();
    assert_eq!(buf.pos, end_pos2);
}

#[test]
fn move_to_end_of_word_whitespace() {
    let mut buf = Buffer::new();
    assert_eq!(buf.char_pos(), 0);
    buf.insert_chars_at_cursor("here are");
    let start_pos = buf.pos;
    assert_eq!(buf.char_pos(), 8);
    buf.insert_chars_at_cursor("      som");
    assert_eq!(buf.char_pos(), 17);
    buf.insert_chars_at_cursor("e words");
    assert_eq!(buf.char_pos(), 24);
    buf.move_to_pos(start_pos);
    assert_eq!(buf.char_pos(), 8);

    buf.move_to_end_of_word();
    assert_eq!(buf.char_pos(), 17);
}

#[test]
fn move_to_end_of_word_whitespace_nonkeywords() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here ar");
    let start_pos = buf.pos;
    buf.insert_chars_at_cursor("e   ,,,");
    let end_pos1 = buf.pos;
    buf.insert_chars_at_cursor(", som");
    let end_pos2 = buf.pos;
    buf.insert_chars_at_cursor("e words");
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word();
    assert_eq!(buf.pos, end_pos1);
    buf.move_to_end_of_word();
    assert_eq!(buf.pos, end_pos2);
}

#[test]
fn replace_chars_at_cursor() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("text");
    let pos = buf.pos;
    buf.insert_chars_at_cursor(" string");
    for _ in 0..buf.char_pos() - pos.char_pos {
        buf.move_left();
    }

    // replace should not move the cursor
    assert_eq!(buf.pos, pos);
    buf.replace_chars_at_cursor("_");
    assert_eq!(buf.to_string(), "text_string");
}

#[test]
fn exclude_eol() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("text");
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
    buf.insert_chars_at_cursor("here are");
    let start_pos = buf.pos;
    buf.insert_chars_at_cursor(" som");
    let end_pos = buf.pos;
    buf.insert_chars_at_cursor("e words");
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word_ws();
    assert_eq!(buf.pos, end_pos);
}

#[test]
fn move_to_end_of_word_ws_comma() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here ar");
    let start_pos = buf.pos;
    buf.insert_char_at_cursor('e');
    let end_pos1 = buf.pos;
    buf.insert_chars_at_cursor(", som");
    let end_pos2 = buf.pos;
    buf.insert_chars_at_cursor("e words");
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word_ws();
    assert_eq!(buf.pos, end_pos1);
    buf.move_to_end_of_word_ws();
    assert_eq!(buf.pos, end_pos2);
}

#[test]
fn move_to_end_of_word_ws_nonkeywords() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here ar");
    let start_pos = buf.pos;
    buf.insert_chars_at_cursor("e,,,,som");
    let end_pos = buf.pos;
    buf.insert_chars_at_cursor("e words");
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word_ws();
    assert_eq!(buf.pos, end_pos);
}

#[test]
fn move_to_end_of_word_ws_whitespace() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here are");
    let start_pos = buf.pos;
    buf.insert_chars_at_cursor("      som");
    let end_pos = buf.pos;
    buf.insert_chars_at_cursor("e words");
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word_ws();
    assert_eq!(buf.pos, end_pos);
}

#[test]
fn move_to_end_of_word_ws_whitespace_nonkeywords() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here ar");
    let start_pos = buf.pos;
    buf.insert_chars_at_cursor("e   ,,,");
    let end_pos1 = buf.pos;
    buf.insert_chars_at_cursor(", som");
    let end_pos2 = buf.pos;
    buf.insert_chars_at_cursor("e words");
    buf.move_to_pos(start_pos);

    buf.move_to_end_of_word_ws();
    assert_eq!(buf.pos, end_pos1);
    buf.move_to_end_of_word_ws();
    assert_eq!(buf.pos, end_pos2);
}

#[test]
fn move_word_simple() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here ");
    let pos1 = buf.pos;
    buf.insert_chars_at_cursor("are ");
    let pos2 = buf.pos;
    buf.insert_chars_at_cursor("some words");
    buf.move_start();

    buf.move_word();
    assert_eq!(buf.pos, pos1);
    buf.move_word();
    assert_eq!(buf.pos, pos2);

    buf.move_start();
    buf.move_word_ws();
    assert_eq!(buf.pos, pos1);
    buf.move_word_ws();
    assert_eq!(buf.pos, pos2);
}

#[test]
fn move_word_whitespace() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("   ");
    let pos1 = buf.char_pos();
    buf.insert_chars_at_cursor("word");
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
    buf.insert_chars_at_cursor("...");
    let pos1 = buf.char_pos();
    buf.insert_chars_at_cursor("word");
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
    buf.insert_chars_at_cursor("...   ");
    let pos1 = buf.char_pos();
    buf.insert_chars_at_cursor("...");
    let pos2 = buf.char_pos();
    buf.insert_chars_at_cursor("word");
    let pos3 = buf.char_pos();
    buf.move_start();

    buf.move_word();
    assert_eq!(buf.char_pos(), pos1);
    buf.move_word();
    assert_eq!(buf.char_pos(), pos2);

    buf.move_start();
    assert!(buf.move_word_ws());
    assert_eq!(buf.char_pos(), pos1);
    assert!(!buf.move_word_ws());
    assert_eq!(buf.char_pos(), pos3);
}

#[test]
fn move_word_and_back() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("here ");
    let pos1 = buf.char_pos();
    buf.insert_chars_at_cursor("are ");
    let pos2 = buf.char_pos();
    buf.insert_chars_at_cursor("some");
    let pos3 = buf.char_pos();
    buf.insert_chars_at_cursor("... ");
    let pos4 = buf.char_pos();
    buf.insert_chars_at_cursor("words");
    let pos5 = buf.char_pos();

    // make sure move_word() and move_word_back() are reflections of eachother

    buf.move_start();
    buf.move_word();
    assert_eq!(buf.char_pos(), pos1);
    buf.move_word();
    assert_eq!(buf.char_pos(), pos2);
    buf.move_word();
    assert_eq!(buf.char_pos(), pos3);
    buf.move_word();
    assert_eq!(buf.char_pos(), pos4);
    buf.move_word();
    assert_eq!(buf.char_pos(), pos5);

    buf.move_word_back();
    assert_eq!(buf.char_pos(), pos4);
    buf.move_word_back();
    assert_eq!(buf.char_pos(), pos3);
    buf.move_word_back();
    assert_eq!(buf.char_pos(), pos2);
    buf.move_word_back();
    assert_eq!(buf.char_pos(), pos1);
    buf.move_word_back();
    assert_eq!(buf.char_pos(), 0);

    buf.move_start();
    buf.move_word_ws();
    assert_eq!(buf.char_pos(), pos1);
    buf.move_word_ws();
    assert_eq!(buf.char_pos(), pos2);
    buf.move_word_ws();
    assert_eq!(buf.char_pos(), pos4);
    buf.move_word_ws();
    assert_eq!(buf.char_pos(), pos5);

    buf.move_word_ws_back();
    assert_eq!(buf.char_pos(), pos4);
    buf.move_word_ws_back();
    assert_eq!(buf.char_pos(), pos2);
    buf.move_word_ws_back();
    assert_eq!(buf.char_pos(), pos1);
    buf.move_word_ws_back();
    assert_eq!(buf.char_pos(), 0);
}

#[test]
fn move_to_char() {
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("words");
    let pos1 = buf.char_pos();
    buf.insert_chars_at_cursor(" wor");
    let d_pos = buf.char_pos();
    buf.insert_chars_at_cursor("ds");

    buf.move_start();
    assert!(buf.move_to_char_right(' ', 1));
    assert_eq!(buf.char_pos(), pos1);
    buf.move_end();
    assert!(buf.move_to_char_left(' ', 1));
    assert_eq!(buf.char_pos(), pos1);
    buf.move_start();
    assert_eq!(buf.move_to_char_right('z', 1), false);
    assert_eq!(buf.char_pos(), 0);
    buf.move_start();
    assert!(buf.move_to_char_right('d', 2));
    assert_eq!(buf.char_pos(), d_pos);
}

#[test]
fn move_and_delete1() {
    // test a simple move
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("words words words");
    buf.move_start();
    {
        let mut dc = buf.start_delete();
        dc.move_word();
        dc.delete();
    }
    assert_eq!(buf.to_string(), "words words".to_string());
}

#[test]
fn move_and_delete2() {
    // test deleting an empty string
    let mut buf = Buffer::new();
    buf.move_start();
    {
        let mut dc = buf.start_delete();
        dc.move_end();
        dc.delete();
    }
    assert_eq!(buf.to_string(), "".to_string());
}

#[test]
fn move_and_delete3() {
    // test deleting from the end to the beginning
    let mut buf = Buffer::new();
    buf.insert_chars_at_cursor("words words words");
    {
        let mut dc = buf.start_delete();
        dc.move_start();
        dc.delete();
    }
    assert_eq!(buf.to_string(), "".to_string());
}
