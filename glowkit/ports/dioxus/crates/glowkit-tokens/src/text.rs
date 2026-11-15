//! Framework-agnostic text timelines and formatting shared by the text components:
//! the typewriter timeline, number formatting for count-ups and token splitting for streaming.

/// Where the typewriter is in its timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeState {
    /// Which string is showing.
    pub index: usize,
    /// How many characters of it are visible.
    pub chars: usize,
    /// True while characters are being typed or deleted (cursor stays solid).
    pub busy: bool,
}

/// Timing for [`type_state`].
#[derive(Clone, Copy, Debug)]
pub struct TypeTiming {
    pub type_cps: f32,
    pub delete_cps: f32,
    pub hold: f32,
    pub gap: f32,
    pub looping: bool,
}

/// Pure typewriter timeline: for each string, type it, hold, delete it, wait, then the next one.
/// Without looping the last string stays typed.
pub fn type_state(t: f32, lens: &[usize], tm: TypeTiming) -> TypeState {
    if lens.is_empty() {
        return TypeState { index: 0, chars: 0, busy: false };
    }
    let ty = |n: usize| n as f32 / tm.type_cps.max(0.1);
    let de = |n: usize| n as f32 / tm.delete_cps.max(0.1);
    let cycle: f32 = lens.iter().map(|&n| ty(n) + tm.hold + de(n) + tm.gap).sum();
    let mut t = t.max(0.0);
    if tm.looping {
        t %= cycle.max(1e-3);
    }
    for (i, &n) in lens.iter().enumerate() {
        let last = i + 1 == lens.len();
        if t < ty(n) {
            return TypeState { index: i, chars: ((t * tm.type_cps) as usize).min(n), busy: true };
        }
        t -= ty(n);
        if !tm.looping && last {
            return TypeState { index: i, chars: n, busy: false };
        }
        if t < tm.hold {
            return TypeState { index: i, chars: n, busy: false };
        }
        t -= tm.hold;
        if t < de(n) {
            let gone = (t * tm.delete_cps) as usize;
            return TypeState { index: i, chars: n.saturating_sub(gone), busy: true };
        }
        t -= de(n);
        if t < tm.gap {
            return TypeState { index: i, chars: 0, busy: false };
        }
        t -= tm.gap;
    }
    let i = lens.len() - 1;
    TypeState { index: i, chars: 0, busy: false }
}


/// Format `value` with `decimals` fraction digits, a thousands `separator` (may be empty) and a
/// `decimal_point`. Negative zero is printed as zero.
///
/// ```
/// assert_eq!(glowkit_tokens::text::format_number(1234567.891, 2, ",", "."), "1,234,567.89");
/// assert_eq!(glowkit_tokens::text::format_number(-9876.6, 0, " ", "."), "-9 877");
/// ```
pub fn format_number(value: f64, decimals: usize, separator: &str, decimal_point: &str) -> String {
    let s = format!("{:.*}", decimals, value.abs());
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (s.as_str(), None),
    };
    let is_zero = s.bytes().all(|b| b == b'0' || b == b'.');
    let mut out = String::with_capacity(s.len() + 8);
    if value < 0.0 && !is_zero {
        out.push('-');
    }
    let n = int.len();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (n - i) % 3 == 0 {
            out.push_str(separator);
        }
        out.push(c);
    }
    if let Some(f) = frac {
        out.push_str(decimal_point);
        out.push_str(f);
    }
    out
}

/// Split text into token-ish chunks: a word plus its trailing whitespace.
pub fn token_ends(text: &str) -> Vec<usize> {
    // char index (exclusive) where each token ends
    let mut ends = Vec::new();
    let mut prev_space = false;
    let mut count = 0;
    for (i, c) in text.chars().enumerate() {
        let space = c.is_whitespace();
        if !space && prev_space {
            ends.push(i);
        }
        prev_space = space;
        count = i + 1;
    }
    if count > 0 {
        ends.push(count);
    }
    ends
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typewriter_timeline() {
        let tm = TypeTiming { type_cps: 10.0, delete_cps: 20.0, hold: 1.0, gap: 0.5, looping: true };
        assert_eq!(type_state(0.0, &[5, 3], tm).chars, 0);
        assert_eq!(type_state(0.25, &[5, 3], tm).chars, 2);
        let s = type_state(0.75, &[5, 3], tm);
        assert_eq!((s.index, s.chars, s.busy), (0, 5, false));
    }

    #[test]
    fn tokens() {
        assert_eq!(token_ends("hi there you"), vec![3, 9, 12]);
    }
}
