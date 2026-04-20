pub mod mc {
    use regex::Regex;
    use std::collections::HashMap;
    
    static REGEX: &str = r#"--(username|version|gameDir|uuid|versionType|fml\.mcVersion|quickPlayPath)\s+(".*?"|\S+)"#;

    pub fn get_cmdline_values(cmdline: &str) -> HashMap<String,String> {
        let regex = Regex::new(REGEX).expect("Failed to create Regex");
        
        let mut result = HashMap::new();

        for captures in regex.captures_iter(&cmdline) {
            let mut key = captures.get(1).map_or("<unknown>",|m| m.as_str().trim()).to_string();
            let mut value = captures.get(2).map_or("<unknown>",|m| m.as_str().trim()).to_string();

            if value.starts_with('"') && value.ends_with('"') {
                value = value.trim_matches('"').to_string();
            }

            if key == "fml.mcVersion" {
                key = "version".to_string();
            }

            result.insert(key,value);
        }

        result
    }
}