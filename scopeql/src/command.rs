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

use std::path::PathBuf;

use clap::ValueHint;

use crate::config;
use crate::load::DataFormat;
use crate::version::version;

/// ScopeDB command-line interface.
///
/// This CLI executes ScopeQL statements. For language syntax and examples, see:
///
///   Docs: https://docs.scopedb.io/
///   Quickstart: https://docs.scopedb.io/guides/quickstart
///   Reference: https://docs.scopedb.io/reference/
///
/// If no command is specified, an interactive REPL starts.
#[derive(Debug, clap::Parser)]
#[command(
    name = "scopeql",
    version,
    long_version = version(),
    styles=styled(),
    args_conflicts_with_subcommands = true,
    verbatim_doc_comment
)]
pub struct Command {
    #[clap(flatten)]
    pub repl_args: ReplArgs,

    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,
}

/// Arguments for the REPL.
#[derive(Default, Debug, Clone, clap::Args)]
pub struct ReplArgs {
    /// Run `scopeql` with the given config file.
    #[clap(long, value_hint = ValueHint::FilePath, value_name = "FILE")]
    pub config_file: Option<PathBuf>,
}

/// Shared arguments for commands that execute scopeql statements.
#[derive(Default, Debug, Clone, clap::Args)]
pub struct ExecArgs {
    /// Run `scopeql` with the given config file.
    #[clap(long, value_hint = ValueHint::FilePath, value_name = "FILE")]
    pub config_file: Option<PathBuf>,

    /// Suppress normal output.
    #[clap(short, long, alias = "silent", default_value = "false")]
    pub quiet: bool,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Csv,
    Jsonl,
}

impl OutputFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::Json => "json",
            Self::Csv => "csv",
            Self::Jsonl => "jsonl",
        }
    }
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Subcommand {
    /// Run scopeql statements.
    #[clap(name = "run")]
    Run {
        #[clap(flatten)]
        args: ExecArgs,
        /// The scopeql script file to run. May contain multiple top-level statements.
        #[clap(group = "input", short, long, value_hint = ValueHint::FilePath)]
        file: Option<PathBuf>,
        /// Output format for query results.
        #[clap(long, value_enum, default_value = "table")]
        format: OutputFormat,
        /// Write output to `<file>` instead of stdout.
        #[clap(short = 'o', long = "output", value_name = "file", value_hint = ValueHint::FilePath)]
        output_file: Option<PathBuf>,
        /// The statement text to run. Use ';' to separate multiple statements.
        #[clap(group = "input", value_name = "STATEMENT")]
        statement: Option<String>,
    },
    /// Perform a load operation of source with transformations.
    #[clap(name = "load")]
    Load {
        #[clap(flatten)]
        args: ExecArgs,
        /// The file path to load the source from.
        #[clap(short, long, value_hint = ValueHint::FilePath)]
        file: PathBuf,
        /// The transformation to apply during the load.
        #[clap(short, long)]
        transform: String,
        /// The source data format.
        #[clap(long, value_enum)]
        format: Option<DataFormat>,
    },
    /// Manage connections.
    #[clap(name = "connection")]
    Connection {
        #[command(subcommand)]
        cmd: ConnectionCommand,
    },
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum ConnectionCommand {
    /// List configured connections.
    #[clap(name = "list")]
    List,
    /// Set the default connection.
    #[clap(name = "default")]
    Default {
        /// The name of the connection to use.
        #[clap(value_name = "CONNECTION_NAME")]
        name: Option<String>,
    },
    /// Add a connection.
    #[clap(name = "add")]
    Add,
    /// Delete the specified connection.
    #[clap(name = "remove")]
    Remove {
        /// The name of the connection to delete.
        #[clap(value_name = "CONNECTION_NAME")]
        name: String,
    },
}

impl ConnectionCommand {
    pub fn run(self) {
        match self {
            ConnectionCommand::List => config::list_connections(),
            ConnectionCommand::Default { name } => match name {
                Some(name) => config::set_default_connection(name.as_str()),
                None => config::show_default_connection(),
            },
            ConnectionCommand::Add => config::add_connection(),
            ConnectionCommand::Remove { name } => config::remove_connection(name.as_str()),
        }
    }
}

fn styled() -> clap::builder::Styles {
    use anstyle::AnsiColor;
    use anstyle::Color;
    use anstyle::Style;

    let default = Style::new();
    let bold = default.bold();
    let bold_underline = bold.underline();

    clap::builder::Styles::styled()
        .usage(bold_underline.fg_color(Some(Color::Ansi(AnsiColor::BrightGreen))))
        .header(bold_underline.fg_color(Some(Color::Ansi(AnsiColor::BrightGreen))))
        .valid(bold_underline.fg_color(Some(Color::Ansi(AnsiColor::Green))))
        .literal(bold.fg_color(Some(Color::Ansi(AnsiColor::BrightCyan))))
        .invalid(bold.fg_color(Some(Color::Ansi(AnsiColor::Red))))
        .error(bold.fg_color(Some(Color::Ansi(AnsiColor::Red))))
        .placeholder(default.fg_color(Some(Color::Ansi(AnsiColor::Cyan))))
}
