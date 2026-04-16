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

use clap::Parser;
use clap::Subcommand;

use crate::client::ScopeQLClient;
use crate::command::OutputFormat;
use crate::global::rt;

#[derive(Debug, Parser)]
#[command(multicall = true, disable_help_subcommand = true)]
pub struct ReplCommand {
    #[command(subcommand)]
    pub cmd: ReplSubCommand,
}

#[derive(Debug, Subcommand)]
pub enum ReplSubCommand {
    /// Cancel the statement with the given ID.
    #[command(name = "/cancel")]
    Cancel(CommandCancel),
    /// Display or set output format.
    #[command(name = "/format")]
    Format(CommandFormat),
    /// Display or manage extra headers for requests.
    #[command(name = "/headers")]
    Headers(CommandHeaders),
    /// Display or set the timing display mode.
    #[command(name = "/timer")]
    Timer(CommandTimer),
    /// Print help.
    #[command(name = "/help", alias = "/?")]
    Help,
}

#[derive(Debug, Parser)]
pub struct CommandHeaders {
    #[command(subcommand)]
    pub action: Option<HeadersAction>,
}

#[derive(Debug, Subcommand)]
pub enum HeadersAction {
    /// Set a header, overwriting any existing value for the same key.
    #[command(name = "set")]
    Set(HeadersSet),
    /// Unset a header by key.
    #[command(name = "unset")]
    Unset(HeadersUnset),
    /// Unset all extra headers.
    #[command(name = "unsetall")]
    UnsetAll,
}

#[derive(Debug, Parser)]
pub struct HeadersSet {
    /// The header to set in 'KEY: VALUE' format.
    #[arg(value_name = "KEY: VALUE")]
    pub header: String,
}

#[derive(Debug, Parser)]
pub struct HeadersUnset {
    /// The header key to remove.
    #[arg(value_name = "KEY")]
    pub key: String,
}

#[derive(Debug, Parser)]
pub struct CommandFormat {
    /// The output format to use; if not specified, show the current format.
    #[arg(value_enum, value_name = "FORMAT")]
    pub format: Option<OutputFormat>,
}

#[derive(Debug, Parser)]
pub struct CommandTimer {
    /// Enable or disable timing display; if not specified, show the current mode.
    #[arg(value_enum)]
    pub mode: Option<TimerMode>,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum TimerMode {
    On,
    Off,
}

#[derive(Debug, Parser)]
pub struct CommandCancel {
    /// The ID of the statement to cancel.
    #[arg(value_name = "STATEMENT_ID")]
    pub statement_id: String,
}

impl CommandCancel {
    pub fn run(self, client: &ScopeQLClient) {
        let statement_id = &self.statement_id;
        let statement_id = match uuid::Uuid::try_parse(statement_id) {
            Ok(statement_id) => statement_id,
            Err(err) => {
                eprintln!("error: invalid statement id {statement_id:?}: {err}");
                return;
            }
        };

        let output = rt().block_on(async move {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => None,
                output = client.cancel_statement(statement_id) => Some(output),
            }
        });

        match output {
            Some(Ok(result)) => println!("{}", serde_json::to_string_pretty(&result).unwrap()),
            Some(Err(err)) => eprintln!("error: failed to cancel statement {statement_id}: {err}"),
            None => println!("interrupted"),
        }
    }
}
