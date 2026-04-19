pub fn estimate_tokens(text: &str) -> usize {
    let mut ascii_words = 0usize;
    let mut in_ascii_word = false;
    let mut non_ascii_alnum = 0usize;

    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            if !in_ascii_word {
                ascii_words += 1;
                in_ascii_word = true;
            }
            continue;
        }

        in_ascii_word = false;
        if ch.is_alphanumeric() {
            non_ascii_alnum += 1;
        }
    }

    let non_ascii_tokens = non_ascii_alnum.div_ceil(2);
    let total = ascii_words + non_ascii_tokens;
    if total == 0 {
        text.split_whitespace().count().max(1)
    } else {
        total
    }
}
