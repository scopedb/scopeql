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

use clap::Parser;
use logforth::filter::env_filter::EnvFilterBuilder;

use crate::command::Args;
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
mod pretty;
mod repl;
mod tokenizer;
mod version;

fn main() {
    let cmd = Command::parse();

    let Args { config_file, quiet } = cmd.args();
    if !quiet {
        logforth::starter_log::stdout()
            .filter(EnvFilterBuilder::from_default_env_or("info").build())
            .apply();
    }

    match cmd.subcommand() {
        None => {
            let config = load_config(config_file);
            repl::entrypoint(&config);
        }
        Some(Subcommand::Run { files, statements }) => {
            // command definition ensures exactly one of statement or file is provided
            debug_assert!(
                files.is_empty() ^ statements.is_empty(),
                "files: {files:?}, statements: {statements:?}"
            );

            let config = load_config(config_file);
            for stmt in statements {
                execute::execute(&config, stmt);
            }
            for file in files {
                match std::fs::read_to_string(&file) {
                    Ok(content) => execute::execute(&config, content),
                    Err(err) => {
                        let file = file.display();
                        log::error!("failed to read script file {file}: {err}");
                    }
                }
            }
        }
        Some(Subcommand::Generate { target, output }) => {
            let content = match target {
                GenerateTarget::Config => {
                    let config = Config::default();
                    toml::to_string(&config).expect("default config must be always valid")
                }
            };

            if let Some(output) = output {
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
            let config = load_config(config_file);
            load::load(&config, file, transform, format);
        }
    }
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
