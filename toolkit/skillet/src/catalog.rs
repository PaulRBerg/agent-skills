use std::{collections::BTreeSet, path::Path};

use serde::Serialize;

use crate::{
    dependency::{DeclaredDependency, SkillName, validate_dependencies},
    diagnostic::{Diagnostic, sort_diagnostics},
    error::Error,
    frontmatter::{Frontmatter, parse_skill_file},
    hash::{sha256_file, sha256_tree},
    traversal::{RootRequest, ScanRoot, SkillExposure, discover},
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Skill {
    pub exposure: SkillExposure,
    pub directory_name: String,
    pub name: Option<SkillName>,
    pub frontmatter: Option<Frontmatter>,
    pub dependencies: Vec<DeclaredDependency>,
}

impl Skill {
    pub fn skill_path(&self) -> &Path {
        &self.exposure.exposure_path
    }

    pub fn resolved_skill_path(&self) -> &Path {
        &self.exposure.resolved_path
    }

    pub fn hashes(&self) -> Result<SkillHashes, Error> {
        Ok(SkillHashes {
            skill_sha256: sha256_file(self.resolved_skill_path())?,
            tree_sha256: sha256_tree(self.exposure.resolved_directory())?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SkillHashes {
    pub skill_sha256: String,
    pub tree_sha256: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Catalog {
    pub roots: Vec<ScanRoot>,
    pub skills: Vec<Skill>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Catalog {
    pub fn load(requests: &[RootRequest]) -> Result<Self, Error> {
        let discovery = discover(requests)?;
        let mut diagnostics = Vec::new();
        let mut skills = Vec::with_capacity(discovery.skills.len());

        for exposure in discovery.skills {
            let parsed = parse_skill_file(&exposure.exposure_path);
            diagnostics.extend(parsed.diagnostics);
            let directory_name = exposure
                .exposure_directory()
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let frontmatter_name = parsed
                .frontmatter
                .as_ref()
                .and_then(|frontmatter| frontmatter.name.as_ref())
                .and_then(|name| SkillName::parse(&name.value).ok());
            let name = frontmatter_name.or_else(|| SkillName::parse(&directory_name).ok());
            skills.push(Skill {
                exposure,
                directory_name,
                name,
                frontmatter: parsed.frontmatter,
                dependencies: Vec::new(),
            });
        }

        let known_local_names: BTreeSet<_> = skills
            .iter()
            .filter_map(|skill| SkillName::parse(&skill.directory_name).ok().or_else(|| skill.name.clone()))
            .collect();
        for skill in &mut skills {
            let Some(frontmatter) = skill.frontmatter.as_ref() else {
                continue;
            };
            let directory_name = SkillName::parse(&skill.directory_name).ok();
            let validation = validate_dependencies(
                skill.skill_path(),
                directory_name.as_ref().or(skill.name.as_ref()),
                frontmatter,
                &known_local_names,
            );
            skill.dependencies = validation.dependencies;
            diagnostics.extend(validation.diagnostics);
        }

        skills.sort_by(|left, right| left.exposure.cmp(&right.exposure));
        sort_diagnostics(&mut diagnostics);
        Ok(Self { roots: discovery.roots, skills, diagnostics })
    }

    pub fn skill_names(&self) -> BTreeSet<&SkillName> {
        self.skills.iter().filter_map(|skill| skill.name.as_ref()).collect()
    }

    pub fn exposures_for_resolved_path(&self, path: &Path) -> Vec<&Skill> {
        let mut skills: Vec<_> = self.skills.iter().filter(|skill| skill.resolved_skill_path() == path).collect();
        skills.sort_by_key(|skill| skill.skill_path());
        skills
    }
}
