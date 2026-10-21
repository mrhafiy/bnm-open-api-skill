# Usage guide

This is the practical flow for working with the BNM Open API without guessing.

## 1. Start from the actual task

Map the user question to a dataset before writing code:

- policy rate question? start with `opr`
- ringgit exchange rate? use `exchange-rate`
- USD/MYR reference rate on a specific date? use `kl-usd-reference-rate/date/{YYYY-MM-DD}`
- market or monthly stats? use a path under `msb/...`

If the dataset name is unclear, read the endpoint list in [../bnm-open-api/references/endpoints.md](../bnm-open-api/references/endpoints.md) before trying a guess.

## 2. Always send the required header

```http
Accept: application/vnd.BNM.API.v1+json
```

This is the first thing to verify when a request fails. A plain `application/json` request is the most common cause of an empty or confusing response.

## 3. Keep the base URL consistent

Use the public host:

```text
https://api.bnm.gov.my/public
```

Do not substitute the portal domain (`apikijangportal.bnm.gov.my`) as the data host. The portal is the documentation surface; the actual data lives under the public API host.

## 4. Learn the response shape before relying on it

BNM's payloads are not always a flat array. A common pattern is:

```json
{
  "data": [
    { "new_opr_level": 2.75 }
  ],
  "meta": {}
}
```

Treat the `data` field as the useful payload and inspect the real response structure before building typed models.

## 5. Handle 404 correctly

BNM uses HTTP 404 for two common conditions:

- the date is invalid
- the date is valid but no data has been published yet

This means a 404 may not be a broken client. When a date-specific endpoint returns 404, check whether the date is a weekend, holiday, or a date without a published figure.

## 6. Use the right query fields for exchange-rate calls

Exchange-rate queries often take both filters together:

```bash
python bnm-open-api/scripts/bnm.py get exchange-rate -q session=1700 -q quote=rm
```

Valid values commonly include:

- `session`: `0900`, `1130`, `1200`, `1700`
- `quote`: `rm`, `fx`

Do not assume that every session exists every day. When the user asks for a specific session, send both query parameters together.

## 7. Prefer a small proof loop

Use the smallest successful request to verify assumptions:

```bash
python bnm-open-api/scripts/bnm.py get opr --data-only
python bnm-open-api/scripts/bnm.py get exchange-rate/USD
python bnm-open-api/scripts/bnm.py get opr/year/2025
```

This keeps debugging fast and avoids chasing a large dataset before confirming the endpoint or response format.

## 8. Use the Python client as the reference implementation

The bundled client in [../bnm-open-api/scripts/bnm.py](../bnm-open-api/scripts/bnm.py) demonstrates the expected behaviour:

- required header injection
- path validation
- query string handling
- retry logic for transient HTTP 429/5xx errors
- explicit mapping for HTTP 404

If another client behaves differently, use the Python client as the baseline for expected semantics.
