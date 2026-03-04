//! CLI entry point for Llama-RS (64-bit release binary).

use llama_rs::hello_llama_rust;

fn main() {
    println!("{}", hello_llama_rust());
}
