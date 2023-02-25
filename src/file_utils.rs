use regex::Regex;
use std::fs;
use std::io::Error;
use std::path::Path;

pub fn copy_files_except_regex(
    src: &Path,
    dest: &Path,
    regex_list: &Vec<String>,
) -> Result<(), Error> {
    println!("{}", &src.to_string_lossy());
    println!("{}", &dest.to_string_lossy());
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

#[cfg(test)]
mod tests {
    use super::*;
    use relative_path::RelativePath;
    use std::env::current_dir;

    #[test]
    fn test_copy_files_simple() -> Result<(), Error> {
        let current_dir = current_dir()?;
        let rel_path = RelativePath::new("resources/test/simple").to_logical_path(&current_dir);
        let file_path = Path::new(&rel_path);
        let path_str = setup_test_directory()?;
        let temp_path = Path::new(&path_str);
        let regex_list = vec![String::from(".ignore*")];
        copy_files_except_regex(file_path, temp_path, &regex_list).unwrap();
        let filenames = get_filename_in_directory(&path_str)?;

        assert!(filenames.contains(&String::from("template.jinja")));
        // Assert that ignore me hasn't been copied
        assert_eq!(filenames.contains(&String::from(".ignoreme")), false);
        fs::remove_dir_all(&path_str).expect("Could not remove temp test dir");
        Ok(())
    }

    fn setup_test_directory() -> Result<String, Error> {
        let path_str = "/tmp/unit_test";
        if let Err(e) = fs::remove_dir_all(&path_str) {
            if e.kind() != std::io::ErrorKind::NotFound {
                panic!("Error deleting directory: {:?}", e);
            }
        }
        fs::create_dir(&path_str).expect("Could not create directory.");
        Ok(path_str.parse().unwrap())
    }

    fn get_filename_in_directory(path_str: &str) -> Result<Vec<String>, Error> {
        Ok(fs::read_dir(&path_str)?
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    e.path()
                        .file_name()
                        .and_then(|n| n.to_str().map(|s| String::from(s)))
                })
            })
            .collect::<Vec<String>>())
    }
}
