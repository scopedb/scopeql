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
use logforth::filter::env_filter::EnvFilterBuilder;

use crate::command::Command;
use crate::command::GenerateTarget;
use crate::command::Subcommand;
use crate::config::Config;
use crate::config::load_config;

mod client;
mod command;
mod config;
mod execute;
mod global;
mod load;
mod output;
mod pretty;
mod repl;
mod tokenizer;
mod version;

fn main() {
    let cmd = Command::parse();

    let args = cmd.args();
    init_logger();

    match cmd.subcommand() {
        None => {
            log::info!("starting interactive repl");
            let config = load_config(args.config_file.clone());
            repl::entrypoint(&config);
        }
        Some(Subcommand::Run {
            format,
            file,
            statement,
            output_file,
        }) => {
            let config = load_config(args.config_file.clone());
            match (file, statement) {
                (Some(file), None) => match std::fs::read_to_string(&file) {
                    Ok(content) => {
                        log::info!("running scopeql statements from file {}", file.display());
                        execute::execute(&config, args.quiet, format, content, output_file);
                    }
                    Err(err) => {
                        let file = file.display();
                        log::error!("failed to read script file {file}: {err}");
                        eprintln!("error: failed to read script file {file}: {err}");
                        std::process::exit(1);
                    }
                },
                (None, Some(statement)) => {
                    log::info!("running scopeql statements from inline input");
                    execute::execute(&config, args.quiet, format, statement, output_file);
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
        Some(Subcommand::Generate {
            target,
            output_file,
        }) => {
            log::info!("generating CLI artifact for target {target:?}");
            let content = match target {
                GenerateTarget::Config => {
                    let config = Config::default();
                    toml::to_string(&config).expect("default config must be always valid")
                }
            };

            if let Some(output) = output_file {
                std::fs::write(&output, content).unwrap_or_else(|err| {
                    let output = output.display();
                    let target = match target {
                        GenerateTarget::Config => "configurations",
                    };
                    panic!("failed to write {target} to {output}: {err}")
                });
            } else {
                println!("{content}");
            }
        }
        Some(Subcommand::Load {
            file,
            transform,
            format,
        }) => {
            log::info!("starting load command for {}", file.display());
            let config = load_config(args.config_file.clone());
            load::load(&config, args.quiet, file, transform, format);
        }
    }
}

fn init_logger() {
    let Some(log_dir) = default_log_dir() else {
        return;
    };

    let file = match FileBuilder::new(log_dir, "scopeql")
        .filename_suffix("log")
        .rollover_daily()
        .max_log_files(NonZeroUsize::new(7).expect("non-zero"))
        .build()
    {
        Ok(file) => file,
        Err(_) => return,
    };

    let filter = EnvFilterBuilder::from_default_env_or("info").build();

    let _ = logforth::starter_log::builder()
        .dispatch(|dispatch| dispatch.filter(filter).append(file))
        .try_apply();
}

fn default_log_dir() -> Option<PathBuf> {
    dirs::cache_dir()
        .map(|dir| dir.join("scopeql").join("logs"))
        .or_else(|| dirs::home_dir().map(|dir| dir.join(".scopeql").join("logs")))
}

#[derive(Debug)]
struct Error {
    message: String,
    source: Option<anyhow::Error>,
}

impl Error {
    fn new(message: String) -> Self {
        Self {
            message,
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
