use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use crate::{catalog::Skill, dependency::SkillName, traversal::ScanRoot};

use super::{Finding, RootRecord, Severity};

pub(super) fn check(root: &ScanRoot, skills: &[&Skill], findings: &mut Vec<Finding>) {
    if matches!(root.exposure_path.file_name().and_then(|name| name.to_str()), Some(".agents" | ".claude" | ".codex")) {
        return;
    }
    if !root.exposure_path.join("skills").is_dir() {
        return;
    }
    let active = active_skills(root, skills);
    let path = root.exposure_path.join("README.md");
    if !path.exists() {
        findings.push(Finding::new(
            "README_MISSING",
            Severity::Error,
            &path,
            None,
            false,
            "catalog root is missing README.md",
        ));
        return;
    }
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            findings.push(Finding::new(
                "README_READ_ERROR",
                Severity::Error,
                &path,
                None,
                false,
                format!("could not read README.md: {error}"),
            ));
            return;
        }
    };
    let Some(listed) = listed_skills(&source) else {
        findings.push(Finding::new(
            "README_SKILLS_TABLE_INVALID",
            Severity::Error,
            &path,
            None,
            false,
            "README ## Skills section must contain a Markdown table whose first column is Skill",
        ));
        return;
    };
    for name in active.difference(&listed.keys().cloned().collect()) {
        findings.push(Finding::new(
            "README_SKILL_MISSING",
            Severity::Error,
            &path,
            None,
            false,
            format!("active skill missing from README table: {name}"),
        ));
    }
    for (name, line) in listed {
        if !active.contains(&name) {
            findings.push(Finding::new(
                "README_LISTS_MISSING",
                Severity::Error,
                &path,
                Some(line),
                false,
                format!("README lists missing skill: {name}"),
            ));
        }
    }
}

pub(super) fn root_record(root: &ScanRoot, skills: &[&Skill]) -> RootRecord {
    let readme = root.exposure_path.join("README.md");
    RootRecord {
        path: root.exposure_path.clone(),
        active_skills: active_skills(root, skills).len(),
        readme: readme.exists().then_some(readme),
    }
}

fn listed_skills(source: &str) -> Option<BTreeMap<String, u64>> {
    let mut result = BTreeMap::new();
    let mut in_skills = false;
    let mut header_columns = None;
    let mut in_table = false;
    for (index, line) in source.lines().enumerate() {
        if line.starts_with("## ") {
            in_skills = line.trim() == "## Skills";
            if !in_skills && in_table {
                break;
            }
            continue;
        }
        if !in_skills {
            continue;
        }
        let Some(cells) = table_cells(line) else {
            if in_table && !line.trim().is_empty() {
                break;
            }
            continue;
        };
        let Some(name) = cells.first() else {
            continue;
        };
        if header_columns.is_none() {
            if *name == "Skill" {
                header_columns = Some(cells.len());
            }
            continue;
        }
        if !in_table {
            if header_columns == Some(cells.len()) && cells.iter().all(|cell| markdown_table_separator(cell)) {
                in_table = true;
            } else {
                header_columns = None;
            }
            continue;
        }
        if SkillName::parse(name).is_ok() {
            result.entry((*name).to_owned()).or_insert(index as u64 + 1);
        }
    }
    in_table.then_some(result)
}

fn table_cells(line: &str) -> Option<Vec<&str>> {
    line.contains('|').then(|| line.trim().trim_matches('|').split('|').map(str::trim).collect())
}

fn markdown_table_separator(cell: &str) -> bool {
    let cell = cell.trim_matches(':');
    cell.len() >= 3 && cell.bytes().all(|byte| byte == b'-')
}

fn active_skills(root: &ScanRoot, skills: &[&Skill]) -> BTreeSet<String> {
    skills
        .iter()
        .filter(|skill| skill_belongs_to_root(root, skill.skill_path()))
        .map(|skill| skill.directory_name.clone())
        .collect()
}

fn skill_belongs_to_root(root: &ScanRoot, path: &Path) -> bool {
    if path == root.exposure_path.join("SKILL.md") {
        return true;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    parent.parent() == Some(root.exposure_path.join("skills").as_path()) ||
        (root.exposure_path.file_name().and_then(|name| name.to_str()) == Some("skills") &&
            parent.parent() == Some(root.exposure_path.as_path()))
}
