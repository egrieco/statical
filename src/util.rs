use color_eyre::eyre::{self, Context};
use std::fs::File;
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

pub fn write_template(
    tera: &Tera,
    template_name: &str,
    context: &tera::Context,
    file_path: &Path,
) -> eyre::Result<()> {
    // TODO replace this with a debug or log message
    eprintln!("Writing template to file: {:?}", file_path);
    let output_file = File::create(&file_path)?;
    render_to(tera, template_name, context, output_file)
}
