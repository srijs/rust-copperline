extern crate copperline;

use copperline::Copperline;

fn main() {
    let mut cl = Copperline::new();
    while let Ok(line) = cl.readline(">> ") {
        println!("Line: {}", line);
        cl.history_add(line);
    }
}
