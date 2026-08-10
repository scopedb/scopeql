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

use std::io::IsTerminal;
use std::io::Read;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use clap::Parser;
use logforth::append::file::FileBuilder;
use logforth::filter::rustlog::RustLogFilterBuilder;
use logforth::layout::JsonLayout;

use crate::command::Command;
use crate::command::ExecArgs;
use crate::command::RunArgs;
use crate::command::Subcommand;
use crate::config::load_config;
use crate::global::eprintln_and_error;

mod client;
mod command;
mod config;
mod execute;
mod global;
mod header;
mod load;
mod output;
mod pretty;
mod tokenizer;
mod version;

fn main() {
    let cmd = Command::parse();
    setup_logger();

    match cmd.subcommand {
        Subcommand::Run(args) => run(args),
        Subcommand::Load {
            args: ExecArgs { config_file, quiet },
            file,
            transform,
            format,
        } => {
            log::info!("starting load command for {}", file.display());
            let config = load_config(config_file);
            load::load(&config, quiet, file, transform, format);
        }
        Subcommand::Connection { cmd } => cmd.run(),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ScopeQLInput {
    Stdin,
    File(PathBuf),
    Statement(String),
}

fn run(args: RunArgs) {
    let RunArgs {
        args: ExecArgs { config_file, quiet },
        file,
        statement,
        format,
        output_file,
    } = args;

    let stdin = std::io::stdin();
    let input = resolve_scopeql_input(file, statement, stdin.is_terminal()).unwrap_or_else(|err| {
        eprintln_and_error(format_args!("{err}"));
        std::process::exit(2);
    });
    let source = read_scopeql_input(input, stdin.lock()).unwrap_or_else(|err| {
        eprintln_and_error(format_args!("{err}"));
        std::process::exit(1);
    });

    let config = load_config(config_file);
    execute::execute(&config, quiet, format, source, output_file);
}

fn resolve_scopeql_input(
    file: Option<PathBuf>,
    statement: Option<String>,
    stdin_is_terminal: bool,
) -> Result<ScopeQLInput, Error> {
    match (file, statement) {
        (Some(_), Some(_)) => Err(Error::new(
            "provide either statement text or -f/--file, not both",
        )),
        (None, Some(statement)) => Ok(ScopeQLInput::Statement(statement)),
        (Some(file), None) => Ok(ScopeQLInput::File(file)),
        (None, None) if !stdin_is_terminal => Ok(ScopeQLInput::Stdin),
        (None, None) => Err(Error::new(
            "missing input; provide statement text, use -f/--file, or pipe ScopeQL through stdin",
        )),
    }
}

fn read_scopeql_input(input: ScopeQLInput, mut stdin: impl Read) -> Result<String, Error> {
    match input {
        ScopeQLInput::Statement(statement) => {
            log::info!("running ScopeQL statements from inline input");
            Ok(statement)
        }
        ScopeQLInput::File(file) => {
            log::info!("running ScopeQL statements from file {}", file.display());
            std::fs::read_to_string(&file).map_err(|err| {
                Error::new(format!(
                    "failed to read script file {}: {err}",
                    file.display()
                ))
            })
        }
        ScopeQLInput::Stdin => {
            log::info!("running ScopeQL statements from stdin");
            let mut source = String::new();
            stdin
                .read_to_string(&mut source)
                .map_err(|err| Error::new(format!("failed to read ScopeQL from stdin: {err}")))?;
            Ok(source)
        }
    }
}

fn setup_logger() {
    let Some(log_dir) = dirs::cache_dir()
        .map(|dir| dir.join("scopeql").join("logs"))
        .or_else(|| dirs::home_dir().map(|dir| dir.join(".scopeql").join("logs")))
    else {
        return;
    };

    let Ok(append) = FileBuilder::new(log_dir, "scopeql")
        .layout(JsonLayout::default())
        .rollover_daily()
        .max_log_files(NonZeroUsize::new(7).unwrap())
        .build()
    else {
        return;
    };

    logforth::starter_log::builder()
        .dispatch(|b| {
            b.filter(RustLogFilterBuilder::from_default_env_or("info").build())
                .append(append)
        })
        .apply();
}

#[derive(Debug)]
struct Error {
    message: String,
    source: Option<anyhow::Error>,
}

impl Error {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    fn set_source(mut self, src: impl Into<anyhow::Error>) -> Self {
        debug_assert!(self.source.is_none(), "the source error has been set");
        self.source = Some(src.into());
        self
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|v| v.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn omitted_input_reads_redirected_stdin() {
        assert_eq!(
            resolve_scopeql_input(None, None, false).unwrap(),
            ScopeQLInput::Stdin
        );
    }

    #[test]
    fn omitted_input_rejects_an_interactive_terminal() {
        let error = resolve_scopeql_input(None, None, true).unwrap_err();

        assert_eq!(
            error.to_string(),
            "missing input; provide statement text, use -f/--file, or pipe ScopeQL through stdin"
        );
    }

    #[test]
    fn dash_is_not_a_stdin_alias() {
        assert_eq!(
            resolve_scopeql_input(Some(PathBuf::from("-")), None, true).unwrap(),
            ScopeQLInput::File(PathBuf::from("-"))
        );
    }

    #[test]
    fn positional_statement_is_used_as_inline_input() {
        assert_eq!(
            resolve_scopeql_input(None, Some("SHOW DATABASES;".to_owned()), true).unwrap(),
            ScopeQLInput::Statement("SHOW DATABASES;".to_owned())
        );
    }

    #[test]
    fn stdin_is_read_as_one_script() {
        let source = read_scopeql_input(
            ScopeQLInput::Stdin,
            Cursor::new("SHOW DATABASES;\nSHOW SCHEMAS;"),
        )
        .unwrap();

        assert_eq!(source, "SHOW DATABASES;\nSHOW SCHEMAS;");
    }
}
