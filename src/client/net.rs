// net.rs — Background TCP thread that talks to the Rust server.
//
// Architecture:
//   • A background thread owns the TcpStream and does blocking I/O.
//   • Incoming messages (lines of JSON) are forwarded via `rx`.
//   • Outgoing messages are sent via `tx`.
//   • The GameApp polls `rx` on every animation tick (main thread).

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub struct NetHandle {
    /// Lines received from the server (drained by GameApp::tick)
    pub rx: Receiver<String>,
    /// Lines to send to the server
    tx: Sender<String>,
}

impl NetHandle {
    /// Open a connection; spawns reader + writer threads.
    pub fn connect(host: &str, port: u16) -> anyhow::Result<Self> {
        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect(&addr)?;
        stream.set_nodelay(true)?;

        let (server_tx, server_rx) = mpsc::channel::<String>(); // incoming
        let (client_tx, client_rx) = mpsc::channel::<String>(); // outgoing

        // ── Reader thread ────────────────────────────────────────────────────
        let reader = stream.try_clone()?;
        thread::Builder::new()
            .name("vq-net-reader".into())
            .spawn(move || {
                let lines = BufReader::new(reader).lines();
                for line in lines.flatten() {
                    if server_tx.send(line).is_err() { break; }
                }
            })?;

        // ── Writer thread ────────────────────────────────────────────────────
        let mut writer = stream;
        thread::Builder::new()
            .name("vq-net-writer".into())
            .spawn(move || {
                for mut msg in client_rx {
                    msg.push('\n');
                    if writer.write_all(msg.as_bytes()).is_err() { break; }
                    let _ = writer.flush();
                }
            })?;

        Ok(Self { rx: server_rx, tx: client_tx })
    }

    /// Send a JSON string to the server (non-blocking).
    pub fn send(&self, json: &str) {
        let _ = self.tx.send(json.to_string());
    }
}
