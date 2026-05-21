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

use crate::config::DEFAULT_URL;
use crate::load::DataFormat;
use crate::version::version;

/// ScopeDB Command Line Interface
///
/// If no command is specified, an interactive REPL will be started.
#[derive(Debug, clap::Parser)]
#[command(
    name = "scopeql",
    version,
    long_version = version(),
    styles=styled(),
    args_conflicts_with_subcommands = true
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
    /// Manage the config file
    #[clap(name = "config")]
    Config {
        #[command(subcommand)]
        cmd: ConfigSubcommand,
    },
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum ConfigSubcommand {
    /// List all configured connections.
    #[clap(name = "list-connections")]
    List,
    /// Switch to a different connection by name.
    #[clap(name = "use-connection")]
    Use {
        /// The name of the connection to use.
        name: String,
    },
    /// Add a new connection.
    #[clap(name = "add-connection")]
    Add {
        /// The name of the new connection.
        name: String,
        /// The ScopeDB endpoint URL.
        #[clap(long, default_value = DEFAULT_URL)]
        url: String,
        /// The API key for authentication.
        #[clap(long)]
        api_key: Option<String>,
        /// Additional headers (key=value, comma-separated).
        #[clap(long)]
        headers: Option<String>,

        /// Prompt for values interactively instead of using CLI options.
        #[clap(long, default_value = "false")]
        prompt: bool,
    },
    /// Delete a connection by name.
    #[clap(name = "delete-connection")]
    Delete {
        /// The name of the connection to delete.
        name: String,
    },
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
