const MAX_PATH_LENGTH: usize = 32767;

pub fn is_separator(character: char) -> bool {
    character == '\\' || character == '/'
}

pub fn try_normalize(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > MAX_PATH_LENGTH
        || trimmed.chars().any(char::is_control)
        || has_device_namespace(trimmed)
        || !is_rooted_candidate(trimmed)
    {
        return None;
    }

    let full = full_path(trimmed)?;
    if full.is_empty() || full.chars().count() > MAX_PATH_LENGTH {
        return None;
    }

    Some(trim_ending_separator(&full))
}

pub fn try_normalize_network_share(value: Option<&str>) -> Option<String> {
    let candidate = try_normalize(value)?;
    if !candidate.starts_with("\\\\") {
        return None;
    }

    let segments: Vec<&str> = candidate
        .split(['\\', '/'])
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.len() < 2 || segments.iter().any(|segment| segment.trim().is_empty()) {
        return None;
    }

    Some(candidate)
}

pub fn is_within_root(root_directory: &str, candidate_path: &str) -> bool {
    if root_directory.trim().is_empty() || candidate_path.trim().is_empty() {
        return false;
    }

    let Some(root) = try_normalize(Some(root_directory)) else {
        return false;
    };
    let Some(candidate) = try_normalize(Some(candidate_path)) else {
        return false;
    };

    if equals_ignore_case(&root, &candidate) {
        return true;
    }

    let prefix = if root.ends_with('\\') || root.ends_with('/') {
        root.clone()
    } else {
        format!("{root}\\")
    };
    starts_with_ignore_case(&candidate, &prefix)
}

pub fn is_same_path(left: &str, right: &str) -> bool {
    match (try_normalize(Some(left)), try_normalize(Some(right))) {
        (Some(left), Some(right)) => equals_ignore_case(&left, &right),
        _ => false,
    }
}

pub fn get_parent(path: &str) -> String {
    let trimmed = path.trim_end_matches(['\\', '/']);
    if trimmed.is_empty() {
        return String::new();
    }

    if is_root_path(trimmed) {
        return String::new();
    }

    match trimmed.rfind(['\\', '/']) {
        Some(0) => String::new(),
        Some(index) => {
            let parent = &trimmed[..index];
            if parent.len() == 2 && parent.ends_with(':') {
                format!("{parent}\\")
            } else if parent.starts_with("\\\\") && parent[2..].contains(['\\', '/']) {
                parent.to_string()
            } else if parent.starts_with("\\\\") {
                String::new()
            } else {
                parent.to_string()
            }
        }
        None => String::new(),
    }
}

pub fn get_depth(path: &str) -> usize {
    path.chars()
        .filter(|character| is_separator(*character))
        .count()
}

fn is_drive_root(path: &str) -> bool {
    let characters: Vec<char> = path.chars().collect();
    characters.len() == 3
        && characters[0].is_ascii_alphabetic()
        && characters[1] == ':'
        && is_separator(characters[2])
}

fn is_root_path(path: &str) -> bool {
    if is_drive_root(path) {
        return true;
    }

    if path.starts_with("\\\\") {
        let segments: Vec<&str> = path
            .split(['\\', '/'])
            .filter(|segment| !segment.is_empty())
            .collect();
        return segments.len() == 2;
    }

    false
}

fn has_device_namespace(value: &str) -> bool {
    value.starts_with("\\\\?\\")
        || value.starts_with("\\\\.\\")
        || value.starts_with("//?/")
        || value.starts_with("//./")
}

fn is_rooted_candidate(value: &str) -> bool {
    if value.starts_with("\\\\") || value.starts_with("//") {
        return true;
    }

    let characters: Vec<char> = value.chars().collect();
    characters.len() >= 3
        && characters[0].is_ascii_alphabetic()
        && characters[1] == ':'
        && is_separator(characters[2])
}

fn full_path(value: &str) -> Option<String> {
    let unified = value.replace('/', "\\");
    let (prefix, rest) = split_prefix(&unified);
    let mut segments: Vec<&str> = Vec::new();
    for segment in rest.split('\\') {
        match segment {
            "" | "." => continue,
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }

    let mut result = prefix;
    for segment in segments {
        result.push_str(segment);
        result.push('\\');
    }

    if result.is_empty() {
        return None;
    }

    Some(result)
}

fn split_prefix(value: &str) -> (String, &str) {
    if let Some(after) = value.strip_prefix("\\\\") {
        let mut seen = 0usize;
        let mut index = 0usize;
        for (position, character) in after.char_indices() {
            if is_separator(character) {
                seen += 1;
                if seen == 2 {
                    index = position + 1;
                    break;
                }
            }
        }
        if seen < 2 {
            return (format!("\\\\{after}\\"), "");
        }
        return (format!("\\\\{}\\", &after[..index - 1]), &after[index..]);
    }

    if value.len() >= 2 && value.as_bytes()[1] == b':' {
        return (format!("{}\\", &value[..2]), &value[2..]);
    }

    (String::new(), value)
}

fn trim_ending_separator(value: &str) -> String {
    if is_drive_root(value) {
        return value.to_string();
    }

    let trimmed = value.trim_end_matches(['\\', '/']);
    if trimmed.is_empty() {
        value.to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn equals_ignore_case(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

pub fn starts_with_ignore_case(value: &str, prefix: &str) -> bool {
    match value.get(..prefix.len()) {
        Some(head) => head.eq_ignore_ascii_case(prefix),
        None => false,
    }
}
