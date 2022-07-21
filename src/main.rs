use kat::{parse_config, run};
use std::env::args;
use std::process::exit;

fn main() {
    let cmd_args = args().collect();
    if let Err(e) = parse_config(cmd_args).and_then(run) {
        eprintln!("{}", e);
        exit(1);
    }
}

/// Sample test mod to verify running tests in main.rs works fine!
#[cfg(test)]
mod main_tests {
    use crate::main;

    fn test_main() {
        main();
    }
}
