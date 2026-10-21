package bnm

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"strconv"
	"strings"
	"sync/atomic"
	"testing"
	"time"
)

func clientContract(t *testing.T) map[string][]string {
	t.Helper()
	data, err := os.ReadFile("../rust/tests/client-contract.tsv")
	if err != nil {
		t.Fatal(err)
	}
	contract := make(map[string][]string)
	for _, line := range strings.Split(string(data), "\n") {
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		parts := strings.SplitN(line, "\t", 2)
		if len(parts) != 2 {
			t.Fatalf("invalid contract row %q", line)
		}
		contract[parts[0]] = append(contract[parts[0]], parts[1])
	}
	for _, key := range []string{
		"accept_header", "invalid_path", "not_found_status", "other_status",
		"response_body", "retry_attempts", "retry_status", "valid_path",
	} {
		if len(contract[key]) == 0 {
			t.Fatalf("missing client contract cases for %q", key)
		}
	}
	return contract
}

func newTestClient(srv *httptest.Server) *Client {
	c := New()
	c.BaseURL = srv.URL + "/public"
	c.Backoff = func(int) time.Duration { return 0 }
	return c
}

func TestSendsVersionedAcceptHeaderAndDecodes(t *testing.T) {
	contract := clientContract(t)
	payload := contract["response_body"][0]
	var gotAccept, gotPath, gotQuery string
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotAccept, gotPath, gotQuery = r.Header.Get("Accept"), r.URL.Path, r.URL.RawQuery
		fmt.Fprint(w, payload)
	}))
	defer srv.Close()

	var expected struct {
		Data struct {
			NewOPRLevel float64 `json:"new_opr_level"`
		} `json:"data"`
	}
	if err := json.Unmarshal([]byte(payload), &expected); err != nil {
		t.Fatal(err)
	}
	var out struct {
		NewOPRLevel float64 `json:"new_opr_level"`
	}
	q := url.Values{"session": {"1700"}, "quote": {"rm"}}
	if err := newTestClient(srv).GetInto(context.Background(), "opr", q, &out); err != nil {
		t.Fatal(err)
	}
	if gotAccept != contract["accept_header"][0] {
		t.Errorf("Accept = %q", gotAccept)
	}
	if gotPath != "/public/opr" {
		t.Errorf("path = %q", gotPath)
	}
	if gotQuery != "quote=rm&session=1700" {
		t.Errorf("query = %q", gotQuery)
	}
	if out.NewOPRLevel != expected.Data.NewOPRLevel {
		t.Errorf("decoded %v", out.NewOPRLevel)
	}
}

func TestNotFound(t *testing.T) {
	contract := clientContract(t)
	status, err := strconv.Atoi(contract["not_found_status"][0])
	if err != nil {
		t.Fatal(err)
	}
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(status)
	}))
	defer srv.Close()
	_, err = newTestClient(srv).Get(context.Background(), "exchange-rate/XXX", nil)
	if !errors.Is(err, ErrNotFound) {
		t.Fatalf("err = %v", err)
	}
}

func TestRetriesContractStatuses(t *testing.T) {
	contract := clientContract(t)
	retries, err := strconv.Atoi(contract["retry_attempts"][0])
	if err != nil {
		t.Fatal(err)
	}
	payload := contract["response_body"][0]
	for _, statusText := range contract["retry_status"] {
		status, err := strconv.Atoi(statusText)
		if err != nil {
			t.Fatal(err)
		}
		t.Run(statusText, func(t *testing.T) {
			var hits int32
			srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				if atomic.AddInt32(&hits, 1) <= int32(retries) {
					w.WriteHeader(status)
					return
				}
				fmt.Fprint(w, payload)
			}))
			defer srv.Close()
			if _, err := newTestClient(srv).Get(context.Background(), "retry", nil); err != nil {
				t.Fatal(err)
			}
			if hits != int32(retries+1) {
				t.Errorf("hits = %d, want %d", hits, retries+1)
			}
		})
	}
}

func TestOtherStatusIsAPIError(t *testing.T) {
	contract := clientContract(t)
	status, err := strconv.Atoi(contract["other_status"][0])
	if err != nil {
		t.Fatal(err)
	}
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(status)
	}))
	defer srv.Close()
	_, err := newTestClient(srv).Get(context.Background(), "opr", nil)
	var apiErr *APIError
	if !errors.As(err, &apiErr) || apiErr.Status != status {
		t.Fatalf("err = %v", err)
	}
}

func TestCleanPath(t *testing.T) {
	contract := clientContract(t)
	for _, bad := range contract["invalid_path"] {
		if _, err := CleanPath(bad); !errors.Is(err, ErrInvalidPath) {
			t.Errorf("%q accepted", bad)
		}
	}
	for _, good := range contract["valid_path"] {
		if _, err := CleanPath(good); err != nil {
			t.Errorf("%q rejected: %v", good, err)
		}
	}
}
