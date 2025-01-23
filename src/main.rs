use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

#[derive(Debug, PartialEq)]
struct Match {
    file_path: String,
    line_number: usize,
    line_content: String,
}

fn search_files(search_term: &str, directory: &Path) -> Vec<Match> {
    let mut matches = Vec::new();

    for entry in WalkDir::new(directory)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();

        if let Ok(metadata) = path.metadata() {
            if metadata.len() > 10_000_000 {
                continue;
            }
        }

        if let Ok(file) = File::open(path) {
            let reader = BufReader::new(file);

            for (line_num, line_result) in reader.lines().enumerate() {
                if let Ok(line) = line_result {
                    if line.to_lowercase().contains(&search_term.to_lowercase()) {
                        matches.push(Match {
                            file_path: path.display().to_string(),
                            line_number: line_num + 1,
                            line_content: line,
                        });
                    }
                }
            }
        }
    }

    matches
}

fn display_matches(matches: &[Match]) {
    for (index, m) in matches.iter().enumerate() {
        println!(
            "{}. [Line {}] {}\n   File: {}\n",
            index + 1,
            m.line_number,
            m.line_content.trim(),
            m.file_path
        );
    }
}

fn open_in_vscode(file_path: &str, line_number: usize) {
    Command::new("code")
        .arg("-g")
        .arg(format!("{}:{}", file_path, line_number))
        .spawn()
        .expect("Failed to open VSCode");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: search <term> [directory]");
        std::process::exit(1);
    }

    let search_term = &args[1];
    let directory = if args.len() > 2 {
        Path::new(&args[2])
    } else {
        Path::new(".")
    };

    let matches = search_files(search_term, directory);

    if matches.is_empty() {
        println!("No matches found for '{}'", search_term);
        return;
    }

    display_matches(&matches);

    println!("Enter the number of the file to open in VSCode (or 0 to exit):");

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    if let Ok(selection) = input.trim().parse::<usize>() {
        if selection > 0 && selection <= matches.len() {
            let selected_match = &matches[selection - 1];
            open_in_vscode(&selected_match.file_path, selected_match.line_number);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    fn create_test_file(
        dir: impl AsRef<Path>,
        filename: &str,
        content: &str,
    ) -> std::path::PathBuf {
        let file_path = dir.as_ref().join(filename);
        let mut file = File::create(&file_path).expect("Failed to create test file");
        file.write_all(content.as_bytes())
            .expect("Failed to write to test file");
        file_path
    }

    #[test]
    fn test_basic_file_search() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let dir_path = temp_dir.path();

        create_test_file(
            dir_path,
            "test1.txt",
            "The quick brown frog jumps over the lazy dog",
        );
        create_test_file(dir_path, "test2.txt", "No matches here");
        create_test_file(dir_path, "test3.txt", "FROG in uppercase");

        let matches = search_files("frog", dir_path);

        assert_eq!(matches.len(), 2);

        assert_eq!(
            matches[0].line_content,
            "The quick brown frog jumps over the lazy dog"
        );
        assert_eq!(matches[0].line_number, 1);

        assert!(matches.iter().any(|m| m.line_content.contains("FROG")));
    }

    #[test]
    fn test_search_multiple_files() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let dir_path = temp_dir.path();

        // Create nested directories first
        fs::create_dir_all(dir_path.join("subdir/nested"))
            .expect("Failed to create nested directories");

        create_test_file(dir_path, "file1.txt", "First frog occurrence");
        create_test_file(dir_path.join("subdir"), "file2.txt", "Second frog mention");
        create_test_file(
            dir_path.join("subdir/nested"),
            "file3.txt",
            "Third frog reference",
        );

        let matches = search_files("frog", dir_path);

        assert_eq!(matches.len(), 3);

        let match_contents: Vec<String> = matches.iter().map(|m| m.line_content.clone()).collect();

        assert!(match_contents.contains(&"First frog occurrence".to_string()));
        assert!(match_contents.contains(&"Second frog mention".to_string()));
        assert!(match_contents.contains(&"Third frog reference".to_string()));
    }

    #[test]
    fn test_no_matches() {
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let dir_path = temp_dir.path();

        create_test_file(dir_path, "test1.txt", "Hello world");
        create_test_file(dir_path, "test2.txt", "No matching content");

        let matches = search_files("frog", dir_path);

        assert_eq!(matches.len(), 0);
    }
}
