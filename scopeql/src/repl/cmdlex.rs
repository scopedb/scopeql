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

//! REPL command lexing with Rust-style escape sequences.
//!
//! This module provides REPL command lexing with:
//! - Single quotes: minimal escaping (`\'` and `\\` only)
//! - Double quotes: Rust-style escaping (`\n`, `\t`, `\r`, `\0`, `\"`, `\\`, `\'`)
//! - Unquoted: backslash escapes any character
//! - Whitespace as argument separator

use std::fmt;
use std::iter::Peekable;
use std::str::Chars;

/// Error type for REPL command lexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmdlexError {
    /// Unclosed single quote.
    UnclosedSingleQuote,
    /// Unclosed double quote.
    UnclosedDoubleQuote,
    /// Trailing backslash with no character to escape.
    TrailingBackslash,
    /// Unknown escape sequence.
    UnknownEscape(char),
}

impl fmt::Display for CmdlexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CmdlexError::UnclosedSingleQuote => write!(f, "unclosed single quote"),
            CmdlexError::UnclosedDoubleQuote => write!(f, "unclosed double quote"),
            CmdlexError::TrailingBackslash => write!(f, "trailing backslash"),
            CmdlexError::UnknownEscape(ch) => write!(f, "unknown escape sequence: \\{ch}"),
        }
    }
}

impl std::error::Error for CmdlexError {}

pub fn split(input: &str) -> Result<Vec<String>, CmdlexError> {
    let mut words = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '\'' => words.push(parse_single_quoted(&mut chars)?),
            '"' => words.push(parse_double_quoted(&mut chars)?),
            _ => {
                if ch.is_whitespace() {
                    chars.next();
                } else {
                    words.push(parse_unquoted(&mut chars)?);
                }
            }
        }
    }

    Ok(words)
}

fn parse_single_quoted(chars: &mut Peekable<Chars>) -> Result<String, CmdlexError> {
    chars.next(); // Skip opening single quote
    let mut result = String::new();

    loop {
        match chars.next() {
            None => return Err(CmdlexError::UnclosedSingleQuote),
            Some('\\') => match chars.next() {
                None => return Err(CmdlexError::TrailingBackslash),
                Some('\'') => result.push('\''),
                Some('\\') => result.push('\\'),
                Some(ch) => return Err(CmdlexError::UnknownEscape(ch)),
            },
            Some('\'') => break, // Found closing single quote
            Some(ch) => result.push(ch),
        }
    }

    Ok(result)
}

fn parse_double_quoted(chars: &mut Peekable<Chars>) -> Result<String, CmdlexError> {
    chars.next(); // Skip opening double quote
    let mut result = String::new();

    loop {
        match chars.next() {
            None => return Err(CmdlexError::UnclosedDoubleQuote),
            Some('\\') => {
                // Double quotes: Rust-style escaping
                match chars.next() {
                    None => return Err(CmdlexError::TrailingBackslash),
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('0') => result.push('\0'),
                    Some('"') => result.push('"'),
                    Some('\\') => result.push('\\'),
                    Some('\'') => result.push('\''),
                    Some(ch) => return Err(CmdlexError::UnknownEscape(ch)),
                }
            }
            Some('"') => break, // Found closing double quote
            Some(ch) => result.push(ch),
        }
    }

    Ok(result)
}

fn parse_unquoted(chars: &mut Peekable<Chars>) -> Result<String, CmdlexError> {
    let mut result = String::new();

    loop {
        match chars.peek() {
            None => break,
            Some(ch) if ch.is_whitespace() => break,
            Some('\\') => {
                chars.next();
                // Outside quotes: escape any character
                match chars.next() {
                    None => return Err(CmdlexError::TrailingBackslash),
                    Some(ch) => result.push(ch),
                }
            }
            Some(_) => result.push(chars.next().unwrap()),
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
            Ok(["foo", "bar", "baz"]
                .iter()
                .map(|s| s.to_string())
                .collect())
        );
    }

    #[test]
    fn test_single_quotes() {
        assert_eq!(
            split("foo 'bar baz'"),
            Ok(["foo", "bar baz"].iter().map(|s| s.to_string()).collect())
        );
    }

    #[test]
    fn test_double_quotes() {
        assert_eq!(
            split(r#"foo "bar baz""#),
            Ok(["foo", "bar baz"].iter().map(|s| s.to_string()).collect())
        );
    }

    #[test]
    fn test_escape_outside_quotes() {
        assert_eq!(
            split(r"foo\ bar"),
            Ok(["foo bar"].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r#"foo \"bar\""#),
            Ok(["foo", r#""bar""#].iter().map(|s| s.to_string()).collect())
        );
    }

    #[test]
    fn test_escape_in_double_quotes() {
        // Rust-style escapes work in double quotes
        assert_eq!(
            split(r#""foo\nbar""#),
            Ok(["foo\nbar"].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r#""foo\tbar""#),
            Ok(["foo\tbar"].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r#""foo\rbar""#),
            Ok(["foo\rbar"].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r#""foo\0bar""#),
            Ok(["foo\0bar"].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r#""foo\"bar""#),
            Ok([r#"foo"bar"#].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r#""foo\\bar""#),
            Ok([r#"foo\bar"#].iter().map(|s| s.to_string()).collect())
        );
    }

    #[test]
    fn test_escape_in_single_quotes() {
        assert_eq!(
            split(r"'foo\\'"),
            Ok([r"foo\"].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r"'foo\''"),
            Ok([r"foo'"].iter().map(|s| s.to_string()).collect())
        );
    }

    #[test]
    fn test_unknown_escape_in_double_quotes() {
        assert_eq!(split(r#""foo\xbar""#), Err(CmdlexError::UnknownEscape('x')));
        assert_eq!(split(r#""foo\abar""#), Err(CmdlexError::UnknownEscape('a')));
    }

    #[test]
    fn test_unknown_escape_in_single_quotes() {
        assert_eq!(split(r"'foo\nbar'"), Err(CmdlexError::UnknownEscape('n')));
        assert_eq!(split(r"'foo\abar'"), Err(CmdlexError::UnknownEscape('a')));
    }

    #[test]
    fn test_escape_outside_quotes_accepts_any() {
        // Outside quotes, backslash escapes any character
        assert_eq!(
            split(r"foo\xbar"),
            Ok(["fooxbar"].iter().map(|s| s.to_string()).collect())
        );
        assert_eq!(
            split(r"foo\abar"),
            Ok(["fooabar"].iter().map(|s| s.to_string()).collect())
        );
    }

    #[test]
    fn test_mixed_quotes() {
        assert_eq!(
            split(r#"foo 'bar "baz"' qux"#),
            Ok(["foo", r#"bar "baz""#, "qux"]
                .iter()
                .map(|s| s.to_string())
                .collect())
        );
    }

    #[test]
    fn test_unclosed_quotes() {
        assert_eq!(split("foo 'bar"), Err(CmdlexError::UnclosedSingleQuote));
        assert_eq!(split(r#"foo "bar"#), Err(CmdlexError::UnclosedDoubleQuote));
    }

    #[test]
    fn test_trailing_backslash() {
        assert_eq!(split(r"foo\"), Err(CmdlexError::TrailingBackslash));
        assert_eq!(split(r#""foo\"#), Err(CmdlexError::TrailingBackslash));
        assert_eq!(split(r"'foo\"), Err(CmdlexError::TrailingBackslash));
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
            Ok(["foo", "bar"].iter().map(|s| s.to_string()).collect())
        );
    }
}
