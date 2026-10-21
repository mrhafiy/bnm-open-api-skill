//! `bnm get PATH [-q key=value]... [--data-only] [--base-url URL]`
//!
//! Exit codes: 0 ok, 1 bad usage, 2 not found, 3 HTTP error, 4 network error.

use bnm::{Client, Error};
use std::process::ExitCode;

fn usage(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    eprintln!("usage: bnm get PATH [-q key=value]... [--data-only] [--base-url URL]");
    ExitCode::from(1)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 || args[0] != "get" {
        return usage("expected: get PATH");
    }
    let path = args[1].clone();
    let mut query: Vec<(String, String)> = Vec::new();
    let mut data_only = false;
    let mut base_url: Option<String> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-q" | "--query" => {
                i += 1;
                match args.get(i).and_then(|p| p.split_once('=')) {
                    Some((k, v)) => query.push((k.to_string(), v.to_string())),
                    None => return usage("-q needs key=value"),
                }
            }
            "--data-only" => data_only = true,
            "--base-url" => {
                i += 1;
                match args.get(i) {
                    Some(u) => base_url = Some(u.clone()),
                    None => return usage("--base-url needs a value"),
                }
            }
            other => return usage(&format!("unknown argument {other}")),
        }
        i += 1;
    }

    let mut client = Client::new();
    if let Some(u) = base_url {
        client = client.base_url(u);
    }
    let pairs: Vec<(&str, &str)> = query
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    match client.get_json(&path, &pairs) {
        Ok(resp) => {
            let value = if data_only { &resp.data } else { &resp.raw };
            println!(
                "{}",
                serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
            );
            ExitCode::SUCCESS
        }
        Err(e @ Error::InvalidPath(_)) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
        Err(e @ Error::NotFound(_)) => {
            eprintln!("not found: {e}");
            ExitCode::from(2)
        }
        Err(e @ Error::Api { .. }) => {
            eprintln!("error: {e}");
            ExitCode::from(3)
        }
        Err(e @ Error::Network(_)) => {
            eprintln!("{e}");
            ExitCode::from(4)
        }
    }
}
