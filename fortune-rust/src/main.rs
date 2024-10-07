use std::collections::HashMap;
use std::fs::{File};
use std::io::BufRead;
use std::path::PathBuf;
use rand::random;

fn main() {
    let fortune_dir = option_env!("FORTUNE_HOME")
        .unwrap_or_else(|| "/usr/share/games/fortunes");

    let fortune_path = PathBuf::from(fortune_dir);
    assert!(fortune_path.is_dir());

    let mut count : usize = 0;
    let mut fortune = String::new();
    let mut fortunes : HashMap<usize, String> = HashMap::new();
    if let Ok(dir) = std::fs::read_dir(fortune_path) {
        dir.for_each(|entry| {
            if let Ok(entry) = entry {
                let file_path = entry.path();
                if is_fortune_file(&file_path) {
                    if let Ok(file) = File::open(file_path) {
                        let reader = std::io::BufReader::new(file);
                        reader.lines().for_each(|line| {
                            if let Ok(line) = line {
                                if line == "%" {
                                    fortunes.insert(count, fortune.clone());
                                    count += 1;
                                    fortune.clear();
                                }
                                else {
                                    fortune.push_str(&line);
                                    fortune.push('\n');
                                }
                            }
                        });
                    }
                }
            }
        });
        let random_key = random::<usize>() % fortunes.len();
        println!("{}", fortunes.get(&random_key).unwrap());
    }
    else {
        println!("can't read fortunes directory");
    }
}

fn is_fortune_file(file_path: &PathBuf) -> bool {
    file_path.is_file() && file_path.extension().is_none()
}
