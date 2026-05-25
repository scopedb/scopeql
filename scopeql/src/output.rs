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

use std::fmt::Write;

use exn::Result;
use exn::ResultExt;
use jiff::SignedDuration;
use nu_ansi_term::Color;

use crate::Error;
use crate::client::protocol::StatementEstimatedProgress;
use crate::client::result::ResultSet;
use crate::client::result::Value;
use crate::command::OutputFormat;
use crate::pretty::pretty_print;

pub fn format_result_set(
    result_set: ResultSet,
    duration: SignedDuration,
    progress: StatementEstimatedProgress,
    format: OutputFormat,
    show_timing: bool,
) -> Result<String, Error> {
    match format {
        OutputFormat::Table => format_table(result_set, duration, progress, show_timing),
        OutputFormat::Json => format_json(result_set),
        OutputFormat::Csv => format_csv(result_set),
        OutputFormat::Jsonl => format_jsonl(result_set),
    }
}

fn format_table(
    result_set: ResultSet,
    duration: SignedDuration,
    progress: StatementEstimatedProgress,
    show_timing: bool,
) -> Result<String, Error> {
    let num_rows = match result_set.num_rows() {
        n @ 0..=1 => format!("({n} row)"),
        n => format!("({n} rows)"),
    };

    let header = result_set
        .schema()
        .fields()
        .iter()
        .map(|f| f.name().to_string())
        .collect::<Vec<_>>();

    let rows = result_set
        .into_values()
        .or_raise(|| Error::new("failed to convert result rows"))?;

    const TABLE_STYLE_PRESET: &str = "||--+-++|    ++++++";
    let mut table = comfy_table::Table::new();
    table.load_preset(TABLE_STYLE_PRESET);
    table.set_header(header);
    for row in rows {
        let row = row
            .into_iter()
            .map(|value| match value {
                Value::Null
                | Value::Int(_)
                | Value::UInt(_)
                | Value::Float(_)
                | Value::Timestamp(_)
                | Value::Interval(_)
                | Value::Boolean(_)
                | Value::Binary(_) => value.to_string(),
                Value::String(value) => value,
                Value::Array(value) | Value::Object(value) | Value::Any(value) => {
                    const MAX_COMPACT_LEN: usize = 64;
                    if value.len() > MAX_COMPACT_LEN {
                        pretty_print(&value)
                    } else {
                        value
                    }
                }
            })
            .collect::<Vec<_>>();
        table.add_row(row);
    }

    if !show_timing {
        return Ok(format!("{table}\n{num_rows}"));
    }

    let queue_secs =
        SignedDuration::from_nanos(progress.nanos_from_submitted - progress.nanos_from_started);
    let run_secs = SignedDuration::from_nanos(progress.nanos_from_started);
    let total_secs = duration;

    let queue_secs = Color::LightCyan.paint(format!("{:.3}s", queue_secs.as_secs_f64()));
    let run_secs = Color::LightCyan.paint(format!("{:.3}s", run_secs.as_secs_f64()));
    let total_secs = Color::LightCyan.paint(format!("{:.3}s", total_secs.as_secs_f64()));

    let queue = Color::LightGreen.paint("queue");
    let run = Color::LightGreen.paint("run");
    let total = Color::LightGreen.paint("total");

    Ok(format!(
        "{table}\n{num_rows}\ntime: {queue_secs} {queue} {run_secs} {run} {total_secs} {total}",
    ))
}

fn format_json(result_set: ResultSet) -> Result<String, Error> {
    let fields = result_set
        .schema()
        .fields()
        .iter()
        .map(|field| field.name().to_string())
        .collect::<Vec<_>>();
    let rows = result_set
        .into_values()
        .or_raise(|| Error::new("failed to convert result rows"))?;

    let json_rows = rows
        .into_iter()
        .map(|row| {
            let mut object = serde_json::Map::new();
            for (field, value) in fields.iter().zip(row) {
                object.insert(field.clone(), value_to_json(value));
            }
            serde_json::Value::Object(object)
        })
        .collect::<Vec<_>>();

    serde_json::to_string_pretty(&json_rows)
        .map_err(|err| Error::new(format!("failed to serialize JSON: {err}")).into())
}

fn format_csv(result_set: ResultSet) -> Result<String, Error> {
    let fields = result_set
        .schema()
        .fields()
        .iter()
        .map(|field| field.name().to_string())
        .collect::<Vec<_>>();
    let rows = result_set
        .into_values()
        .or_raise(|| Error::new("failed to convert result rows"))?;

    let mut output = String::new();
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        write_csv_field(&mut output, field);
    }
    output.push('\n');

    for row in rows {
        for (index, value) in row.into_iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            write_csv_field(&mut output, &value.to_string());
        }
        output.push('\n');
    }

    trim_trailing_newlines(&mut output);
    Ok(output)
}

fn format_jsonl(result_set: ResultSet) -> Result<String, Error> {
    let fields = result_set
        .schema()
        .fields()
        .iter()
        .map(|field| field.name().to_string())
        .collect::<Vec<_>>();
    let rows = result_set
        .into_values()
        .or_raise(|| Error::new("failed to convert result rows"))?;

    let mut output = String::new();
    for row in rows {
        let mut object = serde_json::Map::new();
        for (field, value) in fields.iter().zip(row) {
            object.insert(field.clone(), value_to_json(value));
        }
        let line = serde_json::to_string(&serde_json::Value::Object(object))
            .map_err(|err| Error::new(format!("failed to serialize JSONL: {err}")))?;
        writeln!(&mut output, "{line}").unwrap();
    }

    trim_trailing_newlines(&mut output);
    Ok(output)
}

fn trim_trailing_newlines(output: &mut String) {
    while matches!(output.as_bytes().last(), Some(b'\n' | b'\r')) {
        output.pop();
    }
}

fn value_to_json(value: Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Int(value) => serde_json::Value::Number(value.into()),
        Value::UInt(value) => serde_json::Value::Number(value.into()),
        Value::Float(value) => serde_json::Number::from_f64(value)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::Boolean(value) => serde_json::Value::Bool(value),
        Value::String(value) => serde_json::Value::String(value),
        value @ (Value::Timestamp(_) | Value::Interval(_) | Value::Binary(_)) => {
            serde_json::Value::String(value.to_string())
        }
        Value::Array(value) | Value::Object(value) | Value::Any(value) => {
            serde_json::from_str(&value).unwrap_or(serde_json::Value::String(value))
        }
    }
}

fn write_csv_field(output: &mut String, field: &str) {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        output.push('"');
        for ch in field.chars() {
            if ch == '"' {
                output.push_str("\"\"");
            } else {
                output.push(ch);
            }
        }
        output.push('"');
    } else {
        output.push_str(field);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_field_escaping() {
        let cases = [
            ("hello", "hello"),
            ("a,b", "\"a,b\""),
            ("say \"hi\"", "\"say \"\"hi\"\"\""),
            ("line1\nline2", "\"line1\nline2\""),
            ("line1\rline2", "\"line1\rline2\""),
            ("value\r", "\"value\r\""),
            ("", ""),
        ];

        for (input, expected) in cases {
            let mut actual = String::new();
            write_csv_field(&mut actual, input);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn json_value_conversion() {
        assert_eq!(value_to_json(Value::Null), serde_json::Value::Null);
        assert_eq!(value_to_json(Value::Int(42)), serde_json::json!(42));
        assert_eq!(value_to_json(Value::UInt(100)), serde_json::json!(100));
        assert_eq!(value_to_json(Value::Float(3.5)), serde_json::json!(3.5));
        assert_eq!(value_to_json(Value::Boolean(true)), serde_json::json!(true));
        assert_eq!(
            value_to_json(Value::String("hello".into())),
            serde_json::json!("hello")
        );
        assert_eq!(
            value_to_json(Value::Object(r#"{"a":1}"#.into())),
            serde_json::json!({ "a": 1 })
        );
        assert_eq!(
            value_to_json(Value::Object("not json".into())),
            serde_json::json!("not json")
        );
    }

    #[test]
    fn trim_trailing_newlines_preserves_trailing_spaces() {
        let mut output = "value with spaces   \n".to_string();
        trim_trailing_newlines(&mut output);
        assert_eq!(output, "value with spaces   ");

        let mut output = "value with tabs\t\r\n".to_string();
        trim_trailing_newlines(&mut output);
        assert_eq!(output, "value with tabs\t");

        let mut output = "\"value\r\"\n".to_string();
        trim_trailing_newlines(&mut output);
        assert_eq!(output, "\"value\r\"");
    }
}
