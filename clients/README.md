# Language clients

Minimal, dependency-light clients for the BNM Open API. Each one sends the mandatory `Accept: application/vnd.BNM.API.v1+json` header, validates the endpoint path, retries 429/5xx and network errors, and maps HTTP 404 (no data or invalid date) to a distinct "not found" error.

The native client tests consume the shared core behavior cases in `rust/tests/client-contract.tsv`; language-specific test harnesses and implementations remain independent.

| Client | Folder | Runtime deps | CLI |
| --- | --- | --- | --- |
| Go | `go/` | none (standard library) | `go run ./cmd/bnm get opr` |
| TypeScript | `typescript/` | none (global `fetch`, Node 18+) | - |
| Rust | `rust/` | core: none; default features: `ureq`, `serde_json` | `cargo run -- get opr` |
| Python | `../bnm-open-api/scripts/bnm.py` | none | `python bnm.py get opr` |

Replace `YOUR-USERNAME` in `go/go.mod` and `go/cmd/bnm/main.go` with your GitHub username before publishing the Go module.

## Verification status

These clients were written and tested in a sandbox with no access to the npm and crates.io registries and no access to `api.bnm.gov.my`, so none of them has called the live API.

| Client | Verified here | Left to CI after you push |
| --- | --- | --- |
| Go | `gofmt`, `go vet`, 5 tests against a local stub server | live API |
| TypeScript | `src/` type-checks (strict); 6 tests pass when compiled and run on Node 22 | full `npm install`, type-check of the test file against `@types/node`, live API |
| Rust | `rustfmt`; the dependency-free core compiles and its 8 tests pass | the `ureq` transport, `get_json`, the CLI (written, formatted, not compiled), live API |

The GitHub Actions workflow in `.github/workflows/test.yml` builds and tests all of them.
