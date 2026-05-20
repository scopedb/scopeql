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
use std::io;
use std::io::BufRead;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;
use serde::de::IntoDeserializer;
use toml_edit::DocumentMut;

pub(crate) fn candidate_config_paths() -> Vec<PathBuf> {
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

pub(crate) fn set_toml_path(doc: &mut DocumentMut, parts: &[&str], value: toml_edit::Item) {
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

    pub(crate) fn default_connection_name(&self) -> &str {
        &self.default_connection
    }

    pub(crate) fn connections(&self) -> &BTreeMap<String, ConnectionSpec> {
        &self.connections
    }

    pub(crate) fn default_url() -> &'static str {
        "http://127.0.0.1:6543"
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_connection: "default".to_string(),
            connections: BTreeMap::from([(
                "default".to_string(),
                ConnectionSpec {
                    endpoint: Self::default_url().to_string(),
                    api_key: None,
                    headers: vec![],
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

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    headers: Vec<String>,
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

    pub fn headers(&self) -> &[String] {
        &self.headers
    }
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

fn parse_cli_headers(headers: &str) -> Vec<String> {
    headers
        .split(',')
        .map(|h| {
            let h = h.trim();
            if let Some((key, value)) = h.split_once('=') {
                format!("{}: {}", key.trim(), value.trim())
            } else {
                h.to_string()
            }
        })
        .filter(|h| !h.is_empty())
        .collect()
}

fn prompt(message: &str) -> String {
    let mut stdout = io::stdout();
    let _ = write!(stdout, "{message} ");
    let _ = stdout.flush();

    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line).unwrap();
    line.trim().to_string()
}

fn prompt_optional(message: &str) -> Option<String> {
    let val = prompt(message);
    if val.is_empty() { None } else { Some(val) }
}

fn prompt_password(message: &str) -> Option<String> {
    let val = rpassword::prompt_password(message).unwrap_or_default();
    let val = val.trim().to_string();
    if val.is_empty() { None } else { Some(val) }
}

pub(crate) fn config_list() {
    let (_path, doc) = load_document();
    let config =
        Config::deserialize(doc.into_deserializer()).expect("failed to deserialize config");

    let default = config.default_connection_name();

    if config.connections().is_empty() {
        println!("No connections configured.");
        return;
    }

    let max_name_width = config
        .connections()
        .keys()
        .map(|name: &String| name.len())
        .max()
        .unwrap_or(0)
        .max(4);

    for (name, conn) in config.connections() {
        let marker = if name == default { '*' } else { ' ' };
        println!("{marker} {name:<max_name_width$}  {}", conn.endpoint());
    }
}

pub(crate) fn config_use(name: &str) {
    let (path, mut doc) = load_document();

    {
        let config = Config::deserialize(doc.clone().into_deserializer())
            .expect("failed to deserialize config");
        if !config.connections().contains_key(name) {
            eprintln!("Connection '{name}' not found.");
            std::process::exit(1);
        }
    }

    set_toml_path(&mut doc, &["default_connection"], toml_edit::value(name));

    std::fs::write(&path, doc.to_string()).unwrap_or_else(|err| {
        panic!("failed to write config file {}: {err}", path.display());
    });

    println!("Switched to connection '{name}'");
}

pub(crate) fn config_add(
    name: Option<String>,
    url: Option<String>,
    api_key: Option<String>,
    headers: Option<String>,
    prompt_flag: bool,
) {
    let (name, url, api_key, headers_raw) = if prompt_flag {
        let name = prompt("Connection name:");
        let url = prompt("URL:");
        let api_key = prompt_password("API key (optional):");
        let headers = prompt_optional("Headers (key=value, comma-separated, optional):");
        (name, url, api_key, headers)
    } else {
        let name = name.unwrap_or_else(|| {
            eprintln!("Connection name is required.");
            std::process::exit(1);
        });
        let url = url.unwrap_or(Config::default_url().to_string());
        (name, url, api_key, headers)
    };

    let candidates = candidate_config_paths();
    let (path, mut doc, is_new) = if let Some(found) = candidates.iter().find(|path| path.exists())
    {
        log::info!("Loaded config file from {}", found.display());
        let content = std::fs::read_to_string(found).unwrap_or_else(|err| {
            panic!("failed to read config file {}: {err}", found.display());
        });
        let doc = DocumentMut::from_str(&content).unwrap_or_else(|err| {
            panic!("failed to parse config file {}: {err}", found.display());
        });
        (found, doc, false)
    } else {
        let path = candidates
            .first()
            .expect("no candidate config paths available");
        log::info!(
            "No config file found. Creating new config at {}",
            path.display()
        );
        let parent = path.parent().unwrap();
        std::fs::create_dir_all(parent).unwrap_or_else(|err| {
            panic!(
                "failed to create config directory {}: {err}",
                parent.display()
            );
        });
        let mut doc = DocumentMut::new();
        doc["default_connection"] = toml_edit::value(&name);
        doc["connections"] = toml_edit::table();
        (path, doc, true)
    };

    if !is_new {
        let config = Config::deserialize(doc.clone().into_deserializer())
            .expect("failed to deserialize config");
        if config.connections().contains_key(&name) {
            eprintln!("Connection '{name}' already exists.");
            std::process::exit(1);
        }
    }

    doc["connections"][&name] = toml_edit::table();
    doc["connections"][&name]["endpoint"] = toml_edit::value(url);

    if let Some(api_key) = api_key
        && !api_key.is_empty()
    {
        doc["connections"][&name]["api_key"] = toml_edit::value(api_key);
    }

    if let Some(headers_raw) = headers_raw {
        let headers = parse_cli_headers(&headers_raw);
        if !headers.is_empty() {
            let arr: toml_edit::Array = headers.into_iter().collect();
            doc["connections"][&name]["headers"] = toml_edit::value(arr);
        }
    }

    std::fs::write(path, doc.to_string()).unwrap_or_else(|err| {
        panic!("failed to write config file {}: {err}", path.display());
    });

    println!("Added connection '{name}' to {}", path.display());
}

pub(crate) fn config_delete(name: &str) {
    let (path, mut doc) = load_document();

    let config =
        Config::deserialize(doc.clone().into_deserializer()).expect("failed to deserialize config");

    if !config.connections().contains_key(name) {
        eprintln!("Connection '{name}' not found.");
        std::process::exit(1);
    }

    if config.default_connection_name() == name {
        let Some(other) = config.connections().keys().find(|k| *k != name).cloned() else {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_api_key_ignores_empty_string() {
        let connection = ConnectionSpec {
            endpoint: "http://127.0.0.1:6543".to_string(),
            api_key: Some(String::new()),
            headers: vec![],
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
