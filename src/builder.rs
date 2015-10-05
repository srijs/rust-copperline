pub struct Builder {
    seq: Vec<u8>
}

impl Builder {

    pub fn new() -> Builder {
        Builder { seq: Vec::new() }
    }

    pub fn build(self) -> Vec<u8> {
        self.seq
    }

    pub fn append(&mut self, str: &str) {
        self.seq.extend(str.as_bytes());
    }

    pub fn carriage_return(&mut self) {
        self.seq.extend(b"\r");
    }

    pub fn erase_to_right(&mut self) {
        self.seq.extend(b"\x1b[0K");
    }

    pub fn set_cursor_pos(&mut self, pos: usize) {
        self.seq.extend(&format!("\r\x1b[{}C", pos).into_bytes());
    }

    pub fn clear_screen(&mut self) {
        self.seq.extend(b"\x1b[H\x1b[2J")
    }

    pub fn reset_color(&mut self) {
        self.seq.extend(b"\x1b[0m")
    }

    pub fn invert_color(&mut self) {
        self.seq.extend(b"\x1b[7m")
    }

    pub fn ask_cursor_pos(&mut self) {
        self.seq.extend(b"\x1b[6n")
    }

}
