//! Lyrics in another script, in Latin letters: Japanese as romaji, Korean in
//! Revised Romanization, Chinese as pinyin, and everything else (Cyrillic,
//! Greek, Thai, Devanagari...) transliterated letter by letter. Latin text,
//! accents included, is left as it is.
//!
//! Japanese needs a dictionary to know how its kanji are read (IPADIC, built
//! into the app). It's only touched once a Japanese song is romanized, and is
//! read from the executable rather than copied into memory.

use std::borrow::Cow;
use std::sync::OnceLock;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use pinyin::ToPinyin;
use wana_kana::ConvertJapanese;

/// One romanized line per line given. A song with any kana is treated as
/// Japanese throughout, so its kanji-only lines aren't read as Chinese.
pub fn lines(lines: &[String]) -> Vec<String> {
    let japanese = lines.iter().any(|line| line.chars().any(is_kana));
    lines
        .iter()
        .map(|line| if japanese { japanese_line(line) } else { other(line) })
        .collect()
}

fn is_kana(c: char) -> bool {
    matches!(c, '\u{3040}'..='\u{30ff}' | '\u{31f0}'..='\u{31ff}' | '\u{ff66}'..='\u{ff9f}')
}

fn is_han(c: char) -> bool {
    matches!(c, '\u{4e00}'..='\u{9fff}' | '\u{3400}'..='\u{4dbf}' | '\u{f900}'..='\u{faff}' | '\u{3005}')
}

fn is_hangul(c: char) -> bool {
    matches!(c, '\u{ac00}'..='\u{d7a3}')
}

/// Left alone: ASCII, Latin letters with their accents, and general punctuation.
fn is_kept(c: char) -> bool {
    c.is_ascii()
        || matches!(c, '\u{00a0}'..='\u{024f}' | '\u{1e00}'..='\u{1eff}' | '\u{2000}'..='\u{206f}')
        || c.is_whitespace()
}

/// CJK punctuation as its Latin counterpart, and whether it opens (a space
/// goes before it) or closes (it sits against the word before).
fn cjk_punctuation(c: char) -> Option<(&'static str, bool)> {
    Some(match c {
        '「' | '『' | '“' => ("\"", true),
        '」' | '』' | '”' => ("\"", false),
        '（' | '(' => ("(", true),
        '）' | ')' => (")", false),
        '【' | '〔' | '［' => ("[", true),
        '】' | '〕' | '］' => ("]", false),
        '、' | '，' => (",", false),
        '。' | '．' => (".", false),
        '？' => ("?", false),
        '！' => ("!", false),
        '：' => (":", false),
        '；' => (";", false),
        '〜' | '～' => ("~", false),
        '…' => ("...", false),
        '・' | '\u{3000}' => (" ", false),
        _ => return None,
    })
}

// Everything but Japanese.

#[derive(PartialEq, Clone, Copy)]
enum Run {
    Kept,
    Hangul,
    Han,
    Other,
}

fn run_of(c: char) -> Run {
    if is_hangul(c) {
        Run::Hangul
    } else if is_han(c) {
        Run::Han
    } else if is_kept(c) || cjk_punctuation(c).is_some() {
        Run::Kept
    } else {
        Run::Other
    }
}

fn other(line: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let kind = run_of(chars[i]);
        let start = i;
        while i < chars.len() && run_of(chars[i]) == kind {
            i += 1;
        }
        let run: String = chars[start..i].iter().collect();
        match kind {
            Run::Kept => {
                for c in run.chars() {
                    match cjk_punctuation(c) {
                        Some((text, _)) => out.push_str(text),
                        None => out.push(c),
                    }
                }
            }
            Run::Hangul => push_word(&mut out, &korean(&run), chars.get(i)),
            Run::Han => push_word(&mut out, &chinese(&run), chars.get(i)),
            Run::Other => push_word(&mut out, &any_ascii::any_ascii(&run), chars.get(i)),
        }
    }
    out
}

/// Adds a converted run, spaced from letters it would otherwise run into.
fn push_word(out: &mut String, word: &str, next: Option<&char>) {
    if out.chars().last().is_some_and(char::is_alphanumeric) {
        out.push(' ');
    }
    out.push_str(word);
    if next.is_some_and(|c| c.is_alphanumeric() && run_of(*c) == Run::Kept) {
        out.push(' ');
    }
}

/// Pinyin with tone marks, a syllable per character.
fn chinese(run: &str) -> String {
    run.to_pinyin()
        .zip(run.chars())
        .map(|(pinyin, c)| pinyin.map_or_else(|| c.to_string(), |p| p.with_tone().to_string()))
        .collect::<Vec<_>>()
        .join(" ")
}

// Korean: Revised Romanization, syllable by syllable, with a final consonant
// carried over to a following syllable that starts silent (사랑을 → sarangeul).

const INITIALS: [&str; 19] = [
    "g", "kk", "n", "d", "tt", "r", "m", "b", "pp", "s", "ss", "", "j", "jj", "ch", "k", "t", "p", "h",
];
const VOWELS: [&str; 21] = [
    "a", "ae", "ya", "yae", "eo", "e", "yeo", "ye", "o", "wa", "wae", "oe", "yo", "u", "wo", "we", "wi", "yu", "eu",
    "ui", "i",
];
/// A final consonant at the end of a word or before another consonant.
const FINALS: [&str; 28] = [
    "", "k", "k", "k", "n", "n", "n", "t", "l", "k", "m", "l", "l", "l", "p", "l", "m", "p", "p", "t", "t", "ng", "t",
    "t", "k", "t", "p", "t",
];
/// A final consonant carried over to a silent initial, or `None` if it stays
/// put (ㅇ is only ever a final).
const CARRIED: [Option<&str>; 28] = [
    None,
    Some("g"),
    Some("kk"),
    Some("ks"),
    Some("n"),
    Some("nj"),
    Some("nh"),
    Some("d"),
    Some("r"),
    Some("lg"),
    Some("lm"),
    Some("lb"),
    Some("ls"),
    Some("lt"),
    Some("lp"),
    Some("lh"),
    Some("m"),
    Some("b"),
    Some("ps"),
    Some("s"),
    Some("ss"),
    None,
    Some("j"),
    Some("ch"),
    Some("k"),
    Some("t"),
    Some("p"),
    Some(""),
];
const SILENT_INITIAL: usize = 11;

fn korean(run: &str) -> String {
    let syllables: Vec<(usize, usize, usize)> = run
        .chars()
        .map(|c| {
            let index = c as usize - 0xac00;
            (index / (21 * 28), index % (21 * 28) / 28, index % 28)
        })
        .collect();
    let mut out = String::new();
    let mut carried: Option<&str> = None;
    for (i, &(initial, vowel, last)) in syllables.iter().enumerate() {
        match carried.take() {
            Some(consonant) => out.push_str(consonant),
            None => out.push_str(INITIALS[initial]),
        }
        out.push_str(VOWELS[vowel]);
        let next_silent = syllables.get(i + 1).is_some_and(|&(next, _, _)| next == SILENT_INITIAL);
        match CARRIED[last].filter(|_| next_silent) {
            Some(consonant) => carried = Some(consonant),
            None => out.push_str(FINALS[last]),
        }
    }
    out
}

// Japanese.

fn segmenter() -> Option<&'static Segmenter> {
    static SEGMENTER: OnceLock<Option<Segmenter>> = OnceLock::new();
    SEGMENTER
        .get_or_init(|| match load_dictionary("embedded://ipadic") {
            Ok(dictionary) => Some(Segmenter::new(Mode::Normal, dictionary, None)),
            Err(e) => {
                eprintln!("Couldn't load the Japanese dictionary: {e}");
                None
            }
        })
        .as_ref()
}

fn is_japanese(c: char) -> bool {
    is_kana(c) || is_han(c)
}

fn is_small_kana(c: char) -> bool {
    "ぁぃぅぇぉゃゅょゎァィゥェォャュョヮ".contains(c)
}

/// Whether a word joins the one before it ("shitteru", not "shi te ru"):
/// verb endings and helpers after a verb, suffixes, the "-te" particle, and the
/// spoken ん ("nanda").
fn attaches(pos: &[&str], previous: &Previous, surface: &str) -> bool {
    if surface.starts_with(is_small_kana) {
        return true;
    }
    let after_verb = matches!(previous.pos.as_str(), "動詞" | "助動詞") || previous.surface == "ん";
    match (pos.first().copied(), pos.get(1).copied()) {
        (Some("助動詞"), _) => after_verb,
        (Some("名詞"), Some("非自立")) => surface == "ん",
        (Some("動詞"), Some("非自立" | "接尾")) => true,
        (Some("形容詞"), Some("非自立")) => true,
        (Some("名詞"), Some("接尾")) => true,
        (Some("助詞"), Some("接続助詞")) => matches!(surface, "て" | "で" | "ちゃ" | "じゃ"),
        _ => false,
    }
}

fn japanese_line(line: &str) -> String {
    let mut out = String::new();
    for chunk in line.split_whitespace() {
        if !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
        if chunk.chars().any(is_japanese) {
            japanese_chunk(chunk, &mut out);
        } else {
            out.push_str(&other(chunk));
        }
    }
    out
}

fn japanese_chunk(chunk: &str, out: &mut String) {
    let Some(segmenter) = segmenter() else {
        out.push_str(&other(chunk));
        return;
    };
    let Ok(mut tokens) = segmenter.segment(Cow::Borrowed(chunk)) else {
        out.push_str(&other(chunk));
        return;
    };

    // Kana of the words joined so far, turned into romaji together so a small
    // っ carries over between them ("言っちゃう" → "itchau").
    let mut word = String::new();
    let mut previous = Previous::default();
    let mut space_next = !out.is_empty() && !out.ends_with(' ');
    let flush = |word: &mut String, out: &mut String| {
        if !word.is_empty() {
            out.push_str(&romaji(word));
            word.clear();
        }
    };

    for token in tokens.iter_mut() {
        let surface = token.surface.to_string();
        let details: Vec<String> = token.details().iter().map(|detail| detail.to_string()).collect();
        let pos: Vec<&str> = details.iter().map(String::as_str).collect();

        if let Some(c) = surface.chars().next().filter(|_| surface.chars().count() == 1) {
            if let Some((text, opens)) = cjk_punctuation(c) {
                flush(&mut word, out);
                if opens && !out.is_empty() && !out.ends_with(' ') {
                    out.push(' ');
                }
                out.push_str(text);
                space_next = !opens;
                previous = Previous::default();
                continue;
            }
        }

        let kana = particle(&pos, &surface).or_else(|| {
            details
                .get(7)
                .filter(|reading| reading.as_str() != "*" && !reading.is_empty())
                .cloned()
                .or_else(|| surface.chars().all(is_kana).then(|| surface.clone()))
        });
        let Some(kana) = kana else {
            // Not in the dictionary: Latin letters, numbers, or a rare kanji,
            // which stays as it is rather than being read the Chinese way.
            let opened = word.is_empty() && !space_next;
            flush(&mut word, out);
            if !opened && !out.is_empty() && !out.ends_with(' ') {
                out.push(' ');
            }
            if surface.chars().any(is_han) {
                out.push_str(&surface);
            } else {
                out.push_str(&other(&surface));
            }
            space_next = true;
            previous = Previous::from(&pos, &surface);
            continue;
        };

        if !word.is_empty() && !attaches(&pos, &previous, &surface) {
            flush(&mut word, out);
            space_next = true;
        }
        if word.is_empty() && space_next {
            out.push(' ');
        }
        word.push_str(&kana);
        space_next = false;
        previous = Previous::from(&pos, &surface);
    }
    flush(&mut word, out);
}

/// The word before, for deciding whether the next one joins it.
#[derive(Default)]
struct Previous {
    pos: String,
    surface: String,
}

impl Previous {
    fn from(pos: &[&str], surface: &str) -> Self {
        Self {
            pos: pos.first().copied().unwrap_or_default().to_string(),
            surface: surface.to_string(),
        }
    }
}

/// は, へ and を as particles are said "wa", "e" and "o".
fn particle(pos: &[&str], surface: &str) -> Option<String> {
    if pos.first() != Some(&"助詞") {
        return None;
    }
    Some(
        match surface {
            "は" => "ワ",
            "へ" => "エ",
            "を" => "オ",
            _ => return None,
        }
        .to_string(),
    )
}

/// The sounds loanwords spell with a small vowel, which wana_kana gets wrong
/// ("フィクション" → "fyikushon"). Longest first, so ヴァ is matched before ヴ.
const LOANWORD_SOUNDS: [(&str, &str); 26] = [
    ("ファ", "fa"),
    ("フィ", "fi"),
    ("フェ", "fe"),
    ("フォ", "fo"),
    ("フュ", "fyu"),
    ("ティ", "ti"),
    ("ディ", "di"),
    ("トゥ", "tu"),
    ("ドゥ", "du"),
    ("テュ", "tyu"),
    ("デュ", "dyu"),
    ("ウィ", "wi"),
    ("ウェ", "we"),
    ("ウォ", "wo"),
    ("ヴァ", "va"),
    ("ヴィ", "vi"),
    ("ヴェ", "ve"),
    ("ヴォ", "vo"),
    ("ヴュ", "vyu"),
    ("ツァ", "tsa"),
    ("ツィ", "tsi"),
    ("ツェ", "tse"),
    ("ツォ", "tso"),
    ("イェ", "ye"),
    ("クァ", "kwa"),
    ("ヴ", "vu"),
];

/// Katakana or hiragana to romaji, with a long-vowel mark written as the vowel
/// again ("コーヒー" → "koohii").
fn romaji(kana: &str) -> String {
    // Hiragana spellings of the same sounds (ふぃ) are rare, but read the same.
    let mut kana: String = kana
        .chars()
        .map(|c| match c {
            'ぁ'..='ゖ' => char::from_u32(c as u32 + 0x60).unwrap_or(c),
            _ => c,
        })
        .collect();
    for (sound, latin) in LOANWORD_SOUNDS {
        kana = kana.replace(sound, latin);
    }
    let converted = kana.to_romaji();
    let mut out = String::with_capacity(converted.len());
    for c in converted.chars() {
        match (c, out.chars().last()) {
            ('-' | 'ー', Some(vowel @ ('a' | 'i' | 'u' | 'e' | 'o'))) => out.push(vowel),
            _ => out.push(c),
        }
    }
    out
}
