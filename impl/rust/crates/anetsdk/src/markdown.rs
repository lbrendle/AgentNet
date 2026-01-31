use crate::Error;

const MAX_HEADING_LEVEL: usize = 6;

pub fn canonicalize_markdown_profile(input: &str) -> Result<String, Error> {
    let mut normalized = input.replace("\r\n", "\n");
    normalized = normalized.replace('\r', "\n");

    let mut out: Vec<String> = Vec::new();
    let mut blank_run = 0usize;
    let mut in_code_block = false;

    for raw_line in normalized.split('\n') {
        let mut line = raw_line.replace('\t', "    ");
        line = line.trim_end().to_string();

        if in_code_block {
            if is_fence_line(&line) {
                out.push("```".to_string());
                in_code_block = false;
                blank_run = 0;
                continue;
            }
            out.push(line);
            blank_run = 0;
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_run += 1;
            if blank_run <= 2 {
                out.push(String::new());
            }
            continue;
        }
        blank_run = 0;

        if is_fence_line(&line) {
            let fence = canonical_fence(&line)?;
            out.push(fence);
            in_code_block = true;
            continue;
        }

        let canonical = canonicalize_line(trimmed)?;
        out.push(canonical);
    }

    if in_code_block {
        return Err(Error::Cbor("unterminated code fence"));
    }

    Ok(out.join("\n"))
}

pub fn validate_markdown_profile(input: &str) -> Result<(), Error> {
    let canonical = canonicalize_markdown_profile(input)?;
    if canonical != input {
        return Err(Error::Cbor("markdown not canonical"));
    }
    Ok(())
}

fn canonicalize_line(line: &str) -> Result<String, Error> {
    if line.starts_with("    ") {
        return Err(Error::Cbor("indented code blocks not allowed"));
    }
    if contains_html(line) {
        return Err(Error::Cbor("html not allowed"));
    }
    if line.contains("![") {
        return Err(Error::Cbor("images not allowed"));
    }
    if looks_like_table_separator(line) {
        return Err(Error::Cbor("tables not allowed"));
    }

    if let Some(canonical) = canonicalize_heading(line)? {
        return Ok(canonical);
    }
    if let Some(canonical) = canonicalize_blockquote(line)? {
        return Ok(canonical);
    }
    if let Some(canonical) = canonicalize_hr(line)? {
        return Ok(canonical);
    }
    if let Some(canonical) = canonicalize_list(line)? {
        return Ok(canonical);
    }

    validate_links(line)?;
    Ok(line.to_string())
}

fn is_fence_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("```")
}

fn canonical_fence(line: &str) -> Result<String, Error> {
    let trimmed = line.trim_start();
    let rest = &trimmed[3..];
    let lang = rest.trim();
    if lang.is_empty() {
        return Ok("```".to_string());
    }
    if !lang
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '+' | '.'))
    {
        return Err(Error::Cbor("invalid code fence language"));
    }
    Ok(format!("```{}", lang))
}

fn canonicalize_heading(line: &str) -> Result<Option<String>, Error> {
    if !line.starts_with('#') {
        return Ok(None);
    }
    let mut level = 0usize;
    for ch in line.chars() {
        if ch == '#' {
            level += 1;
        } else {
            break;
        }
    }
    if level == 0 || level > MAX_HEADING_LEVEL {
        return Err(Error::Cbor("invalid heading"));
    }
    let rest = line[level..].trim_start();
    if rest.is_empty() {
        return Err(Error::Cbor("empty heading"));
    }
    Ok(Some(format!("{} {}", "#".repeat(level), rest)))
}

fn canonicalize_blockquote(line: &str) -> Result<Option<String>, Error> {
    if !line.starts_with('>') {
        return Ok(None);
    }
    let rest = line[1..].trim_start();
    Ok(Some(format!("> {}", rest)))
}

fn canonicalize_hr(line: &str) -> Result<Option<String>, Error> {
    if line.trim() == "---" {
        return Ok(Some("---".to_string()));
    }
    Ok(None)
}

fn canonicalize_list(line: &str) -> Result<Option<String>, Error> {
    let (indent, rest) = split_indent(line);
    if indent > 3 {
        return Err(Error::Cbor("excessive indent"));
    }
    if rest.starts_with('-') {
        let after = rest[1..].trim_start();
        if after.is_empty() || rest.len() == 1 || !rest.as_bytes()[1].is_ascii_whitespace() {
            return Err(Error::Cbor("invalid list marker"));
        }
        validate_links(after)?;
        return Ok(Some(format!("{}- {}", " ".repeat(indent), after)));
    }
    if rest.starts_with('*') || rest.starts_with('+') {
        return Err(Error::Cbor("invalid list marker"));
    }
    if let Some((digits, after_dot)) = rest.split_once('.') {
        if digits.chars().all(|c| c.is_ascii_digit()) {
            if digits != "1" {
                return Err(Error::Cbor("ordered list must use 1."));
            }
            let after = after_dot.trim_start();
            if after.is_empty()
                || !after_dot
                    .as_bytes()
                    .get(0)
                    .map(|b| b.is_ascii_whitespace())
                    .unwrap_or(false)
            {
                return Err(Error::Cbor("invalid ordered list"));
            }
            validate_links(after)?;
            return Ok(Some(format!("{}1. {}", " ".repeat(indent), after)));
        }
    }
    Ok(None)
}

fn split_indent(line: &str) -> (usize, &str) {
    let mut count = 0usize;
    for ch in line.chars() {
        if ch == ' ' {
            count += 1;
        } else {
            break;
        }
    }
    (count, &line[count..])
}

fn validate_links(line: &str) -> Result<(), Error> {
    let mut idx = 0usize;
    while let Some(start) = line[idx..].find("](") {
        let url_start = idx + start + 2;
        let rest = &line[url_start..];
        let end = rest.find(')').ok_or(Error::Cbor("invalid link"))?;
        let url = rest[..end].trim();
        let scheme_end = url.find(':').ok_or(Error::Cbor("invalid link"))?;
        let scheme = &url[..scheme_end].to_lowercase();
        if scheme != "https" && scheme != "agentnet" && scheme != "did" {
            return Err(Error::Cbor("invalid link scheme"));
        }
        idx = url_start + end + 1;
    }
    Ok(())
}

fn contains_html(line: &str) -> bool {
    let bytes = line.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'<' {
            if let Some(next) = bytes.get(i + 1) {
                let ch = *next as char;
                if ch.is_ascii_alphabetic() || ch == '/' || ch == '!' {
                    return true;
                }
            }
        }
    }
    false
}

fn looks_like_table_separator(line: &str) -> bool {
    if !line.contains('|') {
        return false;
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    for ch in trimmed.chars() {
        if ch != '|' && ch != '-' && ch != ':' && ch != ' ' {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::fs;

    #[derive(Deserialize)]
    struct MarkdownTests {
        cases: Vec<MarkdownCase>,
    }

    #[derive(Deserialize)]
    struct MarkdownCase {
        id: String,
        input: String,
        canonical: String,
        valid: bool,
    }

    #[test]
    fn markdown_profile_vectors() {
        let data = fs::read_to_string("../../../../spec/agentnet-markdown-tests-v0.1.json")
            .expect("read markdown tests");
        let tests: MarkdownTests = serde_json::from_str(&data).expect("parse markdown tests");

        for case in tests.cases {
            let canonicalized = canonicalize_markdown_profile(&case.input);
            match canonicalized {
                Ok(val) => {
                    assert_eq!(val, case.canonical, "{} canonical mismatch", case.id);
                }
                Err(_) => {
                    assert!(
                        case.canonical.is_empty(),
                        "{} should not canonicalize",
                        case.id
                    );
                }
            }
            let valid = validate_markdown_profile(&case.input).is_ok();
            assert_eq!(valid, case.valid, "{} validity mismatch", case.id);
        }
    }
}
