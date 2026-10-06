use crate::model::{Diagnostic, HeaderPolicy, Language, Settings};
use regex::Regex;
use serde_json::Value;
use std::{borrow::Cow, fs, sync::LazyLock};

static COPYRIGHT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?:Copyright(?: \([cC]\))? |SPDX-FileCopyrightText: )(?P<years>(?P<start>[0-9]{4})(?:-(?P<end>[0-9]{4}))?),? (?P<owner>.+)$",
    )
    .expect("valid copyright expression")
});
thread_local! {
    static COPYRIGHT_CACHE: Regex = COPYRIGHT.clone();
}
static CODING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[ \t\x0c]*#.*?coding[:=][ \t]*([-_.a-zA-Z0-9]+)")
        .expect("valid encoding expression")
});
static SPDX: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../src/lint_my_headers/supported-licenses.json"
    ))
    .expect("valid bundled SPDX data")
});
static LEGACY: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../src/lint_my_headers/legacy-license-notices.json"
    ))
    .expect("valid bundled legacy SPDX data")
});

fn normalize_newlines(source: &str) -> String {
    source.replace("\r\n", "\n").replace('\r', "\n")
}

fn uncomment<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    if line.trim_end_matches('\n').is_empty() {
        return Some(line);
    }
    line.strip_prefix(marker)
        .map(|text| text.strip_prefix(' ').unwrap_or(text))
}

pub fn render_header(text: &str, language: Language) -> String {
    text.split_inclusive('\n')
        .map(|line| {
            if line.trim_end_matches('\n').is_empty() {
                line.into()
            } else {
                format!("{} {line}", language.comment())
            }
        })
        .collect()
}

pub fn build_policy(settings: &Settings, current_year: i32) -> Result<HeaderPolicy, String> {
    if settings.year < 1000 || settings.year > current_year {
        return Err(format!("Invalid first copyright year: {}", settings.year));
    }
    if settings.owner.is_empty() || settings.owner.contains(['\n', '\r']) {
        return Err("Please specify a single-line copyright owner".into());
    }
    let mut notices = Vec::new();
    if let Some(id) = settings.license.as_ref().filter(|id| !id.is_empty()) {
        if !settings.license_path.is_file() {
            return Err("Unable to locate local copy of license text.".into());
        }
        let current = SPDX["licenses"]
            .as_array()
            .expect("bundled licenses array")
            .iter()
            .find(|entry| entry["licenseId"] == *id);
        for (entry, urls_key) in [(LEGACY["licenses"].get(id), "urls"), (current, "seeAlso")] {
            let Some(entry) = entry else { continue };
            let name = entry["name"].as_str().expect("bundled license name");
            let urls = entry[urls_key].as_array().expect("bundled license URLs");
            let fallback = format!("https://spdx.org/licenses/{id}.html");
            let urls: Vec<&str> = if urls.is_empty() {
                vec![&fallback]
            } else {
                urls.iter()
                    .map(|url| url.as_str().expect("bundled URL"))
                    .collect()
            };
            for url in urls {
                let notice = format!(
                    "This program is licensed under the {name}.\n\
                     See LICENSE or go to <{url}> for full license details.\n"
                );
                if !notices.contains(&notice) {
                    notices.push(notice);
                }
            }
        }
        if notices.is_empty() {
            return Err(format!("Invalid license identifier: {id}"));
        }
        notices.push(format!("SPDX-License-Identifier: {id}\n"));
    } else if let Some(path) = &settings.license_notice {
        if !path.is_file() {
            return Err("Unable to locate the text of the license notice.".into());
        }
        let notice = normalize_newlines(&fs::read_to_string(path).map_err(|e| e.to_string())?);
        // Existing Python-commented notices remain usable alongside plain-text notices.
        let notice: String = notice
            .split_inclusive('\n')
            .map(|line| uncomment(line, "#").unwrap_or(line))
            .collect();
        if notice.trim().is_empty() {
            return Err("Custom license notice must contain non-empty text.".into());
        }
        notices.push(notice);
    } else {
        return Err(
            "One of the following args needs to be specified: 'license', 'license-notice'".into(),
        );
    }
    let years = if settings.year == current_year {
        current_year.to_string()
    } else {
        format!("<FILE_CREATION_YEAR>-{current_year}")
    };
    Ok(HeaderPolicy {
        owner: settings.owner.clone(),
        starting_year: settings.year,
        current_year,
        expected_header: format!(
            "Copyright (C) {years}, {}.\n\n{}",
            settings.owner, notices[0]
        ),
        license_notices: notices,
    })
}

#[derive(Default)]
pub struct ContentAnalysis {
    pub diagnostic: Option<Diagnostic>,
    pub replacement: Option<Vec<u8>>,
}

#[derive(Default)]
pub(crate) struct Inspection {
    pub diagnostic: Option<Diagnostic>,
    pub edit: Option<YearEdit>,
}

pub(crate) struct YearEdit {
    range: std::ops::Range<usize>,
    start: i32,
    end: i32,
}

impl YearEdit {
    pub fn apply(&self, raw: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(raw.len().saturating_add(5));
        bytes.extend_from_slice(&raw[..self.range.start]);
        bytes.extend_from_slice(format!("{}-{}", self.start, self.end).as_bytes());
        bytes.extend_from_slice(&raw[self.range.end..]);
        bytes
    }
}

fn problem(path: &str, line: usize, code: &str, message: impl Into<String>) -> Inspection {
    Inspection {
        diagnostic: Some(Diagnostic {
            path: path.into(),
            line: line.max(1),
            column: 1,
            code: code.into(),
            message: message.into(),
            fixable: false,
        }),
        edit: None,
    }
}

fn comment_or_blank(line: &str) -> bool {
    line.trim_start_matches([' ', '\t', '\x0c'])
        .starts_with(['#', '\r', '\n'])
        || line.trim_matches([' ', '\t', '\x0c']).is_empty()
}

fn encoding_cookie(line: &[u8]) -> Result<Option<String>, String> {
    // Python detects cookies by decoding each of the first two physical lines as UTF-8.
    let line = std::str::from_utf8(line).map_err(|_| "invalid or missing encoding declaration")?;
    Ok(CODING.captures(line).map(|captures| {
        let label = captures[1].to_ascii_lowercase().replace('_', "-");
        let prefix = &label[..label.len().min(12)];
        if prefix == "utf-8" || prefix.starts_with("utf-8-") {
            "utf-8".into()
        } else if matches!(prefix, "latin-1" | "iso-8859-1" | "iso-latin-1")
            || prefix.starts_with("latin-1-")
            || prefix.starts_with("iso-8859-1-")
            || prefix.starts_with("iso-latin-1-")
        {
            "latin-1".into()
        } else {
            label
        }
    }))
}

fn decode_source(raw: &[u8]) -> Result<Cow<'_, str>, String> {
    let bom = raw.starts_with(b"\xef\xbb\xbf");
    let raw = if bom { &raw[3..] } else { raw };
    let mut lines = raw.split_inclusive(|byte| *byte == b'\n');
    let first = lines.next().unwrap_or_default();
    let mut cookie = encoding_cookie(first)?;
    if cookie.is_none() && comment_or_blank(std::str::from_utf8(first).unwrap_or_default()) {
        cookie = encoding_cookie(lines.next().unwrap_or_default())?;
    }
    let label = cookie.as_deref().unwrap_or("utf-8");
    if bom && label != "utf-8" {
        return Err("encoding problem: utf-8".into());
    }
    match label {
        "utf-8" | "utf8" | "u8" | "utf" | "cp65001" | "utf8-ucs2" | "utf8-ucs4" => {
            return std::str::from_utf8(raw)
                .map(Cow::Borrowed)
                .map_err(|e| e.to_string());
        }
        "ascii" | "us-ascii" | "646" | "ansi-x3.4-1968" | "ansi-x3.4-1986" | "ansi-x3-4-1968"
        | "cp367" | "csascii" | "ibm367" | "iso646-us" | "iso-646.irv-1991" | "iso-ir-6" | "us" => {
            return if raw.is_ascii() {
                Ok(Cow::Borrowed(
                    std::str::from_utf8(raw).expect("ASCII is UTF-8"),
                ))
            } else {
                Err("invalid byte in ASCII source".into())
            };
        }
        "latin-1" | "latin1" | "iso8859-1" | "iso-8859-1" | "l1" | "8859" | "cp819" | "ibm819"
        | "csisolatin1" | "iso8859" | "iso-ir-100" | "latin" => {
            return Ok(Cow::Owned(
                raw.iter().map(|byte| char::from(*byte)).collect(),
            ));
        }
        "cp1252" | "windows-1252" | "1252" => {}
        _ => return Err(format!("unsupported source encoding: {label}")),
    }
    // Python rejects the five undefined Windows-1252 byte values.
    if raw
        .iter()
        .any(|b| matches!(b, 0x81 | 0x8d | 0x8f | 0x90 | 0x9d))
    {
        return Err("undefined byte in Windows-1252 source".into());
    }
    const CP1252_EXTENDED: [char; 32] = [
        '€', '\0', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\0', 'Ž', '\0', '\0',
        '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\0', 'ž', 'Ÿ',
    ];
    Ok(Cow::Owned(
        raw.iter()
            .map(|&byte| {
                if (0x80..=0x9f).contains(&byte) {
                    CP1252_EXTENDED[usize::from(byte - 0x80)]
                } else {
                    char::from(byte)
                }
            })
            .collect(),
    ))
}

fn is_shebang(source: &str, language: Language) -> bool {
    !matches!(language, Language::C | Language::Cpp | Language::Go)
        && source.strip_prefix("#!").is_some_and(|tail| {
            // Crate attributes and comment-prefixed Rust #! forms are not shebangs.
            language != Language::Rust
                || !["[", "//", "/*"]
                    .iter()
                    .any(|p| tail.trim_start().starts_with(p))
        })
}

fn preamble(lines: &[&str], language: Language) -> (usize, bool) {
    let shebang = usize::from(lines.first().is_some_and(|line| is_shebang(line, language)));
    let cookie_end = if language != Language::Python {
        0
    } else if lines.first().is_some_and(|line| CODING.is_match(line)) {
        1
    } else if lines.len() > 1 && comment_or_blank(lines[0]) && CODING.is_match(lines[1]) {
        2
    } else {
        0
    };
    let end = if language == Language::Go {
        lines
            .iter()
            .take_while(|line| {
                line.strip_prefix("//go:build")
                    .or_else(|| line.strip_prefix("// +build"))
                    .is_some_and(|tail| tail.starts_with([' ', '\t', '\r', '\n']))
            })
            .count()
    } else {
        let tools_version = usize::from(
            language == Language::Swift
                && lines.first().is_some_and(|line| {
                    line.strip_prefix("//")
                        .is_some_and(|tail| tail.trim_start().starts_with("swift-tools-version:"))
                }),
        );
        shebang.max(cookie_end).max(tools_version)
    };
    if end == 0 {
        return (0, true);
    }
    let separator = lines
        .get(end)
        .is_some_and(|line| line.trim_end_matches(['\r', '\n']).is_empty());
    (end + usize::from(separator), separator)
}

fn is_copyright(text: &str) -> bool {
    text.starts_with("Copyright") || text.starts_with("SPDX-FileCopyrightText:")
}

fn prose_license_count(text: &str) -> usize {
    // ponytail: count the supported prose prefixes; add explicit formats when more notices need SPDX mixing.
    text.lines()
        .filter(|line| {
            let text = line.trim_start_matches([' ', '\t', '#', '/', '*', '!']);
            text.starts_with("This program is licensed under ")
                || text.starts_with("Licensed under ")
        })
        .count()
}

// Inspect only the leading comment region; the first code token ends header validation.
#[derive(Default)]
struct LeadingComments<'a> {
    end: usize,
    copyrights: Vec<(usize, &'a str)>,
    ambiguous: bool,
    identifiers: usize,
    prose_licenses: usize,
}

fn leading_comments(source: &str, language: Language) -> LeadingComments<'_> {
    let mut remaining = source;
    let mut result = LeadingComments::default();
    let mut line = 0;
    loop {
        let trimmed = remaining.trim_start_matches([' ', '\t', '\x0c', '\r', '\n']);
        line += remaining[..remaining.len() - trimmed.len()]
            .bytes()
            .filter(|&b| b == b'\n')
            .count();
        remaining = trimmed;
        let offset = source.len() - remaining.len();
        let shebang = offset == 0 && is_shebang(remaining, language);
        let end = if shebang || remaining.starts_with(language.comment()) {
            let end = remaining.find('\n').map_or(remaining.len(), |i| i + 1);
            let text = remaining[..end].trim_end_matches(['\r', '\n']);
            if !shebang {
                let tail = &text[language.comment().len()..];
                let field = if language.comment() == "//" {
                    tail.trim_start_matches('/')
                        .trim_start_matches('!')
                        .trim_start()
                } else {
                    tail.trim_start()
                };
                result.identifiers += usize::from(field.starts_with("SPDX-License-Identifier:"));
                result.prose_licenses += prose_license_count(field);
                if is_copyright(tail.trim_start())
                    || (language.comment() == "//"
                        && is_copyright(
                            tail.trim_start_matches('/')
                                .trim_start_matches('!')
                                .trim_start(),
                        ))
                {
                    result
                        .copyrights
                        .push((line, tail.strip_prefix(' ').unwrap_or(text)));
                }
            }
            end
        } else if language.comment() == "//" && remaining.starts_with("/*") {
            let mut end = 2;
            let mut depth = 1;
            while depth > 0 {
                let tail = &remaining[end..];
                let close = tail.find("*/").unwrap_or(tail.len());
                let open = if matches!(language, Language::Rust | Language::Swift) {
                    tail[..close].find("/*").unwrap_or(close)
                } else {
                    close
                };
                if open < close {
                    depth += 1;
                    end += open + 2;
                } else if close < tail.len() {
                    depth -= 1;
                    end += close + 2;
                } else {
                    result.end = source.len();
                    result.ambiguous = true;
                    return result;
                }
            }
            if remaining[..end].contains("Copyright") {
                result.copyrights.push((line, &remaining[..end]));
            }
            let content = &remaining[2..end - 2];
            result.prose_licenses += prose_license_count(content);
            result.identifiers += content
                .lines()
                .filter(|line| {
                    line.trim_start_matches([' ', '\t', '/', '*', '!'])
                        .starts_with("SPDX-License-Identifier:")
                })
                .count();
            end
        } else {
            result.end = offset;
            return result;
        };
        line += remaining[..end].bytes().filter(|&b| b == b'\n').count();
        remaining = &remaining[end..];
    }
}

fn block_header(comment: &str) -> Option<Vec<String>> {
    let text = comment
        .strip_prefix("/*")?
        .strip_suffix("*/")?
        .trim_end_matches([' ', '\t']);
    // Documentation comments and nested delimiters need manual review.
    if text.starts_with(['*', '!']) || text.contains("/*") || text.contains("*/") {
        return None;
    }
    Some(
        text.split_inclusive('\n')
            .map(|line| {
                let line = line.trim_start_matches([' ', '\t']);
                let line = line
                    .strip_prefix('*')
                    .map(|text| text.strip_prefix(' ').unwrap_or(text))
                    .unwrap_or(line);
                let mut line = normalize_newlines(line);
                // A closing delimiter completes the final notice line without adding source bytes.
                if !line.ends_with('\n') {
                    line.push('\n');
                }
                line
            })
            .collect(),
    )
}

pub fn analyze(raw: &[u8], policy: &HeaderPolicy, display_path: &str) -> ContentAnalysis {
    let content = inspect(raw, policy, display_path);
    ContentAnalysis {
        replacement: content.edit.as_ref().map(|edit| edit.apply(raw)),
        diagnostic: content.diagnostic,
    }
}

pub(crate) fn inspect(raw: &[u8], policy: &HeaderPolicy, display_path: &str) -> Inspection {
    let language =
        Language::from_path(std::path::Path::new(display_path)).unwrap_or(Language::Python);
    let source = match if language == Language::Python {
        decode_source(raw)
    } else {
        std::str::from_utf8(raw.strip_prefix(b"\xef\xbb\xbf").unwrap_or(raw))
            .map(Cow::Borrowed)
            .map_err(|e| e.to_string())
    } {
        Ok(source) => source,
        Err(error) => {
            return problem(
                display_path,
                1,
                "LMH007",
                format!("unable to decode source: {error}"),
            );
        }
    };
    let LeadingComments {
        end,
        copyrights: comments,
        ambiguous: ambiguous_header,
        identifiers: identifier_count,
        prose_licenses,
    } = leading_comments(&source, language);
    let source = &source[..end];
    let ambiguous_header = ambiguous_header
        || (matches!(language, Language::Javascript | Language::Typescript)
            && source.contains(['\u{2028}', '\u{2029}']));
    // Normalizing bare CR could turn an ambiguous header into an authorized repair.
    let bare_cr =
        source.as_bytes().iter().enumerate().any(|(index, byte)| {
            *byte == b'\r' && source.as_bytes().get(index + 1) != Some(&b'\n')
        });
    let source = if source.contains('\r') {
        Cow::Owned(normalize_newlines(source))
    } else {
        Cow::Borrowed(source)
    };
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let (header_index, separator_valid) = preamble(&lines, language);
    if ambiguous_header {
        return problem(
            display_path,
            header_index + 1,
            "LMH006",
            "malformed or ambiguous header",
        );
    }
    if header_index >= lines.len() || comments.is_empty() {
        return problem(
            display_path,
            header_index + 1,
            "LMH001",
            "missing legal header",
        );
    }
    let block = comments[0].1.starts_with("/*");
    let block_lines = block.then(|| block_header(comments[0].1)).flatten();
    let header: Vec<&str> = if let Some(block_lines) = &block_lines {
        block_lines.iter().map(String::as_str).collect()
    } else {
        lines[header_index..]
            .iter()
            .map_while(|line| uncomment(line, language.comment()))
            .collect()
    };
    let copyright_index = header.iter().position(|line| is_copyright(line));
    let first_index = header
        .iter()
        .position(|line| !line.trim_end_matches('\n').is_empty());
    let spdx_first = first_index.is_some_and(|index| {
        header[index].starts_with("SPDX-License-Identifier:")
            && copyright_index.is_some_and(|copyright| {
                copyright > index
                    && header[index + 1..copyright]
                        .iter()
                        .all(|line| line.trim_end_matches('\n').is_empty())
            })
    });
    let copyright_text = copyright_index
        .and_then(|index| header.get(index))
        .map(|line| line.trim_end_matches('\n'))
        .unwrap_or_default();
    let captures = COPYRIGHT_CACHE.with(|regex| regex.captures(copyright_text));
    let identifiers: Vec<_> = header
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("SPDX-License-Identifier:"))
        .collect();
    let misplaced = (block && comments[0].0 != header_index)
        || (!block && comments[0].0 != header_index + copyright_index.unwrap_or_default())
        || (first_index != copyright_index && !spdx_first);
    let ambiguous = comments.len() != 1
        || identifier_count > 1
        || identifier_count != identifiers.len()
        || (block && block_lines.is_none())
        || (block && header.iter().filter(|line| !line.trim().is_empty()).count() < 2)
        || captures.is_none()
        || header.iter().filter(|line| is_copyright(line)).count() != 1;
    if bare_cr || !separator_valid || misplaced || ambiguous {
        return problem(
            display_path,
            comments[0].0 + 1,
            "LMH006",
            "malformed, misplaced, or ambiguous header",
        );
    }
    let copyright_index = copyright_index.expect("matched copyright line");
    let line_index = if block { comments[0].0 } else { header_index } + copyright_index;
    let captures = captures.expect("matched copyright comment");
    let owner = &captures["owner"];
    let owner = if !copyright_text.starts_with("SPDX-FileCopyrightText:") && owner != policy.owner {
        owner.strip_suffix('.').unwrap_or(owner)
    } else {
        owner
    };
    if owner != policy.owner {
        return problem(
            display_path,
            line_index + 1,
            "LMH002",
            format!(
                "copyright owner is '{}'; expected '{}'",
                owner, policy.owner
            ),
        );
    }
    let start_year: i32 = captures["start"].parse().expect("four ASCII digits");
    let end_year = captures
        .name("end")
        .map(|year| year.as_str().parse::<i32>().expect("four ASCII digits"));
    if start_year < policy.starting_year
        || start_year > policy.current_year
        || end_year.is_some_and(|end| end < start_year || end > policy.current_year)
    {
        return problem(
            display_path,
            line_index + 1,
            "LMH003",
            format!(
                "copyright year '{}' is outside the accepted range",
                &captures["years"]
            ),
        );
    }
    let spdx = copyright_text.starts_with("SPDX-FileCopyrightText:") || !identifiers.is_empty();
    let notice_index = copyright_index + 2;
    let prose_notice = policy.license_notices.iter().find(|notice| {
        if !header
            .get(copyright_index + 1)
            .is_some_and(|line| line.trim_end_matches('\n').is_empty())
        {
            return false;
        }
        let mut expected = notice.as_str();
        for part in header[notice_index.min(header.len())..].iter().copied() {
            if let Some(tail) = part.strip_prefix(expected) {
                // A notice without a newline must still match a complete source line.
                return expected.ends_with('\n') || tail.is_empty() || tail.starts_with('\n');
            }
            let Some(rest) = expected.strip_prefix(part) else {
                return false;
            };
            expected = rest;
        }
        expected.is_empty()
    });
    let complete_notice = prose_notice.filter(|notice| {
        !notice.starts_with("SPDX-License-Identifier:") || notice.lines().count() > 1
    });
    let accepted_prose =
        complete_notice.map_or(usize::from(!spdx), |notice| prose_license_count(notice));
    if prose_licenses > accepted_prose {
        return problem(
            display_path,
            line_index + 1,
            "LMH006",
            "mixed or ambiguous license declarations",
        );
    }
    if let Some(&(identifier_index, _)) = identifiers.first().filter(|_| complete_notice.is_none())
    {
        let last_field = copyright_index.max(identifier_index);
        if header[first_index.expect("matched header")..=last_field]
            .iter()
            .any(|line| {
                !line.trim_end_matches('\n').is_empty()
                    && !is_copyright(line)
                    && !line.starts_with("SPDX-License-Identifier:")
            })
            || header
                .get(last_field + 1)
                .is_some_and(|line| !line.trim_end_matches('\n').is_empty())
            || (block
                && header[last_field + 1..]
                    .iter()
                    .any(|line| !line.trim_end_matches('\n').is_empty()))
        {
            return problem(
                display_path,
                line_index + 1,
                "LMH006",
                "mixed or ambiguous license declarations",
            );
        }
    }
    if !spdx
        && !header
            .get(copyright_index + 1)
            .is_some_and(|line| line.trim_end_matches('\n').is_empty())
    {
        return problem(
            display_path,
            line_index + 2,
            "LMH006",
            "expected one blank line between copyright and license notice",
        );
    }
    let notice_matches = if spdx {
        identifiers.len() == 1
            && (policy.license_notices.iter().any(|notice| {
                notice.starts_with("SPDX-License-Identifier:")
                    && notice.trim_end_matches('\n') == identifiers[0].1.trim_end_matches('\n')
            }) || complete_notice.is_some_and(|notice| {
                notice
                    .lines()
                    .any(|line| line == identifiers[0].1.trim_end_matches('\n'))
            }))
    } else {
        prose_notice.is_some()
    };
    if !notice_matches {
        return problem(
            display_path,
            if spdx {
                identifiers.first().map_or(line_index + 2, |(index, _)| {
                    (if block { comments[0].0 } else { header_index }) + index + 1
                })
            } else {
                line_index + 3
            },
            "LMH005",
            "missing or mismatched license notice",
        );
    }
    let last_year = end_year.unwrap_or(start_year);
    if last_year == policy.current_year {
        return Inspection::default();
    }
    let mut result = problem(
        display_path,
        line_index + 1,
        "LMH004",
        format!(
            "copyright year ends at {last_year}; expected {}",
            policy.current_year
        ),
    );
    // Splice only the ASCII year bytes. The original encoding, preamble and body stay untouched.
    let years = captures.name("years").expect("matched years");
    let needle = &copyright_text[..years.end()];
    let raw_lines: Vec<&[u8]> = raw
        .split_inclusive(|byte| *byte == b'\n')
        .take(line_index + 1)
        .collect();
    if let Some(raw_line) = raw_lines.get(line_index) {
        let offsets: Vec<usize> = raw_line
            .windows(needle.len())
            .enumerate()
            .filter_map(|(index, bytes)| (bytes == needle.as_bytes()).then_some(index))
            .collect();
        if offsets.len() == 1
            && (1000..=9999).contains(&start_year)
            && (1000..=9999).contains(&policy.current_year)
        {
            let year_start = raw_lines[..line_index]
                .iter()
                .map(|line| line.len())
                .sum::<usize>()
                + offsets[0]
                + years.start();
            // Only digits/a hyphen change inside the single validated copyright field.
            // All supported encodings preserve ASCII, so syntax and other header fields stay valid.
            result.edit = Some(YearEdit {
                range: year_start..year_start + captures["years"].len(),
                start: start_year,
                end: policy.current_year,
            });
            result
                .diagnostic
                .as_mut()
                .expect("stale diagnostic")
                .fixable = true;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> HeaderPolicy {
        HeaderPolicy {
            owner: "Example Owner".into(),
            starting_year: 2022,
            current_year: 2030,
            license_notices: vec!["License notice.\n".into()],
            expected_header: String::new(),
        }
    }

    fn header(years: &str) -> String {
        format!("# Copyright (C) {years}, Example Owner.\n\n# License notice.\n\nvalue = 'café'\n")
    }

    fn inspect(source: impl AsRef<[u8]>) -> ContentAnalysis {
        analyze(source.as_ref(), &policy(), "x.py")
    }

    #[test]
    fn common_prose_and_spdx_layouts_preserve_every_byte_except_the_year() {
        let mut policy = policy();
        policy
            .license_notices
            .push("SPDX-License-Identifier: Apache-2.0\n".into());
        for path in [
            "x.py", "x.js", "x.ts", "x.rs", "x.go", "x.swift", "x.sh", "x.c", "x.cpp",
        ] {
            let language = Language::from_path(std::path::Path::new(path)).unwrap();
            let preamble = match language {
                Language::Python => "#!/usr/bin/env python3\n# coding: utf-8\n\n",
                Language::Javascript | Language::Typescript => "#!/usr/bin/env node\n\n",
                Language::Rust => "#!/usr/bin/env rust-script\n\n",
                Language::Go => "//go:build linux\n\n",
                Language::Swift => "// swift-tools-version: 5.9\n\n",
                Language::Bash => "#!/usr/bin/env bash\n\n",
                _ => "",
            };
            for copyright in [
                "Copyright (C) 2024, Example Owner.",
                "Copyright (c) 2024 Example Owner",
                "Copyright 2024 Example Owner",
                "SPDX-FileCopyrightText: 2024 Example Owner",
            ] {
                let mut headers = vec![format!(
                    "{copyright}\n\nSPDX-License-Identifier: Apache-2.0\n"
                )];
                if copyright.starts_with("SPDX-") {
                    headers.push(format!(
                        "{copyright}\nSPDX-License-Identifier: Apache-2.0\n"
                    ));
                    headers.push(format!(
                        "SPDX-License-Identifier: Apache-2.0\n{copyright}\n"
                    ));
                } else {
                    headers.push(format!("{copyright}\n\nLicense notice.\n"));
                }
                for text in headers {
                    let mut layouts = vec![render_header(&text, language)];
                    if language.comment() == "//" {
                        layouts.push(format!(
                            "/*\n{} */\n",
                            text.lines()
                                .map(|line| format!(" * {line}\n"))
                                .collect::<String>()
                        ));
                        layouts.push(format!("/* {text}*/\n"));
                        layouts.push(format!("/*\n{} */\n", text.trim_end_matches('\n')));
                    }
                    let notes = render_header(
                        "Note: SPDX-License-Identifier: is a keyword; SPDX-License-Identifier: here is not a field.\n",
                        language,
                    );
                    layouts = layouts
                        .into_iter()
                        .flat_map(|layout| [layout.clone(), format!("{layout}\n{notes}")])
                        .collect();
                    for layout in layouts {
                        for preamble in ["", preamble] {
                            for newline in ["\n", "\r\n"] {
                                let raw = format!(
                                "\u{feff}{preamble}{layout}\nbody = \"2024 Copyright Example Owner\"\n"
                            )
                            .replace('\n', newline);
                                let result = analyze(raw.as_bytes(), &policy, path);
                                assert_eq!(
                                    result.diagnostic.as_ref().unwrap().code,
                                    "LMH004",
                                    "{path}: {raw:?}"
                                );
                                let fixed = result
                                    .replacement
                                    .unwrap_or_else(|| panic!("{path}: {raw:?}"));
                                assert_eq!(fixed, raw.replacen("2024", "2024-2030", 1).as_bytes());
                                assert!(
                                    analyze(&fixed, &policy, path).diagnostic.is_none(),
                                    "{path}: {raw:?}"
                                );
                            }
                        }
                    }
                }
            }
            let mut custom = policy.clone();
            custom.license_notices = vec!["License notice mentions SPDX-License-Identifier: twice: SPDX-License-Identifier:.\n".into()];
            let raw = render_header(
                &format!(
                    "Copyright 2024 Example Owner\n\n{}",
                    custom.license_notices[0]
                ),
                language,
            );
            let result = analyze(raw.as_bytes(), &custom, path);
            assert_eq!(
                result.replacement.unwrap(),
                raw.replacen("2024", "2024-2030", 1).as_bytes()
            );
            custom.license_notices =
                vec!["SPDX-License-Identifier: Apache-2.0\nSee LICENSE for full terms.\n".into()];
            let raw = render_header(
                &format!(
                    "Copyright 2024 Example Owner\n\n{}",
                    custom.license_notices[0]
                ),
                language,
            );
            let result = analyze(raw.as_bytes(), &custom, path);
            assert_eq!(
                result.replacement.unwrap(),
                raw.replacen("2024", "2024-2030", 1).as_bytes()
            );
        }
    }

    #[test]
    fn ambiguous_spdx_and_block_headers_never_offer_repairs() {
        let mut policy = policy();
        policy
            .license_notices
            .push("SPDX-License-Identifier: Apache-2.0\n".into());
        let text =
            "SPDX-FileCopyrightText: 2024 Example Owner\nSPDX-License-Identifier: Apache-2.0\n";
        for path in [
            "x.py", "x.js", "x.ts", "x.rs", "x.go", "x.swift", "x.sh", "x.c", "x.cpp",
        ] {
            let language = Language::from_path(std::path::Path::new(path)).unwrap();
            let header = render_header(text, language);
            for (raw, code) in [
                (header.replace("Example Owner", "Other Owner"), "LMH002"),
                (header.replace("2024", "2031"), "LMH003"),
                (header.replace("2024", "20x4"), "LMH006"),
                (header.replace("Apache-2.0", "MIT"), "LMH005"),
                (header.replace("Apache-2.0", "Apache-2.0 OR MIT"), "LMH005"),
                (format!("{header}{header}"), "LMH006"),
                (
                    render_header(
                        &text.replace(
                            "SPDX-License-Identifier:",
                            "Wrong notice.\nSPDX-License-Identifier:",
                        ),
                        language,
                    ),
                    "LMH006",
                ),
                (
                    format!(
                        "{header}{} SPDX-FileCopyrightText: 2024 Other Owner\n",
                        language.comment()
                    ),
                    "LMH006",
                ),
                (
                    format!(
                        "{header}\n{} This program is licensed under the MIT License.\n",
                        language.comment()
                    ),
                    "LMH006",
                ),
                (
                    format!(
                        "{header}\n{} Licensed under the MIT License.\n",
                        language.comment()
                    ),
                    "LMH006",
                ),
                (
                    format!(
                        "{header}{} SPDX-License-Identifier: Apache-2.0\n",
                        language.comment()
                    ),
                    "LMH006",
                ),
                (header.replace('\n', "\r"), "LMH006"),
                (format!("code\n{header}"), "LMH001"),
                (
                    format!(
                        "{} SPDX-FileCopyrightText: 2024 Example Owner\n",
                        language.comment()
                    ),
                    "LMH005",
                ),
            ] {
                let result = analyze(raw.as_bytes(), &policy, path);
                assert_eq!(result.diagnostic.unwrap().code, code, "{path}: {raw:?}");
                assert!(result.replacement.is_none(), "{path}: {raw:?}");
            }
            if language.comment() == "//" {
                let raw = format!("{header}/// SPDX-FileCopyrightText: 2024 Other Owner\n");
                assert_eq!(
                    analyze(raw.as_bytes(), &policy, path)
                        .diagnostic
                        .unwrap()
                        .code,
                    "LMH006"
                );
                for inner in [
                    " SPDX-License-Identifier: Apache-2.0\n * an example Copyright line\n",
                    " Copyright (C) 2024, Example Owner.\n\nLicense notice.\n/* nested */\n",
                ] {
                    let raw = format!("/*{inner}*/\n");
                    let result = analyze(raw.as_bytes(), &policy, path);
                    assert_eq!(result.diagnostic.unwrap().code, "LMH006", "{path}: {raw:?}");
                    assert!(result.replacement.is_none());
                }
                for text in [
                    "Copyright 2024 Example Owner\n\nWrong notice.\nSPDX-License-Identifier: Apache-2.0\n",
                    "Copyright 2024 Example Owner\nSPDX-License-Identifier: Apache-2.0\nWrong notice.\n",
                ] {
                    let raw = format!("/*\n{text}*/\n");
                    let result = analyze(raw.as_bytes(), &policy, path);
                    assert_eq!(result.diagnostic.unwrap().code, "LMH006", "{path}: {raw:?}");
                    assert!(result.replacement.is_none());
                }
                for raw in [
                    "/*\nCopyright 2024 Example Owner\n\nLicense notice.\n*/\n// SPDX-License-Identifier: MIT\n".to_string(),
                    format!("/*\n{text}"),
                    format!("{header}\n/* This program is licensed under the MIT License. */\n"),
                    format!("/*\n{text}*/\n// Licensed under the MIT License.\n"),
                    "/*\nCopyright 2024 Example Owner\n\nLicense notice.\nLicensed under the MIT License.\n*/\n".to_string(),
                    "/*\nCopyright 2024 Example Owner\n\nLicense notice.\n*/ /* Licensed under the MIT License. */\n".to_string(),
                    format!("/*\n{} */ /* Licensed under the MIT License. */\n", text.trim_end_matches('\n')),
                ] {
                    let result = analyze(raw.as_bytes(), &policy, path);
                    assert_eq!(result.diagnostic.unwrap().code, "LMH006", "{path}: {raw:?}");
                    assert!(result.replacement.is_none());
                }
                for prefix in ["/**", "/*!"] {
                    let raw = format!("{prefix}\n{text}*/\n");
                    let result = analyze(raw.as_bytes(), &policy, path);
                    assert_eq!(result.diagnostic.unwrap().code, "LMH006");
                    assert!(result.replacement.is_none());
                }
            }
        }
    }

    #[test]
    fn shell_comments_quotes_heredocs_and_preambles_are_preserved() {
        let header = "# Copyright (C) 2024, Example Owner.\n\n# License notice.\n\n";
        for body in [
            "value='# Copyright (C) 2024, Other.'\n",
            "value=\"# Copyright (C) 2024, Other.\"\n",
            "value=$'# Copyright (C) 2024, Other.'\n",
            "value='\n# Copyright (C) 2024, Other.\n'\n",
            "cat <<'EOF'\n# Copyright (C) 2024, Other.\nEOF\n",
            "cat <<EOF\n# Copyright (C) 2024, Other.\nEOF\n",
            "cat <<-EOF\n\t# Copyright (C) 2024, Other.\n\tEOF\n",
            "printf '%s\\n' word#Copyright \\#Copyright\n",
            "value=\"$(printf '%s' 'café')\"\n",
            "value=${name:-default}\nif [ -n \"$value\" ]; then printf '%s' \"$value\"; fi\n",
            "values=(one two); for value in \"${values[@]}\"; do echo \"$value\"; done\n",
        ] {
            for preamble in ["", "#!/bin/sh\n\n", "#!/usr/bin/env bash\n\n"] {
                for newline in ["\n", "\r\n"] {
                    let raw = format!("\u{feff}{preamble}{header}{body}").replace('\n', newline);
                    let result = analyze(raw.as_bytes(), &policy(), "x.sh");
                    assert!(
                        result.replacement.is_some(),
                        "{body}: {:?}",
                        result.diagnostic
                    );
                    let fixed = result.replacement.unwrap();
                    assert_eq!(fixed, raw.replacen("2024", "2024-2030", 1).as_bytes());
                    assert!(analyze(&fixed, &policy(), "x.sh").diagnostic.is_none());
                }
            }
        }
        for (body, code) in [
            ("# Copyright (C) 2024, Other.\necho ok\n", "LMH006"),
            (
                "value=$(\n# Copyright (C) 2024, Other.\nprintf ok\n)\n",
                "LMH004",
            ),
            (
                "value=`\n# Copyright (C) 2024, Other.\nprintf ok\n`\n",
                "LMH004",
            ),
            ("value='unterminated\n", "LMH004"),
            ("if true; then echo ok\n", "LMH004"),
        ] {
            let result = analyze(format!("{header}{body}").as_bytes(), &policy(), "x.bash");
            assert_eq!(result.diagnostic.unwrap().code, code, "{body}");
            assert_eq!(result.replacement.is_some(), code == "LMH004");
        }
        for (source, code) in [
            (header.replace("Example Owner", "Other"), "LMH002"),
            (header.replace("2024", "2031"), "LMH003"),
            (header.replace("License notice.", "Wrong."), "LMH005"),
            (format!("#!/bin/sh\n{header}echo ok\n"), "LMH006"),
            (header.replace('\n', "\r"), "LMH006"),
            (
                "echo '# Copyright (C) 2030, Example Owner.'\n".into(),
                "LMH001",
            ),
        ] {
            let result = analyze(source.as_bytes(), &policy(), "x.sh");
            assert_eq!(result.diagnostic.unwrap().code, code);
            assert!(result.replacement.is_none());
        }
        assert_eq!(
            analyze(b"\xff", &policy(), "x.sh").diagnostic.unwrap().code,
            "LMH007"
        );
    }

    #[test]
    fn c_and_cpp_comments_literals_and_preprocessor_directives_are_preserved() {
        let header = "// Copyright (C) 2024, Example Owner.\n\n// License notice.\n\n";
        for (path, body) in [
            (
                "x.c",
                "const char *value = \"// Copyright (C) 2024, Other.\";\n",
            ),
            (
                "x.c",
                "const char *value = \"/* Copyright (C) 2024, Other. */\";\n",
            ),
            ("x.c", "char value = '\\''; /* ordinary comment */\n"),
            (
                "x.c",
                "#include <stdio.h>\n#define VALUE 1\n#if VALUE\nint value = 1;\n#else\nint value = 2;\n#endif\n",
            ),
            (
                "x.c",
                "#define MESSAGE \"// Copyright (C) 2024, Other.\"\n#define SUM(a, b) \\\n ((a) + (b))\n",
            ),
            (
                "x.h",
                "#ifndef EXAMPLE_H\n#define EXAMPLE_H\nint value(void);\n#endif\n",
            ),
            (
                "x.h",
                "#define VALUE(x) _Generic((x), int: 1, default: 0)\n",
            ),
            (
                "x.cpp",
                "const auto value = u8\"// Copyright (C) 2024, Other.\";\n",
            ),
            (
                "x.cpp",
                "const auto value = R\"tag(\n// Copyright (C) 2024, Other.\n/* Copyright */\n)tag\";\n",
            ),
            (
                "x.cpp",
                "namespace example { template <typename T> T value(T x) { return x; } }\n",
            ),
            (
                "x.h",
                "#pragma once\nnamespace example { template <typename T> T value(T x) { return x; } }\n",
            ),
            (
                "x.h",
                "const auto value = R\"tag(\n// Copyright (C) 2024, Other.\n)tag\";\n",
            ),
        ] {
            for newline in ["\n", "\r\n"] {
                let raw = format!("\u{feff}{header}{body}").replace('\n', newline);
                let result = analyze(raw.as_bytes(), &policy(), path);
                assert!(
                    result.replacement.is_some(),
                    "{path}: {body}: {:?}",
                    result.diagnostic
                );
                let fixed = result.replacement.unwrap();
                assert_eq!(fixed, raw.replacen("2024", "2024-2030", 1).as_bytes());
                assert!(analyze(&fixed, &policy(), path).diagnostic.is_none());
            }
        }
        for path in ["x.c", "x.cpp", "x.h"] {
            for (body, code) in [
                ("// Copyright (C) 2024, Other.\nint value = 1;\n", "LMH006"),
                ("/// Copyright (C) 2024, Other.\nint value = 1;\n", "LMH006"),
                (
                    "/*! Copyright (C) 2024, Other. */\nint value = 1;\n",
                    "LMH006",
                ),
                ("#if 0\n// Copyright (C) 2024, Other.\n#endif\n", "LMH004"),
                ("const char *value = \"unterminated;\n", "LMH004"),
                ("/* unterminated\n", "LMH006"),
                ("int value( {\n", "LMH004"),
            ] {
                let result = analyze(format!("{header}{body}").as_bytes(), &policy(), path);
                assert_eq!(result.diagnostic.unwrap().code, code, "{path}: {body}");
                assert_eq!(result.replacement.is_some(), code == "LMH004");
            }
            for (source, code) in [
                (header.replace("Example Owner", "Other"), "LMH002"),
                (header.replace("2024", "2031"), "LMH003"),
                (header.replace("License notice.", "Wrong."), "LMH005"),
                (format!("#pragma once\n\n{header}"), "LMH001"),
                (header.replace('\n', "\r"), "LMH006"),
                (
                    "/* Copyright (C) 2024, Example Owner. */\n\n// License notice.\n".into(),
                    "LMH006",
                ),
                (
                    "const char *value = \"// Copyright (C) 2030, Example Owner.\";\n".into(),
                    "LMH001",
                ),
            ] {
                let result = analyze(source.as_bytes(), &policy(), path);
                assert_eq!(result.diagnostic.unwrap().code, code, "{path}: {source}");
                assert!(result.replacement.is_none());
            }
            assert_eq!(
                analyze(b"\xff", &policy(), path).diagnostic.unwrap().code,
                "LMH007"
            );
        }
    }

    #[test]
    fn swift_comments_strings_interpolation_and_preambles_are_preserved() {
        let header = "// Copyright (C) 2024, Example Owner.\n\n// License notice.\n\n";
        for body in [
            "let value = \"// Copyright (C) 2024, Other.\"\n",
            "let value = ##\"// Copyright (C) 2024, Other.\"##\n",
            "let value = \"\"\"\n// Copyright (C) 2024, Other.\n\"\"\"\n",
            "let value = #\"\"\"\n// Copyright (C) 2024, Other.\n\"\"\"#\n",
            "let value = \"café \\(1 + 2)\"\n",
            "let value = #\"café \\#(1 + 2)\"#\n",
            "let pattern = #/[// Copyright]/#\n",
            "actor Example { func value() async -> Int { 1 } }\n",
            "/* outer /* nested */ comment */\nlet value = 1\n",
            "#if DEBUG\nlet value = 1\n#endif\n",
        ] {
            for preamble in [
                "",
                "#!/usr/bin/env swift\n\n",
                "// swift-tools-version: 5.9\n\n",
            ] {
                let raw = format!("\u{feff}{preamble}{header}{body}").replace('\n', "\r\n");
                let fixed = analyze(raw.as_bytes(), &policy(), "x.swift")
                    .replacement
                    .unwrap();
                assert_eq!(
                    fixed,
                    raw.replacen("2024", "2024-2030", 1).as_bytes(),
                    "{body}"
                );
                assert!(analyze(&fixed, &policy(), "x.swift").diagnostic.is_none());
            }
        }
        for (body, code) in [
            ("// Copyright (C) 2024, Other.\nlet value = 1\n", "LMH006"),
            ("/// Copyright (C) 2024, Other.\nlet value = 1\n", "LMH006"),
            (
                "/* outer /* Copyright (C) 2024, Other. */ comment */\nlet value = 1\n",
                "LMH006",
            ),
            (
                "let value = \"\\(1 /* Copyright (C) 2024, Other. */)\"\n",
                "LMH004",
            ),
            ("let value = #\"unterminated\n", "LMH004"),
        ] {
            let result = analyze(format!("{header}{body}").as_bytes(), &policy(), "x.swift");
            assert_eq!(result.diagnostic.unwrap().code, code, "{body}");
            assert_eq!(result.replacement.is_some(), code == "LMH004");
        }
        let no_separator = format!("// swift-tools-version: 5.9\n{header}let value = 1\n");
        assert_eq!(
            analyze(no_separator.as_bytes(), &policy(), "Package.swift")
                .diagnostic
                .unwrap()
                .code,
            "LMH006"
        );
        assert_eq!(
            analyze(b"\xff", &policy(), "x.swift")
                .diagnostic
                .unwrap()
                .code,
            "LMH007"
        );
        assert_eq!(
            analyze(
                b"let value = \"// Copyright (C) 2030, Example Owner.\"\n",
                &policy(),
                "x.swift"
            )
            .diagnostic
            .unwrap()
            .code,
            "LMH001"
        );
    }

    #[test]
    fn go_comments_literals_build_tags_and_byte_preserving_repairs() {
        let header = "// Copyright (C) 2024, Example Owner.\n\n// License notice.\n\n";
        for body in [
            "package example\nconst value = \"// Copyright (C) 2024, Other.\"\n",
            "package example\nconst value = `\n// Copyright (C) 2024, Other.\n`\n",
            "package example\nconst slash = '/'\nconst quote = '\\''\n",
            "package example\nfunc identity[T any](value T) T { return value }\n",
            "//go:generate echo example\npackage example\n/* cgo flags */\nimport \"C\"\n",
        ] {
            for preamble in [
                "",
                "//go:build linux\n// +build linux\n\n",
                "//go:build\tlinux\n\n",
            ] {
                let raw = format!("\u{feff}{preamble}{header}{body}").replace('\n', "\r\n");
                let fixed = analyze(raw.as_bytes(), &policy(), "x.go")
                    .replacement
                    .unwrap();
                assert_eq!(
                    fixed,
                    raw.replacen("2024", "2024-2030", 1).as_bytes(),
                    "{body}"
                );
                assert!(analyze(&fixed, &policy(), "x.go").diagnostic.is_none());
            }
        }
        assert!(
            analyze(
                format!("{header}//go:build linux\n\npackage example\n").as_bytes(),
                &policy(),
                "x.go"
            )
            .replacement
            .is_some()
        );
        for (body, code) in [
            ("// Copyright (C) 2024, Other.\npackage example\n", "LMH006"),
            (
                "/* Copyright (C) 2024, Other. */\npackage example\n",
                "LMH006",
            ),
            ("package example\nconst value = `unterminated\n", "LMH004"),
            ("package example\n/* unterminated\n", "LMH004"),
        ] {
            let result = analyze(format!("{header}{body}").as_bytes(), &policy(), "x.go");
            assert_eq!(result.diagnostic.unwrap().code, code, "{body}");
            assert_eq!(result.replacement.is_some(), code == "LMH004");
        }
        let no_separator = format!("//go:build linux\n{header}package example\n");
        assert_eq!(
            analyze(no_separator.as_bytes(), &policy(), "x.go")
                .diagnostic
                .unwrap()
                .code,
            "LMH006"
        );
        assert_eq!(
            analyze(b"\xff", &policy(), "x.go").diagnostic.unwrap().code,
            "LMH007"
        );
        assert_eq!(
            analyze(
                b"package example\nconst value = `// Copyright (C) 2030, Example Owner.`\n",
                &policy(),
                "x.go"
            )
            .diagnostic
            .unwrap()
            .code,
            "LMH001"
        );
    }

    #[test]
    fn rust_comments_literals_attributes_and_byte_preserving_repairs() {
        let header = "// Copyright (C) 2024, Example Owner.\n\n// License notice.\n\n";
        for body in [
            "#![allow(dead_code)]\nconst VALUE: &str = \"café\";\n",
            "const VALUE: &str = r###\"\n// Copyright (C) 2024, Other.\n\"###;\n",
            "const VALUE: &[u8] = br##\"// Copyright (C) 2024, Other.\"##;\n",
            "const VALUE: &std::ffi::CStr = c\"// Copyright (C) 2024, Other.\";\n",
            "const VALUE: &std::ffi::CStr = cr#\"// Copyright (C) 2024, Other.\"#;\n",
            "#[doc = \"// Copyright (C) 2024, Other.\"]\nfn example() {}\n",
            "fn borrow<'a>(value: &'a str) -> &'a str { let slash = '/'; value }\n",
            "/* outer /* nested */ comment */\nfn main() {}\n",
            "macro_rules! value { ($v:expr) => { $v }; }\nfn main() { value!(\"// Copyright (C) 2024, Other.\"); }\n",
        ] {
            for preamble in ["", "#!/usr/bin/env rust-script\n\n"] {
                let raw = format!("\u{feff}{preamble}{header}{body}").replace('\n', "\r\n");
                let fixed = analyze(raw.as_bytes(), &policy(), "x.rs")
                    .replacement
                    .unwrap();
                assert_eq!(
                    fixed,
                    raw.replacen("2024", "2024-2030", 1).as_bytes(),
                    "{body}"
                );
                assert!(analyze(&fixed, &policy(), "x.rs").diagnostic.is_none());
            }
        }
        for (body, code) in [
            ("// Copyright (C) 2024, Other.\nfn main() {}\n", "LMH006"),
            (
                "/* outer /* Copyright (C) 2024, Other. */ comment */\nfn main() {}\n",
                "LMH006",
            ),
            ("//! Copyright (C) 2024, Other.\nfn main() {}\n", "LMH006"),
            ("/// Copyright (C) 2024, Other.\nfn main() {}\n", "LMH006"),
            (
                "macro_rules! value { () => { // Copyright (C) 2024, Other.\n1 }; }\n",
                "LMH004",
            ),
            ("const VALUE: &str = r##\"unterminated;\n", "LMH004"),
        ] {
            let result = analyze(format!("{header}{body}").as_bytes(), &policy(), "x.rs");
            assert_eq!(result.diagnostic.unwrap().code, code, "{body}");
            assert_eq!(result.replacement.is_some(), code == "LMH004");
        }
        for attribute in [
            "#![allow(dead_code)]",
            "#! [allow(dead_code)]",
            "#!/* comment */[allow(dead_code)]",
        ] {
            let raw = format!("{attribute}\n\n{header}fn main() {{}}\n");
            assert_eq!(
                analyze(raw.as_bytes(), &policy(), "x.rs")
                    .diagnostic
                    .unwrap()
                    .code,
                "LMH001"
            );
        }
        assert_eq!(
            analyze(b"\xff", &policy(), "x.rs").diagnostic.unwrap().code,
            "LMH007"
        );
        assert_eq!(
            analyze(
                b"const EXAMPLE: &str = \"// Copyright (C) 2030, Example Owner.\";",
                &policy(),
                "x.rs"
            )
            .diagnostic
            .unwrap()
            .code,
            "LMH001"
        );
    }

    #[test]
    fn javascript_typescript_and_jsx_headers_preserve_the_body() {
        let header = "// Copyright (C) 2024, Example Owner.\n\n// License notice.\n\n";
        for (path, body) in [
            (
                "x.js",
                "const s = '// Copyright (C) 2024, Other.'; const re = /[//]/;\n",
            ),
            (
                "x.ts",
                "const s: string = `\n// Copyright (C) 2024, Other.\n`;\n",
            ),
            ("x.ts", "const id = <T>(value: T): T => value;\n"),
            (
                "x.tsx",
                "const view = <div>\n// Copyright (C) 2024, Other.\n</div>;\n",
            ),
            (
                "x.jsx",
                "const view = <div>{`// Copyright (C) 2024, Other.`}</div>;\n",
            ),
        ] {
            let raw =
                format!("\u{feff}#!/usr/bin/env node\n\n{header}{body}").replace('\n', "\r\n");
            let fixed = analyze(raw.as_bytes(), &policy(), path)
                .replacement
                .unwrap();
            assert_eq!(
                fixed,
                raw.replacen("2024", "2024-2030", 1).as_bytes(),
                "{path}: {body}"
            );
            assert!(analyze(&fixed, &policy(), path).diagnostic.is_none());
        }
        for (body, code) in [
            ("// Copyright (C) 2024, Other.\n", "LMH006"),
            ("/* Copyright (C) 2024, Other. */\n", "LMH006"),
            (
                "const s = `${(\n// Copyright (C) 2024, Other.\n1)}`;\n",
                "LMH004",
            ),
            ("const broken = `unterminated;\n", "LMH004"),
        ] {
            let result = analyze(format!("{header}{body}").as_bytes(), &policy(), "x.tsx");
            assert_eq!(result.diagnostic.unwrap().code, code, "{body}");
            assert_eq!(result.replacement.is_some(), code == "LMH004");
        }
        for (raw, code) in [
            (header.replace("Example Owner", "Other"), "LMH002"),
            (header.replace("2024", "2031"), "LMH003"),
            (header.replace("License notice.", "Wrong."), "LMH005"),
            (format!("const value = 1; {header}"), "LMH001"),
            (format!("#!/usr/bin/env node\n{header}"), "LMH006"),
            (header.replace('\n', "\r"), "LMH006"),
        ] {
            assert_eq!(
                analyze(raw.as_bytes(), &policy(), "x.js")
                    .diagnostic
                    .unwrap()
                    .code,
                code
            );
        }
        assert_eq!(
            analyze(b"\xff", &policy(), "x.ts").diagnostic.unwrap().code,
            "LMH007"
        );
        assert_eq!(
            analyze(
                b"const s = '// Copyright (C) 2030, Example Owner.';",
                &policy(),
                "x.js"
            )
            .diagnostic
            .unwrap()
            .code,
            "LMH001"
        );
    }

    #[test]
    fn only_verified_python_encodings_are_decoded() {
        assert_eq!(
            decode_source(b"# coding: cp1252\n\n# \x80\n").unwrap(),
            "# coding: cp1252\n\n# €\n"
        );
        for label in [
            "utf-7",
            "cp932",
            "shift_jis",
            "iso-8859-9",
            "cp1250",
            "cp1251",
        ] {
            let raw = format!("# coding: {label}\n\n{}", header("2024"));
            let result = inspect(raw.as_bytes());
            assert_eq!(result.diagnostic.unwrap().code, "LMH007");
            assert!(result.replacement.is_none());
        }
    }

    #[test]
    fn diagnostics_and_precedence() {
        let valid = header("2030");
        assert!(inspect(valid.as_bytes()).diagnostic.is_none());
        let cases = [
            ("value = 1\n".into(), "LMH001", 1),
            (
                header("2024").replace("Example Owner", "Someone Else"),
                "LMH002",
                1,
            ),
            (header("2031"), "LMH003", 1),
            (header("2024"), "LMH004", 1),
            (
                header("2024").replace("# License notice.", "# Wrong."),
                "LMH005",
                3,
            ),
            (header("20x4"), "LMH006", 1),
            (format!("# coding: made-up-codec\n\n{valid}"), "LMH007", 1),
            (header("2024").replacen("\n\n", "\n", 1), "LMH006", 2),
            (format!("#!/usr/bin/python\n{valid}"), "LMH006", 2),
        ];
        for (source, code, line) in cases {
            let result = inspect(source.as_bytes());
            let diagnostic = result.diagnostic.unwrap();
            assert_eq!(diagnostic.code, code, "{source}");
            assert_eq!(diagnostic.line, line, "{source}");
            assert_eq!(diagnostic.fixable, code == "LMH004");
            assert_eq!(result.replacement.is_some(), code == "LMH004");
        }
    }

    #[test]
    fn only_leading_copyright_comments_count() {
        for literal in [
            "\"\"\"\n# Copyright (C) 2024, Other.\n\"\"\"",
            "'''\n# Copyright (C) 2024, Other.\n'''",
            "r'\"# Copyright (C) 2024, Other.\"'",
        ] {
            let source = format!("{}\nEXAMPLE = {literal}\n", header("2030"));
            assert!(inspect(source.as_bytes()).diagnostic.is_none());
        }
        let duplicate = format!("{}\n# Copyright (C) 2024, Other.\n", header("2030"));
        assert!(inspect(duplicate.as_bytes()).diagnostic.is_none());
        let example =
            "EXAMPLE = '''\n# Copyright (C) 2030, Example Owner.\n\n# License notice.\n'''\n";
        assert_eq!(
            inspect(example.as_bytes()).diagnostic.unwrap().code,
            "LMH001"
        );
    }

    #[test]
    fn first_code_line_ends_header_validation_in_every_language() {
        for (path, body) in [
            ("x.py", "value =\n"),
            ("x.js", "const value =\n"),
            ("x.tsx", "const value =\n"),
            ("x.rs", "fn broken( {\n"),
            ("x.go", "package example\n"),
            ("x.swift", "let value =\n"),
            ("x.sh", "value='unterminated\n"),
            ("x.c", "int value( {\n"),
            ("x.cpp", "namespace example {\n"),
            ("x.h", "namespace example {\n"),
        ] {
            let language = Language::from_path(std::path::Path::new(path)).unwrap();
            let header = render_header(
                "Copyright (C) 2024, Example Owner.\n\nLicense notice.\n\n",
                language,
            );
            let notice = format!("{} Copyright (C) 2024, Other.\n", language.comment());
            let raw = format!("{header}{body}{notice}");
            assert_eq!(
                analyze(raw.as_bytes(), &policy(), path)
                    .replacement
                    .unwrap(),
                raw.replacen("2024", "2024-2030", 1).as_bytes()
            );
            let missing = analyze(format!("{body}{header}").as_bytes(), &policy(), path);
            assert_eq!(missing.diagnostic.unwrap().code, "LMH001", "{path}");
            assert!(missing.replacement.is_none());
            let duplicate = analyze(
                format!("{header}{notice}{body}").as_bytes(),
                &policy(),
                path,
            );
            assert_eq!(duplicate.diagnostic.unwrap().code, "LMH006", "{path}");
            assert!(duplicate.replacement.is_none());
        }
    }

    #[test]
    fn leading_block_comments_end_before_code_on_the_same_line() {
        for path in ["x.js", "x.ts", "x.rs", "x.go", "x.swift", "x.c", "x.cpp"] {
            let header = "// Copyright (C) 2024, Example Owner.\n\n// License notice.\n\n";
            for comment in ["/* ordinary */", "/* outer /* nested */ ordinary */"] {
                let raw = format!("{header}{comment} code\n// Copyright (C) 2024, Other.\n");
                assert_eq!(
                    analyze(raw.as_bytes(), &policy(), path)
                        .replacement
                        .unwrap(),
                    raw.replacen("2024", "2024-2030", 1).as_bytes(),
                    "{path}"
                );
            }
            for tail in [
                "/* Copyright */ code\n",
                "/* ordinary */ // Copyright (C) 2024, Other.\n",
                "/* unterminated\n",
            ] {
                let result = analyze(format!("{header}{tail}").as_bytes(), &policy(), path);
                assert_eq!(result.diagnostic.unwrap().code, "LMH006", "{path}: {tail}");
                assert!(result.replacement.is_none());
            }
        }
    }

    #[test]
    fn non_script_languages_do_not_skip_hashbang_code() {
        for path in ["x.c", "x.cpp", "x.h", "x.go"] {
            let raw = "#!/bin/sh\n\n// Copyright (C) 2024, Example Owner.\n\n// License notice.\n";
            let result = analyze(raw.as_bytes(), &policy(), path);
            assert_eq!(result.diagnostic.unwrap().code, "LMH001", "{path}");
            assert!(result.replacement.is_none());
        }
    }

    #[test]
    fn javascript_unicode_line_separators_cannot_authorize_a_repair() {
        for path in ["x.js", "x.ts", "x.jsx", "x.tsx"] {
            for separator in ['\u{2028}', '\u{2029}'] {
                let mut policy = policy();
                policy.owner = format!("Example{separator}Owner");
                let raw = format!(
                    "// Copyright (C) 2024, {}.\n\n// License notice.\n\nconst value = 1;\n",
                    policy.owner
                );
                let result = analyze(raw.as_bytes(), &policy, path);
                assert_eq!(result.diagnostic.unwrap().code, "LMH006");
                assert!(result.replacement.is_none());
            }
            let raw = "// Copyright (C) 2024, Example Owner.\n\n// License notice.\n\nconst value = '\u{2028}\u{2029}';\n";
            assert!(
                analyze(raw.as_bytes(), &policy(), path)
                    .replacement
                    .is_some()
            );
        }
    }

    #[test]
    fn repairs_preserve_bom_preambles_crlf_encoding_and_body() {
        for preamble in [
            "",
            "#!/usr/bin/env python3\n\n",
            "# coding: utf-8\n\n",
            "#!/usr/bin/env python3\n# coding: utf-8\n\n",
        ] {
            let raw = format!("\u{feff}{preamble}{}", header("2024"))
                .replace('\n', "\r\n")
                .into_bytes();
            let fixed = inspect(&raw).replacement.unwrap();
            assert_eq!(
                fixed,
                String::from_utf8(raw)
                    .unwrap()
                    .replacen("2024", "2024-2030", 1)
                    .as_bytes()
            );
            assert!(inspect(&fixed).diagnostic.is_none());
        }
        let text = format!("# coding: latin-1\n\n{}", header("2024"));
        let raw: Vec<u8> = text.chars().map(|c| c as u8).collect();
        let repaired = analyze(&raw, &policy(), "latin.py").replacement.unwrap();
        let expected: Vec<u8> = text
            .replacen("2024", "2024-2030", 1)
            .chars()
            .map(|c| c as u8)
            .collect();
        assert_eq!(repaired, expected);
        assert_eq!(
            decode_source(b"# coding: latin-1\n\n# \x80\n").unwrap(),
            "# coding: latin-1\n\n# \u{80}\n"
        );
    }

    #[test]
    fn repair_requires_four_digit_years_even_with_unchecked_policy() {
        for (first, current) in [(0, 2030), (24, 2030), (2024, 10000)] {
            let mut policy = policy();
            policy.starting_year = first;
            policy.current_year = current;
            let result = analyze(header(&format!("{first:04}")).as_bytes(), &policy, "x.py");
            assert_eq!(result.diagnostic.as_ref().unwrap().code, "LMH004");
            assert!(!result.diagnostic.unwrap().fixable);
            assert!(result.replacement.is_none());
        }
    }

    #[test]
    fn encoding_failures_never_offer_repair() {
        for raw in [
            b"\xef\xbb\xbf# coding: latin-1\n".as_slice(),
            b"# coding: ascii\n\n# \xff\n",
            b"# coding: utf-8\n\n# \xff\n",
            b"# coding: cp1252\n\n# \x81\n",
            b"# coding: cp1250\n\n# \x81\n",
            b"# coding: cp1251\n\n# \x98\n",
            b"# coding: iso-8859-9\n\n# \x80\n",
            b"# coding: latin-1 \xff\n",
        ] {
            let result = inspect(raw);
            assert_eq!(result.diagnostic.unwrap().code, "LMH007");
            assert!(result.replacement.is_none());
        }
        let source = format!("value = 0\n# coding: utf-8\n\n{}", header("2030"));
        assert_eq!(
            inspect(source.as_bytes()).diagnostic.unwrap().code,
            "LMH001"
        );
    }

    #[test]
    fn bare_carriage_returns_do_not_authorize_a_repair() {
        for years in ["2024", "2030"] {
            let source = header(years).replace('\n', "\r");
            let result = inspect(source.as_bytes());
            let diagnostic = result.diagnostic.unwrap();
            assert_eq!(diagnostic.code, "LMH006");
            assert_eq!(diagnostic.line, 1);
            assert!(!diagnostic.fixable);
            assert!(result.replacement.is_none());
        }
        let mixed = header("2024").replacen('\n', "\r", 1);
        assert!(inspect(mixed.as_bytes()).replacement.is_none());
    }

    #[test]
    fn body_syntax_and_later_copyright_notices_are_ignored() {
        for body in ["x = 0xZZ\n", "x = ) )\n", "x = f\"{\n 1 + 1\n}\"\n"] {
            let source = format!("{}{body}", header("2024"));
            // Malformed bodies alone do not prevent an unambiguous year-only repair.
            assert!(inspect(source.as_bytes()).replacement.is_some());
            for tail in [
                "# Copyright (C) 2024, Other Owner.\n",
                "example = '# Copyright (C) 2024, Other Owner.'\n",
            ] {
                let result = inspect(format!("{source}{tail}").as_bytes());
                assert_eq!(
                    result.replacement.unwrap(),
                    format!("{source}{tail}")
                        .replacen("2024", "2024-2030", 1)
                        .as_bytes()
                );
            }
        }
    }

    #[test]
    fn formatted_or_template_string_bodies_are_ignored() {
        for prefix in ["f", "F", "rf", "fr", "t", "T", "tr", "rt", "tR", "RT"] {
            let source = format!(
                "{}\nx = {prefix}'''{{\n# Copyright (C) 2024, Other Owner.\n1\n}}'''\n",
                header("2024")
            );
            let result = inspect(source.as_bytes());
            assert_eq!(
                result.replacement.unwrap(),
                source.replacen("2024", "2024-2030", 1).as_bytes(),
                "{prefix}"
            );
        }
        let literal = format!(
            "{}\nx = '''{{\n# Copyright (C) 2024, Other Owner.\n1\n}}'''\n",
            header("2024")
        );
        assert!(inspect(literal.as_bytes()).replacement.is_some());
    }

    #[test]
    fn literal_formatted_headers_and_escaped_braces_are_not_comments() {
        for value in [
            "# Copyright (C) {year}, Example Owner.",
            "{year}\n# Copyright (C) 2024, Example Owner.",
            "{{\n# Copyright (C) 2024, Example Owner.\n}}",
        ] {
            let source = format!("{}\nx = f'''{value}'''\n", header("2024"));
            assert!(inspect(source.as_bytes()).replacement.is_some(), "{value}");
        }
        let source = include_str!("../.github/smoke_distribution.py");
        let mut fixture_policy = policy();
        fixture_policy.owner = "François-Guillaume Fernandez".into();
        fixture_policy.current_year = 2026;
        fixture_policy.license_notices = vec![
            "This program is licensed under the Apache License 2.0.\n\
             See LICENSE or go to <https://www.apache.org/licenses/LICENSE-2.0> for full license details.\n".into(),
        ];
        assert!(
            analyze(source.as_bytes(), &fixture_policy, "smoke_distribution.py")
                .diagnostic
                .is_none()
        );
    }
}
