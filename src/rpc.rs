//! JSON-RPC client for herdr's unix socket (protocol 17).
//!
//! Framing is newline-delimited JSON. herdr serves one request per
//! connection — it closes the stream after responding — except for
//! `events.subscribe`, which holds the connection open and pushes one
//! `{event, data}` envelope per line.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

#[derive(Clone, Debug)]
pub struct Client {
    socket: PathBuf,
}

impl Client {
    /// herdr exports HERDR_SOCKET_PATH into plugin commands and pane
    /// environments both, so a daemon spawned from either binds the same
    /// session. Absent the variable, the default session's socket applies.
    pub fn from_env() -> Self {
        let socket = std::env::var_os("HERDR_SOCKET_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| config_home().join("herdr/herdr.sock"));
        Self { socket }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket
    }

    /// herdr keeps a session's socket, session.json, and config.toml in one
    /// directory, so the socket's parent locates them all.
    pub fn session_dir(&self) -> PathBuf {
        self.socket
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    pub fn call(&self, method: &str, params: Value) -> Result<Value> {
        let mut stream = UnixStream::connect(&self.socket)
            .with_context(|| format!("connecting to herdr at {}", self.socket.display()))?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        let request = json!({ "id": "livery", "method": method, "params": params });
        stream.write_all(request.to_string().as_bytes())?;
        stream.write_all(b"\n")?;

        let mut line = String::new();
        BufReader::new(stream)
            .read_line(&mut line)
            .with_context(|| format!("reading {method} response"))?;
        let response: Value =
            serde_json::from_str(&line).with_context(|| format!("parsing {method} response"))?;
        if let Some(err) = response.get("error") {
            let message = err
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown herdr error");
            bail!("{method}: {message}");
        }
        response
            .get("result")
            .cloned()
            .with_context(|| format!("{method} response missing result"))
    }

    /// Returns a reader that yields one pushed event envelope per line for as
    /// long as the server holds the subscription open.
    pub fn subscribe(&self, types: &[&str]) -> Result<BufReader<UnixStream>> {
        let mut stream = UnixStream::connect(&self.socket)
            .with_context(|| format!("connecting to herdr at {}", self.socket.display()))?;
        let subscriptions: Vec<Value> = types.iter().map(|t| json!({ "type": t })).collect();
        let request = json!({
            "id": "livery-sub",
            "method": "events.subscribe",
            "params": { "subscriptions": subscriptions },
        });
        stream.write_all(request.to_string().as_bytes())?;
        stream.write_all(b"\n")?;

        let mut reader = BufReader::new(stream);
        let mut ack = String::new();
        reader
            .read_line(&mut ack)
            .context("reading subscribe ack")?;
        let response: Value = serde_json::from_str(&ack).context("parsing subscribe ack")?;
        if response.get("error").is_some() {
            bail!("events.subscribe rejected: {}", ack.trim());
        }
        Ok(reader)
    }
}

pub fn home() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").expect("HOME unset"))
}

fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".config"))
}
