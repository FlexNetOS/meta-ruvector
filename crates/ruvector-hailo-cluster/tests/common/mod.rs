//! Shared helpers for CLI integration tests (`embed_cli`, `stats_cli`,
//! `bench_cli`). Lives at `tests/common/mod.rs` rather than `tests/common.rs`
//! so Cargo treats it as a non-test source file (the directory form is
//! the canonical idiom for test-suite-shared helpers).

#![allow(dead_code)]

use std::net::{SocketAddr, TcpListener};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Return the path to a Cargo-built integration-test binary.
///
/// Use a runtime lookup instead of `env!("CARGO_BIN_EXE_...")` so
/// `cargo clippy --all-targets` can type-check the integration tests
/// without requiring Cargo to define binary paths at compile time.
pub fn cargo_bin(name: &str) -> String {
    let key = format!("CARGO_BIN_EXE_{name}");
    std::env::var(&key).unwrap_or_else(|_| panic!("{key} not set by Cargo"))
}

/// Path to the built fakeworker binary — provided by Cargo when tests run.
pub fn fakeworker_bin() -> String {
    cargo_bin("ruvector-hailo-fakeworker")
}

/// Allocate a free TCP port by binding briefly + dropping. Cheap race
/// (port could be reused before fakeworker grabs it) but in CI this
/// has been reliable across the existing GrpcTransport tests.
pub fn free_port() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    drop(l);
    port
}

/// Spawn fakeworker on `port`. Pass an empty `fingerprint` to use the
/// fakeworker's default (it picks one from `RUVECTOR_FAKE_FINGERPRINT`
/// or falls back to its built-in literal). `dim` is the embedding
/// dimensionality the worker reports.
///
/// Polls TCP-connect with a 1s ceiling so the test isn't racing the
/// worker's bind. Panics (after reaping the child) if the worker never
/// becomes reachable.
pub fn spawn_fakeworker(port: u16, dim: usize, fingerprint: &str) -> std::process::Child {
    let bind: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let mut cmd = Command::new(fakeworker_bin());
    cmd.env("RUVECTOR_FAKE_BIND", bind.to_string())
        .env("RUVECTOR_FAKE_DIM", dim.to_string())
        // Suppress fakeworker's startup logs during tests — we don't
        // assert on them.
        .env("RUST_LOG", "warn")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if !fingerprint.is_empty() {
        cmd.env("RUVECTOR_FAKE_FINGERPRINT", fingerprint);
    }
    let mut child = cmd.spawn().expect("spawn fakeworker");

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if std::net::TcpStream::connect_timeout(&bind, Duration::from_millis(50)).is_ok() {
            return child;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    // Reap before panicking so we don't leak a zombie.
    let _ = child.kill();
    let _ = child.wait();
    panic!("fakeworker on {} never accepted connections", bind);
}
