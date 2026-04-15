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

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

use reqwest::header::HeaderMap;
use scopeql_parser::TokenKind;

use crate::client::ScopeQLClient;
use crate::command::OutputFormat;
use crate::config::Config;
use crate::global;
use crate::global::eprintln_and_error;
use crate::tokenizer::run_tokenizer;

pub fn execute(
    config: &Config,
    quiet: bool,
    format: OutputFormat,
    stmts: String,
    output_file: Option<PathBuf>,
    headers: HeaderMap,
) {
    let connection = config
        .get_default_connection()
        .expect("no default connection in config");
    let mut client = ScopeQLClient::from_connection(connection);
    client.set_headers(headers);

    let statements = match top_level_statements(&stmts) {
        Ok(statements) => statements,
        Err(err) => {
            eprintln_and_error(format_args!("failed to parse statements: {err:?}"));
            std::process::exit(1);
        }
    };

    if statements.is_empty() {
        return;
    }

    if statements.len() > 1 && !supports_multi_statement_output(format, quiet) {
        log::error!(
            "run command received multiple top-level statements with incompatible output mode {}",
            format.as_str()
        );
        eprintln!(
            "error: --format {} does not support multiple top-level statements; use --quiet, --format table, --format jsonl, or wrap statements in a transaction",
            format.as_str()
        );
        std::process::exit(1);
    }

    let mut output_file = match output_file {
        Some(output_file) => match OpenOptions::new()
            .append(true)
            .create(true)
            .open(&output_file)
        {
            Ok(file) => Some(file),
            Err(err) => {
                log::error!(
                    "failed to open output file: {}; {err:?}",
                    output_file.display()
                );
                return;
            }
        },
        None => None,
    };

    for stmt in statements {
        let id = uuid::Uuid::now_v7();
        log::info!("executing statement {id}");

        match global::rt().block_on(client.execute_statement(
            id,
            stmt.to_string(),
            format,
            true,
            |_, _| (),
        )) {
            Ok(output) => {
                log::info!("statement {id} completed successfully");
                if let Some(ref mut output_file) = output_file {
                    output_file
                        .write_all(output.as_bytes())
                        .unwrap_or_else(|err| {
                            log::error!(
                                "failed to write output for statement {id} to file: {err:?}",
                            );
                        });
                } else if !quiet {
                    println!("{output}");
                }
            }
            Err(err) => {
                eprintln_and_error(format_args!("statement {id} failed: {err:?}"));
                std::process::exit(1);
            }
        }
    }
}

fn supports_multi_statement_output(output: OutputFormat, quiet: bool) -> bool {
    quiet || matches!(output, OutputFormat::Table | OutputFormat::Jsonl)
}

fn top_level_statements(source: &str) -> exn::Result<Vec<&str>, crate::Error> {
    let tokens = run_tokenizer(source)?;
    let mut statements = vec![];
    let mut start = 0;
    let mut in_transaction = false;
    let mut in_statement = false;

    for token in &tokens {
        if !in_statement {
            start = token.span.start;
            in_statement = true;
        }

        match token.kind {
            TokenKind::BEGIN => in_transaction = true,
            TokenKind::END => in_transaction = false,
            TokenKind::SemiColon if !in_transaction => {
                let statement = source[start..token.span.start].trim();
                if !statement.is_empty() {
                    statements.push(statement);
                }
                in_statement = false;
            }
            _ => {}
        }
    }

    if in_statement {
        let statement = source[start..].trim();
        if !statement.is_empty() {
            statements.push(statement);
        }
    }

    Ok(statements)
}

#[cfg(test)]
mod tests {
    use super::supports_multi_statement_output;
    use super::top_level_statements;
    use crate::command::OutputFormat;

    #[test]
    fn transaction_is_a_single_top_level_statement() {
        let statements =
            top_level_statements("BEGIN; INSERT INTO t VALUES (1); INSERT INTO t VALUES (2); END;")
                .unwrap();

        assert_eq!(
            statements,
            vec!["BEGIN; INSERT INTO t VALUES (1); INSERT INTO t VALUES (2); END"]
        );
    }

    #[test]
    fn multiple_top_level_statements_are_detected() {
        let statements = top_level_statements("SELECT 1; SELECT 2;").unwrap();

        assert_eq!(statements, vec!["SELECT 1", "SELECT 2"]);
    }

    #[test]
    fn trailing_whitespace_after_semicolon_is_ignored() {
        let statements = top_level_statements("SELECT 1;   \n\t").unwrap();

        assert_eq!(statements, vec!["SELECT 1"]);
    }

    #[test]
    fn multi_statement_output_policy_allows_table_jsonl_and_quiet() {
        assert!(supports_multi_statement_output(OutputFormat::Table, false));
        assert!(supports_multi_statement_output(OutputFormat::Jsonl, false));
        assert!(supports_multi_statement_output(OutputFormat::Json, true));
    }

    #[test]
    fn multi_statement_output_policy_rejects_json_and_csv() {
        assert!(!supports_multi_statement_output(OutputFormat::Json, false));
        assert!(!supports_multi_statement_output(OutputFormat::Csv, false));
    }
}
