mod audit;
mod fix;
mod model;
mod render;
mod resource;

use std::{
    collections::BTreeSet,
    env,
    ffi::OsStr,
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::{
    RunOutcome,
    catalog::Catalog,
    cli::DoctorArgs,
    dependency::SkillName,
    error::Error,
    traversal::{RootRequest, ScanRoot, normalize_roots, recognized_skill_file},
};

pub use model::{Counts, Finding, Fix, Report, RootRecord, Severity};

pub fn run(args: DoctorArgs) -> Result<RunOutcome, Error> {
    let plan = ScopePlan::new(&args)?;
    let catalog = Catalog::load(&plan.resolution_roots)?;
    let selection = plan.resolve(&catalog)?;
    let report = audit::build_report(&catalog, &selection, args.dependencies_only, args.fix_safe);
    let output = render::render(&report, args.format)?;
    io::stdout()
        .lock()
        .write_all(output.as_bytes())
        .map_err(|error| Error::io("write doctor report to", Path::new("stdout"), error))?;

    let exit_code = if report.counts.fix_errors > 0 {
        3
    } else if report.counts.findings > 0 {
        1
    } else {
        0
    };
    Ok(RunOutcome::with_exit_code(exit_code))
}

struct ScopePlan {
    report_roots: Vec<ScanRoot>,
    resolution_roots: Vec<RootRequest>,
    direct_skills: BTreeSet<PathBuf>,
    catalog_roots: Vec<ScanRoot>,
    selected_names: BTreeSet<String>,
}

impl ScopePlan {
    fn new(args: &DoctorArgs) -> Result<Self, Error> {
        let selected_names = selected_names(&args.skill)?;
        let requested = if args.root.is_empty() {
            vec![RootRequest::explicit(
                env::current_dir()
                    .map_err(|error| Error::io("resolve current directory for", Path::new("."), error))?,
            )]
        } else {
            args.root.iter().map(RootRequest::explicit).collect()
        };
        let report_roots = normalize_roots(&requested)?;
        let mut resolution_roots = Vec::new();
        let mut direct_skills = BTreeSet::new();
        let mut catalog_roots = Vec::new();

        for root in &report_roots {
            if let Some(skill_path) = conventional_direct_skill(root) {
                let parent = root.exposure_path.parent().expect("a conventional direct skill has a parent");
                resolution_roots.push(RootRequest::explicit(parent));
                resolution_roots.push(RootRequest::explicit(&root.exposure_path));
                direct_skills.insert(skill_path);
            } else {
                resolution_roots.push(RootRequest::explicit(&root.exposure_path));
                catalog_roots.push(root.clone());
            }
        }
        Ok(Self { report_roots, resolution_roots, direct_skills, catalog_roots, selected_names })
    }

    fn resolve(self, catalog: &Catalog) -> Result<AuditSelection, Error> {
        let explicitly_filtered = !self.selected_names.is_empty();
        if explicitly_filtered {
            let discovered: BTreeSet<_> = catalog.skills.iter().map(|skill| skill.directory_name.clone()).collect();
            let missing = self.selected_names.difference(&discovered).cloned().collect::<Vec<_>>();
            if !missing.is_empty() {
                return Err(Error::DoctorSkillNotDiscovered(missing));
            }
        }

        let skill_paths = catalog
            .skills
            .iter()
            .filter(|skill| {
                if explicitly_filtered {
                    self.selected_names.contains(&skill.directory_name)
                } else {
                    self.direct_skills.contains(skill.skill_path()) ||
                        self.catalog_roots.iter().any(|root| recognized_skill_file(root, skill.skill_path()))
                }
            })
            .map(|skill| skill.skill_path().to_path_buf())
            .collect();
        let readme_roots = if explicitly_filtered {
            BTreeSet::new()
        } else {
            self.catalog_roots.iter().map(|root| root.exposure_path.clone()).collect()
        };
        Ok(AuditSelection { roots: self.report_roots, skill_paths, readme_roots })
    }
}

struct AuditSelection {
    roots: Vec<ScanRoot>,
    skill_paths: BTreeSet<PathBuf>,
    readme_roots: BTreeSet<PathBuf>,
}

impl AuditSelection {
    fn includes(&self, path: &Path) -> bool {
        self.skill_paths.contains(path)
    }

    fn checks_readme(&self, root: &ScanRoot) -> bool {
        self.readme_roots.contains(&root.exposure_path)
    }
}

fn conventional_direct_skill(root: &ScanRoot) -> Option<PathBuf> {
    let parent = root.exposure_path.parent()?;
    let skill_path = root.exposure_path.join("SKILL.md");
    (parent.file_name() == Some(OsStr::new("skills")) && skill_path.is_file()).then_some(skill_path)
}

fn selected_names(values: &[String]) -> Result<BTreeSet<String>, Error> {
    let mut selected = BTreeSet::new();
    let mut invalid = BTreeSet::new();
    for value in values {
        if SkillName::parse(value).is_ok() {
            selected.insert(value.clone());
        } else {
            invalid.insert(value.clone());
        }
    }
    if invalid.is_empty() { Ok(selected) } else { Err(Error::InvalidSkillFilter(invalid.into_iter().collect())) }
}
