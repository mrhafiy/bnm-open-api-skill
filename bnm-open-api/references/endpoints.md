# BNM Open API endpoint reference

Base URL: `https://api.bnm.gov.my/public`. All requests: `GET`, header `Accept: application/vnd.BNM.API.v1+json`.

**Confidence column**
- **Confirmed** - path seen in the source of a working open-source client.
- **Reported** - named in a third-party tool's documentation; path form not independently verified.
- **Unconfirmed** - dataset exists per BNM's announcement but no path was found. Check the API Explorer.

## Contents
- Rates and market data
- Monthly statistics (`msb/...`)
- Response shapes seen so far
- Dataset list per BNM's launch announcement

## Rates and market data

| Path | Dataset | Parameters | Confidence |
| --- | --- | --- | --- |
| `exchange-rate` | MYR against about 26-27 currencies; buying, selling, middle | query `session` (`0900`, `1130`, `1200`, `1700`), `quote` (`rm`, `fx`) | Confirmed |
| `exchange-rate/{code}` e.g. `exchange-rate/USD` | One currency | none | Reported |
| `kl-usd-reference-rate` | Kuala Lumpur USD/MYR reference rate | none | Confirmed |
| `kl-usd-reference-rate/date/{YYYY-MM-DD}` | Reference rate on a given day | date in path | Reported |
| `usd-interbank-intraday-rate` | USD/MYR interbank intraday high/low | none | Confirmed |
| `interest-rate` | Interbank / money-market deposit rates by tenor | none | Reported |
| `interest-volume` | Transaction volumes by tenor | none | Reported |
| `islamic-interbank-rate` | Islamic interbank rates | none | Confirmed |
| `my-overnight-rate-i` | Malaysia overnight rate (Islamic); name taken from a client tool, confirm in the Explorer | none | Confirmed (path), description inferred |
| `interbank-swap` | Interbank swap volumes by maturity | none | Confirmed |
| `fx-turn-over` | Daily FX turnover | none | Confirmed |
| `kijang-emas` | Kijang Emas gold coin buying and selling prices (MYR) for `one_oz`, `half_oz`, `quarter_oz` | year and month history exists; exact path form not confirmed | Confirmed (latest) |
| `base-rate` | Base Rate (BR), Base Lending Rate (BLR) and indicative effective lending rate per institution | an 8-character SWIFT bank code filter exists in one client; how it is passed is not confirmed | Confirmed (latest) |
| `opr` | Overnight Policy Rate decisions | none | Confirmed |
| `opr/year/{year}` e.g. `opr/year/2025` | All OPR decisions in a year | year in path | Reported |
| `consumer-alert` | Financial Consumer Alert list | none | Confirmed |
| Renminbi FX forward price | Listed in the launch announcement | - | Unconfirmed |

## Monthly statistics (`msb/...`)

These paths mirror table numbers from BNM's monthly statistics. Only the tables below were found in a working client; others probably exist.

| Path | Dataset (from client tool names) | Parameters |
| --- | --- | --- |
| `msb/1.1` | Reserve money | none |
| `msb/1.2` | Currency in circulation by denomination | none |
| `msb/1.3` | Monetary aggregates M1, M2, M3 | none |
| `msb/1.3.1` | Broad money M3 | none |
| `msb/1.4` | BNM statement of assets and liabilities | none |
| `msb/1.7` | Banking system statement of assets and liabilities | query `type`: `commercial`, `islamic`, `investment`, `total` |
| `msb/2.1` | Interest rates of banking institutions | none |
| `msb/2.5` | Malaysian Government Securities market indicative yield | none |
| `msb/2.7` | Volume of transactions, interbank money market | none |
| `msb/2.8` | Volume of transactions, KL foreign exchange market | none |
| `msb/3.1` | Federal government finance | none |
| `msb/3.5.12a` | Labour market indicators, financial services sector | none |
| `msb/3.8` | External reserves | none |
| `msb/4.11` | Takaful key indicators | none |
| `msb/bih/market-highlights` | Bond market highlights | none |

All `Confirmed` (path seen in client code); descriptions are taken from the client's tool names.

## Response shapes seen so far

Treat these as examples, not a schema. Always inspect the actual response.

- Exchange rate record: `currency_code`, `unit` (1 or 100 observed), and `rate` with `date`, `buying_rate`, `selling_rate`, `middle_rate`.
- OPR by year: records include `change_in_opr` and `new_opr_level`.
- Most clients read the array or object under `data`.

## Dataset list per BNM's launch announcement

Exchange Rate; Renminbi FX Forward Price; USD/MYR Interbank Intraday Rate; KL USD/MYR Reference Rate; Islamic Interbank Rates; Interest Rates and Volumes; Daily FX Turnover; Interbank Swaps; Base Rates; OPR; Kijang Emas Price; Financial Consumer Alert. Monthly highlights and statistics were announced as coming later; they appear to be served under `msb/`.
