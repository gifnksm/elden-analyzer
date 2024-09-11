//! Analyze videos of Eldenring playing and extract information.
//!
//! # Usage
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! elden-analyzer = "0.0.0"
//! ```

#![doc(html_root_url = "https://docs.rs/elden-analyzer/0.0.0")]

use clap::Parser;

#[derive(clap::Parser)]
pub struct App {}

pub fn main() {
    let _args = App::parse();
}
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
