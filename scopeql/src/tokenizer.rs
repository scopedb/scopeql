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

use scopeql_parser::TokenKind;
use scopeql_parser::Tokenizer;

#[derive(Clone, PartialEq, Eq)]
pub struct Token<'a> {
    pub source: &'a str,
    pub kind: TokenKind,
    pub span: Range<usize>,
}

pub fn run_tokenizer(source: &'_ str) -> Result<Vec<Token<'_>>, ()> {
    let mut tokens = vec![];
    let mut tokenizer = Tokenizer::new(source);
    while let Some(token) = tokenizer.next() {
        let kind = token?;
        tokens.push(Token {
            source,
            kind,
            span: tokenizer.span(),
        });
    }
    Ok(tokens)
}
