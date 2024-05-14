
use napi_derive::napi;
use keypressrs;

#[cfg(target_os="windows")]
pub mod win32 {
    pub use winreg::{RegKey,enums::HKEY_CURRENT_USER};
    pub const STEAMREGPATH: &str = "SOFTWARE\\Valve\\Steam";
}

#[cfg(target_os="linux")]
pub mod linux {
    pub use std::{str,path::Path,process::Command,fs::{File,metadata},io::Read};
    pub use dirs::home_dir;
    pub use keyvalues_parser::{Vdf,Value};
}

#[cfg(target_os="linux")]
fn get_key_value(value: &keyvalues_parser::Value, key: &str) -> Option<String> {
    use linux::Value;

    match value {
        Value::Obj(obj) => {
            for (k, v) in obj.iter() {
                for item in v {
                    if k.to_lowercase() == key.to_lowercase() {
                        if let Value::Str(val) = item {
                            return Some(val.clone().to_string());
                        }
                    } else {
                        if let Some(result) = get_key_value(item, key) {
                            return Some(result);
                        }
                    }
                }
            }
        }
        _ => {}
    }

    None
}

#[cfg(target_os="linux")]
fn read_vdf(vdf: String,key: &str) -> String {
    use linux::{File,Read,Vdf};

    let mut contents = String::new();

    let _file = File::open(&vdf)
        .expect(&format!("Failed to open \"{}\"",vdf))
        .read_to_string(&mut contents)
        .expect(&format!("Failed to read \"{}\"",vdf));

    let parsed = Vdf::parse(&contents)
        .expect(&format!("Failed to parse contents of \"{}\"",vdf));

    if let Some(value) = Some(parsed.value) {
        if let Some(result) = get_key_value(&value, key) {
            return result;
        }
    }

    "".to_string()
}

#[cfg(target_os="linux")]
pub fn get_linux_steam_path() -> String {
    use linux::home_dir;

    if let Some(home) = home_dir() {
        let steam_dir = home.join(".steam");
        let registry_vdf = steam_dir.join("registry.vdf");

        return read_vdf(registry_vdf.to_string_lossy().into_owned(),"SourceModInstallPath")
            .replace("/steamapps\\sourcemods","")
    }

    "".to_string()
}

#[napi]
pub fn get_steam_path() -> String {
    #[cfg(target_os="windows")] {
        use win32::*;

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
        use win32::*;

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
        use linux::{str,Command,File,Read};

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

                gamename = read_vdf(acf,"name");
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

#[napi]
pub fn press_key(key: u16) {
    keypressrs::simulate_keypress(key);
}

#[napi]
pub fn deps_installed() -> bool {
    keypressrs::deps_installed()
}