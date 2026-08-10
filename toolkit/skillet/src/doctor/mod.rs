mod audit;
mod fix;
mod model;
mod render;
mod resource;

use std::env;
use std::io::{self, Write};
use std::path::Path;

use crate::RunOutcome;
use crate::catalog::Catalog;
use crate::cli::DoctorArgs;
use crate::error::Error;
use crate::traversal::RootRequest;

pub use model::{Counts, Finding, Fix, Report, RootRecord, Severity};

pub fn run(args: DoctorArgs) -> Result<RunOutcome, Error> {
    let roots =
        if args.root.is_empty() {
            vec![RootRequest::explicit(env::current_dir().map_err(|error| {
                Error::io("resolve current directory for", Path::new("."), error)
            })?)]
        } else {
            args.root.iter().map(RootRequest::explicit).collect()
        };
    let catalog = Catalog::load(&roots)?;
    let report = audit::build_report(&catalog, args.dependencies_only, args.fix_safe);
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
