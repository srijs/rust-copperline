use error::Error;
use edit::{EditCtx, EditResult, edit};

pub trait RunIO {
    fn write(&mut self, Vec<u8>) -> Result<(), Error>;
    fn read_byte(&mut self) -> Result<u8, Error>;
}


pub fn run<'a>(mut ctx: EditCtx<'a>, io: &mut RunIO) -> Result<String, Error> {
    loop {
        match edit(&mut ctx) {
            EditResult::Cont(line) => {
                try!(io.write(line));
                let byte = try!(io.read_byte());
                ctx.fill(byte);
            },
            EditResult::Halt(res) => { return res; }
        }
    }
}

#[cfg(test)]
mod test {
    use encoding::all::ASCII;
    use super::super::error::Error;
    use super::super::edit::EditCtx;
    use super::super::history::History;
    use super::{RunIO, run};

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
    }

    #[test]
    fn error_eof_on_empty_input() {
        let mut io = TestIO { input: vec![], output: vec![] };
        let h = History::new();
        let ctx = EditCtx::new("foo> ", &h, ASCII);
        assert_eq!(run(ctx, &mut io), Err(Error::EndOfFile));
    }

    #[test]
    fn ok_abc_with_return() {
        let mut io = TestIO { input: vec![65, 66, 67, 13], output: vec![] };
        let h = History::new();
        let ctx = EditCtx::new("foo> ", &h, ASCII);
        assert_eq!(run(ctx, &mut io), Ok("ABC".to_string()));
    }

}
