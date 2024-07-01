//! Extract the sentences which best summarize a document.
//! 
//! The algorithm uses a heuristic which identifies a "core" sentence
//! based on tf-idf cosine distance to the document at large,
//! and then gathers all sentences that have small cosine distances
//! to the "core" sentence.
//! 
//! # Example
//! 
//! ```rust
//! # use summary::{Language, Summarizer};
//! let summarizer = Summarizer::new(Language::English);
//! let text = "See Spot. See Spot run. Run Spot, run!";
//! let n = 2.try_into().unwrap();
//! for sentence in summarizer.summarize_sentences(text, n) {
//!     println!("{sentence}");
//! }
//! ```
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    num::NonZeroU32,
};

use unicode_segmentation::UnicodeSegmentation;

type IdfMap = HashMap<Box<str>, f64>;

/// Document summarizer.
pub struct Summarizer {
    stemmer: Stemmer,
    stop_words: StopWords,
}

impl Summarizer {
    /// Create a new `Summarizer`.
    #[must_use]
    pub fn new(language: Language) -> Self {
        let stemmer = Stemmer::new(language);
        let stop_words = StopWords::new(language);
        Self {
            stemmer,
            stop_words,
        }
    }

    /// Create a new `Summarizer` that is language agnostic.
    pub fn new_language_agnostic() -> Self {
        let stemmer = Stemmer(None);
        let stop_words = StopWords(HashSet::new());
        Self {
            stemmer,
            stop_words,
        }
    }

    #[inline(never)] // discourage monomorphization bloat
    fn summarize_indices<'a>(&self, text: &'a str) -> (Vec<&'a str>, Vec<u32>) {
        assert!(
            u32::try_from(text.len()).is_ok(),
            "can not summarize texts longer than 4 GiB"
        );

        let Self {
            stemmer,
            stop_words,
        } = self;

        let sentences = sentences(text);
        if sentences.is_empty() {
            return Default::default();
        }
        let idfs = idfs(&sentences, stop_words, stemmer);
        let tf_idfs = tf_idfs(&sentences, &idfs, stop_words, stemmer);
        let overall = tf_idf(&sentences, &idfs, stop_words, stemmer);

        let i = tf_idfs
            .iter()
            .enumerate()
            .map(|(i, tf_idf)| (i, OrdFloat(cosine_compare(tf_idf, &overall))))
            .max_by_key(|(_, x)| *x)
            .unwrap()
            .0;

        let best_match = &tf_idfs[i];

        let mut indices: Vec<_> = (0..u32::try_from(tf_idfs.len()).unwrap()).collect();
        indices.sort_unstable_by_key(|&i| {
            let i = usize::try_from(i).unwrap();
            let tf_idf = &tf_idfs[i];
            OrdFloat(-cosine_compare(tf_idf, best_match))
        });

        (sentences, indices)
    }

    /// Provide a summary for the text, reduced by a given ratio.
    ///
    /// The ratio is applied to the byte-wise length of the text.
    /// An attempt will be made to return a summary that is
    /// as close to the ratio as possible without exceeding it.
    /// However if this would result in 0 sentences,
    /// the summary is rounded up to 1 sentence.
    ///
    /// # Panics
    ///
    /// Panics if the provided text is longer than 4 GiB,
    /// or if the provided ratio is not in `0.0..=1.0`.
    #[must_use]
    pub fn summarize_ratio<'a>(&self, text: &'a str, ratio: f64) -> Vec<&'a str> {
        assert!((0.0..=1.0).contains(&ratio));
        let (sentences, mut indices) = self.summarize_indices(text);
        if sentences.is_empty() {
            return Vec::new();
        }

        let target = (ratio * (text.len() as f64)).round() as usize;
        let mut total_len = 0;
        let end = indices
            .iter()
            .enumerate()
            .find_map(|(i, &j)| {
                let j = usize::try_from(j).unwrap();
                total_len += sentences[j].trim_end().len() + 1;
                if total_len > target {
                    Some(i)
                } else {
                    None
                }
            })
            .unwrap_or(indices.len())
            .max(1);
        indices.truncate(end);

        summarize_impl(sentences, indices)
    }

    /// Provide a `n` sentence summary for the text.
    ///
    /// If the text is not longer than `n` sentences,
    /// the entire text is returned.
    ///
    /// # Panics
    ///
    /// Panics if the provided text is longer than 4 GiB.
    #[must_use]
    pub fn summarize_sentences<'a>(&self, text: &'a str, n: NonZeroU32) -> Vec<&'a str> {
        let (sentences, mut indices) = self.summarize_indices(text);
        if sentences.is_empty() {
            return Vec::new();
        }
        indices.truncate(n.get().try_into().unwrap());
        summarize_impl(sentences, indices)
    }
}

struct Stemmer(Option<rust_stemmers::Stemmer>);

impl Stemmer {
    fn new(language: Language) -> Self {
        use rust_stemmers::Algorithm;

        #[rustfmt::skip]
        let algo = match language {
            Language::Arabic     => Algorithm::Arabic,
            Language::Danish     => Algorithm::Danish,
            Language::Dutch      => Algorithm::Dutch,
            Language::English    => Algorithm::English,
            Language::Finnish    => Algorithm::Finnish,
            Language::French     => Algorithm::French,
            Language::German     => Algorithm::German,
            Language::Greek      => Algorithm::Greek,
            Language::Hungarian  => Algorithm::Hungarian,
            Language::Italian    => Algorithm::Italian,
            Language::Norwegian  => Algorithm::Norwegian,
            Language::Portuguese => Algorithm::Portuguese,
            Language::Romanian   => Algorithm::Romanian,
            Language::Russian    => Algorithm::Russian,
            Language::Spanish    => Algorithm::Spanish,
            Language::Swedish    => Algorithm::Swedish,
            Language::Tamil      => Algorithm::Tamil,
            Language::Turkish    => Algorithm::Turkish,
            _ => {
                return Self(None);
            }
        };
        Self(Some(rust_stemmers::Stemmer::create(algo)))
    }

    fn stem(&self, s: &str) -> Box<str> {
        let tmp: Cow<str>;
        let s = if let Some(stemmer) = &self.0 {
            tmp = stemmer.stem(s);
            &tmp
        } else {
            s
        };
        s.to_lowercase().into_boxed_str()
    }
}

#[derive(Default)]
struct StopWords(HashSet<Box<str>>);

impl StopWords {
    fn new(language: Language) -> Self {
        use stop_words::LANGUAGE as Dict;

        #[rustfmt::skip]
        let lang = match language {
            Language::Afrikaans  => Dict::Afrikaans,
            Language::Arabic     => Dict::Arabic,
            Language::Armenian   => Dict::Armenian,
            Language::Basque     => Dict::Basque,
            Language::Bengali    => Dict::Bengali,
            Language::Breton     => Dict::Breton,
            Language::Bulgarian  => Dict::Bulgarian,
            Language::Catalan    => Dict::Catalan,
            Language::Chinese    => Dict::Chinese,
            Language::Croatian   => Dict::Croatian,
            Language::Czech      => Dict::Czech,
            Language::Danish     => Dict::Danish,
            Language::Dutch      => Dict::Dutch,
            Language::English    => Dict::English,
            Language::Esperanto  => Dict::Esperanto,
            Language::Estonian   => Dict::Estonian,
            Language::Finnish    => Dict::Finnish,
            Language::French     => Dict::French,
            Language::Galician   => Dict::Galician,
            Language::German     => Dict::German,
            Language::Greek      => Dict::Greek,
            Language::Gujarati   => Dict::Gujarati,
            Language::Hausa      => Dict::Hausa,
            Language::Hebrew     => Dict::Hebrew,
            Language::Hindi      => Dict::Hindi,
            Language::Hungarian  => Dict::Hungarian,
            Language::Indonesian => Dict::Indonesian,
            Language::Irish      => Dict::Irish,
            Language::Italian    => Dict::Italian,
            Language::Japanese   => Dict::Japanese,
            Language::Korean     => Dict::Korean,
            Language::Kurdish    => Dict::Kurdish,
            Language::Latin      => Dict::Latin,
            Language::Latvian    => Dict::Latvian,
            Language::Lithuanian => Dict::Lithuanian,
            Language::Malay      => Dict::Malay,
            Language::Marathi    => Dict::Marathi,
            Language::Norwegian  => Dict::Norwegian,
            Language::Persian    => Dict::Persian,
            Language::Polish     => Dict::Polish,
            Language::Portuguese => Dict::Portuguese,
            Language::Romanian   => Dict::Romanian,
            Language::Russian    => Dict::Russian,
            Language::Slovak     => Dict::Slovak,
            Language::Slovenian  => Dict::Slovenian,
            Language::Somali     => Dict::Somali,
            Language::Sotho      => Dict::Sotho,
            Language::Spanish    => Dict::Spanish,
            Language::Swahili    => Dict::Swahili,
            Language::Swedish    => Dict::Swedish,
            Language::Tagalog    => Dict::Tagalog,
            Language::Thai       => Dict::Thai,
            Language::Ukrainian  => Dict::Ukrainian,
            Language::Urdu       => Dict::Urdu,
            Language::Vietnamese => Dict::Vietnamese,
            Language::Yoruba     => Dict::Yoruba,
            Language::Zulu       => Dict::Zulu,
            Language::Turkish    => Dict::Turkish,
            Language::Tamil      => return Self(HashSet::default()),
        };
        let set = stop_words::get(lang)
            .into_iter()
            .map(|x| x.to_lowercase().into_boxed_str())
            .collect();
        Self(set)
    }

    fn contains(&self, s: &str) -> bool {
        let s = s.to_lowercase();
        self.0.contains(&*s)
    }
}

/// A document's language.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub enum Language {
    Afrikaans,
    Arabic,
    Armenian,
    Basque,
    Bengali,
    Breton,
    Bulgarian,
    Catalan,
    Chinese,
    Croatian,
    Czech,
    Danish,
    Dutch,
    English,
    Esperanto,
    Estonian,
    Finnish,
    French,
    Galician,
    German,
    Greek,
    Gujarati,
    Hausa,
    Hebrew,
    Hindi,
    Hungarian,
    Indonesian,
    Irish,
    Italian,
    Japanese,
    Korean,
    Kurdish,
    Latin,
    Latvian,
    Lithuanian,
    Malay,
    Marathi,
    Norwegian,
    Persian,
    Polish,
    Portuguese,
    Romanian,
    Russian,
    Slovak,
    Slovenian,
    Somali,
    Sotho,
    Spanish,
    Swahili,
    Swedish,
    Tagalog,
    Tamil,
    Thai,
    Turkish,
    Ukrainian,
    Urdu,
    Vietnamese,
    Yoruba,
    Zulu,
}

#[inline(never)] // discourage monomorphization bloat
fn summarize_impl(mut sentences: Vec<&str>, mut indices: Vec<u32>) -> Vec<&str> {
    indices.sort_unstable();
    let end = *indices.last().unwrap() + 1;
    sentences.truncate(end.try_into().unwrap());

    let mut indices = &*indices;
    let mut i = 0;
    sentences.retain(|_| {
        let keep = if i == indices[0] {
            indices = &indices[1..];
            true
        } else {
            false
        };
        i += 1;
        keep
    });
    sentences
}

fn sentences(text: &str) -> Vec<&str> {
    text.unicode_sentences().collect()
}

fn tf_idfs(
    sentences: &[&str],
    idfs: &IdfMap,
    stop_words: &StopWords,
    stemmer: &Stemmer,
) -> Vec<IdfMap> {
    sentences
        .iter()
        .copied()
        .map(|sentence| tf_idf(&[sentence], idfs, stop_words, stemmer))
        .collect()
}

#[derive(Clone, Copy, PartialEq)]
#[repr(transparent)]
struct OrdFloat(f64);

impl Eq for OrdFloat {}

impl PartialOrd for OrdFloat {
    fn partial_cmp(&self, rhs: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(rhs))
    }
}

impl Ord for OrdFloat {
    fn cmp(&self, rhs: &Self) -> core::cmp::Ordering {
        self.0.total_cmp(&rhs.0)
    }
}

fn cosine_compare(a: &IdfMap, b: &IdfMap) -> f64 {
    let mut dotprod = 0.0;
    for (word, x) in a {
        if let Some(y) = b.get(word) {
            dotprod += x * y;
        }
    }
    // The inputs are already normalized into unit vectors,
    // so the dot product is identical to the cosine similarity.
    dotprod
}

fn tf_idf(sentences: &[&str], idfs: &IdfMap, stop_words: &StopWords, stemmer: &Stemmer) -> IdfMap {
    let mut word_counts = HashMap::<_, u32>::new();
    let words = sentences.iter().flat_map(|s| s.unicode_words());
    for word in words {
        if stop_words.contains(word) {
            continue;
        }
        let word = stemmer.stem(word);
        *word_counts.entry(word).or_default() += 1;
    }
    let mut idf_map: IdfMap = word_counts
        .into_iter()
        .map(|(word, tf)| {
            let tf = f64::from(tf);
            let idf = *idfs.get(&word).unwrap_or(&0.0);
            let tf_idf = tf * idf;
            (word, tf_idf)
        })
        .collect();
    let mag = idf_map.values().map(|x| x * x).sum::<f64>().sqrt();
    for v in idf_map.values_mut() {
        *v /= mag;
    }
    idf_map
}

fn idfs(sentences: &[&str], stop_words: &StopWords, stemmer: &Stemmer) -> IdfMap {
    let n = f64::from(u32::try_from(sentences.len()).unwrap());
    let mut word_counts = HashMap::<_, u32>::new();
    for sentence in sentences {
        let mut set = HashSet::new();
        for word in sentence.unicode_words() {
            if stop_words.contains(word) {
                continue;
            }
            let word = stemmer.stem(word);
            set.insert(word);
        }
        for word in set {
            *word_counts.entry(word).or_default() += 1;
        }
    }
    word_counts
        .into_iter()
        .map(|(word, count)| {
            let idf = (n / f64::from(count)).log2();
            (word, idf)
        })
        .collect()
}
