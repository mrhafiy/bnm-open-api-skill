// Package bnm is a minimal client for the Bank Negara Malaysia (BNM) Open API.
//
// Standard library only. It sends the mandatory versioned Accept header,
// retries transient failures, and reports HTTP 404 (no data or invalid date)
// as ErrNotFound. Unofficial; not affiliated with or endorsed by BNM.
package bnm

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"regexp"
	"strings"
	"time"
)

const (
	// DefaultBaseURL is the data host. The apikijangportal site is docs only.
	DefaultBaseURL = "https://api.bnm.gov.my/public"
	// DefaultAPIVersion is used to build the Accept header.
	DefaultAPIVersion = 1
	userAgent         = "bnm-open-api-skill/0.1"
)

var (
	// ErrNotFound means HTTP 404. BNM uses it for invalid dates and for
	// valid dates with no data (weekends, holidays, unpublished sessions).
	ErrNotFound = errors.New("bnm: no data or invalid date (HTTP 404)")
	// ErrInvalidPath means the endpoint path failed validation.
	ErrInvalidPath = errors.New("bnm: invalid endpoint path")

	pathOK = regexp.MustCompile(`^[A-Za-z0-9._-]+(/[A-Za-z0-9._-]+)*$`)
)

// APIError is returned for non-404 HTTP errors.
type APIError struct {
	Status int
	Body   string
}

func (e *APIError) Error() string { return fmt.Sprintf("bnm: HTTP %d: %s", e.Status, e.Body) }

// Response holds a decoded BNM reply. Data is the "data" field when present;
// Raw is the full body so callers can inspect the rest of the envelope.
type Response struct {
	Data json.RawMessage
	Raw  json.RawMessage
}

// Decode unmarshals the "data" field (or the whole body if there is none).
func (r *Response) Decode(v any) error {
	if len(r.Data) > 0 {
		return json.Unmarshal(r.Data, v)
	}
	return json.Unmarshal(r.Raw, v)
}

// Client talks to the BNM Open API. The zero value is not ready; use New.
type Client struct {
	BaseURL    string
	APIVersion int
	HTTPClient *http.Client
	MaxRetries int
	// Backoff returns the wait before retry number attempt (1-based).
	Backoff func(attempt int) time.Duration
}

// New returns a Client with sensible defaults.
func New() *Client {
	return &Client{
		BaseURL:    DefaultBaseURL,
		APIVersion: DefaultAPIVersion,
		HTTPClient: &http.Client{Timeout: 30 * time.Second},
		MaxRetries: 2,
		Backoff: func(attempt int) time.Duration {
			d := time.Duration(1<<uint(attempt)) * time.Second
			if d > 8*time.Second {
				d = 8 * time.Second
			}
			return d
		},
	}
}

// AcceptHeader returns the mandatory versioned Accept value.
func (c *Client) AcceptHeader() string {
	return fmt.Sprintf("application/vnd.BNM.API.v%d+json", c.APIVersion)
}

// CleanPath validates and normalises an endpoint path such as "opr/year/2025".
func CleanPath(p string) (string, error) {
	p = strings.Trim(strings.TrimSpace(p), "/")
	if p == "" || !pathOK.MatchString(p) {
		return "", fmt.Errorf("%w: %q", ErrInvalidPath, p)
	}
	for _, seg := range strings.Split(p, "/") {
		if seg == ".." {
			return "", fmt.Errorf("%w: %q", ErrInvalidPath, p)
		}
	}
	return p, nil
}

func retryable(status int) bool {
	return status == 429 || status == 500 || status == 502 || status == 503 || status == 504
}

func (c *Client) wait(ctx context.Context, attempt int) error {
	if c.Backoff == nil {
		return nil
	}
	d := c.Backoff(attempt)
	if d <= 0 {
		return ctx.Err()
	}
	t := time.NewTimer(d)
	defer t.Stop()
	select {
	case <-ctx.Done():
		return ctx.Err()
	case <-t.C:
		return nil
	}
}

// Get performs GET {BaseURL}/{path}?{query} and returns the decoded reply.
func (c *Client) Get(ctx context.Context, path string, query url.Values) (*Response, error) {
	clean, err := CleanPath(path)
	if err != nil {
		return nil, err
	}
	target := strings.TrimRight(c.BaseURL, "/") + "/" + clean
	if len(query) > 0 {
		target += "?" + query.Encode()
	}
	hc := c.HTTPClient
	if hc == nil {
		hc = http.DefaultClient
	}

	for attempt := 0; ; attempt++ {
		req, err := http.NewRequestWithContext(ctx, http.MethodGet, target, nil)
		if err != nil {
			return nil, err
		}
		req.Header.Set("Accept", c.AcceptHeader())
		req.Header.Set("User-Agent", userAgent)

		resp, err := hc.Do(req)
		if err != nil {
			if attempt < c.MaxRetries && ctx.Err() == nil {
				if werr := c.wait(ctx, attempt+1); werr != nil {
					return nil, werr
				}
				continue
			}
			return nil, err
		}
		body, rerr := io.ReadAll(io.LimitReader(resp.Body, 16<<20))
		resp.Body.Close()
		if rerr != nil {
			return nil, rerr
		}

		switch {
		case resp.StatusCode == http.StatusNotFound:
			return nil, ErrNotFound
		case resp.StatusCode >= 200 && resp.StatusCode < 300:
			var env struct {
				Data json.RawMessage `json:"data"`
			}
			if jerr := json.Unmarshal(body, &env); jerr != nil {
				return nil, &APIError{Status: resp.StatusCode, Body: "response was not valid JSON: " + jerr.Error()}
			}
			return &Response{Data: env.Data, Raw: body}, nil
		case retryable(resp.StatusCode) && attempt < c.MaxRetries:
			if werr := c.wait(ctx, attempt+1); werr != nil {
				return nil, werr
			}
			continue
		default:
			snippet := string(body)
			if len(snippet) > 300 {
				snippet = snippet[:300]
			}
			return nil, &APIError{Status: resp.StatusCode, Body: snippet}
		}
	}
}

// GetInto is Get followed by Decode into v.
func (c *Client) GetInto(ctx context.Context, path string, query url.Values, v any) error {
	r, err := c.Get(ctx, path, query)
	if err != nil {
		return err
	}
	return r.Decode(v)
}
