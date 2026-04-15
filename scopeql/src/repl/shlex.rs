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

//! Simple shell-like lexical splitting.
//!
//! This module provides basic shell-style word splitting with support for:
//! - Single and double quotes
//! - Backslash escaping
//! - Whitespace as word separator

use std::fmt;

/// Error type for shell lexical splitting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShlexError {
    /// Unclosed single quote.
    UnclosedSingleQuote,
    /// Unclosed double quote.
    UnclosedDoubleQuote,
    /// Trailing backslash with no character to escape.
    TrailingBackslash,
}

impl fmt::Display for ShlexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShlexError::UnclosedSingleQuote => write!(f, "unclosed single quote"),
            ShlexError::UnclosedDoubleQuote => write!(f, "unclosed double quote"),
            ShlexError::TrailingBackslash => write!(f, "trailing backslash"),
        }
    }
}

impl std::error::Error for ShlexError {}

/// Split a string into words using shell-like rules.
///
/// Returns an error if the input has unclosed quotes or trailing backslash.
pub fn split(input: &str) -> Result<Vec<String>, ShlexError> {
    let mut words = Vec::new();
    let mut current_word = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(ch) = chars.next() {
        match ch {
            '\\' if !in_single_quote => {
                // Backslash escaping (not in single quotes)
                if let Some(next_ch) = chars.next() {
                    current_word.push(next_ch);
                } else {
                    // Trailing backslash
                    return Err(ShlexError::TrailingBackslash);
                }
            }
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            ch if ch.is_whitespace() && !in_single_quote && !in_double_quote => {
                // Whitespace outside quotes: word boundary
                if !current_word.is_empty() {
                    words.push(current_word.clone());
                    current_word.clear();
                }
            }
            ch => {
                // Regular character
                current_word.push(ch);
            }
        }
    }

    // Check for unclosed quotes
    if in_single_quote {
        return Err(ShlexError::UnclosedSingleQuote);
    }
    if in_double_quote {
        return Err(ShlexError::UnclosedDoubleQuote);
    }

    // Push the last word if any
    if !current_word.is_empty() {
        words.push(current_word);
    }

    Ok(words)
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
    fn test_escape() {
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
