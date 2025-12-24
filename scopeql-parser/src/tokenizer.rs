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

use std::ops::Range;

use logos::Lexer;
use logos::Logos;

#[derive(Debug)]
pub struct Tokenizer<'source> {
    lexer: Lexer<'source, TokenKind>,
    eoi: bool,
}

impl<'source> Tokenizer<'source> {
    pub fn new(source: &'source str) -> Self {
        Self {
            lexer: TokenKind::lexer(source),
            eoi: false,
        }
    }

    pub fn slice(&self) -> &'source str {
        self.lexer.slice()
    }

    pub fn span(&self) -> Range<usize> {
        self.lexer.span()
    }
}

impl<'source> Iterator for Tokenizer<'source> {
    type Item = Result<TokenKind, ()>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.lexer.next() {
            Some(Err(())) => Some(Err(())),
            Some(Ok(kind)) => Some(Ok(kind)),
            None => {
                if self.eoi {
                    // already emitted EOI
                    None
                } else {
                    // emit EOI; the next call will return None
                    self.eoi = true;
                    Some(Ok(TokenKind::EOI))
                }
            }
        }
    }
}

#[derive(Logos, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// A special token representing the end of input.
    EOI,

    // Skipped tokens
    /// Whitespace characters.
    #[regex(r"[ \t\r\n\f]+")]
    Whitespace,

    /// Single-line or multi-line comments.
    #[regex(r"--[^\r\n\f]*")]
    #[regex(r"/\*([^\*]|(\*[^/]))*\*/")]
    Comment,

    /// Unquoted identifiers.
    ///
    /// The identifier will be normalized to lowercase, and thus only ASCII letters are allowed.
    /// Otherwise, the normalization may subtly change the intent of the identifier.
    #[regex(r#"[_a-zA-Z][_a-zA-Z0-9]*"#)]
    Ident,

    #[regex(r#"'([^'\\]|\\.|'')*'"#)]
    #[regex(r#""([^"\\]|\\.|"")*""#)]
    #[regex(r#"`([^`\\]|\\.|``)*`"#)]
    LiteralString,
    #[regex(r"[xX]'[a-fA-F0-9]*'")]
    LiteralHexBinaryString,

    #[regex(r"[0-9]+(_|[0-9])*")]
    LiteralInteger,
    /// Hexadecimal integer literals with '0x' prefix.
    #[regex(r"0[xX][a-fA-F0-9]+")]
    LiteralHexInteger,
    /// Floating point literals.
    #[regex(r"[0-9]+[eE][+-]?[0-9]+")]
    #[regex(r"[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?")]
    LiteralFloat,

    // Symbols
    #[token("=")]
    Eq,
    #[token("<>")]
    #[token("!=")]
    NotEq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    Lte,
    #[token(">=")]
    Gte,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Multiply,
    #[token("/")]
    Divide,
    #[token("%")]
    Modulo,
    #[token("||")]
    Concat,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token(":")]
    Colon,
    #[token("::")]
    DoubleColon,
    #[token(";")]
    SemiColon,
    #[token("$")]
    Dollar,
    #[token("=>")]
    Arrow,

    // Case-insensitive keywords
    #[token("ADD", ignore(case))]
    ADD,
    #[token("AGGREGATE", ignore(case))]
    AGGREGATE,
    #[token("ALL", ignore(case))]
    ALL,
    #[token("ALTER", ignore(case))]
    ALTER,
    #[token("ANALYZE", ignore(case))]
    ANALYZE,
    #[token("AND", ignore(case))]
    AND,
    #[token("ANY", ignore(case))]
    ANY,
    #[token("ARRAY", ignore(case))]
    ARRAY,
    #[token("AS", ignore(case))]
    AS,
    #[token("ASC", ignore(case))]
    ASC,
    #[token("BEGIN", ignore(case))]
    BEGIN,
    #[token("BETWEEN", ignore(case))]
    BETWEEN,
    #[token("BOOLEAN", ignore(case))]
    BOOLEAN,
    #[token("BY", ignore(case))]
    BY,
    #[token("CASE", ignore(case))]
    CASE,
    #[token("CAST", ignore(case))]
    CAST,
    #[token("CLUSTER", ignore(case))]
    CLUSTER,
    #[token("COLUMN", ignore(case))]
    COLUMN,
    #[token("COMMENT", ignore(case))]
    COMMENT,
    #[token("CREATE", ignore(case))]
    CREATE,
    #[token("DATABASES", ignore(case))]
    DATABASES,
    #[token("DATABASE", ignore(case))]
    DATABASE,
    #[token("DELETE", ignore(case))]
    DELETE,
    #[token("DESC", ignore(case))]
    DESC,
    #[token("DESCRIBE", ignore(case))]
    DESCRIBE,
    #[token("DISTINCT", ignore(case))]
    DISTINCT,
    #[token("DROP", ignore(case))]
    DROP,
    #[token("ELSE", ignore(case))]
    ELSE,
    #[token("END", ignore(case))]
    END,
    #[token("EQUALITY", ignore(case))]
    EQUALITY,
    #[token("EXCLUDE", ignore(case))]
    EXCLUDE,
    #[token("EXEC", ignore(case))]
    EXEC,
    #[token("EXISTS", ignore(case))]
    EXISTS,
    #[token("EXPLAIN", ignore(case))]
    EXPLAIN,
    #[token("FALSE", ignore(case))]
    FALSE,
    #[token("FIRST", ignore(case))]
    FIRST,
    #[token("FLOAT", ignore(case))]
    FLOAT,
    #[token("FROM", ignore(case))]
    FROM,
    #[token("FULL", ignore(case))]
    FULL,
    #[token("GROUP", ignore(case))]
    GROUP,
    #[token("IF", ignore(case))]
    IF,
    #[token("IN", ignore(case))]
    IN,
    #[token("INDEX", ignore(case))]
    INDEX,
    #[token("INNER", ignore(case))]
    INNER,
    #[token("INSERT", ignore(case))]
    INSERT,
    #[token("INT", ignore(case))]
    INT,
    #[token("INTERVAL", ignore(case))]
    INTERVAL,
    #[token("INTO", ignore(case))]
    INTO,
    #[token("IS", ignore(case))]
    IS,
    #[token("JOB", ignore(case))]
    JOB,
    #[token("JOBS", ignore(case))]
    JOBS,
    #[token("JOIN", ignore(case))]
    JOIN,
    #[token("KEY", ignore(case))]
    KEY,
    #[token("LAST", ignore(case))]
    LAST,
    #[token("LEFT", ignore(case))]
    LEFT,
    #[token("LIMIT", ignore(case))]
    LIMIT,
    #[token("MATERIALIZED", ignore(case))]
    MATERIALIZED,
    #[token("NODEGROUP", ignore(case))]
    NODEGROUP,
    #[token("NOT", ignore(case))]
    NOT,
    #[token("NULL", ignore(case))]
    NULL,
    #[token("NULLS", ignore(case))]
    NULLS,
    #[token("OBJECT", ignore(case))]
    OBJECT,
    #[token("OFFSET", ignore(case))]
    OFFSET,
    #[token("ON", ignore(case))]
    ON,
    #[token("OPTIMIZE", ignore(case))]
    OPTIMIZE,
    #[token("OR", ignore(case))]
    OR,
    #[token("ORDER", ignore(case))]
    ORDER,
    #[token("OUTER", ignore(case))]
    OUTER,
    #[token("PERCENT", ignore(case))]
    PERCENT,
    #[token("PLAN", ignore(case))]
    PLAN,
    #[token("RANGE", ignore(case))]
    RANGE,
    #[token("RENAME", ignore(case))]
    RENAME,
    #[token("REPLACE", ignore(case))]
    REPLACE,
    #[token("RESUME", ignore(case))]
    RESUME,
    #[token("RIGHT", ignore(case))]
    RIGHT,
    #[token("SAMPLE", ignore(case))]
    SAMPLE,
    #[token("SCHEDULE", ignore(case))]
    SCHEDULE,
    #[token("SCHEMAS", ignore(case))]
    SCHEMAS,
    #[token("SCHEMA", ignore(case))]
    SCHEMA,
    #[token("SEARCH", ignore(case))]
    SEARCH,
    #[token("SELECT", ignore(case))]
    SELECT,
    #[token("SET", ignore(case))]
    SET,
    #[token("SHOW", ignore(case))]
    SHOW,
    #[token("STATEMENTS", ignore(case))]
    STATEMENTS,
    #[token("STRING", ignore(case))]
    STRING,
    #[token("SUSPEND", ignore(case))]
    SUSPEND,
    #[token("TABLE", ignore(case))]
    TABLE,
    #[token("TABLES", ignore(case))]
    TABLES,
    #[token("THEN", ignore(case))]
    THEN,
    #[token("TIMESTAMP", ignore(case))]
    TIMESTAMP,
    #[token("TO", ignore(case))]
    TO,
    #[token("TRUE", ignore(case))]
    TRUE,
    #[token("UINT", ignore(case))]
    UINT,
    #[token("UNION", ignore(case))]
    UNION,
    #[token("UPDATE", ignore(case))]
    UPDATE,
    #[token("VACUUM", ignore(case))]
    VACUUM,
    #[token("VALUES", ignore(case))]
    VALUES,
    #[token("VIEW", ignore(case))]
    VIEW,
    #[token("VIEWS", ignore(case))]
    VIEWS,
    #[token("WHEN", ignore(case))]
    WHEN,
    #[token("WHERE", ignore(case))]
    WHERE,
    #[token("WINDOW", ignore(case))]
    WINDOW,
    #[token("WITH", ignore(case))]
    WITH,
    #[token("WITHIN", ignore(case))]
    WITHIN,
    #[token("XOR", ignore(case))]
    XOR,

    // Command-line only tokens
    #[cfg(feature = "command")]
    #[token("\\")]
    BackSlash,
    #[cfg(feature = "command")]
    #[token("CANCEL", ignore(case))]
    CANCEL,
}

impl TokenKind {
    pub fn is_literal(&self) -> bool {
        use TokenKind::*;

        matches!(
            self,
            LiteralFloat
                | LiteralInteger
                | LiteralString
                | LiteralHexBinaryString
                | LiteralHexInteger
        )
    }

    pub fn is_symbol(&self) -> bool {
        use TokenKind::*;

        #[cfg(feature = "command")]
        if matches!(self, BackSlash) {
            return true;
        }

        matches!(
            self,
            Eq | NotEq
                | Lt
                | Gt
                | Lte
                | Gte
                | Plus
                | Minus
                | Multiply
                | Divide
                | Modulo
                | Concat
                | LParen
                | RParen
                | LBracket
                | RBracket
                | LBrace
                | RBrace
                | Comma
                | Dot
                | Colon
                | DoubleColon
                | SemiColon
                | Dollar
                | Arrow
        )
    }

    pub fn is_keyword(&self) -> bool {
        use TokenKind::*;

        !self.is_literal()
            && !self.is_symbol()
            && !matches!(self, Ident | EOI | Whitespace | Comment)
    }

    pub fn is_reserved_keyword(&self) -> bool {
        use TokenKind::*;

        matches!(
            self,
            FROM | JOIN
                | VALUES
                | WHERE
                | ORDER
                | DISTINCT
                | LIMIT
                | SELECT
                | AGGREGATE
                | WINDOW
                | WITHIN
                | GROUP
                | INSERT
                | UNION
                | SAMPLE
                | NULL
                | TRUE
                | FALSE
                | AS
                | BY
                | ON
                | CASE
                | WHEN
                | THEN
                | ELSE
                | END
                | CAST
                | NOT
                | IS
                | IN
                | BETWEEN
                | AND
                | OR
        )
    }
}
