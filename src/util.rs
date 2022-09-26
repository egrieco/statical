use color_eyre::eyre::{self, Context};
use std::io::Write;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tera::Tera;

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

pub fn render(tera: &Tera, template_name: &str, context: &tera::Context) -> eyre::Result<String> {
    Ok(tera.render(template_name, context)?)
}

pub fn render_to(
    tera: &Tera,
    template_name: &str,
    context: &tera::Context,
    write: impl Write,
) -> eyre::Result<()> {
    Ok(tera.render_to(template_name, context, write)?)
}
