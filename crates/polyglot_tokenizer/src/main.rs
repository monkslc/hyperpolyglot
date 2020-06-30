use std::{
    env,
    fs::File,
    io::{ErrorKind, Read},
};

use polyglot_tokenizer::Tokenizer;

fn main() {
    if let Some(file_name) = env::args().skip(1).next() {
        match File::open(&file_name) {
            Ok(mut file) => {
                let mut content = String::new();
                match file.read_to_string(&mut content) {
                    Ok(_) => Tokenizer::new(&content[..]).tokens().for_each(|token| {
                        println!("{:?}", token);
                    }),
                    Err(e) => println!("Error reading file: {}", e),
                }
            }
            Err(e) => match e.kind() {
                ErrorKind::NotFound => {
                    println!("File {} not found", file_name);
                }
                _ => println!("Error opening file: {}", e),
            },
        }
    } else {
        println!("Filename not provided");
    }
}
