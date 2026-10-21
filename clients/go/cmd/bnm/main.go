// Command bnm queries the BNM Open API from the shell.
//
//	bnm get opr
//	bnm get exchange-rate -q session=1700 -q quote=rm
//
// Exit codes: 0 ok, 1 bad usage, 2 not found, 3 HTTP error, 4 network error.
package main

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"net/url"
	"os"
	"strings"

	bnm "github.com/YOUR-USERNAME/bnm-open-api-skill/clients/go"
)

type queryFlags []string

func (q *queryFlags) String() string     { return strings.Join(*q, ",") }
func (q *queryFlags) Set(v string) error { *q = append(*q, v); return nil }

func main() { os.Exit(run(os.Args[1:])) }

func run(args []string) int {
	if len(args) < 2 || args[0] != "get" {
		fmt.Fprintln(os.Stderr, "usage: bnm get PATH [-q key=value]... [-data-only]")
		return 1
	}
	fs := flag.NewFlagSet("get", flag.ContinueOnError)
	var q queryFlags
	fs.Var(&q, "q", "query parameter key=value (repeatable)")
	dataOnly := fs.Bool("data-only", false, "print only the data field")
	baseURL := fs.String("base-url", "", "override base URL")
	path := args[1]
	if err := fs.Parse(args[2:]); err != nil {
		return 1
	}

	values := url.Values{}
	for _, pair := range q {
		k, v, ok := strings.Cut(pair, "=")
		if !ok {
			fmt.Fprintf(os.Stderr, "error: query must look like key=value, got %q\n", pair)
			return 1
		}
		values.Add(k, v)
	}

	c := bnm.New()
	if *baseURL != "" {
		c.BaseURL = *baseURL
	}
	resp, err := c.Get(context.Background(), path, values)
	var apiErr *bnm.APIError
	switch {
	case errors.Is(err, bnm.ErrInvalidPath):
		fmt.Fprintln(os.Stderr, "error:", err)
		return 1
	case errors.Is(err, bnm.ErrNotFound):
		fmt.Fprintln(os.Stderr, "not found:", err)
		return 2
	case errors.As(err, &apiErr):
		fmt.Fprintln(os.Stderr, "error:", err)
		return 3
	case err != nil:
		fmt.Fprintln(os.Stderr, "network error:", err)
		return 4
	}

	out := []byte(resp.Raw)
	if *dataOnly && len(resp.Data) > 0 {
		out = resp.Data
	}
	var pretty bytes.Buffer
	if json.Indent(&pretty, out, "", "  ") != nil {
		pretty.Reset()
		pretty.Write(out)
	}
	fmt.Println(pretty.String())
	return 0
}
