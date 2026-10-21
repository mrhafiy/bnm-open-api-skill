/**
 * Minimal client for the Bank Negara Malaysia (BNM) Open API.
 *
 * Zero runtime dependencies (uses the global `fetch`, Node 18+). Sends the
 * mandatory versioned Accept header, retries transient failures, and reports
 * HTTP 404 (no data or invalid date) as BnmNotFoundError.
 * Unofficial; not affiliated with or endorsed by Bank Negara Malaysia.
 *
 * Browser use is not verified: BNM's CORS policy is undocumented, so prefer
 * calling this from a server.
 */

export const DEFAULT_BASE_URL = "https://api.bnm.gov.my/public";

const PATH_OK = /^[A-Za-z0-9._-]+(\/[A-Za-z0-9._-]+)*$/;
const RETRY_STATUS = new Set([429, 500, 502, 503, 504]);

/** HTTP 404: BNM uses it for invalid dates and for dates with no data. */
export class BnmNotFoundError extends Error {
  override name = "BnmNotFoundError";
  constructor(public readonly url: string) {
    super(`no data or invalid date for ${url}`);
  }
}

/** Any other non-2xx HTTP status, or a body that is not valid JSON. */
export class BnmApiError extends Error {
  override name = "BnmApiError";
  constructor(
    public readonly status: number,
    public readonly body: string,
  ) {
    super(`HTTP ${status}: ${body}`);
  }
}

/** The host could not be reached (after retries). */
export class BnmNetworkError extends Error {
  override name = "BnmNetworkError";
}

/** The endpoint path failed validation. */
export class BnmInvalidPathError extends Error {
  override name = "BnmInvalidPathError";
}

export interface BnmOptions {
  /** Defaults to DEFAULT_BASE_URL. */
  baseUrl?: string;
  /** Used to build the Accept header. Defaults to 1. */
  apiVersion?: number;
  /** Per-request timeout. Defaults to 30000. */
  timeoutMs?: number;
  /** Retries for 429/5xx and network errors. Defaults to 2. */
  retries?: number;
  /** Wait before retry number `attempt` (1-based). Defaults to 2^attempt s, max 8 s. */
  backoffMs?: (attempt: number) => number;
  /** Inject a fetch implementation (tests, proxies). */
  fetch?: typeof fetch;
}

export interface BnmResponse<T = unknown> {
  /** The `data` field when present, otherwise the whole body. */
  data: T;
  /** The full decoded body, so the rest of the envelope can be inspected. */
  raw: unknown;
}

export type Query = Record<string, string | number | boolean>;

/** Validate and normalise an endpoint path such as "opr/year/2025". */
export function cleanPath(path: string): string {
  const p = path.trim().replace(/^\/+|\/+$/g, "");
  if (!p || !PATH_OK.test(p) || p.split("/").includes("..")) {
    throw new BnmInvalidPathError(`invalid endpoint path: ${JSON.stringify(path)}`);
  }
  return p;
}

/** The mandatory versioned Accept value. */
export function acceptHeader(apiVersion = 1): string {
  return `application/vnd.BNM.API.v${apiVersion}+json`;
}

const defaultBackoff = (attempt: number): number => Math.min(2 ** attempt, 8) * 1000;
const sleep = (ms: number): Promise<void> =>
  ms > 0 ? new Promise((resolve) => setTimeout(resolve, ms)) : Promise.resolve();

/** GET {baseUrl}/{path}?{query} and return the decoded reply. */
export async function bnmFetch<T = unknown>(
  path: string,
  query?: Query,
  options: BnmOptions = {},
): Promise<BnmResponse<T>> {
  const {
    baseUrl = DEFAULT_BASE_URL,
    apiVersion = 1,
    timeoutMs = 30_000,
    retries = 2,
    backoffMs = defaultBackoff,
    fetch: fetchImpl = fetch,
  } = options;

  let url = `${baseUrl.replace(/\/+$/, "")}/${cleanPath(path)}`;
  if (query && Object.keys(query).length > 0) {
    const qs = new URLSearchParams(
      Object.entries(query).map(([k, v]): [string, string] => [k, String(v)]),
    );
    url += `?${qs.toString()}`;
  }

  for (let attempt = 0; ; attempt++) {
    let res: Response;
    try {
      res = await fetchImpl(url, {
        method: "GET",
        headers: { Accept: acceptHeader(apiVersion) },
        signal: AbortSignal.timeout(timeoutMs),
      });
    } catch (err) {
      if (attempt < retries) {
        await sleep(backoffMs(attempt + 1));
        continue;
      }
      throw new BnmNetworkError(err instanceof Error ? err.message : String(err));
    }

    if (res.status === 404) throw new BnmNotFoundError(url);

    const text = await res.text();
    if (res.ok) {
      let raw: unknown;
      try {
        raw = JSON.parse(text);
      } catch {
        throw new BnmApiError(res.status, "response was not valid JSON");
      }
      const data =
        raw !== null && typeof raw === "object" && "data" in raw
          ? (raw as { data: unknown }).data
          : raw;
      return { data: data as T, raw };
    }
    if (RETRY_STATUS.has(res.status) && attempt < retries) {
      await sleep(backoffMs(attempt + 1));
      continue;
    }
    throw new BnmApiError(res.status, text.slice(0, 300));
  }
}

/** Reusable client with fixed options. */
export class BnmClient {
  constructor(private readonly options: BnmOptions = {}) {}

  get<T = unknown>(path: string, query?: Query): Promise<BnmResponse<T>> {
    return bnmFetch<T>(path, query, this.options);
  }
}
