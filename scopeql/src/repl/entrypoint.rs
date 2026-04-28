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

use std::fmt::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use clap::CommandFactory;
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
use crate::repl::command::TimerMode;
use crate::repl::command::render_repl_parse_error;
use crate::repl::highlight::ScopeQLHighlighter;
use crate::repl::lexer;
use crate::repl::prompt::CommandLinePrompt;
use crate::repl::prompt::PromptRenderState;
use crate::repl::prompt::StatusHinter;
use crate::repl::validate::ScopeQLValidator;
use crate::tokenizer::tokenize;

// TODO: This is a workaround for reedline's Ctrl-C handling, which clears the
// buffer before bubbling up Signal::CtrlC and prevents us from deciding whether
// to clear or exit based on the prompt contents.
const CTRL_C_PROMPT_COMMAND: &str = "\0scopeql:prompt-ctrl-c\0";
const CTRL_D_PROMPT_COMMAND: &str = "\0scopeql:prompt-ctrl-d\0";

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

fn prompt_status_line(
    connection_name: &str,
    endpoint: &str,
    output_format: OutputFormat,
    show_timer: bool,
) -> String {
    format!(
        "{} · {} · {} · timer {}",
        connection_name,
        endpoint,
        output_format.as_str(),
        if show_timer { "on" } else { "off" },
    )
}

pub struct ReplState<'a> {
    pub config: &'a Config,
    pub connection_name: String,
    pub client: ScopeQLClient,
    pub prompt: CommandLinePrompt,
    pub prompt_state: Arc<Mutex<PromptRenderState>>,
    pub output_format: OutputFormat,
    pub show_timer: bool,
}

impl<'a> ReplState<'a> {
    fn new(config: &'a Config) -> Self {
        let connection_name = config.default_connection_name().to_owned();
        let connection = config
            .get_connection(&connection_name)
            .expect("no default connection in config");
        let output_format = OutputFormat::Table;
        let show_timer = true;
        let prompt_state = Arc::new(Mutex::new(PromptRenderState::new(prompt_status_line(
            &connection_name,
            connection.endpoint(),
            output_format,
            show_timer,
        ))));

        Self {
            config,
            connection_name,
            client: ScopeQLClient::from_connection(connection),
            prompt: CommandLinePrompt::new(Arc::clone(&prompt_state)),
            prompt_state,
            output_format,
            show_timer,
        }
    }

    fn endpoint(&self) -> &str {
        self.config
            .get_connection(&self.connection_name)
            .map(|connection| connection.endpoint())
            .unwrap_or_default()
    }

    fn refresh_prompt_status(&self) {
        self.prompt_state
            .lock()
            .unwrap()
            .set_line(prompt_status_line(
                &self.connection_name,
                self.endpoint(),
                self.output_format,
                self.show_timer,
            ));
    }

    fn set_output_format(&mut self, output_format: OutputFormat) {
        self.output_format = output_format;
        self.refresh_prompt_status();
    }

    fn set_show_timer(&mut self, show_timer: bool) {
        self.show_timer = show_timer;
        self.refresh_prompt_status();
    }

    fn switch_connection(&mut self, name: String) -> Result<(), String> {
        let Some(connection) = self.config.get_connection(&name) else {
            let profiles = self
                .config
                .connection_names()
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!(
                "unknown connection profile {name:?}; available profiles: {profiles}"
            ));
        };

        self.client = ScopeQLClient::from_connection(connection);
        self.connection_name = name;
        self.refresh_prompt_status();
        Ok(())
    }
}

pub fn entrypoint(config: &Config) {
    let mut repl = ReplState::new(config);

    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::HistoryHintComplete,
    );
    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('c'),
        ReedlineEvent::ExecuteHostCommand(CTRL_C_PROMPT_COMMAND.to_owned()),
    );
    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('d'),
        ReedlineEvent::ExecuteHostCommand(CTRL_D_PROMPT_COMMAND.to_owned()),
    );

    let hinter = StatusHinter::new(
        DefaultHinter::default().with_style(Style::new().fg(Color::DarkGray)),
        Arc::clone(&repl.prompt_state),
    );

    let mut line_editor = Reedline::create()
        .use_bracketed_paste(true)
        .with_validator(Box::new(ScopeQLValidator))
        .with_highlighter(Box::new(ScopeQLHighlighter))
        .with_hinter(Box::new(hinter))
        .with_edit_mode(Box::new(Emacs::new(keybindings)));

    if let Some(history) = make_file_history() {
        line_editor = line_editor.with_history(Box::new(history));
    }

    loop {
        let input = line_editor
            .read_line(&repl.prompt)
            .expect("failed to read next line");
        let input = match input {
            Signal::Success(input)
                if input == CTRL_C_PROMPT_COMMAND || input == CTRL_D_PROMPT_COMMAND =>
            {
                if line_editor.current_buffer_contents().is_empty() {
                    println!();
                    break;
                } else {
                    line_editor.run_edit_commands(&[EditCommand::Clear]);
                    continue;
                }
            }
            Signal::Success(input) => input,
            _ => {
                println!();
                break;
            }
        };
        let input = input.trim();

        // special repl command
        if input.starts_with("/") && !input.starts_with("/*") {
            let mut args = match lexer::lex(input) {
                lexer::LexerResult::Complete(args) => args,
                lexer::LexerResult::Incomplete => {
                    eprintln!("error: failed to parse incomplete repl command");
                    continue;
                }
                lexer::LexerResult::UnknownEscape(ch) => {
                    eprintln!("error: failed to parse unknown escape char: \\{ch}");
                    continue;
                }
            };
            args.insert(0, String::new());

            let cmd = match ReplCommand::try_parse_from(args) {
                Ok(cmd) => cmd,
                Err(err) => {
                    eprint!("{}", render_repl_parse_error(err));
                    continue;
                }
            };

            match cmd.cmd {
                ReplSubCommand::Connection(connection) => {
                    if let Err(err) = repl.switch_connection(connection.name) {
                        eprintln!("error: {err}");
                        continue;
                    }
                    println!(
                        "Connection is set to {} ({})",
                        repl.connection_name,
                        repl.endpoint()
                    );
                }
                ReplSubCommand::Format(format) => {
                    repl.set_output_format(format.format);
                    println!("Output format is set to {}", repl.output_format.as_str());
                }
                ReplSubCommand::Timer(timer) => match timer.mode {
                    TimerMode::On => {
                        repl.set_show_timer(true);
                        println!("Timer is set to on");
                    }
                    TimerMode::Off => {
                        repl.set_show_timer(false);
                        println!("Timer is set to off");
                    }
                },
                ReplSubCommand::Cancel(cancel) => cancel.run(&repl.client),
                ReplSubCommand::Help => {
                    let cmd = ReplCommand::command();

                    let width = cmd
                        .get_subcommands()
                        .map(|c| c.get_name().len())
                        .max()
                        .unwrap_or(0);

                    println!("Commands:");
                    for subcommand in cmd.get_subcommands() {
                        let mut message = format!("  {:width$}", subcommand.get_name());
                        if let Some(about) = subcommand.get_about() {
                            write!(&mut message, "  {about}").unwrap();
                        }
                        println!("{message}");
                    }
                }
            }
            continue;
        }

        let tokens = match tokenize(input) {
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

            let client = &repl.client;
            let output_format = repl.output_format;
            let show_timer = repl.show_timer;
            let output = global::rt().block_on({
                let pb = pb.clone();
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
                Some(Err(err)) => eprintln!("error: statement {statement_id} failed: {err:?}"),
                None => {
                    let output = global::rt().block_on(client.cancel_statement(statement_id));
                    match output {
                        Ok(_) => println!("Statement {statement_id} has been cancelled"),
                        Err(err) => {
                            eprintln!("error: failed to cancel statement {statement_id}: {err:?}")
                        }
                    }
                }
            }
        }

        line_editor.run_edit_commands(&[EditCommand::InsertString(
            outstanding.trim_start().to_string(),
        )]);
    }
}
