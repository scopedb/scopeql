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
use clap::error::ContextKind;
use clap::error::ContextValue;
use clap::error::ErrorKind;

use crate::client::ScopeQLClient;
use crate::command::OutputFormat;
use crate::global::rt;

#[derive(Debug, Parser)]
#[command(name = "", disable_help_subcommand = true)]
pub struct ReplCommand {
    #[command(subcommand)]
    pub cmd: ReplSubCommand,
}

#[derive(Debug, Subcommand)]
pub enum ReplSubCommand {
    /// Switch connection profile.
    #[command(name = "/connection")]
    Connection(CommandConnection),
    /// Set output format.
    #[command(name = "/format")]
    Format(CommandFormat),
    /// Enable or disable timing display.
    #[command(name = "/timer")]
    Timer(CommandTimer),
    /// Cancel the statement with the given ID.
    #[command(name = "/cancel")]
    Cancel(CommandCancel),
    /// Print help.
    #[command(name = "/help")]
    Help,
}

#[derive(Debug, Parser)]
pub struct CommandConnection {
    /// The connection name to use.
    #[arg(value_name = "NAME")]
    pub name: String,
}

#[derive(Debug, Parser)]
pub struct CommandFormat {
    /// The output format to use.
    #[arg(value_enum, value_name = "FORMAT")]
    pub format: OutputFormat,
}

#[derive(Debug, Parser)]
pub struct CommandTimer {
    /// Enable or disable timing display.
    #[arg(value_enum)]
    pub mode: TimerMode,
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
            Some(Err(err)) => {
                eprintln!("error: failed to cancel statement {statement_id}: {err:?}")
            }
            None => println!("interrupted"),
        }
    }
}

pub fn render_repl_parse_error(err: clap::Error) -> String {
    if matches!(
        err.kind(),
        ErrorKind::DisplayHelp
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
            | ErrorKind::DisplayVersion
    ) {
        return err.render().to_string();
    }

    let mut message = match err.kind() {
        ErrorKind::InvalidValue | ErrorKind::ValueValidation => {
            let arg = context_string(&err, ContextKind::InvalidArg)
                .map(|arg| display_arg(&arg))
                .unwrap_or_else(|| "argument".to_string());

            match context_string(&err, ContextKind::InvalidValue) {
                Some(value) if value.is_empty() => format!("error: missing value for {arg}"),
                Some(value) => format!("error: invalid value {:?} for {arg}", value),
                None => format!("error: invalid value for {arg}"),
            }
        }
        ErrorKind::InvalidSubcommand => {
            let command = context_string(&err, ContextKind::InvalidSubcommand)
                .unwrap_or_else(|| "command".to_string());
            format!("error: unknown command {:?}", command)
        }
        ErrorKind::UnknownArgument => {
            let arg = context_string(&err, ContextKind::InvalidArg)
                .unwrap_or_else(|| "argument".to_string());
            format!("error: unexpected argument {:?}", arg)
        }
        ErrorKind::MissingRequiredArgument => {
            let args = context_values(&err, ContextKind::InvalidArg)
                .into_iter()
                .map(|arg| display_arg(&arg))
                .collect::<Vec<_>>();
            match args.as_slice() {
                [] => "error: missing required argument".to_string(),
                [arg] => format!("error: missing required argument {arg}"),
                args => format!("error: missing required arguments: {}", args.join(", ")),
            }
        }
        ErrorKind::TooManyValues => {
            let value = context_string(&err, ContextKind::InvalidValue)
                .unwrap_or_else(|| "value".to_string());
            let arg = context_string(&err, ContextKind::InvalidArg)
                .map(|arg| display_arg(&arg))
                .unwrap_or_else(|| "argument".to_string());
            format!("error: unexpected value {:?} for {arg}", value)
        }
        ErrorKind::TooFewValues | ErrorKind::WrongNumberOfValues => {
            let arg = context_string(&err, ContextKind::InvalidArg)
                .map(|arg| display_arg(&arg))
                .unwrap_or_else(|| "argument".to_string());
            format!("error: wrong number of values for {arg}")
        }
        ErrorKind::NoEquals => {
            let arg = context_string(&err, ContextKind::InvalidArg)
                .unwrap_or_else(|| "argument".to_string());
            format!("error: {arg} requires '='")
        }
        _ => format!(
            "error: {}",
            err.kind().as_str().unwrap_or("failed to parse command")
        ),
    };

    append_values(
        &mut message,
        "possible values",
        context_values(&err, ContextKind::ValidValue),
    );
    append_values(
        &mut message,
        "available",
        context_values(&err, ContextKind::ValidSubcommand),
    );
    append_suggestion(
        &mut message,
        context_string(&err, ContextKind::SuggestedValue)
            .or_else(|| context_string(&err, ContextKind::SuggestedSubcommand))
            .or_else(|| context_string(&err, ContextKind::SuggestedArg)),
    );
    append_usage(&mut message, context_string(&err, ContextKind::Usage));
    message.push('\n');
    message
}

fn context_string(err: &clap::Error, kind: ContextKind) -> Option<String> {
    let values = context_values(err, kind);
    if values.len() == 1 {
        values.into_iter().next()
    } else {
        None
    }
}

fn context_values(err: &clap::Error, kind: ContextKind) -> Vec<String> {
    err.context()
        .find_map(|(context_kind, value)| {
            (context_kind == kind).then(|| context_value_to_strings(value))
        })
        .unwrap_or_default()
}

fn context_value_to_strings(value: &ContextValue) -> Vec<String> {
    match value {
        ContextValue::None => vec![],
        ContextValue::String(value) => vec![value.clone()],
        ContextValue::Strings(values) => values.clone(),
        ContextValue::StyledStr(value) => vec![value.to_string()],
        ContextValue::StyledStrs(values) => values.iter().map(ToString::to_string).collect(),
        ContextValue::Bool(value) => vec![value.to_string()],
        ContextValue::Number(value) => vec![value.to_string()],
        _ => vec![value.to_string()],
    }
}

fn display_arg(arg: &str) -> String {
    arg.trim()
        .strip_prefix('<')
        .and_then(|arg| arg.strip_suffix('>'))
        .unwrap_or(arg)
        .to_string()
}

fn append_values(message: &mut String, label: &str, values: Vec<String>) {
    if !values.is_empty() {
        message.push_str(&format!("\n{label}: {}", values.join(", ")));
    }
}

fn append_suggestion(message: &mut String, suggestion: Option<String>) {
    if let Some(suggestion) = suggestion.filter(|suggestion| !suggestion.is_empty()) {
        message.push_str(&format!("\nhint: did you mean {:?}?", suggestion));
    }
}

fn append_usage(message: &mut String, usage: Option<String>) {
    let Some(usage) = usage else {
        return;
    };
    let usage = usage.trim();
    if usage.is_empty() {
        return;
    }
    let usage = usage
        .strip_prefix("Usage:")
        .or_else(|| usage.strip_prefix("usage:"))
        .map(str::trim)
        .unwrap_or(usage);

    message.push_str(&format!("\nusage: {usage}"));
}
