#!/usr/bin/env python3
"""Apply types.rs patch to add cnb_registry field to AppState."""
import sys

P = r"C:\_dev\fileit\src-tauri\src\types.rs"
s = open(P, encoding="utf-8").read()

# Add CnbRegistry struct definition
if "pub struct CnbRegistry" not in s:
    struct_pos = s.find("pub struct InstanceMatch {")
    if struct_pos != -1:
        cnb_registry_code = """
/// CNB institution classification result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CnbInstitution {
    /// Canonical institution name from CNB registry.
    pub canonical_name: String,
    /// Institution category.
    pub category: InstitutionCategory,
    /// Confidence 0.0\u20131.0.
    pub confidence: f32,
}

/// CNB registry response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CnbRegistryResponse {
    /// Institutions found.
    pub institutions: Vec<CnbInstitution>,
}

/// CNB registry entry for storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CnbRegistryEntry {
    /// Institution name.
    pub name: String,
    /// Institution category.
    pub category: InstitutionCategory,
    /// Priority/order.
    pub priority: u32,
    /// Institution metadata.
    pub metadata: serde_json::Value,
}

/// CNB registry.
#[derive(Debug, Clone)]
pub struct CnbRegistry {
    /// Institutions loaded.
    pub institutions: Vec<CnbRegistryEntry>,
    /// Registry metadata.
    pub metadata: serde_json::Value,
}

impl CnbRegistry {
    /// Create a new CNB registry.
    pub fn new(institutions: Vec<CnbRegistryEntry>, metadata: serde_json::Value) -> Self {
        Self { institutions, metadata }
    }

    /// Get the number of institutions.
    pub fn count(&self) -> usize {
        self.institutions.len()
    }
}

"""
        s = s[:struct_pos] + cnb_registry_code + s[struct_pos:]

open(P, "w", encoding="utf-8").write(s)
print("types.rs patch applied successfully")