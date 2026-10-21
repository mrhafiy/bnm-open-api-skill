import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";
import type { AddressInfo } from "node:net";
import { after, before, test } from "node:test";
import {
  BnmApiError,
  BnmClient,
  BnmInvalidPathError,
  BnmNotFoundError,
  cleanPath,
} from "../src/index.js";

const contract = new Map<string, string[]>();
for (const line of readFileSync("../rust/tests/client-contract.tsv", "utf8").split(/\r?\n/)) {
  if (!line || line.startsWith("#")) continue;
  const separator = line.indexOf("\t");
  assert.notEqual(separator, -1, `invalid contract row: ${line}`);
  const key = line.slice(0, separator);
  const value = line.slice(separator + 1);
  contract.set(key, [...(contract.get(key) ?? []), value]);
}
const values = (key: string): string[] => {
  const found = contract.get(key);
  assert.ok(found && found.length > 0, `missing client contract cases for ${key}`);
  return found;
};
for (const key of [
  "accept_header",
  "invalid_path",
  "not_found_status",
  "other_status",
  "response_body",
  "retry_attempts",
  "retry_status",
  "valid_path",
]) {
  values(key);
}

const seen: { url: string; accept: string | undefined }[] = [];
const retryHits = new Map<number, number>();
let server: Server;
let base = "";
const responseBody = JSON.parse(values("response_body")[0]);
const retryAttempts = Number(values("retry_attempts")[0]);

before(async () => {
  server = createServer((req: IncomingMessage, res: ServerResponse) => {
    seen.push({ url: req.url ?? "", accept: req.headers.accept });
    const send = (code: number, body: unknown) => {
      res.writeHead(code, { "content-type": "application/json" });
      res.end(JSON.stringify(body));
    };
    if (req.url?.startsWith("/public/opr")) send(200, responseBody);
    else if (req.url?.startsWith("/public/retry/")) {
      const status = Number(req.url.split("/").at(-1));
      const hits = (retryHits.get(status) ?? 0) + 1;
      retryHits.set(status, hits);
      if (hits <= retryAttempts) send(status, { error: "busy" });
      else send(200, responseBody);
    } else if (req.url?.startsWith("/public/forbidden")) {
      send(Number(values("other_status")[0]), { error: "no" });
    } else {
      send(Number(values("not_found_status")[0]), { error: "none" });
    }
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  base = `http://127.0.0.1:${(server.address() as AddressInfo).port}/public`;
});

after(() => {
  server.close();
});

const client = () => new BnmClient({ baseUrl: base, backoffMs: () => 0 });

test("sends the versioned Accept header and decodes data", async () => {
  const { data, raw } = await client().get<{ new_opr_level: number }>("opr");
  assert.equal(seen.at(-1)?.accept, values("accept_header")[0]);
  assert.deepEqual(data, (responseBody as { data: unknown }).data);
  assert.deepEqual(raw, responseBody);
});

test("encodes the query string", async () => {
  await client().get("opr", { session: "1700", quote: "rm" });
  assert.equal(seen.at(-1)?.url, "/public/opr?session=1700&quote=rm");
});

test("404 raises BnmNotFoundError", async () => {
  await assert.rejects(client().get("exchange-rate/XXX"), BnmNotFoundError);
});

test("retries contract statuses", async () => {
  for (const statusText of values("retry_status")) {
    const status = Number(statusText);
    retryHits.set(status, 0);
    const { data } = await client().get("retry/" + status);
    assert.deepEqual(data, (responseBody as { data: unknown }).data);
    assert.equal(retryHits.get(status), retryAttempts + 1);
  }
});

test("other statuses raise BnmApiError", async () => {
  await assert.rejects(client().get("forbidden"), (err: unknown) => {
    assert.ok(err instanceof BnmApiError);
    assert.equal(err.status, Number(values("other_status")[0]));
    return true;
  });
});

test("cleanPath rejects bad paths and accepts real ones", () => {
  for (const bad of values("invalid_path")) {
    assert.throws(() => cleanPath(bad), BnmInvalidPathError, bad);
  }
  for (const good of values("valid_path")) {
    cleanPath(good);
  }
});
