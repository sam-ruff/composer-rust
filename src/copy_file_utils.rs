use log::trace;
use std::fs;
use std::io::Error;
use std::path::Path;

pub fn copy_files_with_ignorefile(
    src: &Path,
    dest: &Path,
    ignore_file: Option<&Path>,
) -> Result<(), Error> {
    trace!(
        "Copying files: Src: {}, Dest: {}",
        src.to_string_lossy(),
        dest.to_string_lossy()
    );
    let git_ignore_file: gitignore::File;
    let mut file: Option<&gitignore::File> = None;
    // If an ignore file hasn't been created specify an empty one
    if ignore_file.is_some() {
        let ignore_path: &Path = ignore_file.unwrap();
        trace!("Using ignorefile: {}", ignore_path.display().to_string());
        git_ignore_file = gitignore::File::new(&ignore_path).unwrap();
        file = Some(&git_ignore_file);
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();

        // If the entry is a directory, recursively copy its contents
        if path.is_dir() {
            let new_dest = dest.join(entry.file_name());
            fs::create_dir_all(&new_dest)?;
            copy_files_with_ignorefile(&path, &new_dest, ignore_file)?;
        } else if path.is_file() {
            // If the entry is a file and it doesn't match any of the regexes,
            // copy it to the destination directory
            let mut should_skip: bool = false;
            if file.is_some() {
                should_skip = file.unwrap().is_excluded(&path).unwrap_or(false);
            }
            if ignore_file.is_none() || !should_skip {
                let new_dest = dest.join(entry.file_name());
                trace!(
                    "Copying file: {} to {}",
                    &path.as_path().display().to_string(),
                    &new_dest.as_path().display().to_string()
                );
                fs::copy(&path, &new_dest)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_utils;
    use log::LevelFilter;
    use random_string::generate;
    use relative_path::RelativePath;
    use std::env::current_dir;

    #[test]
    fn test_copy_files_simple() -> Result<(), Error> {
        log_utils::setup_logging(LevelFilter::Trace, true);
        println!();
        trace!("Running simple copy test.");
        let current_dir = current_dir()?;
        let rel_path = RelativePath::new("resources/test/simple").to_logical_path(&current_dir);
        let file_path = Path::new(&rel_path);
        let path_str = setup_test_directory()?;
        let temp_path = Path::new(&path_str);
        let ignore_path = rel_path.join(Path::new(".composerignore"));
        copy_files_with_ignorefile(file_path, temp_path, Some(&ignore_path)).unwrap();
        // Assert the template has been copied
        assert_file_exists(temp_path, "template.jinja", true);
        // Assert that ignore me hasn't been copied
        assert_file_exists(temp_path, ".ignoreme", false);
        // Remove the test dir
        fs::remove_dir_all(&path_str).expect("Could not remove temp test dir");
        Ok(())
    }

    #[test]
    fn test_copy_files_no_ignore() -> Result<(), Error> {
        log_utils::setup_logging(LevelFilter::Trace, true);
        println!();
        trace!("Running no ignore test.");
        let current_dir = current_dir()?;
        let rel_path = RelativePath::new("resources/test/simple").to_logical_path(&current_dir);
        let file_path = Path::new(&rel_path);
        let path_str = setup_test_directory()?;
        let temp_path = Path::new(&path_str);
        copy_files_with_ignorefile(file_path, temp_path, Option::None).unwrap();
        // Assert the template has been copied
        assert_file_exists(temp_path, "template.jinja", true);
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
    fn test_copy_files_complex() -> Result<(), Error> {
        log_utils::setup_logging(LevelFilter::Trace, true);
        println!();
        trace!("Running complex ignore test.");
        let current_dir = current_dir()?;
        let rel_path = RelativePath::new("resources/test/complex").to_logical_path(&current_dir);
        let file_path = Path::new(&rel_path);
        let path_str = setup_test_directory()?;
        let temp_path = Path::new(&path_str);
        let ignore_path = rel_path.join(Path::new(".composerignore"));
        copy_files_with_ignorefile(file_path, temp_path, Some(&ignore_path)).unwrap();
        assert_file_exists(temp_path, "subDir/template.jinja", true);
        assert_file_exists(temp_path, "template.jinja", true);
        assert_file_exists(temp_path, "notFound", false);
        assert_file_exists(temp_path, "subDir/notFound", false);
        assert_file_exists(temp_path, "subDir/subDir2/aFile.txt", true);
        assert_file_exists(temp_path, "subDir/subDir2/notFound2", false);
        assert_file_exists(temp_path, "subDir/subDir2/alsoIgnored", false);
        fs::remove_dir_all(&path_str).expect("Could not remove temp test dir");
        Ok(())
    }

    fn setup_test_directory() -> Result<String, Error> {
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
