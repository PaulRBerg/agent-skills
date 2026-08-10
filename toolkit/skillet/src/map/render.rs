use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use crate::cli::MapFormat;
use crate::error::Error;

use super::model::{EdgeType, MapReport, Provenance};

pub fn render(report: &MapReport, format: MapFormat) -> Result<String, Error> {
    match format {
        MapFormat::Text => Ok(render_text(report)),
        MapFormat::Json => {
            let mut output = serde_json::to_string_pretty(report)
                .map_err(|error| Error::Serialization(error.to_string()))?;
            output.push('\n');
            Ok(output)
        }
        MapFormat::Dot => Ok(render_dot(report)),
    }
}

fn render_text(report: &MapReport) -> String {
    let mut output = String::new();
    writeln!(output, "Skill Map").unwrap();
    writeln!(output, "Schema version: {}", report.schema_version).unwrap();
    writeln!(output, "Roots:").unwrap();
    for root in &report.roots {
        writeln!(
            output,
            "- {} -> {} ({:?})",
            display_path(&root.exposure_path),
            display_path(&root.resolved_path),
            root.mode
        )
        .unwrap();
    }
    writeln!(output, "Skills: {}", report.counts.skills).unwrap();
    writeln!(output, "Edges: {}", report.counts.edges).unwrap();
    writeln!(output, "Duplicates: {}", report.counts.duplicates).unwrap();
    writeln!(output, "Unresolved: {}", report.counts.unresolved).unwrap();

    if let Some(portfolio) = &report.portfolio {
        writeln!(output, "\nPortfolio:").unwrap();
        writeln!(output, "- repository: {}", display_path(&portfolio.repository_root)).unwrap();
        for root in &portfolio.user_roots {
            writeln!(
                output,
                "- {}: {} ({})",
                root.client,
                display_path(&root.path),
                if root.present { "present" } else { "missing" }
            )
            .unwrap();
        }
    }

    if !report.skills.is_empty() {
        writeln!(output, "\nSkills:").unwrap();
        for skill in &report.skills {
            writeln!(
                output,
                "- {} [{}; {:?}; {:?}] {} -> {}",
                skill.name,
                skill.clients.join(","),
                skill.location,
                skill.kind,
                display_path(&skill.exposure_path),
                display_path(&skill.resolved_path)
            )
            .unwrap();
        }
    }

    let dependencies: Vec<_> =
        report.edges.iter().filter(|edge| edge.edge_type == EdgeType::Dependency).collect();
    if !dependencies.is_empty() {
        writeln!(output, "\nDependencies:").unwrap();
        for edge in dependencies {
            writeln!(
                output,
                "- {} -> {} ({}; {}:{})",
                edge.source.as_deref().unwrap_or("<external>"),
                edge.target,
                match edge.provenance {
                    Provenance::Declared => "declared",
                    Provenance::Inferred => "inferred",
                },
                display_path(&edge.path),
                edge.line
            )
            .unwrap();
            if let Some(snippet) = &edge.snippet {
                writeln!(output, "  {}", display_string(snippet)).unwrap();
            }
        }
    }

    let external: Vec<_> =
        report.edges.iter().filter(|edge| edge.edge_type == EdgeType::ExternalReference).collect();
    if !external.is_empty() {
        writeln!(output, "\nExternal references:").unwrap();
        for edge in external {
            writeln!(output, "- {} ({}:{})", edge.target, display_path(&edge.path), edge.line)
                .unwrap();
            if let Some(snippet) = &edge.snippet {
                writeln!(output, "  {}", display_string(snippet)).unwrap();
            }
        }
    }

    if !report.duplicates.is_empty() {
        writeln!(output, "\nDuplicate installs:").unwrap();
        for duplicate in &report.duplicates {
            writeln!(output, "- {}", duplicate.name).unwrap();
            for path in &duplicate.exposure_paths {
                writeln!(output, "  {}", display_path(path)).unwrap();
            }
        }
    }

    if !report.unresolved.is_empty() {
        writeln!(output, "\nUnresolved skill-like references:").unwrap();
        for edge in &report.unresolved {
            let source =
                edge.source.as_deref().map(|source| format!("{source} -> ")).unwrap_or_default();
            writeln!(
                output,
                "- {}{} ({}:{})",
                source,
                edge.target,
                display_path(&edge.path),
                edge.line
            )
            .unwrap();
            if let Some(snippet) = &edge.snippet {
                writeln!(output, "  {}", display_string(snippet)).unwrap();
            }
        }
    }

    if report.duplicates.is_empty() && report.edges.is_empty() && report.unresolved.is_empty() {
        writeln!(output, "\nNo cross-references found.").unwrap();
    }

    if let Some(skipped) = &report.skipped {
        writeln!(output, "\nSkipped paths:").unwrap();
        for value in skipped
            .directories
            .iter()
            .chain(&skipped.files)
            .chain(&skipped.macos_protected_home_paths)
            .chain(&skipped.always_ignored_home_paths)
            .chain(&skipped.broad_scan_cache_paths)
            .chain(&skipped.catalog_sources)
        {
            writeln!(output, "- {}", display_string(value)).unwrap();
        }
    }

    output
}

fn render_dot(report: &MapReport) -> String {
    let mut output = String::from("digraph skill_map {\n  rankdir=\"LR\";\n");
    let mut pairs = BTreeSet::new();
    for edge in &report.edges {
        if edge.edge_type != EdgeType::Dependency {
            continue;
        }
        let Some(source) = edge.source.as_ref() else {
            continue;
        };
        pairs.insert((source.clone(), edge.target.clone()));
    }
    for (source, target) in pairs {
        writeln!(output, "  {} -> {};", dot_quote(&source), dot_quote(&target)).unwrap();
    }
    for duplicate in &report.duplicates {
        writeln!(output, "  {} [shape=box, style=\"dashed\"];", dot_quote(&duplicate.name))
            .unwrap();
    }
    output.push_str("}\n");
    output
}

fn display_path(path: &Path) -> String {
    display_string(&path.to_string_lossy())
}

fn display_string(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string cannot fail")
}

fn dot_quote(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character => output.push(character),
        }
    }
    output.push('"');
    output
}
