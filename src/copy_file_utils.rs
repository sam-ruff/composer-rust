use std::fs;

use std::error::Error;
use std::path::Path;

pub fn copy_files_with_ignorefile(
    src: &Path,
    dest: &Path,
    ignore_file: Option<&Path>,
) -> Result<(), Box<dyn Error>> {
    // Print a log message to show which files are being copied
    trace!(
        "Copying files: Src: {}, Dest: {}",
        src.to_string_lossy(),
        dest.to_string_lossy()
    );

    // Create a `gitignore::File` object from the ignore file, if specified
    let exclude_file = match ignore_file {
        Some(path) => {
            // Print a log message to show which ignore file is being used
            let ignore_path = path.to_string_lossy();
            trace!("Using ignorefile: {}", ignore_path);
            // Create a `gitignore::File` object from the ignore file
            let exclude_file = gitignore::File::new(path)?;
            // Return the `gitignore::File` object as an `Option`
            Some(exclude_file)
        }
        None => None,
    };

    // Iterate through the entries in the source directory
    for entry in fs::read_dir(src)? {
        // Unwrap the entry and get its path
        let entry = entry?;
        let entry_path = entry.path();

        // If the entry is a directory, recursively copy its contents to the destination
        if entry_path.is_dir() {
            let new_dest = dest.join(entry.file_name());
            fs::create_dir_all(&new_dest)?;
            copy_files_with_ignorefile(&entry_path, &new_dest, ignore_file)?;
        }
        // If the entry is a file and isn't excluded by the ignore file, copy it to the destination
        else if entry_path.is_file() {
            // Check if the file is excluded by the ignore file, if specified
            let mut should_skip = false;
            if let Some(exclude_file) = &exclude_file {
                should_skip = exclude_file.is_excluded(&entry_path).unwrap_or(false);
            }
            // Copy the file to the destination if it's not excluded by the ignore file
            if ignore_file.is_none() || !should_skip {
                let new_dest = dest.join(entry.file_name());
                // Print a log message to show which file is being copied
                trace!(
                    "Copying file: {} to {}",
                    &entry_path.display(),
                    &new_dest.display()
                );
                fs::copy(&entry_path, &new_dest)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use random_string::generate;
    use relative_path::RelativePath;
    use std::env::current_dir;

    #[test]
    fn test_copy_files_simple() -> Result<(), Box<dyn Error>> {
        trace!("Running test_copy_files_simple.");
        let current_dir = current_dir()?;
        let rel_path = RelativePath::new("resources/test/simple").to_logical_path(&current_dir);
        let file_path = Path::new(&rel_path);
        let path_str = setup_test_directory()?;
        let temp_path = Path::new(&path_str);
        let ignore_path = rel_path.join(Path::new(".composerignore"));
        copy_files_with_ignorefile(file_path, temp_path, Some(&ignore_path)).unwrap();
        // Assert the template has been copied
        assert_file_exists(temp_path, "template.jinja2", true);
        // Assert that ignore me hasn't been copied
        assert_file_exists(temp_path, ".ignoreme", false);
        // Remove the test dir
        fs::remove_dir_all(&path_str).expect("Could not remove temp test dir");
        Ok(())
    }

    #[test]
    fn test_copy_files_no_ignore() -> Result<(), Box<dyn Error>> {
        trace!("Running test_copy_files_no_ignore");
        let current_dir = current_dir()?;
        let rel_path = RelativePath::new("resources/test/simple").to_logical_path(&current_dir);
        let file_path = Path::new(&rel_path);
        let path_str = setup_test_directory()?;
        let temp_path = Path::new(&path_str);
        copy_files_with_ignorefile(file_path, temp_path, Option::None).unwrap();
        // Assert the template has been copied
        assert_file_exists(temp_path, "template.jinja2", true);
        // Assert that ignore me hasn't been copied
        assert_file_exists(temp_path, ".ignoreme", true);
        // Remove the test dir
        fs::remove_dir_all(&path_str).expect("Could not remove temp test dir");
        Ok(())
    }

    fn assert_file_exists(temp_path: &Path, s: &str, does_exist: bool) {
        assert_eq!(temp_path.join(Path::new(s)).exists(), does_exist);
    }

    #[test]
    fn test_copy_files_complex() -> Result<(), Box<dyn Error>> {
        trace!("Running test_copy_files_complex");
        let current_dir = current_dir()?;
        let rel_path = RelativePath::new("resources/test/complex").to_logical_path(&current_dir);
        let file_path = Path::new(&rel_path);
        let path_str = setup_test_directory()?;
        let temp_path = Path::new(&path_str);
        let ignore_path = rel_path.join(Path::new(".composerignore"));
        copy_files_with_ignorefile(file_path, temp_path, Some(&ignore_path)).unwrap();
        assert_file_exists(temp_path, "subDir/template.jinja2", true);
        assert_file_exists(temp_path, "template.jinja2", true);
        assert_file_exists(temp_path, "notFound", false);
        assert_file_exists(temp_path, "subDir/notFound", false);
        assert_file_exists(temp_path, "subDir/subDir2/aFile.txt", true);
        assert_file_exists(temp_path, "subDir/subDir2/notFound2", false);
        assert_file_exists(temp_path, "subDir/subDir2/alsoIgnored", false);
        fs::remove_dir_all(&path_str).expect("Could not remove temp test dir");
        Ok(())
    }

    fn setup_test_directory() -> Result<String, Box<dyn Error>> {
        let string = generate(8, "abcdefghijklmmnopqrstuvwyz");
        // Create a unique test directory
        let path_str = format!("/tmp/unit_test{}", string);
        if let Err(e) = fs::remove_dir_all(&path_str) {
            if e.kind() != std::io::ErrorKind::NotFound {
                panic!("Error deleting directory: {:?}", e);
            }
        }
        fs::create_dir(&path_str).expect(&format!("Could not create directory '{}'.", &path_str));
        Ok(path_str.parse().unwrap())
    }
}
