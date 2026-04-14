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
#[command(multicall = true)]
pub struct ReplCommand {
    #[command(subcommand)]
    pub cmd: ReplSubCommand,
}

#[derive(Debug, Subcommand)]
pub enum ReplSubCommand {
    /// Cancel the statement with the given ID.
    #[command(name = "cancel")]
    Cancel(CommandCancel),
    /// Set output format (table, json, csv, jsonl).
    #[command(name = "mode")]
    Mode(CommandMode),
    /// Toggle timing display (on/off).
    #[command(name = "timer")]
    Timer(CommandTimer),
}

#[derive(Debug, Parser)]
pub struct CommandMode {
    /// The output format to use.
    #[arg(value_enum, value_name = "FORMAT")]
    pub format: OutputFormat,
}

#[derive(Debug, Parser)]
pub struct CommandTimer {
    /// Enable or disable timing display.
    #[arg(value_enum, value_name = "on|off")]
    pub toggle: TimerToggle,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum TimerToggle {
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
