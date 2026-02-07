use std::fs;
use std::path::Path;
use std::process::Command;

pub fn open_editor(initial_content: &str) -> Result<String, String> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    let tmp_dir = std::env::temp_dir();
    let tmp_file = tmp_dir.join(format!("termquiz_{}.txt", std::process::id()));

    fs::write(&tmp_file, initial_content)
        .map_err(|e| format!("Cannot write temp file: {}", e))?;

    let status = Command::new(&editor)
        .arg(&tmp_file)
        .status()
        .map_err(|e| format!("Cannot open editor '{}': {}", editor, e))?;

    if !status.success() {
        let _ = fs::remove_file(&tmp_file);
        return Err("Editor exited with error".to_string());
    }

    let result = fs::read_to_string(&tmp_file)
        .map_err(|e| format!("Cannot read editor result: {}", e))?;

    let _ = fs::remove_file(&tmp_file);
    Ok(result)
}

pub fn pick_file() -> Result<Option<String>, String> {
    // Try zenity first
    if let Ok(output) = Command::new("zenity")
        .args(["--file-selection"])
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(Some(path));
            }
        }
        // User cancelled
        return Ok(None);
    }

    // Zenity not available - fall back to text input
    // Return None to signal that the TUI should handle path input
    Err("zenity_unavailable".to_string())
}

pub fn validate_file(
    path: &str,
    max_size: Option<u64>,
    accept: &[String],
) -> Result<(), String> {
    let p = Path::new(path);

    if !p.exists() {
        return Err(format!("File not found: {}", path));
    }

    if !p.is_file() {
        return Err(format!("Not a file: {}", path));
    }

    // Check size
    if let Some(max) = max_size {
        let metadata = fs::metadata(p).map_err(|e| format!("Cannot stat file: {}", e))?;
        if metadata.len() > max {
            return Err(format!(
                "File too large: {} bytes (max {} bytes)",
                metadata.len(),
                max
            ));
        }
    }

    // Check extension
    if !accept.is_empty() {
        let ext = p
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        if !accept.iter().any(|a| a == &ext) {
            return Err(format!(
                "File type '{}' not allowed. Accepted: {}",
                ext,
                accept.join(", ")
            ));
        }
    }

    Ok(())
}

pub fn copy_file_to_state(src: &str, repo_dir: &Path, qnum: u32) -> Result<String, String> {
    let src_path = Path::new(src);
    let filename = src_path
        .file_name()
        .ok_or_else(|| "Invalid file name".to_string())?;

    let dest_dir = repo_dir.join("response").join("files").join(format!("q{}", qnum));
    fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Cannot create file dir: {}", e))?;

    let dest = dest_dir.join(filename);
    fs::copy(src_path, &dest).map_err(|e| format!("Cannot copy file: {}", e))?;

    Ok(dest.to_string_lossy().to_string())
}

