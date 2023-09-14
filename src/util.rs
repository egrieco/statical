use color_eyre::eyre::{Context, Result};
use include_dir::DirEntry::File as FileEnt;
use log::debug;
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use crate::model::calendar_collection::{self};

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

pub fn restore_missing_templates(path: &Path) -> Result<()> {
    debug!("creating templates path: {:?}", path);
    fs::create_dir_all(path).wrap_err("could not create templates path")?;

    for template in calendar_collection::TEMPLATE_DIR
        .find("**/*.html")
        .wrap_err("could not get templates")?
    {
        if let FileEnt(t) = template {
            if let (Some(template_name), Some(template_contents)) =
                (t.path().to_str(), t.contents_utf8())
            {
                let template_path = path.join(template_name);
                if template_path.exists() {
                    debug!("template already exists: {:?}", template_path);
                } else {
                    debug!("adding default template: {:?}", template_path);
                    File::create(template_path)
                        .wrap_err("could not create template file")?
                        .write_all(template_contents.as_bytes())
                        .wrap_err("could not write to template file")?;
                }
            }
        }
    }
    Ok(())
}

pub fn restore_missing_assets(path: &Path) -> Result<()> {
    debug!("creating assets path: {:?}", path);
    fs::create_dir_all(path).wrap_err("could not create assets path")?;

    // TODO: handle assets other than CSS
    for asset in calendar_collection::ASSETS_DIR
        .find("*.css")
        .wrap_err("could not get assets")?
    {
        // TODO: handle subdirectories of the assets path
        if let FileEnt(t) = asset {
            // TODO: might need to change this to binary handling of we have images involved
            if let (Some(asset_name), Some(asset_contents)) = (t.path().to_str(), t.contents_utf8())
            {
                let asset_path = path.join(asset_name);
                if asset_path.exists() {
                    debug!("asset already exists: {:?}", asset_path);
                } else {
                    debug!("adding asset: {:?}", asset_path);
                    File::create(asset_path)
                        .wrap_err("could not create asset file")?
                        .write_all(asset_contents.as_bytes())
                        .wrap_err("could not write to asset file")?;
                }
            }
        }
    }
    Ok(())
}
