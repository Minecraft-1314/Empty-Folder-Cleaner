use std::collections::HashSet;

use regex::Regex;

use super::path_guard;

const MAX_PATTERN_LENGTH: usize = 260;
const MAX_PATTERN_COUNT: usize = 260;

pub struct IgnoreMatcher {
    ignored_paths: HashSet<String>,
    patterns: Vec<Regex>,
}

impl IgnoreMatcher {
    pub fn new(ignored_paths: &[String], patterns: &[String]) -> Self {
        let normalized: HashSet<String> = ignored_paths
            .iter()
            .filter(|path| !path.trim().is_empty())
            .map(|path| path.to_lowercase())
            .collect();
        let mut compiled: Vec<Regex> = Vec::new();
        for pattern in patterns.iter().take(MAX_PATTERN_COUNT) {
            let trimmed = pattern.trim();
            if !is_usable_pattern(trimmed) {
                continue;
            }
            if let Ok(regex) = Regex::new(&wildcard_to_regex(trimmed)) {
                compiled.push(regex);
            }
        }
        Self {
            ignored_paths: normalized,
            patterns: compiled,
        }
    }

    pub fn is_ignored(&self, path: &str) -> bool {
        if path.is_empty() {
            return false;
        }

        if !self.ignored_paths.is_empty()
            && self.ignored_paths.contains(&normalize(path).to_lowercase())
        {
            return true;
        }

        self.matches_name(last_segment(path))
    }

    pub fn matches_name(&self, name: &str) -> bool {
        if name.is_empty() || self.patterns.is_empty() {
            return false;
        }

        self.patterns.iter().any(|pattern| pattern.is_match(name))
    }
}

pub fn is_usable_pattern(pattern: &str) -> bool {
    let length = pattern.chars().count();
    length > 0 && length <= MAX_PATTERN_LENGTH && !pattern.chars().any(char::is_control)
}

fn normalize(path: &str) -> String {
    path_guard::try_normalize(Some(path)).unwrap_or_else(|| path.trim().to_string())
}

fn last_segment(path: &str) -> &str {
    let trimmed = path.trim_end_matches(['\\', '/']);
    match trimmed.rfind(['\\', '/']) {
        Some(index) => &trimmed[index + 1..],
        None => trimmed,
    }
}

fn wildcard_to_regex(wildcard: &str) -> String {
    let mut builder = String::with_capacity(wildcard.len() + 8);
    builder.push_str("(?i)^");
    for character in wildcard.chars() {
        match character {
            '*' => builder.push_str(".*"),
            '?' => builder.push('.'),
            other => builder.push_str(&regex::escape(&other.to_string())),
        }
    }
    builder.push('$');
    builder
}
