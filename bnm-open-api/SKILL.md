---
name: bnm-open-api
description: How to query Bank Negara Malaysia's (BNM) free Open API at api.bnm.gov.my for official Malaysian rates and statistics - MYR exchange rates, KL USD/MYR reference rate, Overnight Policy Rate (OPR), base and lending rates, Kijang Emas gold coin prices, interbank and Islamic interbank rates, FX turnover, MGS yields, and monthly monetary and banking statistics. Use this skill whenever the user asks for current or historical Malaysian central-bank data, the OPR, ringgit exchange rates, Kijang Emas prices, BNM base rates, or mentions BNM OpenAPI or apikijangportal, even if they never name the API. Covers the required Accept version header, endpoint paths, query parameters, 404 behaviour, and a standard-library-only Python client.
---

# BNM Open API

Official Malaysian central-bank rates and statistics, served as JSON. Free, no API key.

## Essentials (get these right or requests fail)

- **Data host:** `https://api.bnm.gov.my/public/<path>`. The site `apikijangportal.bnm.gov.my` is the documentation and API Explorer portal, not the data host.
- **Required header:** every request must send a version in the `Accept` header: `Accept: application/vnd.BNM.API.v1+json`. BNM's FAQ marks this as mandatory.
- **Method:** `GET` only. No authentication.
- **Payload:** JSON with the useful content under `data`. Do not assume the rest of the envelope; inspect it at runtime.
- **404 is ambiguous:** it is returned both for an invalid date and for a valid date with no data (weekends, public holidays, unpublished sessions).

## Quick start

```bash
curl -s -H "Accept: application/vnd.BNM.API.v1+json" https://api.bnm.gov.my/public/opr
```

Bundled client (Python 3.9+, standard library only; sets the header, retries 429/5xx, handles 404):

```bash
python scripts/bnm.py list                          # known endpoints
python scripts/bnm.py get opr --data-only           # latest OPR decision
python scripts/bnm.py get opr/year/2025             # OPR decisions in a year
python scripts/bnm.py get exchange-rate/USD         # one currency
python scripts/bnm.py get exchange-rate -q session=1700 -q quote=rm
python scripts/bnm.py get kl-usd-reference-rate/date/2025-01-15
```

Exit codes: `0` ok, `2` not found (no data or bad date), `3` other HTTP error, `4` network error, `1` bad usage. From Python: `from bnm import fetch`.

**Go, TypeScript or Rust project?** Read `references/integrations.md` for ready-to-use clients and minimal snippets in each language.

## Which endpoint?

| User wants | Path |
| --- | --- |
| Policy rate | `opr` (history: `opr/year/{year}`) |
| MYR vs other currencies | `exchange-rate` or `exchange-rate/{ISO code}` |
| Official USD/MYR fixing | `kl-usd-reference-rate` (`/date/{YYYY-MM-DD}`) |
| Gold coin prices | `kijang-emas` |
| Bank base / lending rates | `base-rate` |
| Money-market and interbank rates | `interest-rate`, `interbank-swap`, `islamic-interbank-rate` |
| Monthly statistics (M3, reserves, MGS yields, ...) | `msb/<table number>` |

Full list with parameters, field names and confidence levels: read `references/endpoints.md`.

## Workflow

1. Map the request to a path using the table above, or `references/endpoints.md` for anything else.
2. Call it with `scripts/bnm.py` (or any HTTP client that sends the `Accept` header).
3. Read the effective date and the unit from the response before quoting a number. Exchange-rate records carry a `unit` (for example 100 units of a currency per quote), so do not report a rate as per-1 without checking.
4. On 404 for a "latest as of date X" question, step back one day at a time (up to about a week) and tell the user which date the figure is actually from.
5. Report the value, its effective date, and the source: "Source: Bank Negara Malaysia Open API".

## Gotchas

- **Missing or wrong `Accept` header** is the first thing to check when a call fails. Send the full vendor type, not `application/json`.
- **Exchange-rate sessions:** BNM publishes rates in sessions. Clients accept `session` values `0900`, `1130`, `1200`, `1700` and `quote` values `rm`, `fx`. Not every session exists on every date. When using these filters, send both parameters together as the clients do.
- **Year and month history** uses path segments (`.../year/{year}`, and for some series a month segment) and portal "By Year" endpoints require the year. Only `opr/year/{year}` and `kl-usd-reference-rate/date/{date}` are confirmed forms; check others in the API Explorer before relying on them.
- **`msb/1.7`** takes a `type` query parameter: `commercial`, `islamic`, `investment` or `total`.
- **Unreliable listings:** some third-party catalogues show a base URL of `apikijangportal.bnm.gov.my/api` with `/datasets` routes. That does not match BNM's own announcement or any working client; do not use it.
- **No published rate limit was found.** Be polite: cache results (most series update at most daily) and avoid tight loops.
- **Data is provided as-is.** BNM's disclaimer states no warranty on accuracy or timeliness. Do not present figures as financial advice.

## When a path or parameter is unknown

Do not guess. Open the API Explorer at https://apikijangportal.bnm.gov.my/openapi, choose the dataset, set Path and Query String parameters (check the `indicators` options per report; "By Year" endpoints need `year`), click **Update URL**, then **Run**. The Schema and Response tabs show the real shape. Questions to BNM go through the portal's Support form.

## Terms

Dataset terms: https://www.bnm.gov.my/terms-of-use-bnm-datasets. This skill is unofficial and not endorsed by Bank Negara Malaysia.
