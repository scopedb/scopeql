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

use std::time::Duration;

use exn::Result;
use exn::ResultExt;
use exn::bail;
use uuid::Uuid;

use crate::Error;
use crate::client::connection::Client;
use crate::client::protocol::IngestData;
use crate::client::protocol::IngestRequest;
use crate::client::protocol::IngestResult;
use crate::client::protocol::Response;
use crate::client::protocol::ResultFormat;
use crate::client::protocol::StatementCancelResult;
use crate::client::protocol::StatementEstimatedProgress;
use crate::client::protocol::StatementRequest;
use crate::client::protocol::StatementRequestParams;
use crate::client::protocol::StatementStatus;
use crate::command::OutputFormat;
use crate::config::ConnectionConfig;
use crate::output::format_result_set;

mod connection;
pub(crate) mod protocol;
pub(crate) mod result;

#[derive(Debug)]
pub struct ScopeQLClient {
    client: Client,
}

impl ScopeQLClient {
    pub fn from_connection(connection: &ConnectionConfig) -> Self {
        let client = reqwest::ClientBuilder::new()
            .no_proxy()
            .build()
            .expect("failed to create HTTP client");

        let endpoint = connection.endpoint().to_owned();
        let api_key = connection.api_key().map(str::to_owned);
        let headers = crate::header::parse_headers(connection.headers())
            .unwrap_or_else(|err| panic!("invalid headers in config: {err}"));

        ScopeQLClient {
            client: Client::new(endpoint, client, api_key, headers).unwrap(),
        }
    }

    pub async fn load_jsonlines(
        &self,
        jsonlines: String,
        transform: String,
    ) -> Result<IngestResult, Error> {
        let data = IngestData::Json { rows: jsonlines };
        let format = data.format();
        let make_error = || Error::new(format!("failed to load {format} data: {transform}"));

        match self
            .client
            .ingest(IngestRequest {
                data,
                statement: transform.clone(),
            })
            .await
            .or_raise(make_error)?
        {
            Response::Success(result) => Ok(result),
            Response::Failed(err) => {
                Err(Error::new(format!("fail to insert data: {err}"))).or_raise(make_error)
            }
        }
    }

    pub async fn execute_statement(
        &self,
        statement_id: Uuid,
        statement: String,
        output_format: OutputFormat,
        show_timing: bool,
        display_progress: impl Fn(&'static str, StatementEstimatedProgress),
    ) -> Result<String, Error> {
        let make_error = || {
            Error::new(format!(
                "failed to execute statement ({statement_id}): {statement}"
            ))
        };

        let start_time = jiff::Timestamp::now();
        display_progress("Submitting", StatementEstimatedProgress::default());

        let mut status = match self
            .client
            .submit_statement(StatementRequest {
                statement: statement.clone(),
                statement_id: Some(statement_id),
                exec_timeout: None,
                params: StatementRequestParams {
                    format: ResultFormat::Json,
                },
            })
            .await
            .or_raise(make_error)?
        {
            Response::Success(status) => status,
            Response::Failed(err) => {
                bail!(Error::new(format!("failed to submit statement: {err}")));
            }
        };

        loop {
            match status {
                StatementStatus::Pending(s) => {
                    display_progress("Pending", s.progress.clone());
                }
                StatementStatus::Running(s) => {
                    display_progress("Running", s.progress.clone());
                }
                StatementStatus::Finished(s) => {
                    let elapsed = start_time.duration_until(jiff::Timestamp::now());
                    return format_result_set(
                        s.result_set(),
                        elapsed,
                        s.progress.clone(),
                        output_format,
                        show_timing,
                    );
                }
                StatementStatus::Failed(s) => {
                    bail!(Error::new(format!("statement failed: {}", s.message)));
                }
                StatementStatus::Cancelled(s) => {
                    bail!(Error::new(format!("statement cancelled: {}", s.message)));
                }
            }

            const DEFAULT_FETCH_INTERVAL: Duration = Duration::from_millis(42);
            tokio::time::sleep(DEFAULT_FETCH_INTERVAL).await;

            status = match self
                .client
                .fetch_statement(
                    statement_id,
                    StatementRequestParams {
                        format: ResultFormat::Json,
                    },
                )
                .await
                .or_raise(make_error)?
            {
                Response::Success(status) => status,
                Response::Failed(err) => {
                    bail!(Error::new(format!("failed to fetch statement: {err}")));
                }
            }
        }
    }

    pub async fn cancel_statement(
        &self,
        statement_id: Uuid,
    ) -> Result<StatementCancelResult, Error> {
        match self.client.cancel_statement(statement_id).await? {
            Response::Success(response) => Ok(response),
            Response::Failed(err) => {
                bail!(Error::new(format!("failed to cancel statement: {err}")));
            }
        }
    }
}
