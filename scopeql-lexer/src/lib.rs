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

pub enum TokenKind {
    /// A special token representing the end of input.
    EOI,

    // skipped
    /// Whitespace characters.
    Whitespace,
    /// Single line comments.
    Comment,
    /// Multi-line comments.
    CommentBlock,

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
    /// Arrow (`->`).
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
