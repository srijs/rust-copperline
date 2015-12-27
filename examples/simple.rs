extern crate copperline;

use copperline::Copperline;

fn main() {

    let cfg = copperline::Config {
        encoding: copperline::Encoding::Utf8,
        mode: copperline::EditMode::Vi
    };

    let mut cl = Copperline::new();
    while let Ok(line) = cl.read_line(">> ", &cfg) {
        println!("Line: {}", line);
        cl.add_history(line);
    }
}
