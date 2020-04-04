extern crate bar;
extern crate foo;

use bar;
use bar::car::*;
use foo::{self, quix};

fn main() {
    println!("Hello {}", "World");

    panic!("Goodbye")
}
