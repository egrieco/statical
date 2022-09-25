use std::{
    fs,
    path::{Path, PathBuf},
};

use color_eyre::eyre::Context;

/// Takes a base dir and subdir, creates the subdirectory if it does not exist
pub fn create_subdir(
    base_output_dir: &Path,
    subdir_name: &str,
) -> Result<PathBuf, color_eyre::Report> {
    let output_dir = base_output_dir.join(subdir_name);
    if !output_dir.exists() {
        fs::create_dir(&output_dir).context(format!(
            "could not create {} dir in {:?}",
            subdir_name, base_output_dir
        ))?;
    }
    Ok(output_dir)
}
