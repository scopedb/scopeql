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

use std::collections::BTreeMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexerResult {
    /// Complete command line arguments.
    Complete(Vec<String>),
    /// Incomplete command line arguments.
    Incomplete,
    /// Unknown escape sequence.
    UnknownEscape(char),
}

enum State {
    /// Within a delimiter.
    Delimiter,
    /// After backslash, but before starting word.
    Backslash,
    /// After backslash and a newline.
    BackslashNewline,
    /// Within an unquoted word.
    Unquoted,
    /// After backslash in an unquoted word.
    UnquotedBackslash,
    /// Within a quoted word.
    Quoted(char),
    /// After backslash inside a quoted word.
    QuotedBackslash(char),
}

pub fn lex(s: &str) -> LexerResult {
    let mut words = vec![];
    let mut word = String::new();
    let mut chars = s.chars();
    let mut state = State::Delimiter;

    loop {
        let ch = chars.next();
        state = match state {
            State::Delimiter => match ch {
                None => return LexerResult::Complete(words),
                Some('\\') => State::Backslash,
                Some(ch @ '\'' | ch @ '\"') => State::Quoted(ch),
                Some(ch) if ch.is_whitespace() => State::Delimiter,
                Some(ch) => {
                    word.push(ch);
                    State::Unquoted
                }
            },
            State::BackslashNewline => match ch {
                None => return LexerResult::Incomplete,
                Some('\\') => State::Backslash,
                Some(ch @ '\'' | ch @ '\"') => State::Quoted(ch),
                Some(ch) if ch.is_whitespace() => State::Delimiter,
                Some(ch) => {
                    word.push(ch);
                    State::Unquoted
                }
            },
            State::Backslash => match ch {
                None => return LexerResult::Incomplete,
                Some('\n') => State::BackslashNewline,
                Some(c) => {
                    word.push(c);
                    State::Unquoted
                }
            },
            State::Unquoted => match ch {
                None => {
                    words.push(std::mem::take(&mut word));
                    return LexerResult::Complete(words);
                }
                Some(ch @ '\'' | ch @ '\"') => State::Quoted(ch),
                Some('\\') => State::UnquotedBackslash,
                Some(ch) if ch.is_whitespace() => {
                    words.push(std::mem::take(&mut word));
                    State::Delimiter
                }
                Some(ch) => {
                    word.push(ch);
                    State::Unquoted
                }
            },
            State::UnquotedBackslash => match ch {
                None => return LexerResult::Incomplete,
                Some('\n') => State::Unquoted,
                Some(c) => {
                    word.push(c);
                    State::Unquoted
                }
            },
            State::Quoted(quote) => match ch {
                None => return LexerResult::Incomplete,
                Some('\\') => State::QuotedBackslash(quote),
                Some(ch) if ch == quote => State::Unquoted,
                Some(ch) => {
                    word.push(ch);
                    State::Quoted(quote)
                }
            },
            State::QuotedBackslash(quote) => match ch {
                None => return LexerResult::Incomplete,
                Some('\n') => State::Quoted(quote),
                Some(ch) => {
                    static ESCAPES: LazyLock<BTreeMap<char, char>> = LazyLock::new(|| {
                        BTreeMap::from_iter([
                            ('n', '\n'),
                            ('t', '\t'),
                            ('r', '\r'),
                            ('0', '\0'),
                            ('\'', '\''),
                            ('\"', '\"'),
                            ('\\', '\\'),
                        ])
                    });

                    if let Some(ch) = ESCAPES.get(&ch).cloned() {
                        word.push(ch);
                        State::Quoted(quote)
                    } else {
                        return LexerResult::UnknownEscape(ch);
                    }
                }
            },
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete(words: &[&str]) -> LexerResult {
        LexerResult::Complete(words.iter().map(|word| word.to_string()).collect())
    }

    #[test]
    fn test_simple_split() {
        assert_eq!(lex("foo bar baz"), complete(&["foo", "bar", "baz"]));
    }

    #[test]
    fn test_quoted_words() {
        assert_eq!(lex("foo 'bar baz'"), complete(&["foo", "bar baz"]));
        assert_eq!(lex(r#"foo "bar baz""#), complete(&["foo", "bar baz"]));
    }

    #[test]
    fn test_escape_outside_quotes() {
        assert_eq!(lex(r"foo\ bar"), complete(&["foo bar"]));
        assert_eq!(lex(r#"foo \"bar\""#), complete(&["foo", r#""bar""#]));
    }

    #[test]
    fn test_escape_in_double_quotes() {
        assert_eq!(lex(r#""foo\nbar""#), complete(&["foo\nbar"]));
        assert_eq!(lex(r#""foo\tbar""#), complete(&["foo\tbar"]));
        assert_eq!(lex(r#""foo\rbar""#), complete(&["foo\rbar"]));
        assert_eq!(lex(r#""foo\0bar""#), complete(&["foo\0bar"]));
        assert_eq!(lex(r#""foo\"bar""#), complete(&[r#"foo"bar"#]));
        assert_eq!(lex(r#""foo\\bar""#), complete(&[r#"foo\bar"#]));
    }

    #[test]
    fn test_escape_in_single_quotes() {
        assert_eq!(lex(r"'foo\\'"), complete(&[r"foo\"]));
        assert_eq!(lex(r"'foo\''"), complete(&[r"foo'"]));
    }

    #[test]
    fn test_unknown_escape_in_quotes() {
        assert_eq!(lex(r#""foo\xbar""#), LexerResult::UnknownEscape('x'));
        assert_eq!(lex(r#""foo\abar""#), LexerResult::UnknownEscape('a'));
        assert_eq!(lex(r"'foo\abar'"), LexerResult::UnknownEscape('a'));
    }

    #[test]
    fn test_escape_outside_quotes_accepts_any() {
        assert_eq!(lex(r"foo\xbar"), complete(&["fooxbar"]));
        assert_eq!(lex(r"foo\abar"), complete(&["fooabar"]));
    }

    #[test]
    fn test_mixed_quotes() {
        assert_eq!(
            lex(r#"foo 'bar "baz"' qux"#),
            complete(&["foo", r#"bar "baz""#, "qux"])
        );
    }

    #[test]
    fn test_unclosed_quotes() {
        assert_eq!(lex("foo 'bar"), LexerResult::Incomplete);
        assert_eq!(lex(r#"foo "bar"#), LexerResult::Incomplete);
    }

    #[test]
    fn test_trailing_backslash() {
        assert_eq!(lex(r"foo\"), LexerResult::Incomplete);
        assert_eq!(lex(r#""foo\"#), LexerResult::Incomplete);
        assert_eq!(lex(r"'foo\"), LexerResult::Incomplete);
    }

    #[test]
    fn test_delimiters() {
        assert_eq!(lex(""), LexerResult::Complete(vec![]));
        assert_eq!(lex("   "), LexerResult::Complete(vec![]));
        assert_eq!(lex("foo    bar"), complete(&["foo", "bar"]));
    }
}
