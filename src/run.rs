use error::Error;
use edit::{EditCtx, EditResult, edit};
use builder::Builder;
use parser::{parse_cursor_pos, ParseError, ParseSuccess};

pub trait RunIO {

    fn write(&mut self, Vec<u8>) -> Result<(), Error>;
    fn read_byte(&mut self) -> Result<u8, Error>;
    fn read_seq(&mut self) -> Result<Vec<u8>, Error>;

    fn prompt(&mut self, w: Vec<u8>) -> Result<Vec<u8>, Error> {
        try!(self.write(w));
        self.read_seq()
    }

}

fn query_cursor_pos(io: &mut RunIO) -> Result<(u64, u64), Error> {
    let mut line = Builder::new();
    line.ask_cursor_pos();
    try!(io.write(line.build()));
    let mut seq = vec![];
    loop {
        match parse_cursor_pos(&seq) {
            Ok(ParseSuccess(pos, _)) => {
                return Ok(pos);
            },
            Err(ParseError::Error(_)) => {
                return Err(Error::ParseError);
            },
            Err(ParseError::Incomplete) => {
                let byte = try!(io.read_byte());
                seq.push(byte);
            }
        }
    }
}

fn protect_newline(io: &mut RunIO) -> Result<(), Error> {
    let (x, _) = try!(query_cursor_pos(io));
    if x > 1 {
        let mut line = Builder::new();
        line.invert_color();
        line.append("%\n");
        line.reset_color();
        try!(io.write(line.build()));
    }
    Ok(())
}

fn run_edit<'a>(mut ctx: EditCtx<'a>, io: &mut RunIO) -> Result<String, Error> {
    loop {
        match edit(&mut ctx) {
            EditResult::Cont(line) => {
                let bytes = try!(io.prompt(line));
                for byte in bytes.iter() {
                    ctx.fill(*byte);
                }
            },
            EditResult::Halt(res) => { return res; }
        }
    }
}


pub fn run<'a>(ctx: EditCtx<'a>, io: &mut RunIO) -> Result<String, Error> {
    try!(protect_newline(io));
    run_edit(ctx, io)
}

#[cfg(test)]
mod test {
    use encoding::all::ASCII;
    use super::super::error::Error;
    use super::super::edit::EditCtx;
    use super::super::history::History;
    use super::{RunIO, run_edit};
    use edit::EditMode;

    pub struct TestIO {
        input: Vec<u8>,
        output: Vec<u8>
    }

    impl RunIO for TestIO {
        fn write(&mut self, w: Vec<u8>) -> Result<(), Error> {
            self.output.extend(w);
            Ok(())
        }
        fn read_byte(&mut self) -> Result<u8, Error> {
            if self.input.len() > 0 {
                Ok(self.input.remove(0))
            } else {
                Err(Error::EndOfFile)
            }
        }

        fn read_seq(&mut self) -> Result<Vec<u8>, Error> {
            // XXX return more than one byte
            if self.input.len() > 0 {
                Ok(vec![self.input.remove(0)])
            } else {
                Err(Error::EndOfFile)
            }
        }
    }

    #[test]
    fn error_eof_on_empty_input() {
        let mut io = TestIO { input: vec![], output: vec![] };
        let h = History::new();
        let ctx = EditCtx::new("foo> ", &h, ASCII, EditMode::Emacs);
        assert_eq!(run_edit(ctx, &mut io), Err(Error::EndOfFile));
    }

    #[test]
    fn ok_empty_after_return() {
        let mut io = TestIO { input: vec![13], output: vec![] };
        let h = History::new();
        let ctx = EditCtx::new("foo> ", &h, ASCII, EditMode::Emacs);
        assert_eq!(run_edit(ctx, &mut io), Ok("".to_string()));
    }

    #[test]
    fn ok_ascii_after_return() {
        let mut io = TestIO { input: vec![65, 66, 67, 13], output: vec![] };
        let h = History::new();
        let ctx = EditCtx::new("foo> ", &h, ASCII, EditMode::Emacs);
        assert_eq!(run_edit(ctx, &mut io), Ok("ABC".to_string()));
    }

    /// Make sure integers don't overflow in vi mode when using large command counts.
    #[test]
    fn no_integer_overflow() {
        let mut input_vec = vec![27];
        for _ in 0..50 {
            input_vec.push('9' as u8);
        }
        input_vec.push(13);
        let mut io = TestIO { input: input_vec, output: vec![] };
        let h = History::new();
        let ctx = EditCtx::new("foo> ", &h, ASCII, EditMode::Vi);
        assert_eq!(run_edit(ctx, &mut io), Ok("".to_string()));
    }

    macro_rules! vi_cmd_vec {
        ($str:expr) => {{
            let input_str = $str;
            let mut input_vec = Vec::with_capacity(input_str.len());
            input_vec.extend(input_str.as_bytes());
            input_vec
        }}
    }

    macro_rules! test_vi_cmds {
        ($cmd_str:expr, $result:expr) => {{
            let mut io = TestIO { input: vi_cmd_vec!($cmd_str), output: vec![] };
            let h = History::new();
            let ctx = EditCtx::new("foo> ", &h, ASCII, EditMode::Vi);
            assert_eq!(run_edit(ctx, &mut io), Ok($result.to_owned()));
        }}
    }

    /// Test some vi commands.
    #[test]
    fn vi_commands() {
        test_vi_cmds!("abc 123\x1bbdw\x0d", "abc ");
        test_vi_cmds!("Everything except the last char will be deleted.\x1bd0\x0d", ".");
        test_vi_cmds!("delete everything\x1b0d$\x0d", "");
        test_vi_cmds!("abc\x1b03r \x0d", "   ");
        test_vi_cmds!("this is test\x1b02ea a\x0d", "this is a test");
        test_vi_cmds!("this is a\x1b0A test\x0d", "this is a test");
        test_vi_cmds!("this is test\x1b02wia \x0d", "this is a test");
        test_vi_cmds!("this is test\x1b3hia \x0d", "this is a test");
        test_vi_cmds!("this is test\x1bhhhia \x0d", "this is a test");
        test_vi_cmds!("this is a test\x1b0ftx\x0d", "this is a est");
        test_vi_cmds!("this is a test\x1b0ttx\x0d", "this is atest");
        test_vi_cmds!("this is a test\x1b10x\x0d", "this is a tes");
        test_vi_cmds!("is a test\x1bIthis \x0d", "this is a test");
        test_vi_cmds!("this is a test\x1b0d2ti\x0d", "is a test");
        test_vi_cmds!("this is a test\x1b0d2fi\x0d", "s a test");
        test_vi_cmds!("this is a test\x1bdT \x0d", "this is a t");
        test_vi_cmds!("this is a test\x1bdF \x0d", "this is at");
        test_vi_cmds!("this is a test\x1bcF  real tes\x0d", "this is a real test");
        test_vi_cmds!("this is a test\x1bbbD\x0d", "this is ");
        test_vi_cmds!("this is a test\x1bbbCsome test\x0d", "this is some test");
        test_vi_cmds!("this is a test\x1bbbcwsome\x0d", "this is some test");
        test_vi_cmds!("these are some tests\x1bbbcwthe\x0d", "these are the tests");
        test_vi_cmds!("this is a  test\x1bbbcwsome\x0d", "this is some  test");
        test_vi_cmds!("this is a  test\x1bbhcwgood \x0d", "this is a good test");
        test_vi_cmds!("this is a test\x1bbcwthing\x0d", "this is a thing");
        test_vi_cmds!("delete everything\x1bdd\x0d", "");
        test_vi_cmds!("delete everything\x1bccchange everything\x0d", "change everything");
        test_vi_cmds!("this is a test\x1bbc2T some \x0d", "this is some test");
    }
}
