use chrono::{DateTime, Utc};
use ormlite::model::*;
use ormlite::sqlite::SqliteConnection;
use ormlite::Connection;
use std::error::Error;
use std::path::PathBuf;

use sqlx::migrate::Migrator;
static MIGRATOR: Migrator = sqlx::migrate!("./migrations");


#[derive(Debug)]
pub struct Cache {
    path: PathBuf,
    frequency: usize,
    last_visited: DateTime<Utc>,
}



impl Cache {
    pub fn new(path: &PathBuf) -> Self {
        Self {
            path: path.clone(),
            frequency: 1,
            last_visited: Utc::now(),
        }
    }

    pub fn _merge(&mut self, other: &Cache) {
        self.frequency += other.frequency;
        self.last_visited = other.last_visited;
    }
}

pub fn _collect_cache(path: &PathBuf, cache_vector: &mut Vec<Cache>) {
    let new_cache = Cache::new(path);

    for cache in cache_vector.iter_mut() {
        if cache.path == *path {
            cache._merge(&new_cache);
            return;
        }
    }

    // not found → add new
    cache_vector.push(new_cache);
}


#[derive(Model, Debug)]
#[ormlite(table = "store_cache")]
#[ormlite(insert = "InsertCache")]
pub struct StoredCache {
    #[ormlite(primary_key)]
    pub path: String,
    pub frequency: i64,
    pub last_visited: DateTime<Utc>,
}

pub async fn initialize_db() -> Result<(), Box<dyn Error>> {
   let mut conn = SqliteConnection::connect(&get_db_url()).await?;
    MIGRATOR.run(&mut conn).await?;
    Ok(())
}

fn get_db_url() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/gideon".to_string());
    format!("sqlite://{}/jump.db", home)
}

pub async fn store_cache(cache_vector: Vec<Cache>) -> Result<(), Box<dyn Error>> {
    
    let mut conn = SqliteConnection::connect(&get_db_url()).await?;
    let mut tx = conn.begin().await?;

    for entry in cache_vector {
       
        let path_str = entry.path.to_string_lossy().to_string();
        let freq = entry.frequency as i64;
        ormlite::query!(
            r#"
            INSERT INTO store_cache (path, frequency, last_visited)
            VALUES (?, ?, ?)
            ON CONFLICT(path) DO UPDATE SET 
                frequency = store_cache.frequency + excluded.frequency,
                last_visited = excluded.last_visited
            "#,
            path_str,
            freq,
            entry.last_visited
        )
        .execute(&mut *tx) 
        .await?;
    }
   
    tx.commit().await?;
    
    Ok(())
}

pub async fn fetch_cache() -> Result<Vec<StoredCache>, Box<dyn Error>> {
    let mut conn = SqliteConnection::connect(&get_db_url()).await?;
    
    let cache = StoredCache::select()
        .fetch_all(&mut conn)
        .await?;

    Ok(cache)
}
// cache.rs
pub async fn cleanup_old_entries() -> Result<(), Box<dyn Error>> {
    let mut conn = SqliteConnection::connect(&get_db_url()).await?;
    ormlite::query!("DELETE FROM store_cache WHERE last_visited < datetime('now', '-30 days')")
        .execute(&mut conn) // Use &mut * here
        .await?;
    Ok(())
}