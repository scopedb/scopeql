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

use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use mea::latch::Latch;
use nu_ansi_term::Color;
use nu_ansi_term::Style;
use reedline::DefaultHinter;
use reedline::EditCommand;
use reedline::Emacs;
use reedline::FileBackedHistory;
use reedline::KeyCode;
use reedline::KeyModifiers;
use reedline::Reedline;
use reedline::ReedlineEvent;
use reedline::Signal;
use reedline::default_emacs_keybindings;
use scopeql_parser::TokenKind;

use crate::client::ScopeQLClient;
use crate::command::OutputFormat;
use crate::config::Config;
use crate::global;
use crate::repl::command::ReplCommand;
use crate::repl::command::ReplSubCommand;
use crate::repl::command::TimerToggle;
use crate::repl::highlight::ScopeQLHighlighter;
use crate::repl::prompt::CommandLinePrompt;
use crate::repl::validate::ScopeQLValidator;
use crate::tokenizer::run_tokenizer;

fn make_file_history() -> Option<FileBackedHistory> {
    let Some(home_dir) = dirs::home_dir() else {
        eprintln!("cannot get home directory; history disabled");
        return None;
    };

    let history_file = home_dir.join(".scopeql_history");
    match FileBackedHistory::with_file(1000, history_file) {
        Ok(history) => Some(history),
        Err(err) => {
            eprintln!("warning: cannot open history file: {err}");
            None
        }
    }
}

pub fn entrypoint(config: &Config) {
    let connection = config
        .get_default_connection()
        .expect("no default connection in config");
    let endpoint = connection.endpoint().to_owned();
    let mut output_format = OutputFormat::Table;
    let mut show_timer = true;

    let mut prompt = CommandLinePrompt::default();
    let client = if endpoint.is_empty() {
        eprintln!("error: endpoint is empty");
        return;
    } else {
        prompt.set_endpoint(Some(endpoint.clone()));
        ScopeQLClient::from_connection(connection)
    };

    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::HistoryHintComplete,
    );

    let hinter = DefaultHinter::default().with_style(Style::new().fg(Color::DarkGray));

    let mut state = Reedline::create()
        .use_bracketed_paste(true)
        .with_validator(Box::new(ScopeQLValidator))
        .with_highlighter(Box::new(ScopeQLHighlighter))
        .with_hinter(Box::new(hinter))
        .with_edit_mode(Box::new(Emacs::new(keybindings)));

    if let Some(history) = make_file_history() {
        state = state.with_history(Box::new(history));
    }

    loop {
        let input = state.read_line(&prompt).expect("failed to read next line");
        let input = match input {
            Signal::Success(input) => input,
            Signal::CtrlC | Signal::CtrlD | Signal::ExternalBreak(_) | _ => {
                println!("Exit");
                break;
            }
        };
        let input = input.trim();

        // special repl command
        if let Some(input) = input.strip_prefix("\\") {
            let cmd = match ReplCommand::try_parse_from(input.split_whitespace()) {
                Ok(cmd) => cmd,
                Err(err) => {
                    eprintln!("{err}");
                    continue;
                }
            };

            match cmd.cmd {
                ReplSubCommand::Cancel(cancel) => cancel.run(&client),
                ReplSubCommand::Mode(mode) => {
                    output_format = mode.format;
                    println!("output format: {}", output_format.as_str());
                }
                ReplSubCommand::Timer(timer) => match timer.toggle {
                    TimerToggle::On => {
                        show_timer = true;
                        println!("timer: on");
                    }
                    TimerToggle::Off => {
                        show_timer = false;
                        println!("timer: off");
                    }
                },
            }
            continue;
        }

        let tokens = match run_tokenizer(input) {
            Ok(tokens) => tokens,
            Err(err) => {
                eprintln!("{err}");
                continue;
            }
        };

        let mut stmts_range = vec![];
        let mut start = 0;
        let mut in_transaction = false;
        let mut in_statement = true;

        for token in &tokens {
            // transactions
            match token.kind {
                TokenKind::BEGIN => in_transaction = true,
                TokenKind::END => in_transaction = false,
                _ => {}
            }

            // semicolons
            match token.kind {
                TokenKind::SemiColon => {
                    if in_statement && !in_transaction {
                        let end = token.span.start;
                        stmts_range.push(start..end);
                        start = token.span.end;
                        in_statement = false;
                    }
                }
                _ => {
                    if !in_statement {
                        start = token.span.start;
                        in_statement = true;
                    }
                }
            }
        }

        let outstanding = input[start..].trim_start();
        for range in stmts_range {
            let stmt = input[range].to_string();

            let statement_id = uuid::Uuid::now_v7();
            println!("StatementID: {statement_id}");

            let pb_style = "{spinner:.green} [{elapsed_precise}] {msg:.green.bold.bright} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})";
            let pb = ProgressBar::no_length()
                .with_style(ProgressStyle::with_template(pb_style).unwrap());
            let stop_pb = Arc::new(Latch::new(1));

            global::rt().spawn({
                let pb = pb.clone();
                let stop_pb = stop_pb.clone();
                async move {
                    while stop_pb.try_wait().is_err() {
                        tokio::time::sleep(Duration::from_millis(42)).await;
                        pb.tick();
                    }
                }
            });

            let output = global::rt().block_on({
                let pb = pb.clone();
                let client = &client;
                async move {
                    let fut = client.execute_statement(
                        statement_id,
                        stmt,
                        output_format,
                        show_timer,
                        |status, progress| {
                            pb.set_message(status.to_string());
                            if progress.details.total_uncompressed_bytes > 0 {
                                pb.set_length(progress.details.total_uncompressed_bytes as u64);
                                pb.set_position(
                                    (progress.details.total_percentage() / 100.0
                                        * progress.details.total_uncompressed_bytes as f64)
                                        as u64,
                                );
                            }
                        },
                    );

                    tokio::select! {
                        _ = tokio::signal::ctrl_c() => None,
                        output = fut => Some(output),
                    }
                }
            });

            stop_pb.count_down();
            pb.finish_and_clear();

            match output {
                Some(Ok(output)) => println!("{output}"),
                Some(Err(err)) => eprintln!("error: statement {statement_id} failed: {err}"),
                None => {
                    let output = global::rt().block_on(client.cancel_statement(statement_id));
                    match output {
                        Ok(_) => println!("Statement {statement_id} has been cancelled"),
                        Err(err) => {
                            eprintln!("error: failed to cancel statement {statement_id}: {err}")
                        }
                    }
                }
            }
        }

        state.run_edit_commands(&[EditCommand::InsertString(
            outstanding.trim_start().to_string(),
        )]);
    }
}
