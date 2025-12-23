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

use std::iter::Peekable;
use std::ops::Range;
use std::str::Chars;

use crate::TokenKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexError {
    UnexpectedChar(char),
    UnterminatedString,
    UnterminatedBlockComment,
    InvalidHexInteger,
}

pub struct Lexer<'a> {
    original: &'a str,
    chars: Peekable<Chars<'a>>,
    start_index: usize,
    current_index: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            original: source,
            chars: source.chars().peekable(),
            start_index: 0,
            current_index: 0,
        }
    }

    pub fn span(&self) -> Range<usize> {
        self.start_index..self.current_index
    }

    pub fn slice(&self) -> &'a str {
        &self.original[self.start_index..self.current_index]
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.current_index += c.len_utf8();
        Some(c)
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn matches(&mut self, expected: char) -> bool {
        if let Some(c) = self.peek() {
            if c == expected {
                self.advance();
                return true;
            }
        }
        false
    }

    fn read_while<F>(&mut self, predicate: F)
    where
        F: Fn(char) -> bool,
    {
        while let Some(c) = self.peek() {
            if predicate(c) {
                self.advance();
            } else {
                break;
            }
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<TokenKind, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Skip whitespace
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }

        self.start_index = self.current_index;
        let c = self.advance()?;

        match c {
            // Symbols
            '=' => {
                if self.matches('>') {
                    Some(Ok(TokenKind::Arrow))
                } else {
                    Some(Ok(TokenKind::Eq))
                }
            }
            '<' => {
                if self.matches('=') {
                    Some(Ok(TokenKind::Lte))
                } else if self.matches('>') {
                    Some(Ok(TokenKind::NotEq))
                } else {
                    Some(Ok(TokenKind::Lt))
                }
            }
            '>' => {
                if self.matches('=') {
                    Some(Ok(TokenKind::Gte))
                } else {
                    Some(Ok(TokenKind::Gt))
                }
            }
            '!' => {
                if self.matches('=') {
                    Some(Ok(TokenKind::NotEq))
                } else {
                    Some(Err(LexError::UnexpectedChar('!')))
                }
            }
            '+' => Some(Ok(TokenKind::Plus)),
            '-' => {
                if self.matches('-') {
                    // Single line comment
                    self.read_while(|c| c != '\n' && c != '\r');
                    self.next()
                } else if self.matches('>') {
                    Some(Ok(TokenKind::Arrow))
                } else {
                    Some(Ok(TokenKind::Minus))
                }
            }
            '*' => Some(Ok(TokenKind::Multiply)),
            '/' => {
                if self.matches('*') {
                    // Block comment
                    loop {
                        match self.advance() {
                            Some('*') => {
                                if self.matches('/') {
                                    break;
                                }
                            }
                            Some(_) => continue,
                            None => return Some(Err(LexError::UnterminatedBlockComment)),
                        }
                    }
                    self.next()
                } else {
                    Some(Ok(TokenKind::Divide))
                }
            }
            '%' => Some(Ok(TokenKind::Modulo)),
            '|' => {
                if self.matches('|') {
                    Some(Ok(TokenKind::Concat))
                } else {
                    Some(Err(LexError::UnexpectedChar('|')))
                }
            }
            '(' => Some(Ok(TokenKind::LParen)),
            ')' => Some(Ok(TokenKind::RParen)),
            '[' => Some(Ok(TokenKind::LBracket)),
            ']' => Some(Ok(TokenKind::RBracket)),
            '{' => Some(Ok(TokenKind::LBrace)),
            '}' => Some(Ok(TokenKind::RBrace)),
            ',' => Some(Ok(TokenKind::Comma)),
            '.' => Some(Ok(TokenKind::Dot)),
            ':' => {
                if self.matches(':') {
                    Some(Ok(TokenKind::DoubleColon))
                } else {
                    Some(Ok(TokenKind::Colon))
                }
            }
            ';' => Some(Ok(TokenKind::SemiColon)),
            '$' => Some(Ok(TokenKind::Dollar)),

            // Strings
            '\'' => loop {
                match self.advance() {
                    Some('\'') => {
                        if self.matches('\'') {
                            continue;
                        }
                        return Some(Ok(TokenKind::LiteralString));
                    }
                    Some('\\') => {
                        self.advance();
                    }
                    Some(_) => continue,
                    None => return Some(Err(LexError::UnterminatedString)),
                }
            },
            '"' => loop {
                match self.advance() {
                    Some('"') => {
                        if self.matches('"') {
                            continue;
                        }
                        return Some(Ok(TokenKind::LiteralString));
                    }
                    Some('\\') => {
                        self.advance();
                    }
                    Some(_) => continue,
                    None => return Some(Err(LexError::UnterminatedString)),
                }
            },
            '`' => loop {
                match self.advance() {
                    Some('`') => {
                        if self.matches('`') {
                            continue;
                        }
                        return Some(Ok(TokenKind::LiteralString));
                    }
                    Some('\\') => {
                        self.advance();
                    }
                    Some(_) => continue,
                    None => return Some(Err(LexError::UnterminatedString)),
                }
            },

            // Numbers
            '0'..='9' => {
                // Hex Integer: 0x...
                if c == '0' && (self.matches('x') || self.matches('X')) {
                    let mut has_digit = false;
                    while let Some(peek) = self.peek() {
                        if peek.is_ascii_hexdigit() {
                            self.advance();
                            has_digit = true;
                        } else {
                            break;
                        }
                    }
                    if !has_digit {
                        return Some(Err(LexError::InvalidHexInteger));
                    }
                    return Some(Ok(TokenKind::LiteralHexInteger));
                }

                self.read_while(|c| c.is_ascii_digit() || c == '_');

                let mut checkpoint = self.chars.clone();
                let mut is_float = false;

                if let Some('.') = checkpoint.next() {
                    if let Some(next) = checkpoint.peek() {
                        if next.is_ascii_digit() {
                            is_float = true;
                            self.advance();
                            self.read_while(|c| c.is_ascii_digit());
                        }
                    }
                }

                let mut checkpoint_exp = self.chars.clone();
                if let Some(e) = checkpoint_exp.next() {
                    if e == 'e' || e == 'E' {
                        let mut has_sign = false;
                        if let Some(sign) = checkpoint_exp.peek() {
                            if *sign == '+' || *sign == '-' {
                                checkpoint_exp.next();
                                has_sign = true;
                            }
                        }

                        if let Some(next) = checkpoint_exp.peek() {
                            if next.is_ascii_digit() {
                                is_float = true;
                                self.advance();
                                if has_sign {
                                    self.advance();
                                }
                                self.read_while(|c| c.is_ascii_digit());
                            }
                        }
                    }
                }

                if is_float {
                    Some(Ok(TokenKind::LiteralFloat))
                } else {
                    Some(Ok(TokenKind::LiteralInteger))
                }
            }

            // Identifiers / Keywords / HexString
            'a'..='z' | 'A'..='Z' | '_' => {
                if (c == 'x' || c == 'X') && self.peek() == Some('"') {
                    self.advance();
                    self.read_while(|c| c.is_ascii_hexdigit());
                    if self.matches('"') {
                        return Some(Ok(TokenKind::LiteralHexBinaryString));
                    } else {
                        return Some(Err(LexError::UnterminatedString));
                    }
                }

                self.read_while(|c| c.is_ascii_alphanumeric() || c == '_');
                let text = self.slice();

                if let Some(k) = match_keyword(text) {
                    Some(Ok(k))
                } else {
                    Some(Ok(TokenKind::Ident))
                }
            }

            _ => Some(Err(LexError::UnexpectedChar(c))),
        }
    }
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
