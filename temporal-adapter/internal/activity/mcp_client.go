// File: kaigents/temporal-adapter/internal/activity/mcp_client.go
// Purpose: Minimal MCP (Model Context Protocol) HTTP client for invoking tools
// (web search, URL reading) from within Temporal activities.
// Mirrors the protocol used by the Rust HttpMcpClient in kaigents-core.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package activity

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"sync"
	"time"
)

// McpClient is a minimal MCP HTTP-transport client that supports initialize
// and tools/call. It mirrors the protocol used by the Rust HttpMcpClient.
type McpClient struct {
	endpoint   string
	httpClient *http.Client
	sessionID  string
	mu         sync.Mutex
}

// NewMcpClient creates a new MCP client pointing at the given HTTP endpoint.
func NewMcpClient(endpoint string) *McpClient {
	return &McpClient{
		endpoint:   endpoint,
		httpClient: &http.Client{Timeout: 60 * time.Second},
	}
}

// Initialize performs the MCP handshake and stores the session ID.
func (c *McpClient) Initialize(ctx context.Context) error {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.sessionID != "" {
		return nil
	}

	reqBody, _ := json.Marshal(map[string]interface{}{
		"jsonrpc": "2.0",
		"id":      1,
		"method":  "initialize",
		"params": map[string]interface{}{
			"protocolVersion": "2024-11-05",
			"capabilities":    map[string]interface{}{},
			"clientInfo": map[string]interface{}{
				"name":    "kaigents-temporal-adapter",
				"version": "0",
			},
		},
	})

	req, err := http.NewRequestWithContext(ctx, "POST", c.endpoint, bytes.NewBuffer(reqBody))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json, text/event-stream")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("MCP initialize HTTP error: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("MCP initialize failed (%d): %s", resp.StatusCode, string(body))
	}

	if sid := resp.Header.Get("mcp-session-id"); sid != "" {
		c.sessionID = sid
	} else {
		body, _ := io.ReadAll(resp.Body)
		var result map[string]interface{}
		if err := json.Unmarshal(body, &result); err == nil {
			if sid, ok := result["_mcp_session_id"].(string); ok {
				c.sessionID = sid
			}
		}
		if c.sessionID == "" {
			return fmt.Errorf("MCP initialize missing mcp-session-id header")
		}
	}

	return nil
}

// CallTool invokes an MCP tool by name with the given arguments.
// Returns the raw result content as a string.
func (c *McpClient) CallTool(ctx context.Context, toolName string, arguments map[string]interface{}) (string, error) {
	if err := c.Initialize(ctx); err != nil {
		return "", err
	}

	c.mu.Lock()
	sessionID := c.sessionID
	c.mu.Unlock()

	reqBody, _ := json.Marshal(map[string]interface{}{
		"jsonrpc": "2.0",
		"id":      1,
		"method":  "tools/call",
		"params": map[string]interface{}{
			"name":      toolName,
			"arguments": arguments,
		},
	})

	req, err := http.NewRequestWithContext(ctx, "POST", c.endpoint, bytes.NewBuffer(reqBody))
	if err != nil {
		return "", err
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json, text/event-stream")
	if sessionID != "" {
		req.Header.Set("mcp-session-id", sessionID)
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return "", fmt.Errorf("MCP tools/call HTTP error: %w", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("MCP body read error: %w", err)
	}

	contentType := resp.Header.Get("Content-Type")
	var resultJSON map[string]interface{}

	if strings.HasPrefix(contentType, "text/event-stream") {
		data, err := parseSSEData(string(body))
		if err != nil {
			return "", fmt.Errorf("MCP SSE parse error: %w", err)
		}
		if err := json.Unmarshal([]byte(data), &resultJSON); err != nil {
			return "", fmt.Errorf("MCP SSE JSON decode error: %w (data=%s)", err, data)
		}
	} else {
		if err := json.Unmarshal(body, &resultJSON); err != nil {
			return "", fmt.Errorf("MCP JSON decode error: %w (body=%s)", err, string(body))
		}
	}

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("MCP tools/call HTTP status %d: %s", resp.StatusCode, string(body))
	}

	if errVal, ok := resultJSON["error"]; ok {
		return "", fmt.Errorf("MCP JSON-RPC error: %v", errVal)
	}

	result, ok := resultJSON["result"].(map[string]interface{})
	if !ok {
		return "", fmt.Errorf("MCP missing result field: %s", string(body))
	}

	content, ok := result["content"].([]interface{})
	if !ok || len(content) == 0 {
		resultBytes, _ := json.Marshal(result)
		return string(resultBytes), nil
	}

	first, ok := content[0].(map[string]interface{})
	if !ok {
		resultBytes, _ := json.Marshal(result)
		return string(resultBytes), nil
	}

	if text, ok := first["text"].(string); ok {
		return text, nil
	}

	resultBytes, _ := json.Marshal(result)
	return string(resultBytes), nil
}

func truncate(s string, maxLen int) string {
	if len(s) <= maxLen {
		return s
	}
	return s[:maxLen] + "..."
}

// parseSSEData extracts the last data frame from an SSE response body.
func parseSSEData(body string) (string, error) {
	var lastData string
	for _, line := range strings.Split(body, "\n") {
		trimmed := strings.TrimSpace(line)
		if strings.HasPrefix(trimmed, "data:") {
			lastData = strings.TrimSpace(strings.TrimPrefix(trimmed, "data:"))
		}
	}
	if lastData == "" {
		return "", fmt.Errorf("no data frame in SSE response (first 200 bytes: %s)", truncate(body, 200))
	}
	return lastData, nil
}
