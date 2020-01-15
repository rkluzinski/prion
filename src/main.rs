use std::env;
use std::process;

use prion::Config;

fn main() {
    let config = Config::new(env::args()).unwrap_or_else(|err| {
        println!("Failed to parse arguments: {}", err);
        process::exit(1);
    });

    if let Err(e) = prion::run(config) {
        println!("Application error: {}", e);
        process::exit(1);
    }
}