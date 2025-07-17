#[cfg(target_os = "linux")]
#[link(name = "X11")]
extern "C" {}

use napi_derive::napi;
use keypressrs;
extern crate log as extern_log;
use extern_log::{info,error};
pub mod log;
pub mod wininfo;

#[cfg(target_os="windows")]
pub mod win32 {
    pub use winreg::{RegKey,enums::{HKEY_CURRENT_USER,HKEY_LOCAL_MACHINE}};
    pub const STEAMREGPATH: &str = "SOFTWARE\\Valve\\Steam";
    pub const UNINSTALLPATH: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App";
}

#[cfg(target_os="linux")]
pub mod linux {
    pub use std::{str,path::Path,process::Command,fs::{File,metadata},io::Read};
    pub use dirs::{home_dir,data_local_dir};
    pub use keyvalues_parser::{Vdf,Value};
}

#[cfg(target_os="linux")]
fn get_key_values(value: &keyvalues_parser::Value, key: &str, all: bool) -> Vec<String> {
    use linux::Value;
    let mut res = Vec::new();

    match value {
        Value::Obj(obj) => {
            for (k, v) in obj.iter() {
                for item in v {
                    if k.to_lowercase() == key.to_lowercase() {
                        if let Value::Str(val) = item {
                            res.push(val.clone().to_string());
                            if !all {
                                return res;
                            }
                        }
                    } else {
                        res.extend(get_key_values(item, key, all));
                        if !all && !res.is_empty() {
                            return res;
                        }
                    }
                }
            }
        }
        _ => {}
    }

    res
}

#[cfg(target_os="linux")]
fn read_vdf(vdf: String, key: &str, all: bool) -> Vec<String> {
    use linux::{File, Read, Vdf};

    let mut contents = String::new();

    let mut file = match File::open(&vdf) {
        Ok(file) => file,
        Err(err) => {
            error!("Failed to open \"{}\": {}",vdf,err);
            return Vec::new();
        }
    };

    if let Err(err) = file.read_to_string(&mut contents) {
        error!("Failed to read \"{}\": {}",vdf,err);
        return Vec::new();
    }

    let parsed = match Vdf::parse(&contents) {
        Ok(parsed) => parsed,
        Err(err) => {
            error!("Failed to parse contents of \"{}\": {}",vdf,err);
            return Vec::new();
        }
    };

    get_key_values(&parsed.value,key,all)
}

#[cfg(target_os="linux")]
pub fn get_linux_steam_path() -> String {
    use linux::home_dir;

    if let Some(home) = home_dir() {
        let steam_dir = home.join(".steam");
        let registry_vdf = steam_dir.join("registry.vdf");

        return read_vdf(
                registry_vdf
                .to_string_lossy()
                .into_owned(),
                "SourceModInstallPath",
                false
            )
            .join("")
            .replace("/steamapps\\sourcemods","")
    }

    "".to_string()
}

#[allow(unreachable_code)]
#[napi]
pub fn get_steam_path() -> String {
    #[cfg(target_os="windows")] {
        use win32::{RegKey,HKEY_CURRENT_USER,STEAMREGPATH};

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        match hkcu.open_subkey(STEAMREGPATH) {
            Ok(regkey) => return regkey
                .get_value("SteamPath")
                .unwrap_or_else(|_| "".to_string()),
            Err(err) => error!("Failed to open subkey \"{}\": {}",STEAMREGPATH,err)
        }
    }

    #[cfg(target_os="linux")] {
        return get_linux_steam_path()
    }

    #[cfg(not(any(target_os="windows",target_os="linux")))] {
        error!("Unsupported OS");
    }

    "".to_string()
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

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        appid = match hkcu.open_subkey(STEAMREGPATH) {
            Ok(regkey) => regkey
                .get_value("RunningAppID")
                .unwrap_or_else(|_| 0),
            Err(err) => {
                error!("Failed to open subkey \"{}\": {}",STEAMREGPATH,err);
                0
            }
        };

        if appid != 0 {
            gamename = match hkcu.open_subkey(format!("{}\\Apps\\{}",STEAMREGPATH,appid)) {
                Ok(regkey) => regkey
                    .get_value("Name")
                    .unwrap_or_else(|_| "".to_string()),
                Err(err) => {
                    error!("Failed to open subkey \"{}\": {}",STEAMREGPATH,err);
                    "".to_string()
                }
            }
        }
    }

    #[cfg(target_os="linux")] {
        use linux::{str,Command,Path};

        let cmd = Command::new("sh")
            .arg("-c")
            .arg("ps ax | grep -Ev 'grep|Install' | grep -i 'AppID'")
            .output();

        match cmd {
            Ok(output) => {
                let res = str::from_utf8(&output.stdout);

                match res {
                    Ok(stdout) => {
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
                                let lib_folders = Path::new(&steam_path)
                                    .join("steamapps")
                                    .join("libraryfolders.vdf")
                                    .to_string_lossy()
                                    .into_owned();

                                let lib_paths = read_vdf(lib_folders,"path",true);
                                for lib_path in lib_paths {
                                    let acf = Path::new(&lib_path)
                                        .join("steamapps")
                                        .join(format!("appmanifest_{}.acf",appid));

                                    if acf.exists() {
                                        gamename = read_vdf(acf.to_string_lossy().into_owned(),"name",false).join("");
                                        break;
                                    } else {
                                        error!("Failed to locate \"appmanifest_{}.acf\" in \"{}\"",appid,lib_path);
                                    }
                                }
                            }
                        }
                    },
                    Err(err) => error!("Failed to parse \"res\": {}",err)
                }
            },
            Err(err) => error!("Failed to execute \"psaux\" command: {}",err)
        }
    }
    
    #[cfg(not(any(target_os="windows",target_os="linux")))] {
        error!("Unsupported OS");
    }

    appinfo.push(AppInfo {
        appid,
        gamename
    });

    appinfo
}

#[napi]
pub fn press_key(keys: Vec<u16>) {
    keypressrs::simulate_keypress(keys);
}

#[napi]
pub fn get_hq_icon(appid: u32) -> String {
    #[cfg(target_os="windows")] {
        use win32::{RegKey,HKEY_LOCAL_MACHINE,UNINSTALLPATH};

        let appdir = format!("{} {}",UNINSTALLPATH,appid);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        match hklm.open_subkey(appdir) {
            Ok(regkey) => return regkey
                .get_value("DisplayIcon")
                .unwrap_or_else(|_| "".to_string()),
            Err(err) => error!("Failed to get \"DisplayIcon\": {}",err)
        }
    }

    #[cfg(target_os = "linux")]
    {
        use linux::data_local_dir;

        let resolutions = [
            "256x256",
            "128x128",
            "64x64",
            "32x32",
            "24x24",
            "16x16"
        ];

        if let Some(share) = data_local_dir() {
            let base_dir = share
                .join("icons")
                .join("hicolor");

            if base_dir.exists() {
                for res in &resolutions {
                    let icon_path = base_dir
                        .join(res)
                        .join("apps")
                        .join(format!("steam_icon_{}.png",appid));

                    if icon_path.exists() {
                        return icon_path.to_string_lossy().to_string();
                    } else {
                        error!("Failed to locate \"{:?}\" in \"{:?}\"",icon_path,base_dir);
                    }
                }
            } else {
                error!("Failed to locate \"{:?}\"",base_dir);
            }
        } else {
            error!("Failed to locate \"homedir\"");
        }
    }

    "".to_string()
}

#[napi]
pub fn deps_installed(lib: String) -> String {
    if lib == "keypressrs" {
        if !keypressrs::deps_installed() {
            return "xdotool".to_string();
        };
    } else if lib == "hdr" {
        return hdr_deps();
    } else if lib == "wmctrl" {
        return wmctrl_deps();
    }

    "".to_string()
}

fn hdr_deps() -> String {
    #[cfg(target_os="linux")] {
        use linux::*;

        let deps = vec![
            "libxcb.so",
            "libXrandr.so",
            "libdbus-1.so"
        ];

        for dep in deps {
            let installed = Command::new("sh")
                .args(["-c",&format!("ldconfig -p | grep {}",dep)])
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false);

            if !installed {
                error!("\"{}\" not installed",dep);
                return dep.to_string()
            }
        }
    }

    "".to_string()
}

// Note: Requires `sudo apt install libxcb-xfixes0-dev` to compile on Linux
fn capture_hdr_screenshot(screen: screenshots::Screen,sspath: String,area: Option<(u32,u32,u32,u32)>) -> String {
    // Order of elements for `screen.capture_area()` is y/x/w/h
    let capture = match area {
        Some((y,x,w,h)) => screen.capture_area(y as i32,x as i32,w,h),
        None => screen.capture()
    };

    match capture {
        Ok(img) => {
            let save = img.save(&sspath);

            if let Err(err) = save {
                error!("Failed to save image: {}",err);
                return format!("Failed to save image: {}",err)
            }

            info!("\"{}\" saved successfully",&sspath);
            return format!("\"{}\" saved successfully",&sspath)
        },
        Err(err) => {
            error!("Failed to capture screen: {}",err);
            return format!("Failed to capture screen: {}",err)
        }
    }
}

#[napi]
pub fn hdr_screenshot(monitor_id: u32,sspath: String,area: Option<(u32,u32,u32,u32)>) -> String {
    use screenshots::Screen;

    let screens = Screen::all();

    match screens {
        Ok(screens) => {
            let mut primary = None;
            
            for screen in screens {
                if screen.display_info.id == monitor_id {
                    info!("\"screen.display_info.id\" ({}) matched to \"monitor_id\" ({}) successfully",screen.display_info.id,monitor_id);

                    return capture_hdr_screenshot(screen,sspath,area);
                }

                if screen.display_info.is_primary {
                    primary = Some(screen.clone());
                }
            }

            if let Some(p_screen) = primary {
                error!("No match found for \"monitor_id\" ({}) - fallback to primary monitor",monitor_id);
                return capture_hdr_screenshot(p_screen,sspath,area)
            } else {
                error!("Failed to locate screen matching \"monitor_id\" ({}), and no primary monitor located",monitor_id);
                format!("Failed to locate screen matching \"monitor_id\" ({}), and no primary monitor located",monitor_id)
            }
        }
        Err(err) => {
            error!("Failed to parse monitor list: {}",err);
            format!("Failed to parse monitor list: {}",err)
        }
    }
}

#[napi]
pub fn get_focused_win_path() -> String {
    use active_win_pos_rs::get_active_window;

    match get_active_window() {
        Ok(win) => win.process_path.to_string_lossy().to_string(),
        Err(_) => "".to_string()
    }
}

fn wmctrl_deps() -> String {
    #[cfg(target_os="linux")] {
        use linux::*;
        
        let installed = Command::new("sh")
            .args(["-c","which wmctrl"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);

        if !installed {
            return "wmctrl".to_string()
        }
    }

    "".to_string()
}

#[napi(object)]
pub struct WinBounds {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32
}

#[napi]
pub fn get_window_bounds(windowtitle: String) -> WinBounds {
    use wininfo::wininfo::get_window_bounds;

    let (x,y,width,height) = match get_window_bounds(&windowtitle) {
        Some((x,y,width,height)) => (x,y,width,height),
        None => (0,0,0,0)
    };

    WinBounds {
        width,
        height,
        x,
        y
    }
}