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

use anyhow::Error;
use anyhow::anyhow;
use dialoguer::Confirm;
use dialoguer::Input;
use dialoguer::Password;
use dialoguer::Select;
use serde::Deserialize;
use serde::Serialize;
use serde::de::IntoDeserializer;
use toml_edit::Array;
use toml_edit::DocumentMut;
use toml_edit::Item;
use toml_edit::Table;

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
            .and_then(|path| path.strip_suffix("_auth"))
        {
            set_toml_path(
                config,
                &["connections", name, "auth"],
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

fn deserialize_toml(path: &Path, doc: DocumentMut) -> Result<Config, Error> {
    Config::deserialize(doc.into_deserializer())
        .map_err(|err| anyhow!("failed to deserialize config on {}: {err}", path.display()))
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
                    auth: Some(ConnectionAuthSpec::Direct),
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

    #[serde(flatten)]
    #[serde(default)]
    auth: Option<ConnectionAuthSpec>,
}

impl ConnectionSpec {
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn headers(&self) -> &[String] {
        &self.headers
    }

    pub fn auth(&self) -> &ConnectionAuthSpec {
        self.auth.as_ref().unwrap_or(&ConnectionAuthSpec::Direct)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "auth")]
pub enum ConnectionAuthSpec {
    #[serde(rename = "direct")]
    Direct,
    #[serde(rename = "api_key")]
    ApiKey { api_key: String },
}

impl ConnectionAuthSpec {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::ApiKey { .. } => "api_key",
        }
    }
}

pub(crate) fn get_connections(name: Option<&str>) {
    internal_get_connections(name).unwrap_or_else(|err| {
        eprintln!("Failed to get connections: {err}");
        std::process::exit(1);
    });
}

fn internal_get_connections(name: Option<&str>) -> Result<(), Error> {
    let (path, doc) = load_document()?;
    let config = deserialize_toml(&path, doc)?;

    if config.connections.is_empty() {
        println!("No connections configured.");
        return Ok(());
    }

    let connections = if let Some(name) = name {
        let conn = config
            .connections
            .get(name)
            .ok_or_else(|| anyhow!("Connection '{name}' not found in config."))?;
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
    Ok(())
}

pub(crate) fn use_connection(name: &str) {
    internal_use_connection(name).unwrap_or_else(|err| {
        eprintln!("Failed to switch connection: {err}");
        std::process::exit(1);
    });
}

fn internal_use_connection(name: &str) -> Result<(), Error> {
    let (path, mut doc) = load_document()?;

    let config = deserialize_toml(&path, doc.clone())?;

    if !config.connections.contains_key(name) {
        return Err(anyhow!("Connection '{name}' not found."));
    }

    set_toml_path(&mut doc, &["default_connection"], toml_edit::value(name));

    std::fs::write(&path, doc.to_string())
        .map_err(|err| anyhow!("failed to write config file {}: {err}", path.display()))?;

    println!("Switched to connection '{name}'");
    Ok(())
}

pub(crate) fn set_connection(name: String) {
    let (path, doc) = load_document().unwrap_or_else(|_err| {
        let path = candidate_config_paths()
            .into_iter()
            .next()
            .expect("no candidate config paths");
        println!("Creating new config file at {}", path.display());

        let parent = path.parent().unwrap();
        if let Err(err) = std::fs::create_dir_all(parent) {
            eprintln!(
                "failed to create config directory {}: {err}",
                parent.display()
            );
            std::process::exit(1);
        }

        let mut doc = DocumentMut::new();
        doc["default_connection"] = toml_edit::value(&name);
        (path, doc)
    });

    internal_set_connection(name, path, doc).unwrap_or_else(|err| {
        eprintln!("Failed to set connection: {err}");
        std::process::exit(1);
    });
}

fn internal_set_connection(name: String, path: PathBuf, mut doc: DocumentMut) -> Result<(), Error> {
    let mut config = deserialize_toml(&path, doc.clone())?;

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

        conn.auth = Some(prompt_existing_auth(conn.auth.as_ref()));
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
            auth: Some(auth),
        }
    };

    write_connection(&mut doc, &name, &conn);

    std::fs::write(&path, doc.to_string())
        .map_err(|err| anyhow!("failed to write config file {}: {err}", path.display()))?;

    println!("Set connection '{name}' in {}", path.display());
    Ok(())
}

fn prompt_existing_auth(current: Option<&ConnectionAuthSpec>) -> ConnectionAuthSpec {
    let current = current.unwrap_or(&ConnectionAuthSpec::Direct);
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
        2 => prompt_auth_by_kind(Some(current)),
        _ => unreachable!("dialoguer returned an unknown auth action"),
    }
}

fn prompt_auth_fields(current: &ConnectionAuthSpec) -> ConnectionAuthSpec {
    match current {
        ConnectionAuthSpec::Direct => ConnectionAuthSpec::Direct,
        ConnectionAuthSpec::ApiKey { .. } => ConnectionAuthSpec::ApiKey {
            api_key: prompt_api_key(),
        },
    }
}

fn prompt_auth_by_kind(default: Option<&ConnectionAuthSpec>) -> ConnectionAuthSpec {
    let kinds = ["direct", "api_key"];
    let default = match default {
        None | Some(ConnectionAuthSpec::Direct) => 0,
        Some(ConnectionAuthSpec::ApiKey { .. }) => 1,
    };

    let selection = Select::new()
        .with_prompt("Auth type")
        .items(kinds)
        .default(default)
        .interact()
        .expect("failed to read auth type");

    match selection {
        0 => ConnectionAuthSpec::Direct,
        1 => ConnectionAuthSpec::ApiKey {
            api_key: prompt_api_key(),
        },
        _ => unreachable!("dialoguer returned an unknown auth type"),
    }
}

fn prompt_api_key() -> String {
    Password::new()
        .with_prompt("API key")
        .interact()
        .expect("failed to read API key")
}

fn write_connection(doc: &mut DocumentMut, name: &str, conn: &ConnectionSpec) {
    let connections = doc
        .as_table_mut()
        .entry("connections")
        .or_insert(Item::Table({
            let mut t = Table::new();
            t.set_implicit(true);
            t
        }));
    if !connections.is_table() {
        let mut t = Table::new();
        t.set_implicit(true);
        *connections = Item::Table(t);
    }
    let connections = connections.as_table_mut().unwrap();

    let table = connections
        .entry(name)
        .or_insert(Item::Table(toml_edit::Table::new()));
    let table = table.as_table_mut().unwrap();

    table["endpoint"] = toml_edit::value(&conn.endpoint);
    if conn.headers.is_empty() {
        table.remove("headers");
    } else {
        let mut headers = Array::default();
        for header in &conn.headers {
            headers.push(header.as_str());
        }
        table["headers"] = toml_edit::value(headers);
    }

    match conn.auth() {
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
    internal_delete_connection(name).unwrap_or_else(|err| {
        eprintln!("Failed to delete connection: {err}");
        std::process::exit(1);
    });
}

fn internal_delete_connection(name: &str) -> Result<(), Error> {
    let (path, mut doc) = load_document()?;

    let config = deserialize_toml(&path, doc.clone())?;

    if !config.connections.contains_key(name) {
        return Err(anyhow!("Connection '{name}' not found."));
    }

    if config.default_connection == name {
        let Some(other) = config.connections.keys().find(|k| *k != name).cloned() else {
            return Err(anyhow!("Cannot delete the only connection."));
        };
        set_toml_path(&mut doc, &["default_connection"], toml_edit::value(&other));
        println!("Switched to connection '{other}'");
    }

    doc["connections"].as_table_mut().unwrap().remove(name);

    std::fs::write(&path, doc.to_string())
        .map_err(|err| anyhow!("failed to write config file {}: {err}", path.display()))?;

    println!("Deleted connection '{name}' from {}", path.display());
    Ok(())
}

fn load_document() -> Result<(PathBuf, DocumentMut), Error> {
    let candidates = candidate_config_paths();
    let path = candidates
        .iter()
        .find(|path| path.exists())
        .ok_or_else(|| {
            let paths = candidates
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            anyhow!("config file does not exist in any of [{}]", paths)
        })?;

    let content = std::fs::read_to_string(path)
        .map_err(|err| anyhow!("failed to read config file {}: {err}", path.display()))?;

    let doc = DocumentMut::from_str(&content)
        .map_err(|err| anyhow!("failed to parse config file {}: {err}", path.display()))?;

    log::info!("loaded config from {}", path.display());
    Ok((path.clone(), doc))
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
auth = "direct"
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

    #[test]
    fn test_default_auth() {
        let config: Config = toml::from_str(
            r#"
default_connection = "local"

[connections.local]
endpoint = "http://127.0.0.1:9999"
# headers and api_key will be ignored because auth type is direct by default
api_key = "ignored-api-key"
"#,
        )
        .unwrap();

        let conn = config.get_default_connection().unwrap();
        assert_eq!(conn.endpoint(), "http://127.0.0.1:9999");
        assert_matches!(conn.auth(), ConnectionAuthSpec::Direct);
    }
}
