use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use regex::Regex;
use serde_json::Value;

use crate::{
    catalog::{Catalog, Skill},
    diagnostic::Diagnostic,
    frontmatter::Frontmatter,
    traversal::ScanRoot,
};

use super::{
    fix,
    model::{Counts, Finding, Fix, Report, RootRecord, SCHEMA_VERSION, Severity},
    resource::resource_target,
};

const COORDINATION_EXEMPT_SENTENCE: &str =
    "This skill is coordination-exempt: skip the ai-coord gate for its declared work.";

pub fn build_report(catalog: &Catalog, dependencies_only: bool, fix_safe: bool) -> Report {
    let mut findings = catalog
        .diagnostics
        .iter()
        .filter(|diagnostic| retained_shared_diagnostic(diagnostic, dependencies_only))
        .map(shared_finding)
        .collect::<Vec<_>>();
    let mut fixes = Vec::new();

    for skill in &catalog.skills {
        let Some(frontmatter) = skill.frontmatter.as_ref() else {
            continue;
        };
        let source = match fs::read_to_string(skill.skill_path()) {
            Ok(source) => source,
            Err(error) => {
                findings.push(Finding::new(
                    "SKILL_READ_ERROR",
                    Severity::Error,
                    skill.skill_path(),
                    None,
                    false,
                    format!("could not read SKILL.md: {error}"),
                ));
                continue;
            }
        };
        let raw = raw_frontmatter(&source);
        check_required_frontmatter(skill, frontmatter, dependencies_only, &mut findings);
        check_field_order(skill, frontmatter, dependencies_only, &mut findings);
        if dependencies_only {
            continue;
        }

        check_typed_frontmatter(skill, frontmatter, raw.as_ref(), &mut findings);
        check_coordination(skill, frontmatter, &source, &mut findings);
        check_openai(skill, frontmatter, fix_safe, &mut findings, &mut fixes);
        check_cli_version(skill, frontmatter, &mut findings);
        check_resource_links(skill, &source, &mut findings);
        check_prompt_hygiene(skill, raw.as_ref(), &source, &mut findings);
    }

    let roots = catalog.roots.iter().map(|root| root_record(root, &catalog.skills)).collect::<Vec<_>>();
    if !dependencies_only {
        for root in &catalog.roots {
            check_readme(root, &catalog.skills, &mut findings);
        }
    }

    findings.sort();
    findings.dedup();
    fixes.sort();
    fixes.dedup();
    let counts = counts(&findings, &fixes);
    Report { schema_version: SCHEMA_VERSION, roots, counts, findings, fixes }
}

fn retained_shared_diagnostic(diagnostic: &Diagnostic, dependencies_only: bool) -> bool {
    diagnostic.code.starts_with("FRONTMATTER_") || diagnostic.code.starts_with("SKILL_DEPENDENC") || !dependencies_only
}

fn shared_finding(diagnostic: &Diagnostic) -> Finding {
    Finding::new(
        diagnostic.code.clone(),
        Severity::Error,
        diagnostic.path.clone(),
        diagnostic.line,
        false,
        diagnostic.message.clone(),
    )
}

fn check_required_frontmatter(
    skill: &Skill,
    frontmatter: &Frontmatter,
    dependencies_only: bool,
    findings: &mut Vec<Finding>,
) {
    match frontmatter.name.as_ref() {
        None if !frontmatter.has_field("name") => findings.push(Finding::new(
            "NAME_MISSING",
            Severity::Error,
            skill.skill_path(),
            Some(2),
            false,
            "missing required name",
        )),
        None => findings.push(Finding::new(
            "NAME_INVALID",
            Severity::Error,
            skill.skill_path(),
            frontmatter.field("name").map(|field| field.line),
            false,
            "name must be a string",
        )),
        Some(name) => {
            if crate::dependency::SkillName::parse(&name.value).is_err() {
                findings.push(Finding::new(
                    "NAME_INVALID",
                    Severity::Error,
                    skill.skill_path(),
                    Some(name.line),
                    false,
                    "name must be 1-64 characters of canonical kebab-case",
                ));
            }
            if !dependencies_only && name.value != skill.directory_name {
                findings.push(Finding::new(
                    "NAME_DIRECTORY_MISMATCH",
                    Severity::Error,
                    skill.skill_path(),
                    Some(name.line),
                    false,
                    format!("name {:?} does not match directory {:?}", name.value, skill.directory_name),
                ));
            }
        }
    }

    match frontmatter.description.as_ref() {
        None if !frontmatter.has_field("description") => findings.push(Finding::new(
            "DESCRIPTION_MISSING",
            Severity::Error,
            skill.skill_path(),
            Some(2),
            false,
            "missing required description",
        )),
        None => findings.push(Finding::new(
            "DESCRIPTION_INVALID",
            Severity::Error,
            skill.skill_path(),
            frontmatter.field("description").map(|field| field.line),
            false,
            "description must be a string",
        )),
        Some(description) if description.value.trim().is_empty() => findings.push(Finding::new(
            "DESCRIPTION_MISSING",
            Severity::Error,
            skill.skill_path(),
            Some(description.line),
            false,
            "missing required description",
        )),
        Some(description) if !dependencies_only && description.value.chars().count() > 1024 => {
            findings.push(Finding::new(
                "DESCRIPTION_TOO_LONG",
                Severity::Error,
                skill.skill_path(),
                Some(description.line),
                false,
                format!("description is {} chars; max is 1024", description.value.chars().count()),
            ));
        }
        Some(_) => {}
    }
}

fn check_field_order(skill: &Skill, frontmatter: &Frontmatter, dependencies_only: bool, findings: &mut Vec<Finding>) {
    let actual = frontmatter.fields.iter().map(|field| field.name.clone()).collect::<Vec<_>>();
    let mut expected = actual.iter().filter(|name| name.as_str() != "description").cloned().collect::<Vec<_>>();
    expected.sort();
    if actual.iter().any(|name| name == "description") {
        expected.push("description".to_owned());
    }
    if actual == expected {
        return;
    }
    if frontmatter.has_field("skill-dependencies") {
        findings.push(Finding::new(
            "SKILL_DEPENDENCIES_FIELD_ORDER",
            Severity::Error,
            skill.skill_path(),
            Some(2),
            false,
            "frontmatter containing skill-dependencies must be alphabetized with description last",
        ));
    } else if !dependencies_only {
        findings.push(Finding::new(
            "FRONTMATTER_FIELD_ORDER",
            Severity::Warning,
            skill.skill_path(),
            Some(2),
            false,
            "frontmatter fields must be alphabetized with description last",
        ));
    }
}

fn check_typed_frontmatter(skill: &Skill, frontmatter: &Frontmatter, raw: Option<&Value>, findings: &mut Vec<Finding>) {
    check_string_field(
        skill,
        frontmatter,
        "argument-hint",
        frontmatter.argument_hint.is_some(),
        "ARGUMENT_HINT_INVALID",
        findings,
    );
    check_bool_field(
        skill,
        frontmatter,
        "user-invocable",
        frontmatter.user_invocable.is_some(),
        "USER_INVOCABLE_INVALID",
        findings,
    );
    check_bool_field(
        skill,
        frontmatter,
        "disable-model-invocation",
        frontmatter.disable_model_invocation.is_some(),
        "DISABLE_MODEL_INVOCATION_INVALID",
        findings,
    );
    check_string_field(skill, frontmatter, "context", frontmatter.context.is_some(), "CONTEXT_INVALID", findings);
    check_string_field(skill, frontmatter, "agent", frontmatter.agent.is_some(), "AGENT_INVALID", findings);
    check_string_field(
        skill,
        frontmatter,
        "coordination",
        frontmatter.coordination.is_some(),
        "COORDINATION_INVALID",
        findings,
    );

    if frontmatter.has_field("compatibility") {
        match frontmatter.compatibility.as_ref() {
            None => findings.push(typed_finding(
                skill,
                frontmatter,
                "compatibility",
                "COMPATIBILITY_INVALID",
                "compatibility must be a string",
            )),
            Some(value) if value.value.chars().count() > 500 => findings.push(Finding::new(
                "COMPATIBILITY_TOO_LONG",
                Severity::Error,
                skill.skill_path(),
                Some(value.line),
                false,
                format!("compatibility is {} chars; max is 500", value.value.chars().count()),
            )),
            Some(_) => {}
        }
    }

    if let Some(context) = frontmatter.context.as_ref() &&
        context.value != "fork"
    {
        findings.push(Finding::new(
            "CONTEXT_INVALID",
            Severity::Error,
            skill.skill_path(),
            Some(context.line),
            false,
            "context must be fork when present",
        ));
    }
    if let Some(coordination) = frontmatter.coordination.as_ref() &&
        coordination.value != "exempt"
    {
        findings.push(Finding::new(
            "COORDINATION_INVALID",
            Severity::Error,
            skill.skill_path(),
            Some(coordination.line),
            false,
            "coordination must be exempt when present",
        ));
    }
    if let Some(targets) = frontmatter.install_targets.as_ref() &&
        targets.value.is_none()
    {
        findings.push(Finding::new(
            "INSTALL_TARGETS_INVALID",
            Severity::Error,
            skill.skill_path(),
            Some(targets.line),
            false,
            "metadata.install-targets must be claude-code, codex, or claude-code codex",
        ));
    }
    if frontmatter.has_field("metadata") &&
        raw.and_then(Value::as_object)
            .and_then(|object| object.get("metadata"))
            .is_some_and(|metadata| !metadata.is_object())
    {
        findings.push(typed_finding(skill, frontmatter, "metadata", "METADATA_INVALID", "metadata must be a mapping"));
    }
}

fn check_string_field(
    skill: &Skill,
    frontmatter: &Frontmatter,
    name: &str,
    valid: bool,
    code: &str,
    findings: &mut Vec<Finding>,
) {
    if frontmatter.has_field(name) && !valid {
        findings.push(typed_finding(skill, frontmatter, name, code, format!("{name} must be a string")));
    }
}

fn check_bool_field(
    skill: &Skill,
    frontmatter: &Frontmatter,
    name: &str,
    valid: bool,
    code: &str,
    findings: &mut Vec<Finding>,
) {
    if frontmatter.has_field(name) && !valid {
        findings.push(typed_finding(skill, frontmatter, name, code, format!("{name} must be true or false")));
    }
}

fn typed_finding(
    skill: &Skill,
    frontmatter: &Frontmatter,
    field: &str,
    code: &str,
    message: impl Into<String>,
) -> Finding {
    Finding::new(
        code,
        Severity::Error,
        skill.skill_path(),
        frontmatter.field(field).map(|field| field.line),
        false,
        message,
    )
}

fn check_coordination(skill: &Skill, frontmatter: &Frontmatter, source: &str, findings: &mut Vec<Finding>) {
    let body_start = frontmatter_ranges(source).map_or(0, |(_, body)| body);
    let body = &source[body_start..];
    let normalized = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let mention = coordination_mention().find(body);
    let exact = normalized.contains(COORDINATION_EXEMPT_SENTENCE);
    let exempt = frontmatter.coordination.as_ref().is_some_and(|value| value.value == "exempt");

    if !exempt && let Some(mention) = mention.as_ref() {
        findings.push(Finding::new(
            "COORDINATION_EXEMPT_FRONTMATTER_MISSING",
            Severity::Error,
            skill.skill_path(),
            Some(line_at(source, body_start + mention.start())),
            false,
            "body declares coordination-exempt behavior but frontmatter does not set coordination: exempt",
        ));
    }
    if exempt && !exact && mention.is_none() {
        findings.push(Finding::new(
            "COORDINATION_EXEMPT_SENTENCE_MISSING",
            Severity::Error,
            skill.skill_path(),
            frontmatter.coordination.as_ref().map(|value| value.line),
            false,
            format!("coordination: exempt requires the canonical body sentence: {COORDINATION_EXEMPT_SENTENCE}"),
        ));
    } else if !exact && let Some(mention) = mention {
        findings.push(Finding::new(
            "COORDINATION_EXEMPT_SENTENCE_DRIFT",
            Severity::Error,
            skill.skill_path(),
            Some(line_at(source, body_start + mention.start())),
            false,
            format!(
                "coordination-exempt sentence differs from canonical text; expected: {COORDINATION_EXEMPT_SENTENCE}"
            ),
        ));
    }
}

fn check_openai(
    skill: &Skill,
    frontmatter: &Frontmatter,
    fix_safe: bool,
    findings: &mut Vec<Finding>,
    fixes: &mut Vec<Fix>,
) {
    if frontmatter.has_field("disable-model-invocation") && frontmatter.disable_model_invocation.is_none() {
        return;
    }
    let expected = !frontmatter.disable_model_invocation.as_ref().is_some_and(|value| value.value);
    let path = skill.exposure.exposure_directory().join("agents/openai.yaml");
    if !path.exists() {
        if !fix_safe {
            findings.push(Finding::new(
                "OPENAI_METADATA_MISSING",
                Severity::Error,
                &path,
                None,
                true,
                "missing agents/openai.yaml",
            ));
        } else if let Err(error) = fix::create_metadata(&path, expected) {
            findings.push(Finding::new(
                "OPENAI_METADATA_FIX_FAILED",
                Severity::FixError,
                &path,
                None,
                true,
                format!("failed to create openai.yaml: {error}"),
            ));
        } else {
            fixes.push(Fix::new("OPENAI_METADATA_CREATED", &path, "created agents/openai.yaml"));
        }
        return;
    }
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            findings.push(Finding::new(
                "OPENAI_METADATA_READ_ERROR",
                Severity::Error,
                &path,
                None,
                false,
                format!("could not read agents/openai.yaml: {error}"),
            ));
            return;
        }
    };
    let parsed: Value = match serde_saphyr::from_str(&source) {
        Ok(parsed) => parsed,
        Err(error) => {
            findings.push(Finding::new(
                "OPENAI_METADATA_INVALID",
                Severity::Error,
                &path,
                Some(1),
                false,
                format!("invalid YAML: {error}"),
            ));
            return;
        }
    };
    let Some(object) = parsed.as_object() else {
        findings.push(Finding::new(
            "OPENAI_METADATA_INVALID",
            Severity::Error,
            &path,
            Some(1),
            false,
            "agents/openai.yaml must be a mapping",
        ));
        return;
    };
    let actual = object
        .get("policy")
        .and_then(Value::as_object)
        .and_then(|policy| policy.get("allow_implicit_invocation"))
        .and_then(Value::as_bool);
    let Some(actual) = actual else {
        findings.push(Finding::new(
            "OPENAI_POLICY_MISSING",
            Severity::Error,
            &path,
            line_for(&source, "allow_implicit_invocation"),
            false,
            "missing boolean policy.allow_implicit_invocation",
        ));
        return;
    };
    if actual == expected {
        return;
    }
    if !fix_safe {
        findings.push(Finding::new(
            "OPENAI_POLICY_MISMATCH",
            Severity::Error,
            &path,
            line_for(&source, "allow_implicit_invocation"),
            true,
            format!("allow_implicit_invocation is {actual}, expected {expected}"),
        ));
    } else if let Err(error) = fix::update_policy(&path, &source, expected) {
        findings.push(Finding::new(
            "OPENAI_METADATA_FIX_FAILED",
            Severity::FixError,
            &path,
            None,
            true,
            format!("failed to update openai.yaml: {error}"),
        ));
    } else {
        fixes.push(Fix::new(
            "OPENAI_POLICY_UPDATED",
            &path,
            format!("updated allow_implicit_invocation to {expected}"),
        ));
    }
}

fn check_cli_version(skill: &Skill, frontmatter: &Frontmatter, findings: &mut Vec<Finding>) {
    let Some(name) = frontmatter.name.as_ref() else {
        return;
    };
    if !name.value.starts_with("cli-") {
        return;
    }
    let path = skill.exposure.exposure_directory().join("references/version.txt");
    if !path.exists() {
        findings.push(Finding::new(
            "CLI_VERSION_MISSING",
            Severity::Error,
            &path,
            None,
            false,
            "cli-* skill must maintain references/version.txt",
        ));
        return;
    }
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            findings.push(Finding::new(
                "CLI_VERSION_READ_ERROR",
                Severity::Error,
                &path,
                None,
                false,
                format!("could not read references/version.txt: {error}"),
            ));
            return;
        }
    };
    let lines = source.lines().collect::<Vec<_>>();
    if lines.len() != 1 || !semver().is_match(lines[0]) {
        findings.push(Finding::new(
            "CLI_VERSION_INVALID",
            Severity::Error,
            &path,
            Some(1),
            false,
            "references/version.txt must contain exactly one normalized semver line",
        ));
    }
}

fn check_resource_links(skill: &Skill, source: &str, findings: &mut Vec<Finding>) {
    for regex in [markdown_resource(), script_resource()] {
        for captures in regex.captures_iter(source) {
            let Some(reference) = captures.name("path") else {
                continue;
            };
            let raw = clean_reference(reference.as_str());
            if raw.is_empty() || raw.ends_with('/') || raw.bytes().any(|byte| matches!(byte, b'*' | b'{' | b'}')) {
                continue;
            }
            let Some(target) = resource_target(skill.exposure.exposure_directory(), raw) else {
                findings.push(Finding::new(
                    "RESOURCE_LINK_OUTSIDE_SKILL",
                    Severity::Error,
                    skill.skill_path(),
                    Some(line_at(source, reference.start())),
                    false,
                    format!("resource link must stay inside its skill directory: {raw}"),
                ));
                continue;
            };
            if target.exists() {
                continue;
            }
            findings.push(Finding::new(
                "RESOURCE_LINK_MISSING",
                Severity::Error,
                skill.skill_path(),
                Some(line_at(source, reference.start())),
                false,
                format!("referenced resource does not exist: {raw}"),
            ));
        }
    }
}

fn check_prompt_hygiene(skill: &Skill, raw: Option<&Value>, source: &str, findings: &mut Vec<Finding>) {
    if let Some(model) = raw.and_then(Value::as_object).and_then(|object| object.get("model")).and_then(Value::as_str) &&
        model.eq_ignore_ascii_case("opus")
    {
        findings.push(Finding::new(
            "STALE_MODEL_PIN",
            Severity::Warning,
            skill.skill_path(),
            line_for(source, "model:"),
            false,
            format!("model pin {model:?} is a stale alias; verify that an explicit pin is still needed"),
        ));
    }
    if !completion_evidence().is_match(source) {
        findings.push(Finding::new(
            "COMPLETION_EVIDENCE_MISSING",
            Severity::Warning,
            skill.skill_path(),
            None,
            false,
            "skill has no explicit completion, verification, validation, output, or report contract",
        ));
    }

    for captures in markdown_resource().captures_iter(source) {
        let Some(reference) = captures.name("path") else {
            continue;
        };
        let raw = clean_reference(reference.as_str());
        let Some(target) = resource_target(skill.exposure.exposure_directory(), raw) else {
            continue;
        };
        if target.extension().and_then(|extension| extension.to_str()) != Some("md") || !target.is_file() {
            continue;
        }
        let line_start = source[..reference.start()].rfind('\n').map_or(0, |offset| offset + 1);
        let line_end = source[reference.end()..].find('\n').map_or(source.len(), |offset| reference.end() + offset);
        let line = source[line_start..line_end].to_ascii_lowercase();
        let unconditional = line.contains("mandatory") ||
            line.contains("always read") ||
            (line.contains("read") && line.contains("before"));
        if !unconditional {
            continue;
        }
        let Ok(reference_source) = fs::read_to_string(&target) else {
            continue;
        };
        let lines = reference_source.lines().count();
        if lines >= 400 {
            findings.push(Finding::new(
                "UNCONDITIONAL_REFERENCE_OVERSIZED",
                Severity::Warning,
                skill.skill_path(),
                Some(line_at(source, reference.start())),
                false,
                format!("unconditional reference has {lines} lines: {raw}"),
            ));
        }
    }

    let requirements = authority_clauses(requirement(), source, true);
    let prohibitions = authority_clauses(prohibition(), source, false);
    'outer: for (required_offset, required) in &requirements {
        if required.len() < 3 {
            continue;
        }
        for (prohibited_offset, prohibited) in &prohibitions {
            if prohibited.len() < 3 {
                continue;
            }
            let union = required.union(prohibited).count();
            let overlap = required.intersection(prohibited).count();
            if union > 0 && overlap * 4 >= union * 3 {
                findings.push(Finding::new(
                    "CONFLICTING_AUTHORITY",
                    Severity::Warning,
                    skill.skill_path(),
                    Some(line_at(source, (*required_offset).min(*prohibited_offset))),
                    false,
                    "similar action appears in both requirement and prohibition language; review authority",
                ));
                break 'outer;
            }
        }
    }
}

fn check_readme(root: &ScanRoot, skills: &[Skill], findings: &mut Vec<Finding>) {
    let skills_directory = root.exposure_path.join("skills");
    if !skills_directory.is_dir() {
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
    let listed = readme_skills(&source);
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

fn readme_skills(source: &str) -> BTreeMap<String, u64> {
    let mut result = BTreeMap::new();
    let mut in_skills = false;
    for (index, line) in source.lines().enumerate() {
        if line.starts_with("## ") {
            in_skills = line.trim() == "## Skills";
            continue;
        }
        if !in_skills || !line.starts_with('|') {
            continue;
        }
        let cells = line.trim().trim_matches('|').split('|').map(str::trim).collect::<Vec<_>>();
        let Some(name) = cells.first() else {
            continue;
        };
        if *name != "Skill" && crate::dependency::SkillName::parse(name).is_ok() {
            result.entry((*name).to_owned()).or_insert(index as u64 + 1);
        }
    }
    result
}

fn root_record(root: &ScanRoot, skills: &[Skill]) -> RootRecord {
    let readme = root.exposure_path.join("README.md");
    RootRecord {
        path: root.exposure_path.clone(),
        active_skills: active_skills(root, skills).len(),
        readme: readme.exists().then_some(readme),
    }
}

fn active_skills(root: &ScanRoot, skills: &[Skill]) -> BTreeSet<String> {
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

fn counts(findings: &[Finding], fixes: &[Fix]) -> Counts {
    Counts {
        findings: findings.len(),
        errors: findings.iter().filter(|finding| finding.severity == Severity::Error).count(),
        warnings: findings.iter().filter(|finding| finding.severity == Severity::Warning).count(),
        fixable: findings.iter().filter(|finding| finding.fixable).count(),
        fixes: fixes.len(),
        fix_errors: findings.iter().filter(|finding| finding.severity == Severity::FixError).count(),
    }
}

fn raw_frontmatter(source: &str) -> Option<Value> {
    let (range, _) = frontmatter_ranges(source)?;
    serde_saphyr::from_str(&source[range]).ok()
}

fn frontmatter_ranges(source: &str) -> Option<(std::ops::Range<usize>, usize)> {
    let mut lines = source.split_inclusive('\n');
    let first = lines.next()?;
    if first.trim_end_matches(['\n', '\r']).trim_end() != "---" {
        return None;
    }
    let yaml_start = first.len();
    let mut offset = yaml_start;
    for line in lines {
        let content = line.trim_end_matches(['\n', '\r']).trim_end();
        if content == "---" {
            return Some((yaml_start..offset, offset + line.len()));
        }
        offset += line.len();
    }
    None
}

fn clean_reference(reference: &str) -> &str {
    reference
        .split(['#', '?'])
        .next()
        .unwrap_or_default()
        .trim_end_matches(|character| matches!(character, '.' | ',' | ';' | ':' | ')' | ']' | '}' | '\'' | '"'))
}

fn line_for(source: &str, needle: &str) -> Option<u64> {
    source.find(needle).map(|offset| line_at(source, offset))
}

fn line_at(source: &str, offset: usize) -> u64 {
    source.as_bytes()[..offset.min(source.len())].iter().filter(|byte| **byte == b'\n').count() as u64 + 1
}

fn authority_clauses(regex: &Regex, source: &str, exclude_leading_not: bool) -> Vec<(usize, BTreeSet<String>)> {
    regex
        .captures_iter(source)
        .filter_map(|captures| {
            let whole = captures.get(0)?;
            let clause = captures.get(1)?;
            if exclude_leading_not && clause.as_str().trim_start().to_ascii_lowercase().starts_with("not ") {
                return None;
            }
            let words = word().find_iter(clause.as_str()).take(10).filter_map(|word| {
                let word = word.as_str().to_ascii_lowercase();
                (!matches!(word.as_str(), "the" | "a" | "an" | "to" | "of" | "and" | "or" | "when" | "if"))
                    .then_some(word)
            });
            Some((whole.start(), words.collect()))
        })
        .collect()
}

fn coordination_mention() -> &'static Regex {
    static VALUE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r"(?i)\bThis\s+skill\s+is\s+coordination-exempt\b").unwrap())
}

fn semver() -> &'static Regex {
    static VALUE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$").unwrap())
}

fn markdown_resource() -> &'static Regex {
    static VALUE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r"\]\((?P<path>(?:references|scripts|assets)/[^)\s]+)\)").unwrap())
}

fn script_resource() -> &'static Regex {
    static VALUE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| {
        Regex::new(r"(?:^|[^A-Za-z0-9_./-])uv run (?P<path>scripts/[A-Za-z0-9][A-Za-z0-9._/-]*)").unwrap()
    })
}

fn completion_evidence() -> &'static Regex {
    static VALUE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r"(?im)^##+[^\n]*\b(?:completion|verify|verification|validation|output|report|result|exit codes)\b|\b(?:success means|complete when|completion requires|completion is|finish with)\b").unwrap())
}

fn requirement() -> &'static Regex {
    static VALUE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r"(?i)\b(?:always|must|required to)\s+([^.!?\n]+)").unwrap())
}

fn prohibition() -> &'static Regex {
    static VALUE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r"(?i)\b(?:do not|don't|never|must not|forbid(?:s|den)?)\s+([^.!?\n]+)").unwrap())
}

fn word() -> &'static Regex {
    static VALUE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    VALUE.get_or_init(|| Regex::new(r"[a-zA-Z0-9_-]+").unwrap())
}
