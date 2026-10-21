# bnm-open-api (Rust)

Unofficial minimal client for the Bank Negara Malaysia (BNM) Open API. Not affiliated with or endorsed by BNM.

```rust
use bnm::{Client, Error};

fn main() {
    let client = Client::new();
    match client.get_json("opr", &[]) {
        Ok(resp) => println!("{}", resp.data),
        Err(Error::NotFound(_)) => eprintln!("no data or invalid date"),
        Err(e) => eprintln!("{e}"),
    }
}
```

CLI: `cargo run -- get opr --data-only`, `cargo run -- get exchange-rate -q session=1700 -q quote=rm`.

Features:
- `ureq-transport` (default): blocking HTTPS transport.
- `json` (default): `Client::get_json` returning `serde_json` values.
- With `--no-default-features` the crate is standard-library only; bring your own `Transport` and call `Client::get` for the raw body.

Every request sends `Accept: application/vnd.BNM.API.v1+json`. HTTP 404 becomes `Error::NotFound`; 429/5xx and connection errors are retried twice by default.
