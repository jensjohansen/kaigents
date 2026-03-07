// File: operator/internal/version/version_test.go
// Purpose: Unit tests for the operator version constant.
// Product/business importance: ensures the operator version is set for supportability.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package version

import "testing"

func TestVersionIsNonEmpty(t *testing.T) {
	if Version == "" {
		t.Fatal("Version must not be empty")
	}
}
