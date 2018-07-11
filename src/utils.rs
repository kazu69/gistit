
use std::path::PathBuf;
use std::env;

pub fn get_file_path(file: &str) -> PathBuf {
    let current_dir: PathBuf = env::current_dir().unwrap();
    let mut p = PathBuf::from(&current_dir);
    p.push(&file);
    p
}

pub fn get_home_dir() -> PathBuf {
    let home_dir: PathBuf = match env::home_dir() {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(""),
    };
    home_dir
}
