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

use fastrace_reqwest::traceparent_headers;
use reqwest::IntoUrl;
use reqwest::Url;
use reqwest::header::AUTHORIZATION;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use uuid::Uuid;

use crate::Error;
use crate::client::protocol::IngestRequest;
use crate::client::protocol::IngestResult;
use crate::client::protocol::Response;
use crate::client::protocol::StatementRequest;
use crate::client::protocol::StatementRequestParams;
use crate::client::protocol::StatementStatus;

#[derive(Debug, Clone)]
pub struct Client {
    endpoint: Url,
    client: reqwest::Client,
    authorization: Option<HeaderValue>,
    extra_headers: HeaderMap,
}

impl Client {
    pub(super) fn new<E: IntoUrl>(
        endpoint: E,
        client: reqwest::Client,
        api_key: Option<String>,
        extra_headers: HeaderMap,
    ) -> Result<Self, Error> {
        let authorization = match api_key.filter(|api_key| !api_key.is_empty()) {
            Some(api_key) => Some(HeaderValue::from_str(&format!("Bearer {api_key}")).map_err(
                |err| Error::new("failed to build authorization header").set_source(err),
            )?),
            None => None,
        };

        match endpoint.into_url() {
            Ok(endpoint) => Ok(Self {
                endpoint,
                client,
                authorization,
                extra_headers,
            }),
            Err(err) => Err(Error::new("failed to parse endpoint").set_source(err)),
        }
    }

    #[fastrace::trace]
    pub async fn submit_statement(
        &self,
        request: StatementRequest,
    ) -> Result<Response<StatementStatus>, Error> {
        let url = self.make_url("v1/statements")?;
        let response = self
            .client
            .post(url)
            .headers(self.request_headers())
            .json(&request)
            .send()
            .await
            .map_err(|err| {
                Error::new(format!("failed to submit statement: {request:?}")).set_source(err)
            })?;
        Response::from_http_response(response).await
    }

    #[fastrace::trace]
    pub async fn fetch_statement(
        &self,
        statement_id: Uuid,
        params: StatementRequestParams,
    ) -> Result<Response<StatementStatus>, Error> {
        let path = format!("v1/statements/{statement_id}");
        let url = self.make_url(&path)?;
        let response = self
            .client
            .get(url)
            .headers(self.request_headers())
            .query(&params)
            .send()
            .await
            .map_err(|err| {
                Error::new(format!("failed to fetch statement {statement_id:?}")).set_source(err)
            })?;
        Response::from_http_response(response).await
    }

    #[fastrace::trace]
    pub async fn ingest(&self, request: IngestRequest) -> Result<Response<IngestResult>, Error> {
        let format = request.data.format();
        let url = self.make_url("v1/ingest")?;
        let response = self
            .client
            .post(url)
            .headers(self.request_headers())
            .json(&request)
            .send()
            .await
            .map_err(|err| {
                Error::new(format!("failed to ingest data in {format}")).set_source(err)
            })?;
        Response::from_http_response(response).await
    }

    #[track_caller]
    fn make_url(&self, path: &str) -> Result<Url, Error> {
        self.endpoint
            .join(path)
            .map_err(|err| Error::new("failed to construct URL").set_source(err))
    }

    fn request_headers(&self) -> HeaderMap {
        let mut headers = traceparent_headers();
        if let Some(authorization) = &self.authorization {
            headers.insert(AUTHORIZATION, authorization.clone());
        }
        for (key, value) in &self.extra_headers {
            headers.insert(key, value.clone());
        }
        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_headers_include_bearer_token() {
        let client = Client::new(
            "http://127.0.0.1:6543",
            reqwest::Client::new(),
            Some("test-api-key".to_string()),
            HeaderMap::new(),
        )
        .unwrap();

        let headers = client.request_headers();
        assert_eq!(headers.get(AUTHORIZATION).unwrap(), "Bearer test-api-key");
    }

    #[test]
    fn request_headers_omit_authorization_without_api_key() {
        let client = Client::new(
            "http://127.0.0.1:6543",
            reqwest::Client::new(),
            None,
            HeaderMap::new(),
        )
        .unwrap();

        let headers = client.request_headers();
        assert!(headers.get(AUTHORIZATION).is_none());
    }

    #[test]
    fn request_headers_include_extra_headers() {
        let mut extra_headers = HeaderMap::new();
        extra_headers.insert("x-tenant", HeaderValue::from_static("acme"));
        let client = Client::new(
            "http://127.0.0.1:6543",
            reqwest::Client::new(),
            None,
            extra_headers,
        )
        .unwrap();

        let headers = client.request_headers();
        assert_eq!(headers.get("x-tenant").unwrap(), "acme");
    }
}
