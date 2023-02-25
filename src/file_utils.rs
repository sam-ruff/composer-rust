use regex::Regex;
use std::fs;
use std::io::Error;
use std::path::Path;

pub fn copy_files_except_regex(src: &Path, dest: &Path, regex_list: &[&str]) -> Result<(), Error> {
    // Iterate over the entries in the source directory
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();

        // If the entry is a directory, recursively copy its contents
        if path.is_dir() {
            let new_dest = dest.join(entry.file_name());
            fs::create_dir_all(&new_dest)?;
            copy_files_except_regex(&path, &new_dest, regex_list)?;
        } else if path.is_file() {
            // If the entry is a file and it doesn't match any of the regexes,
            // copy it to the destination directory
            let should_copy = regex_list.iter().all(|regex_str| {
                !Regex::new(regex_str)
                    .unwrap()
                    .is_match(&path.to_string_lossy())
            });

            if should_copy {
                let new_dest = dest.join(entry.file_name());
                fs::copy(&path, &new_dest)?;
            }
        }
    }

    Ok(())
}
