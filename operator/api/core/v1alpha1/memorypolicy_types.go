// File: operator/api/core/v1alpha1/memorypolicy_types.go
// Purpose: Defines the MemoryPolicy CRD schema for Kaigents.
// Product/business importance: Governs memory retention, tiering, and isolation for a workspace.
//
// Copyright (c) 2026 John K Johansen
// License: MIT (see LICENSE)

package v1alpha1

import (
	"k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
)

// MemoryRetentionPolicy defines TTLs for different memory tiers.
type MemoryRetentionPolicy struct {
	ShortTermTtlSeconds int32 `json:"shortTermTtlSeconds,omitempty"`
	LongTermTtlDays     int32 `json:"longTermTtlDays,omitempty"`
	ArchivalTtlDays     int32 `json:"archivalTtlDays,omitempty"`
}

// MemoryCapacityPolicy defines storage limits.
type MemoryCapacityPolicy struct {
	MaxShortTermBytes string `json:"maxShortTermBytes,omitempty"`
	MaxEpisodes       int64  `json:"maxEpisodes,omitempty"`
}

// MemoryGovernancePolicy defines PII and isolation settings.
type MemoryGovernancePolicy struct {
	PiiScrubbing   bool     `json:"piiScrubbing,omitempty"`
	PiiCategories  []string `json:"piiCategories,omitempty"`
	IsolationLevel string   `json:"isolationLevel,omitempty"`
}

// MemoryPolicySpec defines the desired state of a MemoryPolicy.
type MemoryPolicySpec struct {
	Retention  MemoryRetentionPolicy  `json:"retention,omitempty"`
	Capacity   MemoryCapacityPolicy   `json:"capacity,omitempty"`
	Governance MemoryGovernancePolicy `json:"governance,omitempty"`
}

// MemoryPolicyStatus defines the observed state of a MemoryPolicy.
type MemoryPolicyStatus struct {
	ObservedGeneration int64       `json:"observedGeneration,omitempty"`
	Phase              string      `json:"phase,omitempty"`
	Conditions         []Condition `json:"conditions,omitempty"`
	UsageBytes         int64       `json:"usageBytes,omitempty"`
}

// MemoryPolicy is the schema for Kaigents MemoryPolicy resources.
type MemoryPolicy struct {
	v1.TypeMeta   `json:",inline"`
	v1.ObjectMeta `json:"metadata,omitempty"`

	Spec   MemoryPolicySpec   `json:"spec,omitempty"`
	Status MemoryPolicyStatus `json:"status,omitempty"`
}

// MemoryPolicyList contains a list of MemoryPolicy resources.
type MemoryPolicyList struct {
	v1.TypeMeta `json:",inline"`
	v1.ListMeta `json:"metadata,omitempty"`

	Items []MemoryPolicy `json:"items"`
}

// DeepCopyObject copies the MemoryPolicy for runtime.Object.
func (in *MemoryPolicy) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(MemoryPolicy)
	*out = *in
	out.ObjectMeta = *in.ObjectMeta.DeepCopy()
	if in.Status.Conditions != nil {
		out.Status.Conditions = append([]Condition(nil), in.Status.Conditions...)
	}
	if in.Spec.Governance.PiiCategories != nil {
		out.Spec.Governance.PiiCategories = append([]string(nil), in.Spec.Governance.PiiCategories...)
	}
	return out
}

// DeepCopyObject copies the MemoryPolicyList for runtime.Object.
func (in *MemoryPolicyList) DeepCopyObject() runtime.Object {
	if in == nil {
		return nil
	}
	out := new(MemoryPolicyList)
	*out = *in
	if in.Items != nil {
		out.Items = make([]MemoryPolicy, len(in.Items))
		for i := range in.Items {
			out.Items[i] = *in.Items[i].DeepCopyObject().(*MemoryPolicy)
		}
	}
	return out
}
