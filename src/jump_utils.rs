use walkdir::{DirEntry, WalkDir};
use std::io::{self, Write};
use std::fs;
use std::path::{PathBuf, Path};
use std::error::Error;
use std::time::Instant;

use crate::cache;
use crate::cache::Cache;

/// Loads paths from /tmp or SQLite database
pub async fn load_initial_paths(cache_file: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    if cache_file.exists() {
        let data = fs::read_to_string(cache_file)?;
        Ok(data.lines().map(PathBuf::from).collect())
    } else {
        let found = cache::fetch_cache().await?;
        let paths: Vec<PathBuf> = found.iter().map(|item| PathBuf::from(&item.path)).collect();
        sync_temp_file(cache_file, &paths)?;
        Ok(paths)
    }
}

/// Performs the expensive filesystem walk and updates all caches
pub async fn perform_deep_search(cache_file: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let timer = Instant::now();
    eprintln!("Cache miss. Deep searching...");
    
    let fresh_paths = search_paths()?;
    
    // Update SQLite
    let entries = fresh_paths.iter().map(Cache::new).collect();
    cache::store_cache(entries).await?;
    
    // Update /tmp
    sync_temp_file(cache_file, &fresh_paths)?;
    
    eprintln!("Search took {}ms. Cache updated!", timer.elapsed().as_millis());
    Ok(fresh_paths)
}

/// Helper to write the current path list to /tmp for fast future reads
pub fn sync_temp_file(path: &Path, paths: &[PathBuf]) -> Result<(), std::io::Error> {
    let content = paths.iter()
        .map(|p| p.to_string_lossy())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, content)
}



fn is_hidden(entry: &DirEntry) -> bool {
    let ignored = vec!["target", "node_modules", ".git", "build", "venv"];
    if entry.depth() == 0 { return false; }
    let name = entry.file_name().to_str().unwrap_or("");
    name.starts_with('.') || ignored.contains(&name)
}

pub fn search_paths() -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut paths = Vec::new();
    if let Some(home_dir) = home::home_dir() {
        for entry in WalkDir::new(home_dir).into_iter().filter_entry(|e| !is_hidden(e)) {
            paths.push(entry?.path().to_path_buf());
        }
    }
    Ok(paths)
}

pub fn destination_split_vector(destination: &PathBuf) -> Result<Vec<&str>, Box<dyn Error>> {
    Ok(destination.to_str().unwrap_or("").split('/').collect())
}

fn filter_paths(split: &str, available: &[PathBuf]) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    Ok(available.iter()
        .filter(|p| p.to_string_lossy().contains(split))
        .cloned().collect())
}

pub fn narrow_down(split_dest: &[&str], paths: &[PathBuf]) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut found = paths.to_vec();
    for component in split_dest {
        found = filter_paths(component, &found)?;
    }
    let last = split_dest.last().ok_or("Empty path")?;
    let result: Vec<PathBuf> = found.into_iter()
        .filter(|p| p.file_name().map(|n| n == *last).unwrap_or(false))
        .collect();

    if result.is_empty() { Err("No match".into()) } else { Ok(result) }
}

pub fn handle_multiples(found_paths: &Vec<PathBuf>) -> Result<PathBuf, String> {
    eprintln!("Multiple paths found. Choose a number (0 to cancel):\n");
    for (i, path) in found_paths.iter().enumerate() {
        eprintln!("{} . {}", i + 1, path.display());
    }
    eprintln!("0 . None");
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|_| "Read fail")?;
    let choice: usize = input.trim().parse().map_err(|_| "Invalid input")?;

    if choice == 0 { Err("Cancelled".into()) } 
    else if choice <= found_paths.len() { Ok(found_paths[choice - 1].clone()) } 
    else { Err("Out of range".into()) }
}

pub fn format_jump_command(final_path: &PathBuf, curr_dir: &PathBuf) -> Result<String, Box<dyn Error>> {
    let dest_split = destination_split_vector(final_path)?;
    let curr_split = destination_split_vector(curr_dir)?;
    
    let shared_dir = dest_split.iter()
        .find(|&d| curr_split.contains(d))
        .ok_or("No shared directory found")?;

    if Some(&*shared_dir) == curr_split.last() {
        let index = dest_split.iter().position(|&x| x == *shared_dir).unwrap();
        Ok(format!("cd {}", dest_split[(index + 1)..].join("/")))
    } else {
        let index = curr_split.iter().position(|x| x == shared_dir).unwrap();
        let dots = vec!["cd .."; curr_split.len() - index - 1].join(" && ");
        let pos = dest_split.iter().position(|p| p == shared_dir).unwrap();
        Ok(format!("{} && cd {}", dots, dest_split[(pos + 1)..].join("/")))
    }
}