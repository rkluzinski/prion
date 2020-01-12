use std::env;
use std::process;

use prion::Config;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let config = Config::new(&args).unwrap_or_else(|err| {
        println!("Failed to parse arguments: {}", err);
        process::exit(1);
    });

    if let Err(e) = prion::run(config) {
        println!("Application error: {}", e);
        process::exit(1);
    }
}