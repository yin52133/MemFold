pub fn is_memory_noise(summary: &str) -> bool {
    let normalized = normalize_summary(summary);
    if normalized.is_empty() {
        return true;
    }

    let lowered = normalized.to_ascii_lowercase();
    if lowered == "session exited via launcher trap"
        || lowered == "manual hook check from codex"
        || lowered.starts_with("memfold verify turn ")
        || lowered.starts_with("verify turn ")
    {
        return true;
    }

    false
}

pub fn normalize_summary(summary: &str) -> String {
    summary.split_whitespace().collect::<Vec<_>>().join(" ").trim().to_string()
}
