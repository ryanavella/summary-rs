#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|x: (&[u8], u32)| {
    let (data, ratio) = x;
    let ratio = f64::from(ratio) / f64::from(u32::MAX);
    if let Ok(s) = std::str::from_utf8(data) {
        if u32::try_from(s.len()).is_ok() {
            use summary::{Language, Summarizer};

            let summarizer = Summarizer::new(Language::English);
            let summary = summarizer.summarize_ratio(s, ratio);
            let _ = std::hint::black_box(summary);
        }
    }
});
