use std::str;

use error::Error;

pub struct Buffer {
    buf: String,
    pos: usize
}

impl Buffer {

    pub fn new() -> Buffer {
        Buffer{buf: String::new(), pos: 0}
    }

    fn push_str(&mut self, s: &str) {
        self.buf.push_str(s);
        self.pos += s.len();
    }

    pub fn insert_string_at_cursor(&mut self, s: &[u8]) -> Result<(), Error> {
        let c = try!(str::from_utf8(s).map_err(|_| Error::InvalidUTF8));
        self.push_str(c);
        Ok(())
    }

    pub fn delete_char_left_of_cursor(&mut self) {
        if self.pos > 0 {
            self.buf.remove(self.pos-1);
            self.pos -= 1;
        }
    }

    pub fn move_left(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.pos < self.buf.len() {
            self.pos += 1;
        }
    }

    pub fn move_start(&mut self) {
        self.pos = 0;
    }

    pub fn move_end(&mut self) {
        self.pos = self.buf.len();
    }

    pub fn get_line(&self, prompt: &str) -> Vec<u8> {
        let mut seq = String::new();
        seq.push('\r');
        seq.push_str(prompt);
        seq.push_str(&self.buf);
        seq.push_str("\x1b[0K");
        seq.push_str(&format!("\r\x1b[{}C", prompt.len() + self.pos));
        seq.into_bytes()
    }

    pub fn to_string(self) -> String {
        self.buf
    }

}
