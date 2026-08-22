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
use std::future::Future;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use scopeql_parser::TokenKind;

use crate::client::ScopeQLClient;
use crate::command::OutputFormat;
use crate::config::Config;
use crate::global;
use crate::global::eprintln_and_error;
use crate::tokenizer::tokenize;

const CANCEL_TIMEOUT: Duration = Duration::from_secs(5);
const INTERRUPTED_EXIT_CODE: i32 = 130;

#[derive(Debug, PartialEq, Eq)]
enum StatementExecution<T, C> {
    Completed(T),
    Interrupted(Cancellation<C>),
}

#[derive(Debug, PartialEq, Eq)]
enum Cancellation<T> {
    Completed(T),
    TimedOut,
    Interrupted,
}

pub fn execute(
    config: &Config,
    quiet: bool,
    format: OutputFormat,
    stmts: String,
    output_file: Option<PathBuf>,
) {
    let connection = config
        .get_default_connection()
        .expect("no default connection in config");
    let client = match ScopeQLClient::from_connection(connection) {
        Ok(client) => client,
        Err(err) => {
            eprintln_and_error(format_args!("failed to create client from config: {err:?}"));
            std::process::exit(1);
        }
    };

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

        let execution = global::rt().block_on(execute_with_cancellation(
            client.execute_statement(id, stmt.to_string(), format, true, |_, _| ()),
            wait_for_ctrl_c(),
            || async {
                eprintln!("interrupt received; cancelling statement {id}...");
                client.cancel_statement(id).await
            },
            wait_for_ctrl_c(),
            CANCEL_TIMEOUT,
        ));

        match execution {
            StatementExecution::Completed(Ok(output)) => {
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
            StatementExecution::Completed(Err(err)) => {
                eprintln_and_error(format_args!("statement {id} failed: {err:?}"));
                std::process::exit(1);
            }
            StatementExecution::Interrupted(cancellation) => {
                match cancellation {
                    Cancellation::Completed(Ok(result)) => {
                        eprintln!(
                            "statement {id} cancellation result: {}: {}",
                            result.status, result.message
                        );
                        log::info!(
                            "statement {id} cancellation completed with status {}: {}",
                            result.status,
                            result.message
                        );
                    }
                    Cancellation::Completed(Err(err)) => {
                        eprintln!(
                            "warning: failed to cancel statement {id}: {err:?}; it may still be running"
                        );
                        log::warn!("failed to cancel statement {id}: {err:?}");
                    }
                    Cancellation::TimedOut => {
                        eprintln!(
                            "warning: timed out after {} seconds while cancelling statement {id}; it may still be running",
                            CANCEL_TIMEOUT.as_secs()
                        );
                        log::warn!("timed out while cancelling statement {id}");
                    }
                    Cancellation::Interrupted => {
                        eprintln!(
                            "warning: cancellation interrupted; statement {id} may still be running"
                        );
                        log::warn!("cancellation interrupted for statement {id}");
                    }
                }
                std::process::exit(INTERRUPTED_EXIT_CODE);
            }
        }
    }
}

async fn execute_with_cancellation<T, C, F, CF>(
    execution: impl Future<Output = T>,
    interrupt: impl Future<Output = ()>,
    cancel: F,
    cancel_interrupt: impl Future<Output = ()>,
    cancel_timeout: Duration,
) -> StatementExecution<T, C>
where
    F: FnOnce() -> CF,
    CF: Future<Output = C>,
{
    tokio::select! {
        biased;
        () = interrupt => StatementExecution::Interrupted(
            wait_for_cancellation(cancel(), cancel_interrupt, cancel_timeout).await
        ),
        result = execution => StatementExecution::Completed(result),
    }
}

async fn wait_for_cancellation<T>(
    cancellation: impl Future<Output = T>,
    interrupt: impl Future<Output = ()>,
    timeout: Duration,
) -> Cancellation<T> {
    tokio::select! {
        biased;
        () = interrupt => Cancellation::Interrupted,
        result = tokio::time::timeout(timeout, cancellation) => match result {
            Ok(result) => Cancellation::Completed(result),
            Err(_) => Cancellation::TimedOut,
        },
    }
}

async fn wait_for_ctrl_c() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        log::error!("failed to listen for Ctrl+C: {err}");
        std::future::pending::<()>().await;
    }
}

fn supports_multi_statement_output(output: OutputFormat, quiet: bool) -> bool {
    quiet || matches!(output, OutputFormat::Table | OutputFormat::Jsonl)
}

fn top_level_statements(source: &str) -> exn::Result<Vec<&str>, crate::Error> {
    let tokens = tokenize(source)?;
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
    use std::cell::Cell;
    use std::future;
    use std::time::Duration;

    use super::Cancellation;
    use super::StatementExecution;
    use super::execute_with_cancellation;
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

    #[tokio::test]
    async fn completed_execution_does_not_request_cancellation() {
        let cancel_called = Cell::new(false);
        let execution = execute_with_cancellation(
            future::ready("finished"),
            future::pending(),
            || {
                cancel_called.set(true);
                future::ready("cancelled")
            },
            future::pending(),
            Duration::ZERO,
        )
        .await;

        assert_eq!(execution, StatementExecution::Completed("finished"));
        assert!(!cancel_called.get());
    }

    #[tokio::test]
    async fn interrupt_requests_cancellation() {
        let execution = execute_with_cancellation(
            future::pending::<()>(),
            future::ready(()),
            || future::ready("cancelled"),
            future::pending(),
            Duration::from_secs(1),
        )
        .await;

        assert_eq!(
            execution,
            StatementExecution::Interrupted(Cancellation::Completed("cancelled"))
        );
    }

    #[tokio::test]
    async fn second_interrupt_stops_waiting_for_cancellation() {
        let execution = execute_with_cancellation(
            future::pending::<()>(),
            future::ready(()),
            future::pending::<()>,
            future::ready(()),
            Duration::from_secs(1),
        )
        .await;

        assert_eq!(
            execution,
            StatementExecution::Interrupted(Cancellation::Interrupted)
        );
    }

    #[tokio::test]
    async fn cancellation_wait_is_bounded() {
        let execution = execute_with_cancellation(
            future::pending::<()>(),
            future::ready(()),
            future::pending::<()>,
            future::pending(),
            Duration::ZERO,
        )
        .await;

        assert_eq!(
            execution,
            StatementExecution::Interrupted(Cancellation::TimedOut)
        );
    }
}
