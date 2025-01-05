use std::io;
use std::io::{Read, Write};

pub fn cap_input() -> String {

    let mut output = String::new();
    
    io::stdin().read_line(&mut output).expect("Failed to Capture User Input");

    output.trim().to_string()
    
}

pub fn clear_screen() {
    io::stdout().flush().unwrap();
}