use std::io::{self, Write, BufRead};
use crate::error::Result;

pub fn get_user_input(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub fn get_multiline_input(prompt: &str) -> Result<String> {
    println!("{}  (Type 'END' on a new line to finish)", prompt);
    io::stdout().flush()?;
    
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut buffer = String::new();
    let mut lines = Vec::new();
    
    while reader.read_line(&mut buffer)? > 0 {
        let line = buffer.trim_end();
        if line == "END" {
            break;
        }
        // Skip empty lines and single character lines (likely stray characters)
        if !line.is_empty() && line.len() > 1 {
            lines.push(line.to_string());
        }
        buffer.clear();
    }
    
    Ok(lines.join("\n"))
}