pub mod binary;
pub mod fast;
pub mod slow;
mod tests;
pub mod ty;
mod varint;

pub type JsonValue = serde_json::Value;

fn main() {
    println!("Hello world");
}
