extern crate evmplay;

use evmplay::contract::RawContract;

pub fn main() {
    let raw = RawContract::new("./contracts/simple.sol");
    let mut compiled = raw.compile();
    compiled.deploy();
    compiled.call(&"hello", &vec![]);
}