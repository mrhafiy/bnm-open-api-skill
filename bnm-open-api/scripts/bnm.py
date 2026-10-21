#!/usr/bin/env python3
"""Minimal client for the Bank Negara Malaysia (BNM) Open API.

Standard library only. Sends the mandatory versioned Accept header, retries
transient failures, and reports 404 (no data / invalid date) distinctly.

Unofficial; not affiliated with or endorsed by Bank Negara Malaysia.

Examples:
    python bnm.py list
    python bnm.py get opr --data-only
    python bnm.py get exchange-rate/USD
    python bnm.py get exchange-rate -q session=1700 -q quote=rm
    python bnm.py get kl-usd-reference-rate/date/2025-01-15

Exit codes: 0 ok, 1 bad usage, 2 not found, 3 HTTP error, 4 network error.
"""
import argparse
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

DEFAULT_BASE_URL = "https://api.bnm.gov.my/public"
USER_AGENT = "bnm-open-api-skill/0.1"
RETRY_STATUS = (429, 500, 502, 503, 504)
_PATH_OK = re.compile(r"^[A-Za-z0-9._\-]+(/[A-Za-z0-9._\-]+)*$")

# (path, description). Mirrors references/endpoints.md.
KNOWN_ENDPOINTS = [
    ("opr", "Overnight Policy Rate decisions (history: opr/year/{year})"),
    ("exchange-rate", "MYR exchange rates (query: session, quote); one currency: exchange-rate/{code}"),
    ("kl-usd-reference-rate", "KL USD/MYR reference rate (history: /date/{YYYY-MM-DD})"),
    ("usd-interbank-intraday-rate", "USD/MYR interbank intraday rate"),
    ("kijang-emas", "Kijang Emas gold coin prices"),
    ("base-rate", "Base rate, base lending rate, effective lending rate per institution"),
    ("interest-rate", "Interbank / money-market interest rates by tenor"),
    ("interest-volume", "Interbank transaction volumes by tenor"),
    ("islamic-interbank-rate", "Islamic interbank rates"),
    ("my-overnight-rate-i", "Malaysia overnight rate (Islamic)"),
    ("interbank-swap", "Interbank swap volumes"),
    ("fx-turn-over", "Daily FX turnover"),
    ("consumer-alert", "Financial Consumer Alert list"),
    ("msb/1.1", "Reserve money"),
    ("msb/1.2", "Currency in circulation by denomination"),
    ("msb/1.3", "Monetary aggregates M1, M2, M3"),
    ("msb/1.3.1", "Broad money M3"),
    ("msb/1.4", "BNM statement of assets and liabilities"),
    ("msb/1.7", "Banking system statement of assets and liabilities (query: type)"),
    ("msb/2.1", "Interest rates of banking institutions"),
    ("msb/2.5", "MGS market indicative yield"),
    ("msb/2.7", "Interbank money market transaction volume"),
    ("msb/2.8", "KL foreign exchange market transaction volume"),
    ("msb/3.1", "Federal government finance"),
    ("msb/3.5.12a", "Labour market indicators, financial services sector"),
    ("msb/3.8", "External reserves"),
    ("msb/4.11", "Takaful key indicators"),
    ("msb/bih/market-highlights", "Bond market highlights"),
]


class BnmError(Exception):
    """Base class for client errors."""


class NotFound(BnmError):
    """HTTP 404: BNM uses it for both invalid dates and dates with no data."""


class ApiError(BnmError):
    def __init__(self, status, detail=""):
        super().__init__("HTTP %s %s" % (status, detail))
        self.status = status
        self.detail = detail


class NetworkError(BnmError):
    pass


def clean_path(path):
    """Validate and normalise an endpoint path (no scheme, no '..', no query)."""
    path = path.strip().strip("/")
    if not path or ".." in path.split("/") or not _PATH_OK.match(path):
        raise ValueError("invalid endpoint path: %r" % path)
    return path


def accept_header(api_version=1):
    return "application/vnd.BNM.API.v%d+json" % int(api_version)


def fetch(path, query=None, base_url=None, api_version=1, timeout=30, retries=2):
    """GET an endpoint and return the decoded JSON body.

    Raises NotFound on 404, ApiError on other HTTP errors, NetworkError when
    the host cannot be reached, and ValueError for a malformed path.
    """
    base_url = (base_url or os.environ.get("BNM_BASE_URL") or DEFAULT_BASE_URL).rstrip("/")
    url = "%s/%s" % (base_url, clean_path(path))
    if query:
        url += "?" + urllib.parse.urlencode(query)
    request = urllib.request.Request(
        url,
        headers={"Accept": accept_header(api_version), "User-Agent": USER_AGENT},
    )
    attempt = 0
    while True:
        try:
            with urllib.request.urlopen(request, timeout=timeout) as response:
                body = response.read().decode("utf-8")
            try:
                return json.loads(body)
            except json.JSONDecodeError as exc:
                raise ApiError(200, "response was not valid JSON: %s" % exc)
        except urllib.error.HTTPError as exc:
            if exc.code == 404:
                raise NotFound("no data or invalid date for %s" % url)
            if exc.code in RETRY_STATUS and attempt < retries:
                attempt += 1
                time.sleep(min(2 ** attempt, 8))
                continue
            detail = ""
            try:
                detail = exc.read().decode("utf-8", "replace")[:300]
            except Exception:
                pass
            raise ApiError(exc.code, detail)
        except (urllib.error.URLError, TimeoutError, ConnectionError) as exc:
            if attempt < retries:
                attempt += 1
                time.sleep(min(2 ** attempt, 8))
                continue
            raise NetworkError(str(getattr(exc, "reason", exc)))


def _parse_query(pairs):
    query = {}
    for pair in pairs or []:
        if "=" not in pair:
            raise ValueError("query must look like key=value, got %r" % pair)
        key, value = pair.split("=", 1)
        query[key] = value
    return query


def main(argv=None):
    parser = argparse.ArgumentParser(description="Query the BNM Open API (unofficial client).")
    parser.add_argument("--base-url", help="override base URL (default: %s or $BNM_BASE_URL)" % DEFAULT_BASE_URL)
    parser.add_argument("--api-version", type=int, default=1, help="Accept header version (default 1)")
    parser.add_argument("--timeout", type=float, default=30)
    parser.add_argument("--retries", type=int, default=2)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("list", help="show known endpoints")
    get = sub.add_parser("get", help="GET an endpoint under /public")
    get.add_argument("path", help="e.g. opr, exchange-rate/USD, msb/2.5")
    get.add_argument("-q", "--query", action="append", metavar="KEY=VALUE", help="query parameter (repeatable)")
    get.add_argument("--data-only", action="store_true", help="print only the 'data' field when present")
    args = parser.parse_args(argv)

    if args.command == "list":
        width = max(len(p) for p, _ in KNOWN_ENDPOINTS)
        for path, description in KNOWN_ENDPOINTS:
            print("%-*s  %s" % (width, path, description))
        return 0

    try:
        body = fetch(
            args.path,
            query=_parse_query(args.query),
            base_url=args.base_url,
            api_version=args.api_version,
            timeout=args.timeout,
            retries=args.retries,
        )
    except ValueError as exc:
        print("error: %s" % exc, file=sys.stderr)
        return 1
    except NotFound as exc:
        print("not found: %s" % exc, file=sys.stderr)
        return 2
    except ApiError as exc:
        print("error: %s" % exc, file=sys.stderr)
        return 3
    except NetworkError as exc:
        print("network error: %s" % exc, file=sys.stderr)
        return 4

    if args.data_only and isinstance(body, dict) and "data" in body:
        body = body["data"]
    json.dump(body, sys.stdout, indent=2, ensure_ascii=False)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
