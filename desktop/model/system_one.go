package model

import (
	"math"
	"strconv"
	"strings"
)

// SystemOneKind is the review provider System One is picked under. It is not a chat provider: no
// bot runs on it, and it is absent from the provider credentials.
const SystemOneKind = "systemone"

// DefaultSystemOneModel is the model id TypeSafe serves System One under.
const DefaultSystemOneModel = "jev-latest"

// DefaultReviewThreshold is the probability of yes System One needs to run an action unasked.
const DefaultReviewThreshold = 0.9

// MinReviewThreshold is the lowest threshold a user may set.
const MinReviewThreshold = 0.5

// SystemOneService is a root System One is served from.
type SystemOneService struct {
	Title string
	URL   string
}

// SystemOneServices are TypeSafe's and OpenRouter's roots.
var SystemOneServices = []SystemOneService{
	{Title: "TypeSafe", URL: "https://api.typesafe.ai"},
	{Title: "OpenRouter", URL: "https://openrouter.ai/api"},
}

// SystemOne is System One as the account has it: its root, its model, and its key masked.
type SystemOne struct {
	BaseURL string
	Model   string
	Detail  string
}

// SystemOneCredentials is what the sheet saves. A blank key keeps the saved one.
type SystemOneCredentials struct {
	BaseURL string
	APIKey  string
	Model   string
}

// ToSystemOne is the account's System One, or nil when none is connected.
func ToSystemOne(wire *WireSystemOne) *SystemOne {
	if wire == nil {
		return nil
	}
	return &SystemOne{BaseURL: wire.BaseURL, Model: wire.Model, Detail: wire.Detail}
}

// ParseReviewThreshold reads a typed threshold: empty is nil, the default, and anything else must
// be a number from the lowest threshold to 1. The second result is false for a value it refuses.
func ParseReviewThreshold(text string) (*float64, bool) {
	text = strings.TrimSpace(text)
	if text == "" {
		return nil, true
	}
	value, err := strconv.ParseFloat(text, 64)
	if err != nil || math.IsNaN(value) || value < MinReviewThreshold || value > 1 {
		return nil, false
	}
	return &value, true
}

// SameThreshold reports whether two thresholds, each possibly unset, are the same.
func SameThreshold(a, b *float64) bool {
	if a == nil || b == nil {
		return a == b
	}
	return *a == *b
}
