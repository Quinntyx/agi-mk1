use std::io::{Write, stdout};

pub fn println (a: &str) {
    println!("{}", a);
    stdout().flush().unwrap();
}

pub fn print (a: &str) {
    print!("{}", a);
    stdout().flush().unwrap();
}