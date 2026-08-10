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
/// Run a statement with `scopeql run <STATEMENT>`, a script with
/// `scopeql run -f <FILE>`, or pipe ScopeQL through stdin.
#[derive(Debug, clap::Parser)]
#[command(
    name = "scopeql",
    version,
    long_version = version(),
    styles=styled(),
    subcommand_required = true,
    arg_required_else_help = true,
    verbatim_doc_comment
)]
pub struct Command {
    #[command(subcommand)]
    pub subcommand: Subcommand,
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
    /// Run ScopeQL statements from an argument, file, or stdin.
    #[clap(name = "run")]
    Run(RunArgs),
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

#[derive(Debug, Clone, clap::Args)]
pub struct RunArgs {
    #[clap(flatten)]
    pub args: ExecArgs,

    /// Run ScopeQL statements from a script file.
    #[clap(
        short,
        long,
        value_name = "FILE",
        value_hint = ValueHint::FilePath,
        group = "input"
    )]
    pub file: Option<PathBuf>,

    /// The ScopeQL statement text to run. Use ';' to separate statements.
    #[clap(value_name = "STATEMENT", group = "input")]
    pub statement: Option<String>,

    /// Output format for query results.
    #[clap(long, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Write output to `<file>` instead of stdout.
    #[clap(short = 'o', long = "output", value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub output_file: Option<PathBuf>,
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

#[cfg(test)]
mod tests {
    use clap::Parser;
    use clap::error::ErrorKind;

    use super::*;

    #[test]
    fn run_accepts_a_positional_statement() {
        let command = Command::try_parse_from(["scopeql", "run", "SHOW DATABASES;"]).unwrap();

        let Subcommand::Run(args) = command.subcommand else {
            panic!("expected run command");
        };
        assert_eq!(args.file, None);
        assert_eq!(args.statement.as_deref(), Some("SHOW DATABASES;"));
    }

    #[test]
    fn run_accepts_an_explicit_script_file() {
        let command = Command::try_parse_from(["scopeql", "run", "-f", "queries.scopeql"]).unwrap();

        let Subcommand::Run(args) = command.subcommand else {
            panic!("expected run command");
        };
        assert_eq!(args.file, Some(PathBuf::from("queries.scopeql")));
        assert_eq!(args.statement, None);
    }

    #[test]
    fn run_rejects_a_file_and_inline_statement_together() {
        let error =
            Command::try_parse_from(["scopeql", "run", "-f", "queries.scopeql", "SHOW DATABASES;"])
                .unwrap_err();

        assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn root_command_requires_a_subcommand() {
        let error = Command::try_parse_from(["scopeql"]).unwrap_err();

        assert_eq!(
            error.kind(),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }
}
