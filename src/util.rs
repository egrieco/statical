use color_eyre::eyre::{self};
use std::fs::File;
use std::io::Write;
use std::{fs, path::Path};
use tera::Tera;

/// Delete all contents of a directory without modifying the directory itself
///
/// This function prints error messages directly to `STDERR` but otherwise ignores them and does not fail
pub fn delete_dir_contents<P: AsRef<Path>>(path: P) {
    match fs::read_dir(path) {
        Err(e) => eprintln!("could not read output dir: {}", e),
        Ok(dir) => {
            for entry in dir {
                match entry {
                    Err(e) => eprintln!("entry error in output dir: {}", e),
                    Ok(entry) => {
                        let path = entry.path();

                        if path.is_dir() {
                            if let Err(e) = fs::remove_dir_all(path) {
                                eprintln!("could not delete directory in output dir: {}", e);
                            };
                        } else if let Err(e) = fs::remove_file(path) {
                            eprintln!("could not delete file in output dir: {}", e);
                        }
                    }
                }
            }
        }
    }
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
    let output_file = File::create(file_path)?;
    render_to(tera, template_name, context, output_file)
}
