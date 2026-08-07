use serde_json::{Value, json};

pub(super) fn result_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
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
                        "finding_id": { "type": "string", "minLength": 1 },
                        "status": {
                            "enum": ["fixed", "stale", "rejected", "duplicate", "handed_off", "deferred"]
                        },
                        "evidence": { "type": "string", "minLength": 1 },
                        "changed_paths": {
                            "type": "array", "items": { "type": "string" }, "uniqueItems": true
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
