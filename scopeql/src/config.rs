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
use std::path::Path;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use serde::de::IntoDeserializer;
use toml_edit::DocumentMut;

pub fn load_config<P: AsRef<Path>>(config_file: Option<P>) -> Config {
    // Layer 0: the config file
    let content = if let Some(file) = config_file.as_ref().map(AsRef::as_ref) {
        std::fs::read_to_string(file).unwrap_or_else(|err| {
            let file = file.display();
            panic!("failed to read config file {file}: {err}")
        })
    } else {
        let mut candidates = vec![];
        if let Some(home_dir) = dirs::home_dir() {
            candidates.push(home_dir.join(".scopeql").join("config.toml"));
            candidates.push(home_dir.join(".config").join("scopeql").join("config.toml"));
        }
        if let Some(config_dir) = dirs::config_dir() {
            candidates.push(config_dir.join("scopeql").join("config.toml"));
        }
        candidates.sort();
        candidates.dedup();

        candidates
            .into_iter()
            .find_map(|candidate| std::fs::read_to_string(candidate).ok())
            .unwrap_or_else(|| {
                toml::to_string(&Config::default()).expect("failed to serialize default config")
            })
    };

    let mut config = DocumentMut::from_str(&content)
        .unwrap_or_else(|err| panic!("failed to parse config content: {err}"));

    apply_env_overrides(&mut config, std::env::vars());

    Config::deserialize(config.into_deserializer()).expect("failed to deserialize config")
}

fn apply_env_overrides(config: &mut DocumentMut, env: impl IntoIterator<Item = (String, String)>) {
    for (key, value) in env {
        let normalized_key = key.trim().to_lowercase();
        let Some(path) = normalized_key.strip_prefix("scopeql_config_") else {
            continue;
        };

        if path == "default_connection" {
            set_toml_path(config, &["default_connection"], toml_edit::value(value));
            continue;
        }

        if let Some(name) = path
            .strip_prefix("connections_")
            .and_then(|path| path.strip_suffix("_endpoint"))
        {
            set_toml_path(
                config,
                &["connections", name, "endpoint"],
                toml_edit::value(value),
            );
            continue;
        }

        if let Some(name) = path
            .strip_prefix("connections_")
            .and_then(|path| path.strip_suffix("_api_key"))
        {
            set_toml_path(
                config,
                &["connections", name, "api_key"],
                toml_edit::value(value),
            );
            continue;
        }

        log::warn!("ignore unknown environment variable {path} with value {value}");
    }
}

fn set_toml_path(doc: &mut DocumentMut, parts: &[&str], value: toml_edit::Item) {
    let mut current = doc.as_item_mut();

    let len = parts.len();
    assert!(len > 0, "path must not be empty");

    for part in parts.iter().take(len - 1) {
        current = &mut current[part];
    }

    current[parts[len - 1]] = value;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    default_connection: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    connections: BTreeMap<String, ConnectionSpec>,
}

impl Config {
    pub fn get_connection(&self, name: &str) -> Option<&ConnectionSpec> {
        self.connections.get(name)
    }

    pub fn get_default_connection(&self) -> Option<&ConnectionSpec> {
        self.get_connection(&self.default_connection)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_connection: "default".to_string(),
            connections: BTreeMap::from([(
                "default".to_string(),
                ConnectionSpec {
                    endpoint: "http://127.0.0.1:6543".to_string(),
                    api_key: None,
                },
            )]),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConnectionSpec {
    endpoint: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key: Option<String>,
}

impl ConnectionSpec {
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn api_key(&self) -> Option<&str> {
        self.api_key
            .as_deref()
            .filter(|api_key| !api_key.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_api_key_ignores_empty_string() {
        let connection = ConnectionSpec {
            endpoint: "http://127.0.0.1:6543".to_string(),
            api_key: Some(String::new()),
        };

        assert_eq!(connection.api_key(), None);
    }

    #[test]
    fn config_deserializes_connection_api_key() {
        let config: Config = toml::from_str(
            r#"
default_connection = "default"

[connections.default]
endpoint = "http://127.0.0.1:6543"
api_key = "test-api-key"
"#,
        )
        .unwrap();

        assert_eq!(
            config
                .get_default_connection()
                .and_then(ConnectionSpec::api_key),
            Some("test-api-key")
        );
    }

    #[test]
    fn env_overrides_can_set_connection_api_key() {
        let content = toml::to_string(&Config::default()).unwrap();
        let mut doc = DocumentMut::from_str(&content).unwrap();

        apply_env_overrides(
            &mut doc,
            [(
                "SCOPEQL_CONFIG_CONNECTIONS_DEFAULT_API_KEY".to_string(),
                "test-api-key".to_string(),
            )],
        );

        let config = Config::deserialize(doc.into_deserializer()).unwrap();
        assert_eq!(
            config
                .get_default_connection()
                .and_then(ConnectionSpec::api_key),
            Some("test-api-key")
        );
    }
}
