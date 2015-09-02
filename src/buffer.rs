use std::str;
use std::mem::swap;

use error::Error;

pub struct Buffer {
    front_buf: Vec<u8>,
    back_buf: Vec<u8>,
    pos: usize
}

impl Buffer {

    pub fn new() -> Buffer {
        Buffer {
            front_buf: Vec::new(),
            back_buf: Vec::new(),
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

    pub fn replace(&mut self, s: &[u8]) {
        self.front_buf.clear();
        self.front_buf.extend(s);
        self.pos = s.len();
    }

    pub fn insert_byte_at_cursor(&mut self, c: u8) {
        self.front_buf.insert(self.pos, c);
        self.pos += 1;
    }

    pub fn insert_bytes_at_cursor(&mut self, cs: &[u8]) {
        for c in cs {
            self.insert_byte_at_cursor(c.clone());
        }
    }

    pub fn delete_byte_left_of_cursor(&mut self) {
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
        let mut seq = Vec::new();
        seq.extend("\r".as_bytes());
        seq.extend(prompt.as_bytes());
        seq.extend(&self.front_buf);
        seq.extend("\x1b[0K".as_bytes());
        seq.extend(&format!("\r\x1b[{}C", prompt.len() + self.pos).into_bytes());
        seq
    }

    pub fn to_string(self) -> Result<String, Error> {
        String::from_utf8(self.front_buf).map_err(|_| Error::InvalidUTF8)
    }

}
