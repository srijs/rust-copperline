extern crate copperline;

use copperline::Copperline;

fn main() {
    let mut cl = Copperline::new();
    let readline = cl.readline(">> ");
    match readline {
        Ok(line) => println!("Line: {}",line),
        Err(err) => println!("Error: {:?}", err)
    }
}
