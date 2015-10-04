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

    pub fn move_start(&mut self) {
        self.pos = 0;
    }

    pub fn move_end(&mut self) {
        self.pos = self.front_buf.len();
    }

    fn char_pos(&self) -> usize {
         UnicodeWidthStr::width(self.cursor().slice_before())
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
