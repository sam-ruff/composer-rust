use walkdir::WalkDir;

/// Recursively searches a directory for files with any of the specified file extensions.
///
/// # Arguments
///
/// * `dir` - The directory to search for files in.
/// * `extensions` - A slice of file extensions to search for, without the leading dot (e.g. &["jinja2", "j2"]).
///
/// # Returns
///
/// A vector of strings representing the file paths of all files in the directory tree with any of the given file extensions.
pub fn get_files_with_extensions(dir: &str, extensions: &[&str]) -> Vec<String> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() {
                    if let Some(ext) = entry.path().extension() {
                        if extensions.iter().any(|e| ext == *e) {
                            return Some(entry.path().to_string_lossy().into_owned());
                        }
                    }
                }
            }
            None
        })
        .collect()
}

// TODO needs unit tests
pub fn get_files_with_name(dir: &str, name: &str) -> Vec<String> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() {
                    if entry.file_name().to_string_lossy() == name {
                        return Some(entry.path().to_string_lossy().into_owned());
                    }
                }
            }
            None
        })
        .collect()
}

/// Recursively searches a directory for files with any of the specified file names.
pub fn get_files_with_names(dir: &str, names: &[&str]) -> Vec<String> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() {
                    let file_name = entry.file_name().to_string_lossy();
                    if names.iter().any(|n| file_name == *n) {
                        return Some(entry.path().to_string_lossy().into_owned());
                    }
                }
            }
            None
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::utils::walk::get_files_with_extensions;

    use relative_path::RelativePath;
    use std::env::current_dir;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_basic_walk() -> anyhow::Result<()> {
        trace!("Running test_basic_walk.");
        let current_dir = current_dir()?;
        let target_dir =
            RelativePath::new("resources/test/walk_test").to_logical_path(&current_dir);
        let expected = vec![
            "resources/test/walk_test/file1.jinja2",
            "resources/test/walk_test/file2.jinja2",
            "resources/test/walk_test/subfolder/file3.jinja2",
        ];
        let target_dir_str = target_dir.to_str().unwrap();
        let actual = get_files_with_extensions(target_dir_str, &["jinja2"]);
        // We need to remove the base path for our tests so they are generic
        let actual_relative = get_relative_files(actual, &current_dir);
        // Sort both vectors so order doesn't matter.
        let mut expected_sorted = expected.clone();
        expected_sorted.sort();
        let mut actual_sorted = actual_relative.clone();
        actual_sorted.sort();
        // Assert that they are equal
        assert_eq!(expected_sorted, actual_sorted);
        Ok(())
    }

    #[test]
    fn test_walk_multiple_extensions() -> anyhow::Result<()> {
        let current_dir = current_dir()?;
        let target_dir =
            RelativePath::new("resources/test/walk_test").to_logical_path(&current_dir);
        let expected = vec![
            "resources/test/walk_test/file1.jinja2",
            "resources/test/walk_test/file2.jinja2",
            "resources/test/walk_test/file4.j2",
            "resources/test/walk_test/subfolder/file3.jinja2",
        ];
        let target_dir_str = target_dir.to_str().unwrap();
        let actual = get_files_with_extensions(target_dir_str, &["jinja2", "j2"]);
        let actual_relative = get_relative_files(actual, &current_dir);
        let mut expected_sorted = expected.clone();
        expected_sorted.sort();
        let mut actual_sorted = actual_relative.clone();
        actual_sorted.sort();
        assert_eq!(expected_sorted, actual_sorted);
        Ok(())
    }

    fn get_relative_files(files: Vec<String>, base_dir: &PathBuf) -> Vec<String> {
        files
            .into_iter()
            .filter_map(|file| abs_to_rel(&PathBuf::from(file), base_dir))
            .map(|path| path.to_string_lossy().into_owned())
            .collect()
    }

    fn abs_to_rel(abs_path: &Path, base_dir: &PathBuf) -> Option<PathBuf> {
        abs_path
            .strip_prefix(base_dir)
            .ok()
            .map(|rel_path| rel_path.to_path_buf())
    }
}
