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
