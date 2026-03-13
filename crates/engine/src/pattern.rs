pub(super) enum CompiledPattern<'a> {
    Any,
    Exact(&'a [u8]),
    Prefix(&'a [u8]),
    Suffix(&'a [u8]),
    Contains(&'a [u8]),
    PrefixSuffix { prefix: &'a [u8], suffix: &'a [u8] },
    Wildcard(&'a [u8]),
}

impl<'a> CompiledPattern<'a> {
    pub(super) fn new(pattern: Option<&'a [u8]>) -> Self {
        let Some(pattern) = pattern else {
            return Self::Any;
        };
        if pattern.is_empty() || pattern == b"*" {
            return Self::Any;
        }

        let mut first_star = None;
        let mut star_count = 0usize;
        let mut has_question = false;
        let mut has_other_glob = false;

        for (idx, &byte) in pattern.iter().enumerate() {
            match byte {
                b'*' => {
                    star_count += 1;
                    if first_star.is_none() {
                        first_star = Some(idx);
                    }
                }
                b'?' => has_question = true,
                b'[' | b'\\' => has_other_glob = true,
                _ => {}
            }
        }

        if has_question || has_other_glob {
            return Self::Wildcard(pattern);
        }

        match (star_count, first_star) {
            (0, _) => Self::Exact(pattern),
            (1, Some(pos)) if pos == pattern.len() - 1 => Self::Prefix(&pattern[..pos]),
            (1, Some(0)) => Self::Suffix(&pattern[1..]),
            (1, Some(pos)) => Self::PrefixSuffix {
                prefix: &pattern[..pos],
                suffix: &pattern[pos + 1..],
            },
            (2, Some(0)) if pattern[pattern.len() - 1] == b'*' => {
                Self::Contains(&pattern[1..pattern.len() - 1])
            }
            _ => Self::Wildcard(pattern),
        }
    }
}

pub fn wildcard_match(pattern: &[u8], text: &[u8]) -> bool {
    wildcard_match_from(pattern, 0, text, 0)
}

fn wildcard_match_from(
    pattern: &[u8],
    pattern_index: usize,
    text: &[u8],
    text_index: usize,
) -> bool {
    let mut pattern_index = pattern_index;
    let mut text_index = text_index;
    let mut star = None;

    while text_index < text.len() {
        let Some((token, next_pattern_index)) = next_token(pattern, pattern_index) else {
            return match star {
                Some((star_pattern_index, star_text_index)) => {
                    wildcard_match_from(pattern, star_pattern_index, text, star_text_index + 1)
                }
                None => false,
            };
        };

        match token {
            PatternToken::AnySequence => {
                star = Some((next_pattern_index, text_index));
                pattern_index = next_pattern_index;
            }
            PatternToken::AnySingle => {
                pattern_index = next_pattern_index;
                text_index += 1;
            }
            PatternToken::Literal(byte) => {
                if byte == text[text_index] {
                    pattern_index = next_pattern_index;
                    text_index += 1;
                } else if let Some((star_pattern_index, star_text_index)) = star {
                    return wildcard_match_from(
                        pattern,
                        star_pattern_index,
                        text,
                        star_text_index + 1,
                    );
                } else {
                    return false;
                }
            }
            PatternToken::Class(matches) => {
                if matches.matches(text[text_index]) {
                    pattern_index = next_pattern_index;
                    text_index += 1;
                } else if let Some((star_pattern_index, star_text_index)) = star {
                    return wildcard_match_from(
                        pattern,
                        star_pattern_index,
                        text,
                        star_text_index + 1,
                    );
                } else {
                    return false;
                }
            }
        }
    }

    while let Some((token, next_pattern_index)) = next_token(pattern, pattern_index) {
        match token {
            PatternToken::AnySequence => pattern_index = next_pattern_index,
            _ => return false,
        }
    }

    true
}

#[derive(Clone, Copy)]
enum PatternToken<'a> {
    AnySequence,
    AnySingle,
    Literal(u8),
    Class(BoxMatch<'a>),
}

#[derive(Clone, Copy)]
struct BoxMatch<'a> {
    class: &'a [u8],
}

impl BoxMatch<'_> {
    fn matches(self, byte: u8) -> bool {
        let mut index = 0;
        let mut matched = false;
        let mut negated = false;

        if self
            .class
            .first()
            .copied()
            .is_some_and(|byte| byte == b'!' || byte == b'^')
        {
            negated = true;
            index += 1;
        }

        while index < self.class.len() {
            let Some((start, next_index)) = read_class_byte(self.class, index) else {
                break;
            };
            index = next_index;

            if index + 1 < self.class.len()
                && self.class[index] == b'-'
                && let Some((end, range_next_index)) = read_class_byte(self.class, index + 1)
            {
                if start <= byte && byte <= end {
                    matched = true;
                }
                index = range_next_index;
                continue;
            }

            if start == byte {
                matched = true;
            }
        }

        if negated { !matched } else { matched }
    }
}

fn next_token(pattern: &[u8], pattern_index: usize) -> Option<(PatternToken<'_>, usize)> {
    let byte = *pattern.get(pattern_index)?;
    match byte {
        b'*' => Some((PatternToken::AnySequence, pattern_index + 1)),
        b'?' => Some((PatternToken::AnySingle, pattern_index + 1)),
        b'\\' => {
            let escaped = pattern.get(pattern_index + 1).copied().unwrap_or(b'\\');
            let next_index = if pattern_index + 1 < pattern.len() {
                pattern_index + 2
            } else {
                pattern_index + 1
            };
            Some((PatternToken::Literal(escaped), next_index))
        }
        b'[' => match parse_class(pattern, pattern_index) {
            Some((class, next_index)) => Some((PatternToken::Class(class), next_index)),
            None => Some((PatternToken::Literal(b'['), pattern_index + 1)),
        },
        _ => Some((PatternToken::Literal(byte), pattern_index + 1)),
    }
}

fn parse_class(pattern: &[u8], pattern_index: usize) -> Option<(BoxMatch<'_>, usize)> {
    let mut index = pattern_index + 1;
    if index >= pattern.len() {
        return None;
    }

    if matches!(pattern.get(index), Some(b']')) {
        index += 1;
    }

    while index < pattern.len() {
        if pattern[index] == b'\\' {
            index = index.saturating_add(2);
            continue;
        }
        if pattern[index] == b']' {
            return Some((
                BoxMatch {
                    class: &pattern[pattern_index + 1..index],
                },
                index + 1,
            ));
        }
        index += 1;
    }

    None
}

fn read_class_byte(class: &[u8], index: usize) -> Option<(u8, usize)> {
    let byte = *class.get(index)?;
    if byte == b'\\' {
        Some((
            class.get(index + 1).copied().unwrap_or(b'\\'),
            (index + 2).min(class.len()),
        ))
    } else {
        Some((byte, index + 1))
    }
}
