use walkdir::WalkDir;

/// Recursively searches a directory for files with a specific file extension.
///
/// This function uses the `walkdir` crate to recursively traverse a directory and its subdirectories.
/// For each file in the directory tree with a matching file extension, the function adds the file path
/// to a vector of strings that it returns.
///
/// # Examples
///
/// ```
/// fn main() -> anyhow::Result<()> {
///     let files = get_files_with_extension("examples", "yaml");
///
///     assert_eq!(files, vec![
///         "examples/values1.yaml".to_owned(),
///         "examples/values2.yaml".to_owned(),
///         "examples/subdir/values3.yaml".to_owned(),
///     ]);
///
///     Ok(())
/// }
/// ```
///
/// # Arguments
///
/// * `dir` - The directory to search for files in.
/// * `extension` - The file extension to search for, without the leading dot (e.g. "yaml").
///
/// # Returns
///
/// A vector of strings representing the file paths of all files in the directory tree with the given file extension.
pub fn get_files_with_extension(dir: &str, extension: &str) -> Vec<String> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() {
                    if let Some(ext) = entry.path().extension() {
                        if ext == extension {
                            return Some(entry.path().to_string_lossy().into_owned());
                        }
                    }
                }
            }
            None
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::utils::walk::get_files_with_extension;

    use relative_path::RelativePath;
    use std::env::current_dir;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_basic_walk() -> anyhow::Result<()> {
        trace!("Running test_basic_walk.");
        let current_dir = current_dir()?;
        let target_dir = RelativePath::new("resources/test/").to_logical_path(&current_dir);
        let expected = vec![
            "resources/test/complex/subDir/template.jinja2",
            "resources/test/complex/template.jinja2",
            "resources/test/simple/template.jinja2",
            "resources/test/templates/world.jinja2",
            "resources/test/templates/nested-default.jinja2",
        ];
        let target_dir_str = target_dir.to_str().unwrap();
        let actual = get_files_with_extension(target_dir_str, "jinja2");
        // We need to remove the base path for our tests so they are generic
        let actual_relative = get_relative_files(actual, &current_dir);
        // Assert that they are equal
        assert_eq!(expected, actual_relative);
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
