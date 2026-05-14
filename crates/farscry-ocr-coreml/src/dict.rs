use std::fs;

pub const PP_OCR_V5_DICT: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~ \n\t";

pub fn get_character_dict() -> Vec<String> {
    let current_dir = std::env::current_dir().ok();
    if let Some(cwd) = current_dir {
        let dict_path = cwd.join("spike/models/ppocrv5_en_dict.txt");
        if dict_path.exists() {
            if let Ok(content) = fs::read_to_string(&dict_path) {
                let character_entries: Vec<String> = content
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| line.trim().to_string())
                    .collect();

                if !character_entries.is_empty() {
                    return character_entries;
                }
            }
        }
    }

    PP_OCR_V5_DICT.chars().map(|c| c.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dict_size() {
        let dict = get_character_dict();
        assert!(!dict.is_empty());
        eprintln!("Dictionary size: {}", dict.len());
    }
}
