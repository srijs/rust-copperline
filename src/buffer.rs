use std::str;
use std::mem::swap;

use error::Error;

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

    pub fn reset(&mut self) {
        self.front_buf.clear();
        self.pos = 0;
    }

    pub fn replace(&mut self, s: &str) {
        self.front_buf.clear();
        self.front_buf.push_str(s);
        self.pos = s.len();
    }

    pub fn insert_char_at_cursor(&mut self, c: char) {
        let len = c.len_utf8();
        self.front_buf.insert(self.pos, c);
        self.pos += len;
    }

    pub fn insert_string_at_cursor(&mut self, s: &str) {
        for c in s.chars() {
            self.insert_char_at_cursor(c);
        }
    }

    pub fn insert_bytes_at_cursor(&mut self, s: &[u8]) -> Result<(), Error> {
        let c = try!(str::from_utf8(s).map_err(|_| Error::InvalidUTF8));
        self.insert_string_at_cursor(c);
        Ok(())
    }

    pub fn delete_char_left_of_cursor(&mut self) {
        if self.pos > 0 {
            self.front_buf.remove(self.pos-1);
            self.pos -= 1;
        }
    }

    pub fn move_left(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.pos < self.front_buf.len() {
            self.pos += 1;
        }
    }

    pub fn move_start(&mut self) {
        self.pos = 0;
    }

    pub fn move_end(&mut self) {
        self.pos = self.front_buf.len();
    }

    pub fn get_line(&self, prompt: &str) -> Vec<u8> {
        let mut seq = String::new();
        seq.push('\r');
        seq.push_str(prompt);
        seq.push_str(&self.front_buf);
        seq.push_str("\x1b[0K");
        seq.push_str(&format!("\r\x1b[{}C", prompt.len() + self.pos));
        seq.into_bytes()
    }

    pub fn to_string(self) -> String {
        self.front_buf
    }

}
