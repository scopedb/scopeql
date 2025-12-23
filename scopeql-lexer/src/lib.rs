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

use std::str::Chars;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// A special token representing the end of input.
    EOI,

    // skipped
    /// Whitespace characters.
    Whitespace,
    /// Single-line or multi-line comments.
    Comment,

    /// Unquoted identifiers.
    ///
    /// The identifier will be normalized to lower-case.
    Ident,
    /// Quoted strings.
    ///
    /// # Possible Quotations
    ///
    /// * Single quotes (`'identifier'`) as string literals
    /// * Double quotes (`"identifier"`) as string literals
    /// * Backticks (`` `identifier` ``) as identifiers
    LiteralString,
    /// Single quoted strings with 'x' prefix.
    LiteralHexBinaryString,
    /// Unsigned integer literals.
    LiteralInteger,
    /// Hexadecimal integer literals with '0x' prefix.
    LiteralHexInteger,
    /// Floating point literals.
    LiteralFloat,

    // Symbols
    /// Equal sign (`=`).
    Eq,
    /// Not equal sign (`!=` or `<>`).
    NotEq,
    /// Less than sign (`<`).
    Lt,
    /// Greater than sign (`>`).
    Gt,
    /// Less than or equal sign (`<=`).
    Lte,
    /// Greater than or equal sign (`>=`).
    Gte,
    /// Plus sign (`+`).
    Plus,
    /// Minus sign (`-`).
    Minus,
    /// Asterisk sign (`*`).
    Multiply,
    /// Forward slash sign (`/`).
    Divide,
    /// Percent sign (`%`).
    Modulo,
    /// Concatenation operator (`||`).
    Concat,
    /// Left parenthesis (`(`).
    LParen,
    /// Right parenthesis (`)`).
    RParen,
    /// Left bracket (`[`).
    LBracket,
    /// Right bracket (`]`).
    RBracket,
    /// Left brace (`{`).
    LBrace,
    /// Right brace (`}`).
    RBrace,
    /// Comma (`,`).
    Comma,
    /// Dot (`.`).
    Dot,
    /// Colon (`:`).
    Colon,
    /// Double colon (`::`).
    DoubleColon,
    /// Semi-colon (`;`).
    SemiColon,
    /// Dollar sign (`$`).
    Dollar,
    /// Arrow (`=>`).
    Arrow,

    // Case-insensitive keywords
    ADD,
    AGGREGATE,
    ALL,
    ALTER,
    ANALYZE,
    AND,
    ANY,
    ARRAY,
    AS,
    ASC,
    BEGIN,
    BETWEEN,
    BOOLEAN,
    BY,
    CASE,
    CAST,
    CLUSTER,
    COLUMN,
    COMMENT,
    CREATE,
    DATABASES,
    DATABASE,
    DELETE,
    DESC,
    DESCRIBE,
    DISTINCT,
    DROP,
    ELSE,
    END,
    EQUALITY,
    EXCLUDE,
    EXEC,
    EXISTS,
    EXPLAIN,
    FALSE,
    FIRST,
    FLOAT,
    FROM,
    FULL,
    GROUP,
    IF,
    IN,
    INDEX,
    INNER,
    INSERT,
    INT,
    INTERVAL,
    INTO,
    IS,
    JOB,
    JOBS,
    JOIN,
    KEY,
    LAST,
    LEFT,
    LIMIT,
    MATERIALIZED,
    NODEGROUP,
    NOT,
    NULL,
    NULLS,
    OBJECT,
    OFFSET,
    ON,
    OPTIMIZE,
    OR,
    ORDER,
    OUTER,
    PERCENT,
    PLAN,
    RANGE,
    RENAME,
    REPLACE,
    RESUME,
    RIGHT,
    SAMPLE,
    SCHEDULE,
    SCHEMAS,
    SCHEMA,
    SEARCH,
    SELECT,
    SET,
    SHOW,
    STATEMENTS,
    STRING,
    SUSPEND,
    TABLE,
    TABLES,
    THEN,
    TIMESTAMP,
    TO,
    TRUE,
    UINT,
    UNION,
    UPDATE,
    VACUUM,
    VALUES,
    VIEW,
    VIEWS,
    WHEN,
    WHERE,
    WINDOW,
    WITH,
    WITHIN,
    XOR,
}

#[derive(Debug, Clone)]
pub struct Error(String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Error {}

pub struct Lexer<'a> {
    original: &'a str,
    chars: Chars<'a>,

    // indexes in bytes
    start: usize,
    current: usize,

    lookahead: Option<char>,
}

enum State {
    Normal,
    QuotedString(char),
    CommentLine,
    CommentBlock,
    HexString(char),
    HexInteger,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer from the source string.
    pub fn new(source: &'a str) -> Self {
        Self {
            original: source,
            chars: source.chars(),
            start: 0,
            current: 0,
            lookahead: None,
        }
    }

    /// Get the left inclusive, right exclusive span of the current token.
    pub fn span(&self) -> (usize, usize) {
        (self.start, self.current)
    }

    /// Get the string slice of the current token.
    pub fn slice(&self) -> &'a str {
        &self.original[self.start..self.current]
    }
}

impl Lexer<'_> {
    fn consume(&mut self) -> Option<char> {
        if let Some(c) = self.lookahead {
            self.current += c.len_utf8();
            self.lookahead = None;
            Some(c)
        } else {
            let c = self.chars.next()?;
            self.current += c.len_utf8();
            Some(c)
        }
    }

    fn lookahead(&mut self) -> Option<char> {
        if let Some(c) = self.lookahead {
            Some(c)
        } else {
            let c = self.chars.next()?;
            self.lookahead = Some(c);
            Some(c)
        }
    }

    fn consume_digit_or_underscore(&mut self) -> usize {
        let mut count = 0;

        while let Some(c) = self.lookahead() {
            match c {
                '0'..='9' => {
                    count += 1;
                    self.consume();
                }
                '_' => {
                    self.consume();
                }
                _ => break,
            }
        }

        count
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<TokenKind, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut state = State::Normal;
        loop {
            match state {
                State::Normal => {
                    // move cursor to the start of the next token
                    self.start = self.current;
                    let c = self.consume()?;

                    if c.is_whitespace() {
                        while let Some(c) = self.lookahead() {
                            if c.is_whitespace() {
                                self.consume();
                            } else {
                                break;
                            }
                        }
                        return Some(Ok(TokenKind::Whitespace));
                    }

                    match c {
                        // Symbols
                        '=' => {
                            let lookahead = self.lookahead();
                            return if matches!(lookahead, Some('>')) {
                                self.consume();
                                Some(Ok(TokenKind::Arrow))
                            } else {
                                Some(Ok(TokenKind::Eq))
                            };
                        }
                        '<' => {
                            let lookahead = self.lookahead();
                            return match lookahead {
                                Some('=') => {
                                    self.consume();
                                    Some(Ok(TokenKind::Lte))
                                }
                                Some('>') => {
                                    self.consume();
                                    Some(Ok(TokenKind::NotEq))
                                }
                                _ => Some(Ok(TokenKind::Lt)),
                            };
                        }
                        '>' => {
                            let lookahead = self.lookahead();
                            return match lookahead {
                                Some('=') => {
                                    self.consume();
                                    Some(Ok(TokenKind::Gte))
                                }
                                _ => Some(Ok(TokenKind::Gt)),
                            };
                        }
                        '!' => {
                            let lookahead = self.lookahead();
                            return if matches!(lookahead, Some('=')) {
                                self.consume();
                                Some(Ok(TokenKind::NotEq))
                            } else {
                                Some(Err(incomplete_operator("!")))
                            };
                        }
                        '+' => return Some(Ok(TokenKind::Plus)),
                        '-' => {
                            let lookahead = self.lookahead();
                            match lookahead {
                                Some('-') => {
                                    self.consume();
                                    state = State::CommentLine;
                                    continue;
                                }
                                _ => return Some(Ok(TokenKind::Minus)),
                            }
                        }
                        '*' => return Some(Ok(TokenKind::Multiply)),
                        '/' => {
                            let lookahead = self.lookahead();
                            match lookahead {
                                Some('*') => {
                                    self.consume();
                                    state = State::CommentBlock;
                                    continue;
                                }
                                _ => return Some(Ok(TokenKind::Divide)),
                            }
                        }
                        '%' => return Some(Ok(TokenKind::Modulo)),
                        '|' => {
                            let lookahead = self.lookahead();
                            return if matches!(lookahead, Some('|')) {
                                self.consume();
                                Some(Ok(TokenKind::Concat))
                            } else {
                                Some(Err(incomplete_operator("|")))
                            };
                        }
                        '(' => return Some(Ok(TokenKind::LParen)),
                        ')' => return Some(Ok(TokenKind::RParen)),
                        '[' => return Some(Ok(TokenKind::LBracket)),
                        ']' => return Some(Ok(TokenKind::RBracket)),
                        '{' => return Some(Ok(TokenKind::LBrace)),
                        '}' => return Some(Ok(TokenKind::RBrace)),
                        ',' => return Some(Ok(TokenKind::Comma)),
                        '.' => return Some(Ok(TokenKind::Dot)),
                        ':' => {
                            let lookahead = self.lookahead();
                            return if matches!(lookahead, Some(':')) {
                                self.consume();
                                Some(Ok(TokenKind::DoubleColon))
                            } else {
                                Some(Ok(TokenKind::Colon))
                            };
                        }
                        ';' => return Some(Ok(TokenKind::SemiColon)),
                        '$' => return Some(Ok(TokenKind::Dollar)),
                        '\'' | '"' | '`' => {
                            state = State::QuotedString(c);
                            continue;
                        }

                        // Identifiers and Keywords
                        //
                        // Unquoted identifiers can have only ascii letters. If it contains other
                        // Unicode characters, the auto normalization to lower-case may subtly ruin
                        // the intent of the identifier, so we treat them as invalid tokens.
                        '_' | 'a'..='z' | 'A'..='Z' => {
                            if matches!(c, 'x' | 'X') {
                                match self.lookahead() {
                                    Some('\'') | Some('"') => {
                                        // consume the quote
                                        let quote = self.consume().unwrap();
                                        state = State::HexString(quote);
                                        continue;
                                    }
                                    // fallthrough as identifier
                                    _ => {}
                                }
                            }

                            while let Some(c) = self.lookahead() {
                                if matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_') {
                                    self.consume();
                                } else {
                                    break;
                                }
                            }

                            let slice = self.slice();
                            return if let Some(keyword) = match_keyword(slice) {
                                Some(Ok(keyword))
                            } else {
                                Some(Ok(TokenKind::Ident))
                            };
                        }

                        // Numbers
                        '0'..='9' => {
                            if c == '0' {
                                if let Some(lookahead) = self.lookahead() {
                                    if matches!(lookahead, 'x' | 'X') {
                                        self.consume(); // consume 'x' or 'X'
                                        state = State::HexInteger;
                                        continue;
                                    }
                                }
                            }

                            // before a dot or exponent, it is an integer
                            self.consume_digit_or_underscore();

                            // try to parse a dot
                            let mut has_dot = false;
                            if let Some('.') = self.lookahead() {
                                has_dot = true;
                                self.consume(); // consume the dot
                                self.consume_digit_or_underscore();
                            }

                            // try to parse an exponent
                            let mut has_exponent = false;
                            if let Some(c) = self.lookahead() {
                                if matches!(c, 'e' | 'E') {
                                    has_exponent = true;
                                    self.consume(); // consume 'e' or 'E'

                                    // optional sign
                                    if let Some(c) = self.lookahead() {
                                        if matches!(c, '+' | '-') {
                                            self.consume();
                                        }
                                    }

                                    // must have at least one digit as exponent
                                    if self.consume_digit_or_underscore() == 0 {
                                        let slice = self.slice();
                                        return Some(Err(incomplete_float(slice)));
                                    }
                                }
                            }

                            return if has_dot || has_exponent {
                                Some(Ok(TokenKind::LiteralFloat))
                            } else {
                                Some(Ok(TokenKind::LiteralInteger))
                            };
                        }

                        // unknown characters
                        c => return Some(Err(unexpected_character(c))),
                    }
                }
                State::QuotedString(quote) => {
                    // inside a quoted string
                    while let Some(c) = self.consume() {
                        match c {
                            '\\' => {
                                // escape character, consume next character without checking
                                self.consume();
                            }
                            c if c == quote => {
                                // check for escaped quote
                                if let Some(next_c) = self.lookahead() {
                                    if next_c == quote {
                                        // escaped quote, consume it and continue
                                        self.consume();
                                        continue;
                                    }
                                }
                                // end of quoted string
                                return Some(Ok(TokenKind::LiteralString));
                            }
                            _ => {}
                        }
                    }
                    // reached end of input before closing quote
                    return Some(Err(unclosed_quoted_string(quote)));
                }
                State::HexString(quote) => {
                    // inside a hex binary string: x'0123ABCDEF'
                    while let Some(c) = self.lookahead() {
                        if c == quote {
                            // end of hex string
                            self.consume();
                            return Some(Ok(TokenKind::LiteralHexBinaryString));
                        } else if c.is_ascii_hexdigit() {
                            // valid hex digit, continue
                            self.consume();
                            continue;
                        } else {
                            // invalid character in hex string
                            return Some(Err(invalid_hex_string(c)));
                        }
                    }
                    // reached end of input
                    return Some(Err(unclosed_hex_string()));
                }
                State::HexInteger => {
                    // inside a hexadecimal integer: 0x123ABCDEF
                    let mut has_digits = false;
                    while let Some(c) = self.lookahead() {
                        if c.is_ascii_hexdigit() {
                            // valid hex digit, continue
                            has_digits = true;
                            self.consume();
                        } else {
                            break;
                        }
                    }
                    return if has_digits {
                        Some(Ok(TokenKind::LiteralHexInteger))
                    } else {
                        let slice = self.slice();
                        Some(Err(incomplete_hex_integer(slice)))
                    };
                }
                State::CommentLine => {
                    // inside the '-- ...' comment
                    while let Some(c) = self.consume() {
                        match c {
                            '\n' => break,
                            '\r' => {
                                // similar to str::lines(): any carriage return (\r) not immediately
                                // followed by a line feed (\n) does not split a line.
                                if let Some('\n') = self.lookahead() {
                                    self.consume();
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    // either reached end of line or end of input
                    return Some(Ok(TokenKind::Comment));
                }
                State::CommentBlock => {
                    // inside the '/* ... */' comment block
                    while let Some(c) = self.consume() {
                        if c == '*' {
                            if let Some('/') = self.lookahead() {
                                self.consume();
                                return Some(Ok(TokenKind::Comment));
                            }
                        }
                    }
                    // reached end of input before closing */
                    return Some(Err(unclosed_comment_block()));
                }
            }
        }
    }
}

fn unexpected_character(c: char) -> Error {
    Error(format!("unexpected character '{c}'"))
}

fn invalid_hex_string(c: char) -> Error {
    Error(format!("invalid character '{c}' in hex string"))
}

fn unclosed_hex_string() -> Error {
    Error("unclosed hex string: expected x'...'".to_string())
}

fn unclosed_quoted_string(quote: char) -> Error {
    Error(format!("unclosed quoted string of quote '{quote}'"))
}

fn unclosed_comment_block() -> Error {
    Error("unclosed comment block: expected '*/'".to_string())
}

fn incomplete_float(i: &str) -> Error {
    Error(format!("incomplete float: {i}"))
}

fn incomplete_hex_integer(i: &str) -> Error {
    Error(format!("incomplete hex integer: {i}"))
}

fn incomplete_operator(op: &str) -> Error {
    Error(format!("incomplete operator: {op}"))
}

fn match_keyword(text: &str) -> Option<TokenKind> {
    if !text.is_ascii() {
        return None;
    }

    match text.to_ascii_uppercase().as_str() {
        "ADD" => Some(TokenKind::ADD),
        "AGGREGATE" => Some(TokenKind::AGGREGATE),
        "ALL" => Some(TokenKind::ALL),
        "ALTER" => Some(TokenKind::ALTER),
        "ANALYZE" => Some(TokenKind::ANALYZE),
        "AND" => Some(TokenKind::AND),
        "ANY" => Some(TokenKind::ANY),
        "ARRAY" => Some(TokenKind::ARRAY),
        "AS" => Some(TokenKind::AS),
        "ASC" => Some(TokenKind::ASC),
        "BEGIN" => Some(TokenKind::BEGIN),
        "BETWEEN" => Some(TokenKind::BETWEEN),
        "BOOLEAN" => Some(TokenKind::BOOLEAN),
        "BY" => Some(TokenKind::BY),
        "CASE" => Some(TokenKind::CASE),
        "CAST" => Some(TokenKind::CAST),
        "CLUSTER" => Some(TokenKind::CLUSTER),
        "COLUMN" => Some(TokenKind::COLUMN),
        "COMMENT" => Some(TokenKind::COMMENT),
        "CREATE" => Some(TokenKind::CREATE),
        "DATABASES" => Some(TokenKind::DATABASES),
        "DATABASE" => Some(TokenKind::DATABASE),
        "DELETE" => Some(TokenKind::DELETE),
        "DESC" => Some(TokenKind::DESC),
        "DESCRIBE" => Some(TokenKind::DESCRIBE),
        "DISTINCT" => Some(TokenKind::DISTINCT),
        "DROP" => Some(TokenKind::DROP),
        "ELSE" => Some(TokenKind::ELSE),
        "END" => Some(TokenKind::END),
        "EQUALITY" => Some(TokenKind::EQUALITY),
        "EXCLUDE" => Some(TokenKind::EXCLUDE),
        "EXEC" => Some(TokenKind::EXEC),
        "EXISTS" => Some(TokenKind::EXISTS),
        "EXPLAIN" => Some(TokenKind::EXPLAIN),
        "FALSE" => Some(TokenKind::FALSE),
        "FIRST" => Some(TokenKind::FIRST),
        "FLOAT" => Some(TokenKind::FLOAT),
        "FROM" => Some(TokenKind::FROM),
        "FULL" => Some(TokenKind::FULL),
        "GROUP" => Some(TokenKind::GROUP),
        "IF" => Some(TokenKind::IF),
        "IN" => Some(TokenKind::IN),
        "INDEX" => Some(TokenKind::INDEX),
        "INNER" => Some(TokenKind::INNER),
        "INSERT" => Some(TokenKind::INSERT),
        "INT" => Some(TokenKind::INT),
        "INTERVAL" => Some(TokenKind::INTERVAL),
        "INTO" => Some(TokenKind::INTO),
        "IS" => Some(TokenKind::IS),
        "JOB" => Some(TokenKind::JOB),
        "JOBS" => Some(TokenKind::JOBS),
        "JOIN" => Some(TokenKind::JOIN),
        "KEY" => Some(TokenKind::KEY),
        "LAST" => Some(TokenKind::LAST),
        "LEFT" => Some(TokenKind::LEFT),
        "LIMIT" => Some(TokenKind::LIMIT),
        "MATERIALIZED" => Some(TokenKind::MATERIALIZED),
        "NODEGROUP" => Some(TokenKind::NODEGROUP),
        "NOT" => Some(TokenKind::NOT),
        "NULL" => Some(TokenKind::NULL),
        "NULLS" => Some(TokenKind::NULLS),
        "OBJECT" => Some(TokenKind::OBJECT),
        "OFFSET" => Some(TokenKind::OFFSET),
        "ON" => Some(TokenKind::ON),
        "OPTIMIZE" => Some(TokenKind::OPTIMIZE),
        "OR" => Some(TokenKind::OR),
        "ORDER" => Some(TokenKind::ORDER),
        "OUTER" => Some(TokenKind::OUTER),
        "PERCENT" => Some(TokenKind::PERCENT),
        "PLAN" => Some(TokenKind::PLAN),
        "RANGE" => Some(TokenKind::RANGE),
        "RENAME" => Some(TokenKind::RENAME),
        "REPLACE" => Some(TokenKind::REPLACE),
        "RESUME" => Some(TokenKind::RESUME),
        "RIGHT" => Some(TokenKind::RIGHT),
        "SAMPLE" => Some(TokenKind::SAMPLE),
        "SCHEDULE" => Some(TokenKind::SCHEDULE),
        "SCHEMAS" => Some(TokenKind::SCHEMAS),
        "SCHEMA" => Some(TokenKind::SCHEMA),
        "SEARCH" => Some(TokenKind::SEARCH),
        "SELECT" => Some(TokenKind::SELECT),
        "SET" => Some(TokenKind::SET),
        "SHOW" => Some(TokenKind::SHOW),
        "STATEMENTS" => Some(TokenKind::STATEMENTS),
        "STRING" => Some(TokenKind::STRING),
        "SUSPEND" => Some(TokenKind::SUSPEND),
        "TABLE" => Some(TokenKind::TABLE),
        "TABLES" => Some(TokenKind::TABLES),
        "THEN" => Some(TokenKind::THEN),
        "TIMESTAMP" => Some(TokenKind::TIMESTAMP),
        "TO" => Some(TokenKind::TO),
        "TRUE" => Some(TokenKind::TRUE),
        "UINT" => Some(TokenKind::UINT),
        "UNION" => Some(TokenKind::UNION),
        "UPDATE" => Some(TokenKind::UPDATE),
        "VACUUM" => Some(TokenKind::VACUUM),
        "VALUES" => Some(TokenKind::VALUES),
        "VIEW" => Some(TokenKind::VIEW),
        "VIEWS" => Some(TokenKind::VIEWS),
        "WHEN" => Some(TokenKind::WHEN),
        "WHERE" => Some(TokenKind::WHERE),
        "WINDOW" => Some(TokenKind::WINDOW),
        "WITH" => Some(TokenKind::WITH),
        "WITHIN" => Some(TokenKind::WITHIN),
        "XOR" => Some(TokenKind::XOR),
        _ => None,
    }
}
