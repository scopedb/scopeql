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
use std::path::PathBuf;
use std::str::FromStr;

use dialoguer::Confirm;
use dialoguer::Input;
use dialoguer::Select;
use serde::Deserialize;
use serde::Serialize;
use serde::de::IntoDeserializer;
use toml_edit::DocumentMut;

pub const DEFAULT_URL: &str = "http://127.0.0.1:6543";

fn candidate_config_paths() -> Vec<PathBuf> {
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
}

pub fn load_config<P: AsRef<Path>>(config_file: Option<P>) -> Config {
    // Layer 0: the config file
    let content = if let Some(file) = config_file.as_ref().map(AsRef::as_ref) {
        match std::fs::read_to_string(file) {
            Ok(content) => {
                log::info!("loaded config from {}", file.display());
                content
            }
            Err(err) => {
                panic!("failed to read config file {}: {err}", file.display());
            }
        }
    } else {
        let found = candidate_config_paths().into_iter().find_map(|candidate| {
            std::fs::read_to_string(&candidate)
                .ok()
                .map(|content| (candidate, content))
        });
        match found {
            Some((path, content)) => {
                log::info!("loaded config from {}", path.display());
                content
            }
            None => {
                log::info!("no config file exists in candidate paths, using default config");
                toml::to_string(&Config::default()).expect("failed to serialize default config")
            }
        }
    };

    let mut config = DocumentMut::from_str(&content)
        .unwrap_or_else(|err| panic!("failed to parse config content: {err}"));

    // Layer 1: environment variables
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
            .and_then(|path| path.strip_suffix("_headers"))
        {
            set_toml_path(
                config,
                &["connections", name, "headers"],
                toml_edit::value(parse_env_headers(&value)),
            );
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

fn parse_env_headers(value: &str) -> toml_edit::Array {
    let mut headers = toml_edit::Array::default();
    for header in value.lines().map(str::trim).filter(|line| !line.is_empty()) {
        headers.push(header);
    }
    headers
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
                    endpoint: DEFAULT_URL.to_string(),
                    headers: vec![],
                    auth: ConnectionAuthSpec::Direct,
                },
            )]),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectionSpec {
    endpoint: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    headers: Vec<String>,

    #[serde(default, flatten)]
    auth: ConnectionAuthSpec,
}

impl ConnectionSpec {
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn headers(&self) -> &[String] {
        &self.headers
    }

    pub fn auth(&self) -> &ConnectionAuthSpec {
        &self.auth
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(tag = "auth")]
pub enum ConnectionAuthSpec {
    #[default]
    #[serde(rename = "direct")]
    Direct,
    #[serde(rename = "api_key")]
    ApiKey { api_key: String },
}

impl ConnectionAuthSpec {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::ApiKey { .. } => "api_Key",
        }
    }
}

pub(crate) fn get_connections(name: Option<&str>) {
    let (_path, doc) = load_document();
    let config =
        Config::deserialize(doc.into_deserializer()).expect("failed to deserialize config");

    if config.connections.is_empty() {
        println!("No connections configured.");
        return;
    }

    let connections = if let Some(name) = name {
        let conn = config.connections.get(name).unwrap_or_else(|| {
            panic!("Connection '{name}' not found in config.");
        });
        vec![(name, conn)]
    } else {
        config
            .connections
            .iter()
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    };

    let mut table = comfy_table::Table::new();
    table.load_preset(comfy_table::presets::NOTHING);
    table.set_header(["DEFAULT", "NAME", "ENDPOINT", "AUTH", "HEADERS"]);
    for (name, conn) in connections {
        let mut row = vec![];
        if name == config.default_connection.as_str() {
            row.push("*".to_string());
        } else {
            row.push("".to_string());
        }
        row.push(name.to_string());
        row.push(conn.endpoint.to_string());
        row.push(conn.auth().kind().to_string());
        let mut headers = String::new();
        for (i, header) in conn.headers.iter().enumerate() {
            if i > 0 {
                headers.push('\n');
            }
            headers.push_str(header);
        }
        row.push(headers);
        table.add_row(row);
    }

    println!("{table}");
}

pub(crate) fn use_connection(name: &str) {
    let (path, mut doc) = load_document();

    let config =
        Config::deserialize(doc.clone().into_deserializer()).expect("failed to deserialize config");

    if !config.connections.contains_key(name) {
        eprintln!("Connection '{name}' not found.");
        std::process::exit(1);
    }

    set_toml_path(&mut doc, &["default_connection"], toml_edit::value(name));

    std::fs::write(&path, doc.to_string()).unwrap_or_else(|err| {
        panic!("failed to write config file {}: {err}", path.display());
    });

    println!("Switched to connection '{name}'");
}

pub(crate) fn set_connection(name: String) {
    let (path, mut doc) = load_document();

    let mut config =
        Config::deserialize(doc.clone().into_deserializer()).expect("failed to deserialize config");

    let conn = if let Some(conn) = config.connections.get_mut(&name) {
        if Confirm::new()
            .with_prompt("Change endpoint?")
            .default(false)
            .interact()
            .expect("failed to read endpoint confirmation")
        {
            conn.endpoint = Input::new()
                .with_prompt("Endpoint")
                .default(conn.endpoint.clone())
                .interact_text()
                .expect("failed to read endpoint");
        }

        conn.auth = prompt_existing_auth(&conn.auth);
        conn.clone()
    } else {
        let endpoint = Input::new()
            .with_prompt("Endpoint")
            .default(DEFAULT_URL.to_string())
            .interact_text()
            .expect("failed to read endpoint");
        let auth = prompt_auth_by_kind(None);

        ConnectionSpec {
            endpoint,
            headers: vec![],
            auth,
        }
    };

    write_connection(&mut doc, &name, &conn);

    std::fs::write(&path, doc.to_string()).unwrap_or_else(|err| {
        panic!("failed to write config file {}: {err}", path.display());
    });

    println!("Set connection '{name}' in {}", path.display());
}

fn prompt_existing_auth(current: &ConnectionAuthSpec) -> ConnectionAuthSpec {
    let actions = [
        "Keep current auth",
        "Modify current auth fields",
        "Enter auth type",
    ];

    let action = Select::new()
        .with_prompt(format!("Auth [{}]", current.kind()))
        .items(actions)
        .default(0)
        .interact()
        .expect("failed to read auth action");

    match action {
        0 => current.clone(),
        1 => prompt_auth_fields(current),
        2 => prompt_auth_by_kind(Some(current.kind())),
        _ => unreachable!("dialoguer returned an unknown auth action"),
    }
}

fn prompt_auth_fields(current: &ConnectionAuthSpec) -> ConnectionAuthSpec {
    match current {
        ConnectionAuthSpec::Direct => ConnectionAuthSpec::Direct,
        ConnectionAuthSpec::ApiKey { .. } => ConnectionAuthSpec::ApiKey {
            api_key: Input::new()
                .with_prompt("API key")
                .interact_text()
                .expect("failed to read API key"),
        },
    }
}

fn prompt_auth_by_kind(default: Option<&str>) -> ConnectionAuthSpec {
    loop {
        let mut prompt = Input::new().with_prompt("Auth type");
        if let Some(default) = default {
            prompt = prompt.default(default.to_string());
        }

        let auth_type = prompt.interact_text().expect("failed to read auth type");

        match auth_type.trim() {
            "direct" => return ConnectionAuthSpec::Direct,
            "api_key" => {
                return ConnectionAuthSpec::ApiKey {
                    api_key: Input::new()
                        .with_prompt("API key")
                        .interact_text()
                        .expect("failed to read API key"),
                };
            }
            auth_type => {
                eprintln!("Unsupported auth type '{auth_type}'. Use 'direct' or 'api_key'.")
            }
        }
    }
}

fn write_connection(doc: &mut DocumentMut, name: &str, conn: &ConnectionSpec) {
    if !doc["connections"].is_table() {
        doc["connections"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    if doc["connections"].get(name).is_none() {
        doc["connections"][name] = toml_edit::Item::Table(toml_edit::Table::new());
    }

    let table = doc["connections"][name]
        .as_table_mut()
        .expect("connection should be a TOML table");

    table["endpoint"] = toml_edit::value(&conn.endpoint);
    if conn.headers.is_empty() {
        table.remove("headers");
    } else {
        let mut headers = toml_edit::Array::default();
        for header in &conn.headers {
            headers.push(header.as_str());
        }
        table["headers"] = toml_edit::value(headers);
    }

    match &conn.auth {
        ConnectionAuthSpec::Direct => {
            table["auth"] = toml_edit::value("direct");
            table.remove("api_key");
        }
        ConnectionAuthSpec::ApiKey { api_key } => {
            table["auth"] = toml_edit::value("api_key");
            table["api_key"] = toml_edit::value(api_key);
        }
    }
}

pub(crate) fn delete_connection(name: &str) {
    let (path, mut doc) = load_document();

    let config =
        Config::deserialize(doc.clone().into_deserializer()).expect("failed to deserialize config");

    if !config.connections.contains_key(name) {
        eprintln!("Connection '{name}' not found.");
        std::process::exit(1);
    }

    if config.default_connection == name {
        let Some(other) = config.connections.keys().find(|k| *k != name).cloned() else {
            eprintln!("Cannot delete the only connection.");
            std::process::exit(1);
        };
        set_toml_path(&mut doc, &["default_connection"], toml_edit::value(&other));
        println!("Switched to connection '{other}'");
    }

    doc["connections"].as_table_mut().unwrap().remove(name);

    std::fs::write(&path, doc.to_string()).unwrap_or_else(|err| {
        panic!("failed to write config file {}: {err}", path.display());
    });

    println!("Deleted connection '{name}' from {}", path.display());
}

fn load_document() -> (PathBuf, DocumentMut) {
    let path = candidate_config_paths()
        .into_iter()
        .find(|path| path.exists())
        .unwrap_or_else(|| {
            eprintln!("No config file found. Run `scopeql config add-connection` to create one.");
            std::process::exit(1);
        });

    let content = std::fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("failed to read config file {}: {err}", path.display());
    });

    let doc = DocumentMut::from_str(&content).unwrap_or_else(|err| {
        panic!("failed to parse config file {}: {err}", path.display());
    });

    (path, doc)
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;

    #[test]
    fn config_deserializes_connection_api_key() {
        let config: Config = toml::from_str(
            r#"
default_connection = "default"

[connections.default]
endpoint = "http://127.0.0.1:6543"
auth = "api_key"
api_key = "test-api-key"
"#,
        )
        .unwrap();

        let conn = config.get_default_connection().unwrap();
        assert_matches!(
            conn.auth().clone(),
            ConnectionAuthSpec::ApiKey { api_key } if api_key == "test-api-key"
        );
    }

    #[test]
    fn config_deserializes_connection_headers() {
        let config: Config = toml::from_str(
            r#"
default_connection = "default"

[connections.default]
endpoint = "http://127.0.0.1:6543"
headers = ["X-Tenant: acme"]
"#,
        )
        .unwrap();

        assert_eq!(
            config
                .get_default_connection()
                .map(ConnectionSpec::headers)
                .unwrap_or_default(),
            ["X-Tenant: acme"]
        );
    }

    #[test]
    fn env_overrides_can_set_connection_api_key() {
        let content = toml::to_string(&Config::default()).unwrap();
        let mut doc = DocumentMut::from_str(&content).unwrap();

        apply_env_overrides(
            &mut doc,
            [
                (
                    "SCOPEQL_CONFIG_CONNECTIONS_DEFAULT_AUTH".to_string(),
                    "api_key".to_string(),
                ),
                (
                    "SCOPEQL_CONFIG_CONNECTIONS_DEFAULT_API_KEY".to_string(),
                    "test-api-key".to_string(),
                ),
            ],
        );

        let config = Config::deserialize(doc.into_deserializer()).unwrap();
        let conn = config.get_default_connection().unwrap();
        assert_matches!(
            conn.auth().clone(),
            ConnectionAuthSpec::ApiKey { api_key } if api_key == "test-api-key"
        );
    }

    #[test]
    fn env_overrides_can_set_connection_headers() {
        let content = toml::to_string(&Config::default()).unwrap();
        let mut doc = DocumentMut::from_str(&content).unwrap();

        apply_env_overrides(
            &mut doc,
            [(
                "SCOPEQL_CONFIG_CONNECTIONS_DEFAULT_HEADERS".to_string(),
                "X-Tenant: acme".to_string(),
            )],
        );

        let config = Config::deserialize(doc.into_deserializer()).unwrap();
        assert_eq!(
            config
                .get_default_connection()
                .map(ConnectionSpec::headers)
                .unwrap_or_default(),
            ["X-Tenant: acme"]
        );
    }

    #[test]
    fn env_overrides_can_set_multiple_connection_headers() {
        let content = toml::to_string(&Config::default()).unwrap();
        let mut doc = DocumentMut::from_str(&content).unwrap();

        apply_env_overrides(
            &mut doc,
            [(
                "SCOPEQL_CONFIG_CONNECTIONS_DEFAULT_HEADERS".to_string(),
                "X-Tenant: acme\nX-Trace: demo".to_string(),
            )],
        );

        let config = Config::deserialize(doc.into_deserializer()).unwrap();
        assert_eq!(
            config
                .get_default_connection()
                .map(ConnectionSpec::headers)
                .unwrap_or_default(),
            ["X-Tenant: acme", "X-Trace: demo"]
        );
    }
}
