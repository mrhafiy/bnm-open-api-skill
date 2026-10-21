# Troubleshooting guide

This guide covers the issues most likely to appear when calling BNM's public API.

## 1. The request fails with 404

Check these in order:

1. Did you include the required `Accept` header?
2. Is the path spelled correctly?
3. Is the date valid and published?
4. Is the dataset empty for that date or session?

BNM uses 404 for both bad dates and dates without data, so treat it as a data-quality signal rather than a generic transport failure.

## 2. The response looks empty

Typical causes:

- the request was sent against the wrong host
- the `Accept` header was missing or wrong
- the session or quote filters are mismatched
- the date is a holiday or weekend

Check the actual payload shape before assuming a data bug.

## 3. The path is wrong

Some endpoints are straightforward, while others are only known from reports or third-party clients. Before using a less common path, check [../bnm-open-api/references/endpoints.md](../bnm-open-api/references/endpoints.md) and confirm the path pattern in the BNM explorer.

## 4. The endpoint is a date-specific dataset

If the user asked for a historical value and the latest endpoint returns no data, step back one day at a time and record the actual effective date of the published figure.

This is often the right behaviour for reference-rate or overnight figures when the immediate date is not published yet.

## 5. Query parameters are being dropped or mangled

When using a CLI or HTTP client, verify that the URL contains the expected `?key=value` pairs. A common mistake is to append a query string by hand without escaping values properly.

## 6. A client is retrying too aggressively

The bundled clients intentionally retry only transient 429 and 5xx responses. This keeps the tool polite without creating loops against a rate-limited or unavailable API.

## 7. Uncertainty is normal

Some BNM paths are only partially confirmed. If the path is newly reported or inferred from a community client, treat it as provisional until it is verified in the API explorer.

The best practice is: confirm the endpoint, confirm the header, confirm the date, then trust the data.
