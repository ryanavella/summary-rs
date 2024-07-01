# summary

Extract the sentences which best summarize a document.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/ryanavella/summary-rs/blob/master/LICENSE-MIT) [![License: Apache 2.0](https://img.shields.io/badge/license-Apache-blue.svg)](https://github.com/ryanavella/summary-rs/blob/master/LICENSE-APACHE) [![crates.io](https://img.shields.io/crates/v/summary.svg?colorB=319e8c)](https://crates.io/crates/summary) [![docs.rs](https://img.shields.io/badge/docs.rs-summary-yellowgreen)](https://docs.rs/summary)

## Example

```rust
let summarizer = Summarizer::new(Language::English);
let text = "See Spot. See Spot run. Run Spot, run!";
let n = 2.try_into().unwrap();
for sentence in summarizer.summarize_sentences(text, n) {
    println!("{sentence}");
}
```
