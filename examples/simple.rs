extern crate copperline;

use copperline::Copperline;

fn main() {
    let mut cl = Copperline::new();
    while let Ok(line) = cl.read_line(">> ") {
        println!("Line: {}", line);
        cl.add_history(line);
    }
}
