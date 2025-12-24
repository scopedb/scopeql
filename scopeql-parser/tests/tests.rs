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

use comfy_table::Cell;
use comfy_table::Table;
use insta::assert_snapshot;
use scopeql_parser::TokenKind;
use scopeql_parser::Tokenizer;

fn lex(sql: &str) -> String {
    let mut tokenizer = Tokenizer::new(sql);
    let mut table = Table::new();
    table.set_header(["Status", "Token", "Slice", "Span"]);
    while let Some(result) = tokenizer.next() {
        match result {
            Ok(token) => {
                let cell0 = Cell::new("OK");
                let cell1 = Cell::new(format!("{token:?}"));
                let cell2 = match token {
                    TokenKind::Whitespace => Cell::new("<whitespace>"),
                    TokenKind::Comment => Cell::new("<comment>"),
                    _ => Cell::new(tokenizer.slice()),
                };
                let cell3 = Cell::new(format!("{:?}", tokenizer.span()));
                table.add_row([cell0, cell1, cell2, cell3]);
            }
            Err(err) => {
                let cell0 = Cell::new("Err");
                let cell1 = Cell::new(format!("{err:?}"));
                let cell2 = Cell::new(tokenizer.slice());
                let cell3 = Cell::new(format!("{:?}", tokenizer.span()));
                table.add_row([cell0, cell1, cell2, cell3]);
            }
        }
    }

    format!("[INPUT]\n{sql}\n[OUTPUT]\n{table}")
}

#[test]
fn test_ok() {
    assert_snapshot!(lex("create table public.t1 (id int, message string)"));
    assert_snapshot!(lex(
        "values (1, 'a'), (2, 'b'), (3, 'c') insert into public.t1"
    ));
    assert_snapshot!(lex(
        "from system.tables where database_name = 'scopedb' and schema_name != 'system'"
    ));
    assert_snapshot!(lex("select 1, 'a' as a select a as b, $0"));
}

#[test]
fn test_comments() {
    assert_snapshot!(lex(r#"
    -- This is a comment
    select 1;
    /* This is a
       multi-line comment */
    select 2;
    "#));
}

#[test]
fn test_literals_and_escapes() {
    assert_snapshot!(lex(r#"values (1, 'a''b', "c\"d", `e\`f`), (x'FF', 0xFF)"#));
}

#[test]
fn test_numbers() {
    assert_snapshot!(lex("values (123, 123.456, 1e10, 1.2e-3, 0x1A, 0)"));
    // Test dot ambiguity
    // 1. is Int(1) Dot; .2 is Dot Int(2); 3. is Int(3) Dot
    assert_snapshot!(lex("1. .2 3."));
}

#[test]
fn test_symbols() {
    assert_snapshot!(lex(
        "+ - * / % || ( ) [ ] { } , . : :: ; $ -> => = != <> < > <= >="
    ));
}

#[test]
fn test_keywords_case_insensitive() {
    assert_snapshot!(lex("SeLeCt * FrOm T wHeRe Id = 1"));
}

#[test]
fn test_errors() {
    assert_snapshot!(lex("select 'unterminated string"));
    assert_snapshot!(lex("select /* unterminated comment"));
    assert_snapshot!(lex("select 0xG")); // invalid hex
}

#[test]
fn test_struct_access() {
    assert_snapshot!(lex(
        "from t select id - 1, var['message']::string as message"
    ));
}
