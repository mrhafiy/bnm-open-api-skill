# Using the BNM Open API from Go, TypeScript and Rust

Every language needs the same three things: the `Accept: application/vnd.BNM.API.v1+json` header, treating HTTP 404 as "no data or invalid date", and retrying 429/5xx a couple of times. Full clients with tests live in the repository's `clients/` folder (`clients/go`, `clients/typescript`, `clients/rust`). The snippets below are the minimum to copy when you cannot add a dependency.

## Contents
- Go
- TypeScript
- Rust
- Choosing a language client

## Go

Using the client in `clients/go` (standard library only):

```go
c := bnm.New()
resp, err := c.Get(ctx, "opr/year/2025", nil)
if errors.Is(err, bnm.ErrNotFound) { /* no data or bad date */ }
// The struct below is illustrative: inspect the real response before relying on its shape.
var rows []struct {
    NewOPRLevel float64 `json:"new_opr_level"`
}
err = resp.Decode(&rows) // decodes the "data" field
```

Minimal, no client:

```go
req, _ := http.NewRequestWithContext(ctx, "GET", "https://api.bnm.gov.my/public/opr", nil)
req.Header.Set("Accept", "application/vnd.BNM.API.v1+json")
res, err := http.DefaultClient.Do(req)
// check res.StatusCode == 404 before decoding res.Body
```

## TypeScript

Using the client in `clients/typescript` (no runtime dependencies, Node 18+, global `fetch`):

```ts
import { BnmClient, BnmNotFoundError } from "bnm-open-api";

const bnm = new BnmClient();
try {
  // The type argument is illustrative: inspect the real response before relying on its shape.
  const { data } = await bnm.get<{ new_opr_level: number }[]>("opr/year/2025");
} catch (e) {
  if (e instanceof BnmNotFoundError) { /* no data or bad date */ }
}
```

Minimal, no client:

```ts
const res = await fetch("https://api.bnm.gov.my/public/opr", {
  headers: { Accept: "application/vnd.BNM.API.v1+json" },
});
if (res.status === 404) throw new Error("no data or invalid date");
const { data } = await res.json();
```

Run this server-side. BNM's CORS policy is undocumented, so browser calls may be blocked.

## Rust

Using the crate in `clients/rust` (core is standard library only; default features add a `ureq` HTTPS transport and `serde_json`):

```rust
use bnm::{Client, Error};

let client = Client::new();
match client.get_json("exchange-rate/USD", &[]) {
    Ok(resp) => println!("{}", resp.data),
    Err(Error::NotFound(_)) => eprintln!("no data or invalid date"),
    Err(e) => eprintln!("{e}"),
}
```

Minimal, no crate of ours (needs `ureq = "2"`):

```rust
let body = ureq::get("https://api.bnm.gov.my/public/opr")
    .set("Accept", "application/vnd.BNM.API.v1+json")
    .call()?          // Err(ureq::Error::Status(404, _)) means no data or bad date
    .into_string()?;
```

## Choosing a language client

All four clients (Python in `scripts/bnm.py`, plus Go, TypeScript, Rust) share the same behaviour: path validation, versioned header, 404 mapped to a distinct "not found", retries on 429/5xx and network errors, and exit codes 0/1/2/3/4 for the Python, Go and Rust command-line tools. Pick the one matching the project; do not mix them to work around an error, since the cause is almost always the header, the path, or a date with no data.
