use std::env;
use std::path::PathBuf;

use snowball::db::SqliteDb;
use snowball::error::Error;
use snowball::tui;

fn main() -> Result<(), Error> {
    let path = db_path_from_args()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| Error::validation(err.to_string()))?;
    }
    let db = SqliteDb::open(path)?;
    tui::run(db)
}

fn db_path_from_args() -> Result<PathBuf, Error> {
    let args: Vec<String> = env::args().collect();
    if let Some(index) = args.iter().position(|arg| arg == "--db") {
        let path = args
            .get(index + 1)
            .ok_or_else(|| Error::validation("--db requires a path"))?;
        return Ok(PathBuf::from(path));
    }
    Ok(default_db_path())
}

fn default_db_path() -> PathBuf {
    if let Ok(xdg) = env::var("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("snowball").join("snowball.db");
    }
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("snowball")
            .join("snowball.db");
    }
    PathBuf::from("snowball.db")
}
