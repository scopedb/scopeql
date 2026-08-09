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

use dialoguer::Input;
use dialoguer::Password;
use dialoguer::Select;
use serde::Deserialize;
use serde::Serialize;
use serde::de::IntoDeserializer;
use toml_edit::DocumentMut;

use crate::Error;

const FIRST_CONNECTION_NAME: &str = "default";

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

pub fn try_load_config<P: AsRef<Path>>(config_file: Option<P>) -> Result<Option<Config>, Error> {
    let loaded = if let Some(file) = config_file.as_ref().map(AsRef::as_ref) {
        Some((file.to_path_buf(), read_config_document(file)?))
    } else {
        let mut found = None;
        for path in candidate_config_paths() {
            if !path.exists() {
                continue;
            }
            let doc = read_config_document(&path)?;
            found = Some((path, doc));
            break;
        }
        found
    };

    let Some((path, mut doc)) = loaded else {
        log::info!("no config file exists in candidate paths");
        let mut doc = DocumentMut::new();
        apply_env_overrides(&mut doc, std::env::vars());
        return Ok(Config::deserialize(doc.into_deserializer())
            .ok()
            .filter(Config::has_default_connection));
    };

    apply_env_overrides(&mut doc, std::env::vars());
    Ok(Some(deserialize_toml(&path, doc)?))
}

pub fn load_config<P: AsRef<Path>>(config_file: Option<P>) -> Config {
    let config = match try_load_config(config_file) {
        Ok(Some(config)) => config,
        Ok(None) => {
            eprintln!(
                "error: no ScopeQL connection configured; run `scopeql connection add` to create one"
            );
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("error: failed to load config: {err}");
            std::process::exit(1);
        }
    };

    if !config.has_default_connection() {
        eprintln!(
            "error: no default connection configured; run `scopeql connection add` to create one"
        );
        std::process::exit(1);
    }

    config
}

fn read_config_document(path: &Path) -> Result<DocumentMut, Error> {
    let content = std::fs::read_to_string(path).map_err(|err| {
        Error::new(format!(
            "failed to read config file {}: {err}",
            path.display()
        ))
    })?;

    let doc = DocumentMut::from_str(&content).map_err(|err| {
        Error::new(format!(
            "failed to parse config file {}: {err}",
            path.display()
        ))
    })?;

    log::info!("loaded config from {}", path.display());
    Ok(doc)
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
    let (last, parents) = parts.split_last().expect("path must not be empty");

    for part in parents {
        current = &mut current[part];
    }

    current[last] = value;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    default_connection: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    connections: BTreeMap<String, ConnectionSpec>,
}

impl Config {
    pub fn has_default_connection(&self) -> bool {
        self.get_default_connection().is_some()
    }

    pub fn get_connection(&self, name: &str) -> Option<&ConnectionSpec> {
        self.connections.get(name)
    }

    pub fn get_default_connection(&self) -> Option<&ConnectionSpec> {
        self.default_connection
            .as_deref()
            .and_then(|name| self.get_connection(name))
    }
}

fn deserialize_toml(path: &Path, doc: DocumentMut) -> Result<Config, Error> {
    Config::deserialize(doc.into_deserializer()).map_err(|err| {
        Error::new(format!(
            "failed to deserialize config on {}: {err}",
            path.display()
        ))
    })
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConnectionSpec {
    endpoint: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    headers: Vec<String>,

    #[serde(flatten)]
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

pub fn list_connections() {
    do_list_connections().unwrap_or_else(|err| {
        eprintln!("Failed to list connections: {err}");
        std::process::exit(1);
    });
}

fn do_list_connections() -> Result<(), Error> {
    let Some((path, doc)) = load_document()? else {
        println!("No connections configured.");
        return Ok(());
    };
    let config = deserialize_toml(&path, doc)?;

    if config.connections.is_empty() {
        println!("No connections configured.");
        return Ok(());
    }

    let connections = config
        .connections
        .iter()
        .map(|(name, conn)| (name.as_str(), conn))
        .collect::<Vec<_>>();
    print_connection_table(config.default_connection.as_deref(), &connections);
    Ok(())
}

pub fn set_default_connection(name: &str) {
    do_set_default_connection(name).unwrap_or_else(|err| {
        eprintln!("Failed to set default connection: {err}");
        std::process::exit(1);
    });
}

pub fn show_default_connection() {
    do_show_default_connection().unwrap_or_else(|err| {
        eprintln!("Failed to show default connection: {err}");
        std::process::exit(1);
    });
}

fn do_show_default_connection() -> Result<(), Error> {
    let Some((path, doc)) = load_document()? else {
        println!("No default connection configured.");
        return Ok(());
    };
    let config = deserialize_toml(&path, doc)?;

    let Some(name) = config.default_connection.as_deref() else {
        println!("No default connection configured.");
        return Ok(());
    };

    let Some(conn) = config.get_connection(name) else {
        println!("No default connection configured.");
        return Ok(());
    };

    print_connection_table(Some(name), &[(name, conn)]);
    Ok(())
}

fn print_connection_table(
    default_connection: Option<&str>,
    connections: &[(&str, &ConnectionSpec)],
) {
    let mut table = comfy_table::Table::new();
    table.load_preset(comfy_table::presets::NOTHING);
    table.set_header(["DEFAULT", "NAME", "ENDPOINT", "AUTH", "HEADERS"]);

    for (name, conn) in connections {
        let default = if Some(*name) == default_connection {
            "*"
        } else {
            ""
        };

        table.add_row(vec![
            default.to_string(),
            name.to_string(),
            conn.endpoint().to_string(),
            conn.auth().kind().to_string(),
            conn.headers().join("\n"),
        ]);
    }

    println!("{table}");
}

fn do_set_default_connection(name: &str) -> Result<(), Error> {
    let Some((path, mut doc)) = load_document()? else {
        return Err(Error::new(
            "no config file found; run `scopeql connection add` to create one",
        ));
    };
    let config = deserialize_toml(&path, doc.clone())?;

    if !config.connections.contains_key(name) {
        return Err(Error::new(format!("Connection '{name}' not found.")));
    }

    set_toml_path(&mut doc, &["default_connection"], toml_edit::value(name));

    std::fs::write(&path, doc.to_string()).map_err(|err| {
        Error::new(format!(
            "failed to write config file {}: {err}",
            path.display()
        ))
    })?;

    println!("Default connection is set to '{name}'");
    Ok(())
}

pub fn add_connection() {
    let (path, doc) = match load_document() {
        Ok(Some((path, doc))) => (path, doc),
        Ok(None) => {
            let (path, doc) = new_config_document().unwrap_or_else(|err| {
                eprintln!("Failed to create config file: {err}");
                std::process::exit(1);
            });
            println!("Creating new config file at {}", path.display());
            (path, doc)
        }
        Err(err) => {
            eprintln!("Failed to add connection: {err}");
            std::process::exit(1);
        }
    };
    let name = prompt_connection_name(FIRST_CONNECTION_NAME);
    if doc
        .get("connections")
        .and_then(toml_edit::Item::as_table)
        .is_some_and(|connections| connections.contains_key(&name))
    {
        eprintln!("Failed to add connection: Connection '{name}' already exists.");
        std::process::exit(1);
    }

    let conn = prompt_connection_spec();
    add_connection_to_doc(name, path, doc, conn).unwrap_or_else(|err| {
        eprintln!("Failed to add connection: {err}");
        std::process::exit(1);
    });
}

fn prompt_connection_name(default: &str) -> String {
    Input::new()
        .with_prompt("Connection name")
        .default(default.to_string())
        .interact_text()
        .expect("failed to read connection name")
}

fn prompt_connection_spec() -> ConnectionSpec {
    let connection_types = ["API Key", "Direct"];
    let mode = Select::new()
        .with_prompt("Connection type")
        .items(connection_types)
        .default(0)
        .interact()
        .expect("failed to read connection type");

    let endpoint = Input::new()
        .with_prompt("Endpoint")
        .interact_text()
        .expect("failed to read endpoint");

    let auth = match mode {
        0 => ConnectionAuthSpec::ApiKey {
            api_key: Password::new()
                .with_prompt("API Key")
                .interact()
                .expect("failed to read API key"),
        },
        1 => ConnectionAuthSpec::Direct,
        _ => unreachable!("dialoguer returned an unknown connection type"),
    };

    ConnectionSpec {
        endpoint,
        headers: vec![],
        auth,
    }
}

fn add_connection_to_doc(
    name: String,
    path: PathBuf,
    mut doc: DocumentMut,
    conn: ConnectionSpec,
) -> Result<Config, Error> {
    let connections = doc.get("connections").and_then(toml_edit::Item::as_table);
    if connections.is_some_and(|connections| connections.contains_key(&name)) {
        return Err(Error::new(format!("Connection '{name}' already exists.")));
    }

    if !connections.is_some_and(|connections| connections.iter().next().is_some()) {
        doc["default_connection"] = toml_edit::value(&name);
    }
    write_connection(&mut doc, &name, &conn);

    std::fs::write(&path, doc.to_string()).map_err(|err| {
        Error::new(format!(
            "failed to write config file {}: {err}",
            path.display()
        ))
    })?;

    let config = deserialize_toml(&path, doc)?;
    println!("Added connection '{name}' in {}", path.display());
    Ok(config)
}

fn write_connection(doc: &mut DocumentMut, name: &str, conn: &ConnectionSpec) {
    let connections = doc
        .as_table_mut()
        .entry("connections")
        .or_insert(toml_edit::Item::Table({
            let mut t = toml_edit::Table::new();
            t.set_implicit(true);
            t
        }));
    if !connections.is_table() {
        let mut t = toml_edit::Table::new();
        t.set_implicit(true);
        *connections = toml_edit::Item::Table(t);
    }
    let connections = connections.as_table_mut().unwrap();

    let table = connections
        .entry(name)
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
    let table = table.as_table_mut().unwrap();

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

pub fn remove_connection(name: &str) {
    do_remove_connection(name).unwrap_or_else(|err| {
        eprintln!("Failed to delete connection: {err}");
        std::process::exit(1);
    });
}

fn do_remove_connection(name: &str) -> Result<(), Error> {
    let Some((path, mut doc)) = load_document()? else {
        return Err(Error::new(
            "no config file found; run `scopeql connection add` to create one",
        ));
    };
    let config = deserialize_toml(&path, doc.clone())?;

    if !config.connections.contains_key(name) {
        return Err(Error::new(format!("Connection '{name}' not found.")));
    }

    let fallback = config.connections.keys().find(|k| *k != name).cloned();

    doc["connections"].as_table_mut().unwrap().remove(name);
    match fallback {
        Some(other) if config.default_connection.as_deref() == Some(name) => {
            set_toml_path(&mut doc, &["default_connection"], toml_edit::value(&other));
            println!("Switched to connection '{other}'");
        }
        None => {
            doc.as_table_mut().remove("default_connection");
            println!("No connections remain. Run `scopeql connection add` to create one.");
        }
        Some(_) => {}
    }

    std::fs::write(&path, doc.to_string()).map_err(|err| {
        Error::new(format!(
            "failed to write config file {}: {err}",
            path.display()
        ))
    })?;

    println!("Deleted connection '{name}' from {}", path.display());
    Ok(())
}

fn load_document() -> Result<Option<(PathBuf, DocumentMut)>, Error> {
    let candidates = candidate_config_paths();
    let Some(path) = candidates.iter().find(|path| path.exists()) else {
        return Ok(None);
    };

    let doc = read_config_document(path)?;
    Ok(Some((path.clone(), doc)))
}

fn new_config_document() -> Result<(PathBuf, DocumentMut), Error> {
    let path = candidate_config_paths()
        .into_iter()
        .next()
        .ok_or_else(|| Error::new("no candidate config paths"))?;

    let parent = path
        .parent()
        .ok_or_else(|| Error::new(format!("config path {} has no parent", path.display())))?;
    std::fs::create_dir_all(parent).map_err(|err| {
        Error::new(format!(
            "failed to create config directory {}: {err}",
            parent.display()
        ))
    })?;

    Ok((path, DocumentMut::new()))
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;

    const TEST_CONFIG: &str = r#"
default_connection = "default"

[connections.default]
endpoint = "http://127.0.0.1:6543"
auth = "direct"
"#;

    #[test]
    fn config_deserializes_without_default_connection() {
        let config: Config = toml::from_str("").unwrap();

        assert!(!config.has_default_connection());
        assert!(config.connections.is_empty());
    }

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
            conn.auth(),
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
        let mut doc = DocumentMut::from_str(TEST_CONFIG).unwrap();

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
            conn.auth(),
            ConnectionAuthSpec::ApiKey { api_key } if api_key == "test-api-key"
        );
    }

    #[test]
    fn env_overrides_can_set_connection_headers() {
        for (value, expected) in [
            ("X-Tenant: acme", vec!["X-Tenant: acme"]),
            (
                "X-Tenant: acme\nX-Trace: demo",
                vec!["X-Tenant: acme", "X-Trace: demo"],
            ),
        ] {
            let mut doc = DocumentMut::from_str(TEST_CONFIG).unwrap();

            apply_env_overrides(
                &mut doc,
                [(
                    "SCOPEQL_CONFIG_CONNECTIONS_DEFAULT_HEADERS".to_string(),
                    value.to_string(),
                )],
            );

            let config = Config::deserialize(doc.into_deserializer()).unwrap();
            assert_eq!(
                config
                    .get_default_connection()
                    .map(ConnectionSpec::headers)
                    .unwrap_or_default(),
                expected
            );
        }
    }
}
