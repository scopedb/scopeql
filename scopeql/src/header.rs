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

use std::str::FromStr;

use reqwest::header::HeaderMap;
use reqwest::header::HeaderName;
use reqwest::header::HeaderValue;

use crate::Error;

/// Parse a header string in the format "KEY: VALUE" into HeaderName and HeaderValue.
fn parse_header(s: &str) -> Result<(HeaderName, HeaderValue), Error> {
    let (key, value) = s
        .split_once(':')
        .ok_or_else(|| Error::new(format!("invalid header {s:?}; expected 'KEY: VALUE'")))?;

    let key = HeaderName::from_str(key.trim()).map_err(|err| {
        Error::new(format!("invalid header name {:?}", key.trim())).set_source(err)
    })?;

    let value = HeaderValue::from_str(value.trim()).map_err(|err| {
        Error::new(format!("invalid header value {:?}", value.trim())).set_source(err)
    })?;

    Ok((key, value))
}

/// Parse multiple header strings.
pub fn parse_headers(headers: &[String]) -> Result<HeaderMap, Error> {
    let mut map = HeaderMap::new();
    for h in headers {
        let (key, value) = parse_header(h)?;
        map.insert(key, value);
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_header_trims_parts() {
        let (key, value) = parse_header("  X-Test  :  value  ").unwrap();
        assert_eq!(key, "x-test");
        assert_eq!(value, "value");
    }

    #[test]
    fn test_parse_header_missing_colon() {
        let result = parse_header("X-Test value");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expected 'KEY: VALUE'")
        );
    }

    #[test]
    fn test_parse_headers_multiple() {
        let headers = vec![
            "X-Test: value1".to_string(),
            "X-Another: value2".to_string(),
        ];
        let map = parse_headers(&headers).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("x-test").unwrap(), "value1");
        assert_eq!(map.get("x-another").unwrap(), "value2");
    }
}
