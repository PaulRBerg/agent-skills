use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::Path,
    str::FromStr,
};

use serde::Serialize;

use crate::{
    diagnostic::{Diagnostic, sort_diagnostics},
    frontmatter::{DependencyList, Frontmatter, Located},
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct SkillName(String);

impl SkillName {
    pub fn parse(value: &str) -> Result<Self, InvalidSkillName> {
        let valid = !value.is_empty() &&
            value.len() <= 64 &&
            value.split('-').all(|part| {
                !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            });
        if valid { Ok(Self(value.to_owned())) } else { Err(InvalidSkillName) }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SkillName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for SkillName {
    type Err = InvalidSkillName;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidSkillName;

impl fmt::Display for InvalidSkillName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("expected a 1-64 character canonical kebab-case skill name")
    }
}

impl std::error::Error for InvalidSkillName {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ExternalDependency {
    pub owner: String,
    pub repository: String,
    pub skill: SkillName,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum DependencyIdentifier {
    Internal(SkillName),
    External(ExternalDependency),
}

impl DependencyIdentifier {
    pub fn parse(value: &str) -> Result<Self, InvalidDependencyIdentifier> {
        if !value.contains('/') && !value.contains('#') {
            return SkillName::parse(value).map(Self::Internal).map_err(|_| InvalidDependencyIdentifier);
        }

        let (repository_path, skill) =
            value.split_once('#').filter(|(_, skill)| !skill.contains('#')).ok_or(InvalidDependencyIdentifier)?;
        let (owner, repository) = repository_path
            .split_once('/')
            .filter(|(owner, repository)| !owner.contains('/') && !repository.contains('/'))
            .ok_or(InvalidDependencyIdentifier)?;
        if !valid_repository_component(owner) || !valid_repository_component(repository) || repository.ends_with(".git")
        {
            return Err(InvalidDependencyIdentifier);
        }
        Ok(Self::External(ExternalDependency {
            owner: owner.to_owned(),
            repository: repository.to_owned(),
            skill: SkillName::parse(skill).map_err(|_| InvalidDependencyIdentifier)?,
        }))
    }

    pub fn target_name(&self) -> &SkillName {
        match self {
            Self::Internal(name) => name,
            Self::External(external) => &external.skill,
        }
    }

    pub fn as_identifier(&self) -> String {
        match self {
            Self::Internal(name) => name.to_string(),
            Self::External(external) => {
                format!("{}/{}#{}", external.owner, external.repository, external.skill)
            }
        }
    }
}

impl fmt::Display for DependencyIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.as_identifier())
    }
}

fn valid_repository_component(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty() &&
        bytes.first().is_some_and(u8::is_ascii_alphanumeric) &&
        bytes.iter().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidDependencyIdentifier;

impl fmt::Display for InvalidDependencyIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("expected SKILL or ORG/REPO#SKILL")
    }
}

impl std::error::Error for InvalidDependencyIdentifier {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DeclaredDependency {
    pub identifier: DependencyIdentifier,
    pub line: u64,
    pub column: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DependencyValidation {
    pub dependencies: Vec<DeclaredDependency>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn validate_dependencies(
    path: &Path,
    source_name: Option<&SkillName>,
    frontmatter: &Frontmatter,
    known_local_names: &BTreeSet<SkillName>,
) -> DependencyValidation {
    let Some(field) = frontmatter.skill_dependencies.as_ref() else {
        return DependencyValidation::default();
    };

    let mut result = DependencyValidation::default();
    let DependencyList::Sequence(items) = &field.value else {
        result.diagnostics.push(Diagnostic::error(
            "SKILL_DEPENDENCIES_NOT_ARRAY",
            path,
            field.line,
            field.column,
            "skill-dependencies must be a non-empty array of skill identifiers",
        ));
        return result;
    };
    if items.is_empty() {
        result.diagnostics.push(Diagnostic::error(
            "SKILL_DEPENDENCIES_EMPTY",
            path,
            field.line,
            field.column,
            "omit skill-dependencies when the skill has no dependencies",
        ));
        return result;
    }

    let mut parsed = Vec::new();
    let mut occurrences: BTreeMap<&str, Vec<&Located<Option<String>>>> = BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        let Some(value) = item.value.as_deref() else {
            result.diagnostics.push(Diagnostic::error(
                "SKILL_DEPENDENCY_NOT_STRING",
                path,
                item.line,
                item.column,
                format!("skill-dependencies item {} must be a string", index + 1),
            ));
            continue;
        };
        occurrences.entry(value).or_default().push(item);
        match DependencyIdentifier::parse(value) {
            Ok(identifier) => parsed.push((identifier, item.line, item.column)),
            Err(_) => result.diagnostics.push(Diagnostic::error(
                "SKILL_DEPENDENCY_INVALID",
                path,
                item.line,
                item.column,
                format!("invalid skill dependency {value:?}; use SKILL or ORG/REPO#SKILL"),
            )),
        }
    }

    for (identifier, locations) in occurrences {
        if locations.len() > 1 {
            let location = locations[1];
            result.diagnostics.push(Diagnostic::error(
                "SKILL_DEPENDENCY_DUPLICATE",
                path,
                location.line,
                location.column,
                format!("duplicate skill dependency: {identifier}"),
            ));
        }
    }

    for (identifier, line, column) in &parsed {
        if let DependencyIdentifier::Internal(target) = identifier {
            if source_name == Some(target) {
                result.diagnostics.push(Diagnostic::error(
                    "SKILL_DEPENDENCY_SELF",
                    path,
                    *line,
                    *column,
                    format!("skill cannot depend on itself: {target}"),
                ));
            } else if !known_local_names.contains(target) {
                result.diagnostics.push(Diagnostic::error(
                    "SKILL_DEPENDENCY_UNRESOLVED",
                    path,
                    *line,
                    *column,
                    format!("bare skill dependency does not resolve in the scanned roots: {target}"),
                ));
            }
        }
    }

    let identifiers: Vec<_> = parsed.iter().map(|(identifier, _, _)| identifier).collect();
    let mut expected = identifiers.clone();
    expected.sort_by_key(|identifier| (identifier.target_name().as_str().replace('-', ""), identifier.as_identifier()));
    if identifiers != expected {
        result.diagnostics.push(Diagnostic::error(
            "SKILL_DEPENDENCIES_ORDER",
            path,
            field.line,
            field.column,
            "skill-dependencies must be sorted by target skill name, then complete identifier",
        ));
    }

    result.dependencies =
        parsed.into_iter().map(|(identifier, line, column)| DeclaredDependency { identifier, line, column }).collect();
    sort_diagnostics(&mut result.diagnostics);
    result
}
