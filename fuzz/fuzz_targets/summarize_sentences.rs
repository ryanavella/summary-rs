#![no_main]
use libfuzzer_sys::fuzz_target;
use std::num::NonZeroU8;

fuzz_target!(|x: (&[u8], NonZeroU8)| {
    let (data, lines) = x;
    if let Ok(s) = std::str::from_utf8(data) {
        if u32::try_from(s.len()).is_ok() {
            use summary::{Language, Summarizer};

            let summarizer = Summarizer::new(Language::English);
            let summary = summarizer.summarize_sentences(s, lines.into());
            let _ = std::hint::black_box(summary);
        }
    }
});
