"""Offline tests for scripts/bnm.py using a local stub server (no network access to BNM)."""
import json
import os
import sys
import threading
import unittest
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

os.environ["NO_PROXY"] = "127.0.0.1,localhost"
os.environ["no_proxy"] = "127.0.0.1,localhost"

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "bnm-open-api", "scripts"))
import bnm  # noqa: E402

CONTRACT = {}
contract_path = Path(__file__).resolve().parents[1] / "clients" / "rust" / "tests" / "client-contract.tsv"
for line in contract_path.read_text(encoding="utf-8").splitlines():
    if line and not line.startswith("#"):
        key, value = line.split("\t", 1)
        CONTRACT.setdefault(key, []).append(value)
required_contract_keys = {
    "accept_header",
    "invalid_path",
    "not_found_status",
    "other_status",
    "response_body",
    "retry_attempts",
    "retry_status",
    "valid_path",
}
missing_contract_keys = required_contract_keys.difference(CONTRACT)
if missing_contract_keys:
    raise ValueError("missing client contract cases: %s" % ", ".join(sorted(missing_contract_keys)))

SEEN = []
RETRY_HITS = {}


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def _send(self, code, payload):
        body = json.dumps(payload).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        SEEN.append((self.path, self.headers.get("Accept")))
        if self.path.startswith("/public/opr"):
            self._send(200, json.loads(CONTRACT["response_body"][0]))
        elif self.path.startswith("/public/retry/"):
            status = int(self.path.rsplit("/", 1)[1])
            RETRY_HITS[status] = RETRY_HITS.get(status, 0) + 1
            if RETRY_HITS[status] <= int(CONTRACT["retry_attempts"][0]):
                self._send(status, {"error": "busy"})
            else:
                self._send(200, json.loads(CONTRACT["response_body"][0]))
        elif self.path.startswith("/public/forbidden"):
            self._send(int(CONTRACT["other_status"][0]), {"error": "forbidden"})
        else:
            self._send(int(CONTRACT["not_found_status"][0]), {"error": "none"})


class ClientTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = HTTPServer(("127.0.0.1", 0), Handler)
        cls.base = "http://127.0.0.1:%d/public" % cls.server.server_port
        threading.Thread(target=cls.server.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()

    def test_sends_versioned_accept_header(self):
        bnm.fetch("opr", base_url=self.base)
        self.assertEqual(SEEN[-1][1], CONTRACT["accept_header"][0])

    def test_returns_decoded_json(self):
        body = bnm.fetch("opr", base_url=self.base)
        self.assertEqual(body, json.loads(CONTRACT["response_body"][0]))

    def test_query_string_is_encoded(self):
        bnm.fetch("opr", query={"session": "1700", "quote": "rm"}, base_url=self.base)
        self.assertIn("session=1700", SEEN[-1][0])
        self.assertIn("quote=rm", SEEN[-1][0])

    def test_404_raises_not_found(self):
        with self.assertRaises(bnm.NotFound):
            bnm.fetch("exchange-rate/XXX", base_url=self.base)

    def test_retries_contract_statuses(self):
        original = bnm.time.sleep
        bnm.time.sleep = lambda s: None
        try:
            for status_text in CONTRACT["retry_status"]:
                status = int(status_text)
                with self.subTest(status=status):
                    RETRY_HITS[status] = 0
                    body = bnm.fetch("retry/%d" % status, base_url=self.base)
                    self.assertEqual(body, json.loads(CONTRACT["response_body"][0]))
                    self.assertEqual(RETRY_HITS[status], int(CONTRACT["retry_attempts"][0]) + 1)
        finally:
            bnm.time.sleep = original

    def test_other_status_is_api_error(self):
        status = int(CONTRACT["other_status"][0])
        with self.assertRaises(bnm.ApiError) as raised:
            bnm.fetch("forbidden", base_url=self.base)
        self.assertEqual(raised.exception.status, status)

    def test_rejects_bad_paths(self):
        for bad in CONTRACT["invalid_path"]:
            with self.assertRaises(ValueError, msg=bad):
                bnm.clean_path(bad)

    def test_accepts_real_paths(self):
        for good in CONTRACT["valid_path"]:
            bnm.clean_path(good)

    def test_cli_exit_codes(self):
        self.assertEqual(bnm.main(["--base-url", self.base, "get", "opr", "--data-only"]), 0)
        self.assertEqual(bnm.main(["--base-url", self.base, "get", "nothing-here"]), 2)
        self.assertEqual(bnm.main(["get", "../x"]), 1)
        self.assertEqual(bnm.main(["list"]), 0)


if __name__ == "__main__":
    unittest.main()
