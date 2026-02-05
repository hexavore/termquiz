use std::path::{Path, PathBuf};

use crate::git;

pub fn is_git_url(s: &str) -> bool {
    s.starts_with("git@")
        || s.starts_with("https://")
        || s.starts_with("http://")
        || s.ends_with(".git")
}

fn repo_name_from_url(url: &str) -> String {
    let name = url
        .rsplit('/')
        .next()
        .unwrap_or("repo")
        .trim_end_matches(".git");
    name.to_string()
}

fn default_clone_dir(url: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let name = repo_name_from_url(url);
    PathBuf::from(home)
        .join("termquiz-exams")
        .join(name)
}

pub fn resolve_source(
    path_or_url: &str,
    clone_to: Option<&str>,
) -> Result<(PathBuf, PathBuf), String> {
    if is_git_url(path_or_url) {
        let clone_dir = clone_to
            .map(PathBuf::from)
            .unwrap_or_else(|| default_clone_dir(path_or_url));

        if clone_dir.exists() {
            if clone_dir.join(".git").exists() {
                git::git_pull(&clone_dir)?;
            } else {
                return Err(format!(
                    "Directory {} exists but is not a git repo",
                    clone_dir.display()
                ));
            }
        } else {
            git::git_clone(path_or_url, &clone_dir)?;
        }

        let md_file = find_quiz_file(&clone_dir)?;
        Ok((clone_dir, md_file))
    } else {
        let path = Path::new(path_or_url).to_path_buf();
        let path = if path.is_relative() {
            std::env::current_dir()
                .map_err(|e| format!("Cannot get cwd: {}", e))?
                .join(path)
        } else {
            path
        };

        if path.is_file() && path.extension().map_or(false, |e| e == "md") {
            let repo_dir = path
                .parent()
                .ok_or_else(|| "Cannot determine parent directory".to_string())?
                .to_path_buf();
            Ok((repo_dir, path))
        } else if path.is_dir() {
            let md_file = find_quiz_file(&path)?;
            Ok((path, md_file))
        } else {
            Err(format!("Path not found: {}", path.display()))
        }
    }
}

fn find_quiz_file(dir: &Path) -> Result<PathBuf, String> {
    let mut md_files: Vec<PathBuf> = Vec::new();

    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Cannot read directory {}: {}", dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Error reading entry: {}", e))?;
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |e| e == "md") {
            md_files.push(path);
        }
    }

    match md_files.len() {
        0 => Err(format!(
            "No .md quiz files found in {}",
            dir.display()
        )),
        1 => Ok(md_files.remove(0)),
        _ => {
            let names: Vec<String> = md_files
                .iter()
                .map(|p| format!("  - {}", p.file_name().unwrap_or_default().to_string_lossy()))
                .collect();
            Err(format!(
                "Multiple .md files found. Specify which one:\n{}",
                names.join("\n")
            ))
        }
    }
}
