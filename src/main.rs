use clap::Parser;
use std::env::{self};
use std::error::Error;
use std::fs::{self};
use std::io:: Write;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};
use std::io;

mod cache;

use crate::cache::Cache;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    //path to destination file
    #[arg(short, long)]
    pub d: PathBuf,

    // command(s) to execute in destination
    #[arg(short, long, num_args = 1..)]
    pub exe: Option<Vec<String>>,

    //command for an extra go
    //#[arg(short, long)]
    //pub go:PathBuf,

    //return to start point
    #[arg(short, long)]
    pub ret: bool,
}

fn is_hidden(entry: &DirEntry) -> bool {
    let ignored_folders = vec!["target", "node_modules", ".git", "build", "venv"];

    if entry.depth() == 0 {
        return false;
    }

    let name = entry.file_name().to_str().unwrap_or("");
    name.starts_with('.') || ignored_folders.contains(&name)
}

fn search_paths() -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut paths: Vec<PathBuf> = Vec::new();

    if let Some(home_dir) = home::home_dir() {
        for entry in WalkDir::new(home_dir)
            .into_iter()
            .filter_entry(|e| !is_hidden(e))
        {
            let entry = entry?;
            paths.push(entry.path().to_path_buf());
        }
    }

    Ok(paths)
}

fn destination_split_vector(destination: &PathBuf) -> Result<Vec<&str>, Box<dyn Error>> {
    let d: Vec<&str> = destination
        .to_str() 
        .unwrap_or("") 
        .split('/') 
        .collect();

    Ok(d)
}

fn filter_paths(
    split_path: &str,
    available_paths: &[PathBuf],
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    Ok(available_paths
        .iter()
        .filter(|p| p.to_string_lossy().contains(split_path))
        .cloned()
        .collect())
}
fn narrow_down(
    split_destination_path: &[&str],
    paths: &[PathBuf],
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut found_paths = paths.to_vec();

  
    for component in split_destination_path {
        let mut filtered_paths = filter_paths(component, &found_paths)?;
        found_paths.clear();
        found_paths.append(&mut filtered_paths);
    }

    let last_component = split_destination_path.last().ok_or("Empty path")?;

    
    let result: Vec<PathBuf> = found_paths
        .into_iter()
        .filter(|p| {
            p.file_name()
                .map(|name| name == *last_component) 
                .unwrap_or(false)
        })
        .collect();

    if result.is_empty() {
        Err("No matching paths found".into())
    } else {
        Ok(result)
    }
}
fn handle_multiples(found_paths: &Vec<PathBuf>) -> Result<PathBuf, String> {
    eprintln!("The path you selected appears to be in use multiple times.");
    eprintln!("In the future, consider prepending a distinctive parent name.");
    eprintln!("Choose a number for the destination, or 0 to cancel:\n");

    // 1. Use .enumerate() to get the index (i) and the value (path)
    for (i, path) in found_paths.iter().enumerate() {
        eprintln!("{} . {}", i + 1, path.display());
    }
    eprintln!("0 . None");

    // Flush stdout to ensure the prompt appears before input
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|_| "Failed to read line".to_string())?;

    
    let choice: usize = input.trim().parse().map_err(|_| "Invalid number entered")?;

    // 3. Logic for selection
    if choice == 0 {
        Err("Cancelled the jump process".to_string())
    } else if choice <= found_paths.len() {
        // Adjust back to 0-based index
        Ok(found_paths[choice - 1].clone())
    } else {
        Err("Selection out of range".to_string())
    }
}

fn format_jump_command(final_path: &PathBuf, curr_dir: &PathBuf) -> Result<String, Box<dyn Error>> {
   

    let split_destination_path = destination_split_vector(final_path)?;
    let curr_split_path = destination_split_vector(&curr_dir)?;

    let mut shared_dir: Option<&str> = None;

    for d_path in split_destination_path.iter() {
        for curr_path in curr_split_path.iter() {
            if d_path == curr_path {
                shared_dir = Some(*curr_path);
            }
        }
    }

    let shared_dir = shared_dir.ok_or("No shared directory found")?;

    let mut formatted_jump_command = String::new();

    if Some(&shared_dir) == curr_split_path.last() {
        if let Some(index) = split_destination_path.iter().position(|&x| x == shared_dir) {
            let path_slice = split_destination_path[(index + 1)..].join("/");
            let mut command = String::from("cd ");
            command.push_str(&path_slice);
            formatted_jump_command.push_str(&command);

            return Ok(formatted_jump_command);
        }
    } else {
        if let Some(index) = curr_split_path.iter().position(|x| *x == shared_dir) {
            let scroll_back_length = curr_split_path.len() - index - 1;
            let mut command_vec = Vec::new();
            let scrollback_command = String::from("cd ..");
            for _i in 0..scroll_back_length {
                command_vec.push(&scrollback_command);
            }
            if let Some(pos) = split_destination_path.iter().position(|p| *p == shared_dir) {
                let path_slice = split_destination_path[(pos + 1)..].join("/");

                let mut scrollback_slice = command_vec
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<&str>>()
                    .join("&&");
                scrollback_slice.push_str(" &&");
                let mut cd_command = String::from("cd ");
                cd_command.push_str(&path_slice);
                scrollback_slice.push_str(&cd_command);

                return Ok(scrollback_slice);
            }
        }
    }
    Err("Failed to format jump command".into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let current_directory = env::current_dir()?;
    let  cache_path = Path::new("/tmp/jump_cache");
    
    let mut paths: Vec<PathBuf> = if cache_path.exists() {
        let data = fs::read_to_string(cache_path).unwrap();
        data.lines().map(PathBuf::from).collect()
    } else {
        let found = cache::fetch_cache().await.unwrap();

        let _file_content = found
            .iter()
            .map(|p| p.path.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let all_paths: Vec<PathBuf> = found
            .into_iter()
            .map(|item| PathBuf::from(item.path))
            .collect();
        all_paths
    };

    let split_destination_path = destination_split_vector(&args.d).unwrap();

    let  found_paths = match narrow_down(&split_destination_path, &paths) {
        Ok(results) => results,

        Err(_) => {
            paths = search_paths().unwrap_or_default();
            narrow_down(&split_destination_path, &paths)?
        }
    };

    let final_path: PathBuf = if found_paths.len() > 1 {
        handle_multiples(&found_paths)?
    } else {
        found_paths[0].clone()
    };
    if final_path == current_directory {
        eprintln!("Error: You are already in {:?}", final_path);
        return Err("Target directory is the same as current directory".into());
    }
    let mut cache_vector = Vec::new();
    let new_cache = Cache::new(&final_path);
    cache_vector.push(new_cache);
    
    let mut command1 = format_jump_command(&final_path, &current_directory)?;
    if let Some(do_arguments) = &args.exe {
        let joined_cmds = do_arguments.join(" && ");

        command1.push_str(" && ");
        command1.push_str(&joined_cmds);
    }

    if args.ret {
        let return_command = format_jump_command(&current_directory, &final_path)?;
        command1.push_str(" && ");
        command1.push_str(&return_command);
        cache::collect_cache(&current_directory, &mut cache_vector);
    }
    //print command for bash eval
    println!(" {}", command1);
    
    cache::store_cache(cache_vector).await.unwrap();
    Ok(())
}
