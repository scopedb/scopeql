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

#![feature(string_from_utf8_lossy_owned)]

use std::num::NonZeroUsize;
use std::path::PathBuf;

use clap::Parser;
use logforth::append::file::FileBuilder;
use logforth::filter::rustlog::RustLogFilterBuilder;
use logforth::layout::JsonLayout;

use crate::command::Command;
use crate::command::ExecArgs;
use crate::command::ReplArgs;
use crate::command::Subcommand;
use crate::config::Config;
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
mod repl;
mod tokenizer;
mod version;

fn main() {
    let cmd = Command::parse();
    setup_logger();

    match cmd.subcommand {
        None => {
            log::info!("starting interactive repl");
            let ReplArgs { config_file } = cmd.repl_args;
            let config = match load_config(config_file) {
                Ok(Some(config)) if config.has_default_connection() => config,
                Ok(_) => config::create_first_connection().unwrap_or_else(|err| {
                    eprintln!("error: failed to create config: {err}");
                    std::process::exit(1);
                }),
                Err(err) => {
                    eprintln!("error: failed to load config: {err}");
                    std::process::exit(1);
                }
            };
            repl::entrypoint(&config);
        }
        Some(Subcommand::Run {
            args: ExecArgs { config_file, quiet },
            format,
            file,
            statement,
            output_file,
        }) => {
            let config = load_required_config(config_file);
            match (file, statement) {
                (Some(file), None) => match std::fs::read_to_string(&file) {
                    Ok(content) => {
                        log::info!("running scopeql statements from file {}", file.display());
                        execute::execute(&config, quiet, format, content, output_file);
                    }
                    Err(err) => {
                        let file = file.display();
                        eprintln_and_error(format_args!(
                            "failed to read script file {file}: {err}"
                        ));
                        std::process::exit(1);
                    }
                },
                (None, Some(statement)) => {
                    log::info!("running scopeql statements from inline input");
                    execute::execute(&config, quiet, format, statement, output_file);
                }
                (None, None) => {
                    eprintln!("error: missing input; provide statement text or use -f/--file");
                    std::process::exit(1);
                }
                (Some(_), Some(_)) => {
                    eprintln!("error: provide either a statement or -f/--file, not both");
                    std::process::exit(1);
                }
            }
        }
        Some(Subcommand::Load {
            args: ExecArgs { config_file, quiet },
            file,
            transform,
            format,
        }) => {
            log::info!("starting load command for {}", file.display());
            let config = load_required_config(config_file);
            load::load(&config, quiet, file, transform, format);
        }
        Some(Subcommand::Connection { cmd }) => cmd.run(),
    }
}

fn load_required_config(config_file: Option<PathBuf>) -> Config {
    let config = match load_config(config_file) {
        Ok(Some(config)) => config,
        Ok(None) => {
            eprintln!(
                "error: no ScopeQL connection configured; run `scopeql connection add` to create one"
            );
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("error: failed to load config: {err}");
            std::process::exit(1);
        }
    };

    if !config.has_default_connection() {
        eprintln!(
            "error: no default connection configured; run `scopeql connection add` to create one"
        );
        std::process::exit(1);
    }

    config
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
