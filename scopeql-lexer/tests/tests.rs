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

use scopeql_lexer::Lexer;

fn lex(sql: &str) -> Vec<String> {
    let mut lexer = Lexer::new(sql);
    let mut tokens = Vec::new();
    while let Some(res) = lexer.next() {
        match res {
            Ok(kind) => tokens.push(format!("{:?}({:?})", kind, lexer.slice())),
            Err(e) => tokens.push(format!("Err({:?})", e)),
        }
    }
    tokens
}

#[test]
fn test_basic_statements() {
    let sql = "create table public.t1 (id int, message string)";
    insta::assert_debug_snapshot!(lex(sql));

    let sql = "values (1, 'a'), (2, 'b'), (3, 'c') insert into public.t1";
    insta::assert_debug_snapshot!(lex(sql));
}

#[test]
fn test_complex_query() {
    let sql = "from system.tables where database_name = 'scopedb' and schema_name != 'system'";
    insta::assert_debug_snapshot!(lex(sql));

    let sql = "select 1, 'a' as a select a as b, $0";
    insta::assert_debug_snapshot!(lex(sql));
}

#[test]
fn test_comments() {
    let sql = "
    -- This is a comment
    select 1;
    /* This is a 
       multi-line comment */
    select 2;
    ";
    insta::assert_debug_snapshot!(lex(sql));
}

#[test]
fn test_literals_and_escapes() {
    let sql = r#"values (1, 'a''b', "c\"d", `e\`f`), (x'FF', 0xFF)"#;
    insta::assert_debug_snapshot!(lex(sql));
}

#[test]
fn test_numbers() {
    let sql = "values (123, 123.456, 1e10, 1.2e-3, 0x1A, 0)";
    insta::assert_debug_snapshot!(lex(sql));

    // Test dot ambiguity
    let sql = "1. .2 3."; // 1. is Int(1) Dot; .2 is Dot Int(2); 3. is Int(3) Dot
    insta::assert_debug_snapshot!(lex(sql));
}

#[test]
fn test_symbols() {
    let sql = "+ - * / % || ( ) [ ] { } , . : :: ; $ -> => = != <> < > <= >=";
    insta::assert_debug_snapshot!(lex(sql));
}

#[test]
fn test_keywords_case_insensitive() {
    let sql = "SeLeCt * FrOm T wHeRe Id = 1";
    insta::assert_debug_snapshot!(lex(sql));
}

#[test]
fn test_errors() {
    let sql = "select 'unterminated string";
    insta::assert_debug_snapshot!(lex(sql));

    let sql = "select /* unterminated comment";
    insta::assert_debug_snapshot!(lex(sql));

    let sql = "select 0xG"; // Invalid hex
    insta::assert_debug_snapshot!(lex(sql));
}

#[test]
fn test_struct_access() {
    let sql = "from t select id - 1, var['message']::string as message";
    insta::assert_debug_snapshot!(lex(sql));
}
