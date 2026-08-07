use serde_json::{Value, json};

pub(super) fn result_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["results"],
        "properties": {
            "results": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": [
                        "finding_id", "status", "evidence", "changed_paths", "validation",
                        "commit_oid", "canonical_id", "handoff_path"
                    ],
                    "properties": {
                        "finding_id": { "type": "string" },
                        "status": {
                            "enum": ["fixed", "stale", "rejected", "duplicate", "handed_off", "deferred"]
                        },
                        "evidence": { "type": "string" },
                        "changed_paths": {
                            "type": "array", "items": { "type": "string" }
                        },
                        "validation": { "type": "array", "items": { "type": "string" } },
                        "commit_oid": { "type": ["string", "null"] },
                        "canonical_id": { "type": ["string", "null"] },
                        "handoff_path": { "type": ["string", "null"] }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_only_supported_structured_output_keywords() {
        let schema = result_schema();
        let encoded = serde_json::to_string(&schema).unwrap();
        for unsupported in ["\"$schema\"", "\"minLength\"", "\"uniqueItems\""] {
            assert!(!encoded.contains(unsupported), "unsupported keyword {unsupported}: {encoded}");
        }
    }
}
