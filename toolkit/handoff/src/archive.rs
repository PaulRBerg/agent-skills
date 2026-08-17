use std::{
    fs::{self, OpenOptions},
    io,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use chrono::Utc;

use crate::{
    cli::ArchiveArgs,
    error::{Error, Result},
    util::{absolute_regular_handoff, home},
};

pub(crate) fn run(arguments: ArchiveArgs) -> Result<()> {
    let source = absolute_regular_handoff(&arguments.handoff_path)?;
    let origin = origin_name(&source)?;
    let archive_directory = home()?.join(".local/share/task-handoffs/archive").join(origin);
    fs::create_dir_all(&archive_directory).map_err(|error| {
        Error::operational(format!("cannot create archive directory {}: {error}", archive_directory.display()))
    })?;
    let destination = available_destination(&archive_directory, &source)?;
    move_file(&source, &destination)?;
    println!("ARCHIVED\t{}", destination.display());
    Ok(())
}

fn origin_name(source: &Path) -> Result<&std::ffi::OsStr> {
    source
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .ok_or_else(|| Error::operational(format!("cannot determine handoff origin: {}", source.display())))
}

fn available_destination(directory: &Path, source: &Path) -> Result<PathBuf> {
    let filename = source
        .file_name()
        .ok_or_else(|| Error::operational(format!("handoff has no filename: {}", source.display())))?;
    let direct = directory.join(filename);
    if !entry_exists(&direct)? {
        return Ok(direct);
    }

    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| Error::operational(format!("handoff filename is not valid UTF-8: {}", source.display())))?;
    loop {
        let timestamp = Utc::now().format("%Y_%m_%d_%H%M%S");
        let candidate = directory.join(format!("{stem}_{timestamp}.md"));
        if !entry_exists(&candidate)? {
            return Ok(candidate);
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn entry_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(Error::operational(format!("cannot inspect archive target {}: {error}", path.display()))),
    }
}

fn move_file(source: &Path, destination: &Path) -> Result<()> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => copy_then_remove(source, destination),
        Err(error) => Err(Error::operational(format!(
            "cannot move handoff {} to {}: {error}",
            source.display(),
            destination.display()
        ))),
    }
}

fn copy_then_remove(source: &Path, destination: &Path) -> Result<()> {
    let mut input = fs::File::open(source)
        .map_err(|error| Error::operational(format!("cannot open handoff {}: {error}", source.display())))?;
    let mut output = OpenOptions::new().write(true).create_new(true).open(destination).map_err(|error| {
        Error::operational(format!("cannot create archive file {}: {error}", destination.display()))
    })?;
    let result = io::copy(&mut input, &mut output)
        .and_then(|_| output.sync_all())
        .and_then(|()| fs::set_permissions(destination, fs::metadata(source)?.permissions()))
        .and_then(|()| fs::remove_file(source));
    if let Err(error) = result {
        let _ = fs::remove_file(destination);
        return Err(Error::operational(format!(
            "cannot copy handoff {} to {}: {error}",
            source.display(),
            destination.display()
        )));
    }
    Ok(())
}
