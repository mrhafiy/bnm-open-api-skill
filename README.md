# BNM Open API

A practical, standards-based wrapper for Bank Negara Malaysia's public data APIs. This project bundles a reusable agent skill, a small Python client, and language-specific client examples for working with BNM's official rates, reference fixes, and monthly statistics.

> Unofficial. Not affiliated with or endorsed by Bank Negara Malaysia. BNM provides the data as-is; see the [disclaimer](https://apikijangportal.bnm.gov.my/disclaimer) and [dataset terms](https://www.bnm.gov.my/terms-of-use-bnm-datasets).

## What you get

- A reusable agent skill for AI tools at [bnm-open-api/SKILL.md](bnm-open-api/SKILL.md)
- A standard-library Python client at [bnm-open-api/scripts/bnm.py](bnm-open-api/scripts/bnm.py)
- Endpoint notes and confidence labels at [bnm-open-api/references/endpoints.md](bnm-open-api/references/endpoints.md)
- Cross-language integration snippets at [bnm-open-api/references/integrations.md](bnm-open-api/references/integrations.md)
- Full Go, TypeScript and Rust client implementations in [clients/README.md](clients/README.md)

## Why this project exists

BNM's official data portal is useful, but the docs are fragmented and some endpoint patterns are only visible in working clients or the API explorer. This repo consolidates the details that matter most in practice:

- the required `Accept` header
- the correct base URL and path conventions
- the handling of HTTP 404 for missing data or invalid dates
- retry behavior for transient failures
- the minimum client patterns for Python, Go, TypeScript and Rust

## Quick start

### One command, one data point

```bash
curl -s \
  -H "Accept: application/vnd.BNM.API.v1+json" \
  https://api.bnm.gov.my/public/opr
```

### Python client

```bash
python bnm-open-api/scripts/bnm.py list
python bnm-open-api/scripts/bnm.py get opr --data-only
python bnm-open-api/scripts/bnm.py get exchange-rate/USD
python bnm-open-api/scripts/bnm.py get kl-usd-reference-rate/date/2025-01-15
```

## The rule that matters most

Every request must include:

```http
Accept: application/vnd.BNM.API.v1+json
```

This is not optional. BNM's API is sensitive to the versioned media type, and many failures are caused by sending a plain `application/json` request instead.

## Common request patterns

| Need | Typical path |
| --- | --- |
| Current OPR level | `opr` |
| OPR by year | `opr/year/2025` |
| MYR exchange rates | `exchange-rate` |
| One FX pair | `exchange-rate/USD` |
| Daily KL USD/MYR fix | `kl-usd-reference-rate/date/2025-01-15` |
| Gold coin prices | `kijang-emas` |
| Banking / policy rate data | `base-rate`, `interest-rate`, `interbank-swap` |
| Monthly statistics | `msb/1.1`, `msb/2.5`, `msb/3.8`, etc. |

See [bnm-open-api/references/endpoints.md](bnm-open-api/references/endpoints.md) for the current endpoint list and confidence notes.

## Handling edge cases

- `404` is ambiguous: BNM uses it both for a bad date and for a valid date with no published data.
- Some APIs return data under a nested `data` object; inspect the current response rather than assuming a schema.
- Exchange-rate requests regularly use filters such as `session=1700` and `quote=rm`.
- Some year/month history paths are reported but not yet independently confirmed in the portal.

## Documentation map

- [bnm-open-api/SKILL.md](bnm-open-api/SKILL.md) — agent entry point and quick operational guidance
- [bnm-open-api/references/endpoints.md](bnm-open-api/references/endpoints.md) — endpoint catalog, parameters and confidence levels
- [bnm-open-api/references/integrations.md](bnm-open-api/references/integrations.md) — Go, TypeScript and Rust examples
- [clients/README.md](clients/README.md) — client verification status and runtime notes
- [docs/usage-guide.md](docs/usage-guide.md) — practical usage patterns and request workflow
- [docs/troubleshooting.md](docs/troubleshooting.md) — common failures and fixes

## Development and tests

```bash
python -m unittest discover -s tests -v          # Python
(cd clients/go && go test ./...)                 # Go
(cd clients/typescript && npm install && npm test)  # TypeScript
(cd clients/rust && cargo test --all-features)   # Rust
```

The test suite runs against local stub servers and never calls the live BNM API.

## License

MIT. See [LICENSE](LICENSE).
