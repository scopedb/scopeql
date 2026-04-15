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

use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use csv::ReaderBuilder;
use exn::Result;
use exn::ResultExt;
use reqwest::header::HeaderMap;

use crate::Error;
use crate::client::ScopeQLClient;
use crate::config::Config;
use crate::global;
use crate::global::eprintln_and_error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum DataFormat {
    Csv,
    Json,
}

pub fn load(
    config: &Config,
    quiet: bool,
    file: PathBuf,
    transform: String,
    format: Option<DataFormat>,
    headers: HeaderMap,
) {
    let connection = config
        .get_default_connection()
        .expect("no default connection in config");
    let mut client = ScopeQLClient::from_connection(connection);
    for (key, value) in headers {
        if let Some(key) = key {
            client.set_header(key, value);
        }
    }

    let format = match format {
        Some(format) => format,
        None => match file.extension().and_then(|s| s.to_str()) {
            Some("json") => DataFormat::Json,
            Some("csv") => DataFormat::Csv,
            _ => {
                eprintln_and_error(format_args!("unknown data file format: {}", file.display()));
                eprintln!("please specify the format using the --format option");
                std::process::exit(1);
            }
        },
    };
    log::info!("loading {} as {:?}", file.display(), format);

    let content = match format {
        DataFormat::Csv => load_csv_data(file),
        DataFormat::Json => load_json_data(file),
    };

    let data = match content {
        Ok(rows) => rows,
        Err(err) => {
            eprintln_and_error(format_args!("failed to load source data: {err:?}"));
            std::process::exit(1);
        }
    };

    let result = global::rt().block_on(client.load_jsonlines(data, transform));
    match result {
        Ok(result) => {
            log::info!(
                "load completed with {} inserted rows",
                result.num_rows_inserted
            );
            if !quiet {
                match result.num_rows_inserted {
                    0 => println!("no rows were inserted"),
                    1 => println!("successfully inserted 1 row"),
                    n => println!("successfully inserted {n} rows"),
                }
            }
        }
        Err(err) => {
            eprintln_and_error(format_args!("failed to load data: {err:?}"));
            std::process::exit(1);
        }
    }
}

fn load_csv_data(file: PathBuf) -> Result<String, Error> {
    let make_error = || {
        Error::new(format!(
            "failed to load csv data from file: {}",
            file.display()
        ))
    };

    let file = File::open(&file).or_raise(make_error)?;
    let reader = BufReader::new(file);
    let mut reader = ReaderBuilder::new().has_headers(true).from_reader(reader);

    let mut data = String::new();
    for result in reader.records() {
        let mut row = BTreeMap::new();
        let record = result.or_raise(make_error)?;
        for (i, field) in record.iter().enumerate() {
            row.insert(format!("col_{i}"), field.to_string());
        }
        write!(&mut data, "{}", serde_json::to_string(&row).unwrap()).unwrap();
    }
    Ok(data)
}

fn load_json_data(file: PathBuf) -> Result<String, Error> {
    let make_error = || {
        Error::new(format!(
            "failed to load json data from file: {}",
            file.display()
        ))
    };

    let file = File::open(&file).or_raise(make_error)?;
    let reader = BufReader::new(file);
    let reader = serde_json::Deserializer::from_reader(reader);

    let mut data = String::new();
    for row in reader.into_iter::<serde_json::Value>() {
        let row = row.or_raise(make_error)?;
        write!(&mut data, "{}", serde_json::to_string(&row).unwrap()).unwrap();
    }
    Ok(data)
}
