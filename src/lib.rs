
use napi_derive::napi;

#[cfg(target_os="windows")]
pub mod win32 {
    pub use winreg::{RegKey,enums::HKEY_CURRENT_USER};
    pub const STEAMREGPATH: &str = "SOFTWARE\\Valve\\Steam";
}

#[cfg(target_os="linux")]
pub fn get_linux_steam_path() -> String {
    use std::fs;
    use dirs::home_dir;

    if let Some(home) = home_dir() {
        let dirs = [
            home.join(".local/share"),
            home.join(".steam"),
            home.join(".var/apps/com.valvesoftware.Steam/.steam"),
            home.join(".var/snapd/snaps/.steam")
        ];

        for dir in dirs.iter() {
            if let Ok(metadata) = fs::metadata(dir) {
                if metadata.is_dir() {
                    return Some(dir.join("Steam").to_string_lossy().to_string()).unwrap()
                } else {
                    eprintln!("\"{}\" exists, but is not a directory",dir.display());
                }
            } else {
                eprintln!("\"{}\" does not exist",dir.display());
            };
        }
    };

    "".to_string()
}

#[napi]
pub fn get_steam_path() -> String {
    #[cfg(target_os="windows")] {
        use win32::{RegKey,HKEY_CURRENT_USER,STEAMREGPATH};

        return RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(STEAMREGPATH)
            .expect(&format!("Failed to open registry key \"{}\"",STEAMREGPATH))
            .get_value("SteamPath")
            .expect(&format!("Failed to get SteamPath value from \"{}\"",STEAMREGPATH))
    }

    #[cfg(target_os="linux")] {
        return get_linux_steam_path()
    }

    #[cfg(not(any(target_os="windows",target_os="linux")))] {
        eprintln!("Unsupported OS");
        return "".to_string()
    }
}

#[napi(object)]
pub struct AppInfo {
    pub appid: u32,
    pub gamename: String
}

#[allow(unused_assignments)]
#[napi]
pub fn get_app_info() -> Vec<AppInfo> {
    let mut appinfo = Vec::new();
    let mut appid: u32 = 0;
    let mut gamename: String = "".to_string();

    #[cfg(target_os="windows")] {
        use win32::{RegKey,HKEY_CURRENT_USER,STEAMREGPATH};

        appid = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(STEAMREGPATH)
            .expect(&format!("Failed to open registry key \"{}\"",STEAMREGPATH))
            .get_value("RunningAppID")
            .expect(&format!("Failed to get RunningAppID value from \"{}\"",STEAMREGPATH));

        if appid != 0 {
            gamename = RegKey::predef(HKEY_CURRENT_USER)
                .open_subkey(format!("{}\\Apps\\{}",STEAMREGPATH,appid))
                .expect(&format!("Failed to open registry key \"{}\\Apps\\{}\"",STEAMREGPATH,appid))
                .get_value("Name")
                .expect(&format!("Failed to get Name value from \"{}\\Apps\\{}\"",STEAMREGPATH,appid));
        }
    }

    #[cfg(target_os="linux")] {
        use std::{str,process::Command,fs::File,io::Read};
        use keyvalues_parser;

        let output = Command::new("sh")
            .arg("-c")
            .arg("ps aux | grep -v 'grep' | grep -i 'AppID'")
            .output()
            .expect("Failed to execute command");

        let stdout = str::from_utf8(&output.stdout).expect("Failed to parse stdout: Invalid UTF-8");

        if !stdout.is_empty() {
            for line in stdout.lines() {
                if let Some(arg_str) = line.split("AppId=").nth(1) {
                    if let Some(end_idx) = arg_str.find(char::is_whitespace) {
                        if let Ok(parsed_appid) = arg_str[..end_idx].trim().parse::<u32>() {
                            appid = parsed_appid;
                            break;
                        }
                    } else {
                        if let Ok(parsed_appid) = arg_str.trim().parse::<u32>() {
                            appid = parsed_appid;
                            break;
                        }
                    }
                }
            }

            if appid != 0 {
                let steam_path = get_linux_steam_path();
                let acf = std::path::Path::new(&steam_path)
                    .join("steamapps")
                    .join(format!("appmanifest_{}.acf",appid))
                    .to_string_lossy()
                    .into_owned();

                let mut contents = String::new();
                let _file = File::open(&acf)
                    .expect(&format!("Failed to open \"{}\"",acf))
                    .read_to_string(&mut contents)
                    .expect(&format!("Failed to read \"{}\"",acf));

                let parsed = keyvalues_parser::Vdf::parse(&contents)
                    .expect(&format!("Failed to parse contents of \"{}\"",acf));

                if let Some(values) = parsed.value.unwrap_obj().get("name") {
                    for value in values {
                        if let keyvalues_parser::Value::Str(name) = value {
                            gamename = name.to_string();
                            break;
                        }
                    }
                }
            }
        }
    }
    
    #[cfg(not(any(target_os="windows",target_os="linux")))] {
        eprintln!("Unsupported OS");
    }

    appinfo.push(AppInfo {
        appid,
        gamename
    });

    appinfo
}