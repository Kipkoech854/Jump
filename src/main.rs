use clap::Parser;
use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod cache;
mod jump_utils;
use crate::cache::*;


#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    pub d: Option<PathBuf>,

    #[arg(short, long, num_args = 1..)]
    pub exe: Option<Vec<String>>,

    #[arg(short, long)]
    pub ret: bool,

    #[arg(short, long)]
    pub clean: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let global_start = Instant::now();
    let args = Args::parse();
    let current_dir = env::current_dir()?;
    let cache_file = Path::new("/tmp/jump_cache");
    cache::initialize_db().await?;

    if let Some(path) = &args.d {
        // --- PHASE 1: LOAD PATHS ---
        // Try /tmp first, then SQLite, then fallback to deep search
        let paths = jump_utils::load_initial_paths(cache_file).await?;
        let split_dest = jump_utils::destination_split_vector(&path)?;

        // --- PHASE 2: NARROW DOWN & SELECTION ---
        let found_paths = match jump_utils::narrow_down(&split_dest, &paths) {
            Ok(results) => results,
            Err(_) => {
                let search_paths = jump_utils::perform_deep_search(cache_file).await?;
                jump_utils::narrow_down(&split_dest, &search_paths)?
            }
        };

        let final_path = if found_paths.len() > 1 {
            jump_utils::handle_multiples(&found_paths)?
        } else {
            found_paths[0].clone()
        };

        // Prevent jumping to the current directory
        if final_path == current_dir {
            eprintln!("Error: You are already in {:?}", final_path);
            return Err("Target directory is current directory".into());
        }

        // --- PHASE 3: BUILD COMMAND ---
        let mut command = jump_utils::format_jump_command(&final_path, &current_dir)?;

        // Append extra commands (-e)
        if let Some(cmds) = &args.exe {
            command.push_str(&format!(" && {}", cmds.join(" && ")));
        }

        // Handle return flag (-r)
        let mut usage_updates = vec![Cache::new(&final_path)];
        if args.ret {
            command.push_str(&format!(
                " && {}",
                jump_utils::format_jump_command(&current_dir, &final_path)?
            ));
            usage_updates.push(Cache::new(&current_dir));
        }

        // --- PHASE 4: EXECUTE & PERSIST ---
        // Print the command for the shell to 'eval'
        println!(" {}", command);

        // Record the usage in the database
        cache::store_cache(usage_updates).await?;
    }
    if args.clean{
        cache::cleanup_old_entries().await?;
    }
    eprintln!("Total time: {}ms", global_start.elapsed().as_millis());
    Ok(())
}
