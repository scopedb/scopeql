// Copyright 2025 ScopeDB, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Simple shell-like lexical splitting with Rust-style escape sequences.
//!
//! This module provides shell-style word splitting with:
//! - Single quotes: minimal escaping (`\'` and `\\` only)
//! - Double quotes: Rust-style escaping (`\n`, `\t`, `\r`, `\0`, `\"`, `\\`, `\'`)
//! - Unquoted: backslash escapes any character
//! - Whitespace as word separator

use std::fmt;
use std::iter::Peekable;
use std::str::Chars;

/// Error type for shell lexical splitting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShlexError {
    /// Unclosed single quote.
    UnclosedSingleQuote,
    /// Unclosed double quote.
    UnclosedDoubleQuote,
    /// Trailing backslash with no character to escape.
    TrailingBackslash,
    /// Unknown escape sequence.
    UnknownEscape(char),
}

impl fmt::Display for ShlexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShlexError::UnclosedSingleQuote => write!(f, "unclosed single quote"),
            ShlexError::UnclosedDoubleQuote => write!(f, "unclosed double quote"),
            ShlexError::TrailingBackslash => write!(f, "trailing backslash"),
            ShlexError::UnknownEscape(ch) => write!(f, "unknown escape sequence: \\{}", ch),
        }
    }
}

impl std::error::Error for ShlexError {}

/// Split a string into words using shell-like rules with Rust-style escaping.
///
/// Returns an error if the input has unclosed quotes, trailing backslash,
/// or unknown escape sequences in quoted strings.
pub fn split(input: &str) -> Result<Vec<String>, ShlexError> {
    let mut words = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next(); // Skip whitespace
            }
            '\'' => {
                words.push(parse_single_quoted(&mut chars)?);
            }
            '"' => {
                words.push(parse_double_quoted(&mut chars)?);
            }
            _ => {
                words.push(parse_unquoted(&mut chars)?);
            }
        }
    }

    Ok(words)
}

fn parse_single_quoted(chars: &mut Peekable<Chars>) -> Result<String, ShlexError> {
    chars.next(); // Skip opening single quote
    let mut result = String::new();

    loop {
        match chars.next() {
            None => return Err(ShlexError::UnclosedSingleQuote),
            Some('\\') => match chars.next() {
                None => return Err(ShlexError::TrailingBackslash),
                Some('\'') => result.push('\''),
                Some('\\') => result.push('\\'),
                Some(ch) => return Err(ShlexError::UnknownEscape(ch)),
            },
            Some('\'') => break, // Found closing single quote
            Some(ch) => result.push(ch),
        }
    }

    Ok(result)
}

fn parse_double_quoted(chars: &mut Peekable<Chars>) -> Result<String, ShlexError> {
    chars.next(); // Skip opening double quote
    let mut result = String::new();

    loop {
        match chars.next() {
            None => return Err(ShlexError::UnclosedDoubleQuote),
            Some('\\') => {
                // Double quotes: Rust-style escaping
                match chars.next() {
                    None => return Err(ShlexError::TrailingBackslash),
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('0') => result.push('\0'),
                    Some('"') => result.push('"'),
                    Some('\\') => result.push('\\'),
                    Some('\'') => result.push('\''),
                    Some(ch) => return Err(ShlexError::UnknownEscape(ch)),
                }
            }
            Some('"') => break, // Found closing double quote
            Some(ch) => result.push(ch),
        }
    }

    Ok(result)
}

fn parse_unquoted(chars: &mut Peekable<Chars>) -> Result<String, ShlexError> {
    let mut result = String::new();

    loop {
        match chars.peek() {
            None | Some(' ') | Some('\t') | Some('\n') | Some('\r') => break,
            Some('\\') => {
                chars.next();
                // Outside quotes: escape any character
                match chars.next() {
                    None => return Err(ShlexError::TrailingBackslash),
                    Some(ch) => result.push(ch),
                }
            }
            Some(_) => {
                result.push(chars.next().unwrap());
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_split() {
        assert_eq!(
            split("foo bar baz"),
            Ok(vec!["foo", "bar", "baz"]
                .iter()
                .map(|s| s.to_string())
                .collect())
        );
    }

    #[test]
    fn test_single_quotes() {
        assert_eq!(
            split("foo 'bar baz'"),
            Ok(vec!["foo", "bar baz"]
                .iter()
                .map(|s| s.to_string())
                .collect())
        );
    }

    #[test]
    fn test_double_quotes() {
        assert_eq!(
            split(r#"foo "bar baz""#),
            Ok(vec!["foo", "bar baz"]
                .iter()
                .map(|s| s.to_string())
                .collect())
        );
    }

    #[test]
    fn test_escape_outside_quotes() {
        assert_eq!(
            split(r"foo\ bar"),
            Ok(vec!["foo bar"].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r#"foo \"bar\""#),
            Ok(vec!["foo", r#""bar""#]
                .iter()
                .map(|s| s.to_string())
                .collect())
        );
    }

    #[test]
    fn test_escape_in_double_quotes() {
        // Rust-style escapes work in double quotes
        assert_eq!(
            split(r#""foo\nbar""#),
            Ok(vec!["foo\nbar"].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r#""foo\tbar""#),
            Ok(vec!["foo\tbar"].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r#""foo\rbar""#),
            Ok(vec!["foo\rbar"].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r#""foo\0bar""#),
            Ok(vec!["foo\0bar"].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r#""foo\"bar""#),
            Ok(vec![r#"foo"bar"#].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r#""foo\\bar""#),
            Ok(vec![r#"foo\bar"#].iter().map(|s| s.to_string()).collect())
        );
    }

    #[test]
    fn test_escape_in_single_quotes() {
        assert_eq!(
            split(r"'foo\\'"),
            Ok(vec![r"foo\"].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r"'foo\''"),
            Ok(vec![r"foo'"].iter().map(|s| s.to_string()).collect())
        );
    }

    #[test]
    fn test_unknown_escape_in_double_quotes() {
        assert_eq!(split(r#""foo\xbar""#), Err(ShlexError::UnknownEscape('x')));
        assert_eq!(split(r#""foo\abar""#), Err(ShlexError::UnknownEscape('a')));
    }

    #[test]
    fn test_unknown_escape_in_single_quotes() {
        assert_eq!(split(r"'foo\nbar'"), Err(ShlexError::UnknownEscape('n')));
        assert_eq!(split(r"'foo\abar'"), Err(ShlexError::UnknownEscape('a')));
    }

    #[test]
    fn test_escape_outside_quotes_accepts_any() {
        // Outside quotes, backslash escapes any character
        assert_eq!(
            split(r"foo\xbar"),
            Ok(vec!["fooxbar"].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r"foo\abar"),
            Ok(vec!["fooabar"].iter().map(|s| s.to_string()).collect())
        );
    }

    #[test]
    fn test_mixed_quotes() {
        assert_eq!(
            split(r#"foo 'bar "baz"' qux"#),
            Ok(vec!["foo", r#"bar "baz""#, "qux"]
                .iter()
                .map(|s| s.to_string())
                .collect())
        );
    }

    #[test]
    fn test_unclosed_quotes() {
        assert_eq!(split("foo 'bar"), Err(ShlexError::UnclosedSingleQuote));
        assert_eq!(split(r#"foo "bar"#), Err(ShlexError::UnclosedDoubleQuote));
    }

    #[test]
    fn test_trailing_backslash() {
        assert_eq!(split(r"foo\"), Err(ShlexError::TrailingBackslash));
        assert_eq!(split(r#""foo\"#), Err(ShlexError::TrailingBackslash));
        assert_eq!(split(r"'foo\"), Err(ShlexError::TrailingBackslash));
    }

    #[test]
    fn test_empty_and_whitespace() {
        assert_eq!(split(""), Ok(vec![]));
        assert_eq!(split("   "), Ok(vec![]));
    }

    #[test]
    fn test_multiple_spaces() {
        assert_eq!(
            split("foo    bar"),
            Ok(vec!["foo", "bar"].iter().map(|s| s.to_string()).collect())
        );
    }
}
