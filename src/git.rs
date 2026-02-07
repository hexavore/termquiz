use std::path::Path;
use std::process::Command;

fn run_git(args: &[&str], cwd: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(stderr)
    }
}

pub fn git_clone(url: &str, dest: &Path) -> Result<(), String> {
    let dest_str = dest
        .to_str()
        .ok_or_else(|| "Invalid path".to_string())?;

    let parent = dest.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("Cannot create directory: {}", e))?;

    let output = Command::new("git")
        .args(["clone", url, dest_str])
        .output()
        .map_err(|e| format!("Failed to run git clone: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

pub fn git_pull(repo: &Path) -> Result<(), String> {
    run_git(&["pull", "--ff-only"], repo)?;
    Ok(())
}

pub fn is_git_repo(path: &Path) -> bool {
    path.join(".git").exists()
}

pub fn git_add(repo: &Path, paths: &[&str]) -> Result<(), String> {
    let mut args = vec!["add"];
    args.extend(paths);
    run_git(&args, repo)?;
    Ok(())
}

pub fn git_commit(repo: &Path, message: &str) -> Result<(), String> {
    run_git(&["commit", "-m", message], repo)?;
    Ok(())
}

pub fn git_push(repo: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .args(["push"])
        .current_dir(repo)
        .output()
        .map_err(|e| format!("Failed to run git push: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if stderr.contains("rejected") {
            Err(format!("CONFLICT:{}", stderr))
        } else {
            Err(format!("NETWORK:{}", stderr))
        }
    }
}

pub fn has_response_in_history(repo: &Path) -> bool {
    run_git(&["log", "--all", "--format=%H", "--", "response/answers.yaml"], repo)
        .map(|out| !out.trim().is_empty())
        .unwrap_or(false)
}

pub fn has_response_in_worktree(repo: &Path) -> bool {
    repo.join("response").join("answers.yaml").exists()
}

pub fn has_existing_submission(repo: &Path) -> bool {
    has_response_in_history(repo) || has_response_in_worktree(repo)
}
