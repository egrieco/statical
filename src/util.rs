use std::{fs, path::Path};

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
