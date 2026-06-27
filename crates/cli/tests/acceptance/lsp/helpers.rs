use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, Command, Stdio};

pub struct LspSession {
    pub child: std::process::Child,
    pub reader: BufReader<std::process::ChildStdout>,
    pub stdin: ChildStdin,
}

impl LspSession {
    /// Spawn the LSP server and complete the initialize/initialized handshake
    /// with a null rootUri.
    pub fn start() -> Self {
        Self::start_with_root_uri(None)
    }

    /// Spawn the LSP server and complete the initialize/initialized handshake
    /// with the given rootUri.
    pub fn start_with_root_uri(root_uri: Option<&str>) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_agilels"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn `agilels`");

        let stdout = child.stdout.take().expect("stdout");
        let reader = BufReader::new(stdout);
        let stdin = child.stdin.take().expect("stdin");

        let mut session = LspSession {
            child,
            reader,
            stdin,
        };

        let root_uri_json = match root_uri {
            Some(uri) => format!("\"{}\"", uri),
            None => "null".to_string(),
        };
        let init = format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"processId":1234,"rootUri":{},"capabilities":{{}}}}}}"#,
            root_uri_json
        );
        send_lsp_message(&mut session.stdin, &init).unwrap();
        read_lsp_response(&mut session.reader).unwrap();
        send_lsp_message(
            &mut session.stdin,
            r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
        )
        .unwrap();

        session
    }

    pub fn open_document(&mut self, uri: &str, text: &str) {
        let did_open = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "markdown",
                    "version": 1,
                    "text": text
                }
            }
        });
        send_lsp_message(&mut self.stdin, &did_open.to_string()).unwrap();
    }

    pub fn send(&mut self, message: &str) {
        send_lsp_message(&mut self.stdin, message).unwrap();
    }

    pub fn read_notification(&mut self, method: &str) -> Value {
        read_notification(&mut self.reader, method)
    }

    pub fn read_response(&mut self, id: u64) -> Value {
        read_response(&mut self.reader, id)
    }

    pub fn read_raw_response(&mut self) -> String {
        read_lsp_response(&mut self.reader).unwrap()
    }
}

pub fn send_lsp_message<W: Write>(writer: &mut W, message: &str) -> std::io::Result<()> {
    writeln!(writer, "Content-Length: {}", message.len())?;
    writeln!(
        writer,
        "Content-Type: application/vscode-jsonrpc; charset=utf-8"
    )?;
    writeln!(writer)?;
    write!(writer, "{}", message)?;
    writer.flush()?;
    Ok(())
}

pub fn read_lsp_response<R: BufRead>(reader: &mut R) -> std::io::Result<String> {
    let mut headers = std::collections::HashMap::new();
    let mut line = String::new();

    loop {
        line.clear();
        reader.read_line(&mut line)?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            headers.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }

    let content_length: usize = headers
        .get("content-length")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing Content-Length")
        })?;

    let mut message = vec![0u8; content_length];
    reader.read_exact(&mut message)?;

    Ok(String::from_utf8_lossy(&message).to_string())
}

/// Read messages until one whose `method` field matches `target`, discarding others.
pub fn read_notification<R: BufRead>(reader: &mut R, target: &str) -> Value {
    loop {
        let msg = read_lsp_response(reader).expect("expected a message from server");
        let v: Value = serde_json::from_str(&msg).expect("server sent invalid JSON");
        if v["method"].as_str() == Some(target) {
            return v;
        }
    }
}

/// Read messages until one that is a response to `id`, discarding notifications.
pub fn read_response<R: BufRead>(reader: &mut R, id: u64) -> Value {
    loop {
        let msg = read_lsp_response(reader).expect("expected a message from server");
        let v: Value = serde_json::from_str(&msg).expect("server sent invalid JSON");
        if v["id"] == id {
            return v;
        }
    }
}
