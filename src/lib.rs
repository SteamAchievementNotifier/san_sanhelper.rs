use napi_derive::napi;

#[cfg(target_os="windows")]
pub mod win32 {
    pub use winreg::{RegKey,enums::HKEY_CURRENT_USER};
    pub const STEAMREGPATH: &str = "SOFTWARE\\Valve\\Steam";
}

#[napi]
pub fn get_steam_path() -> String {
    let steampath: String;

    if cfg!(target_os="windows") {
        use win32::{RegKey,HKEY_CURRENT_USER};
        use win32::STEAMREGPATH;

        steampath = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(STEAMREGPATH)
            .expect(&format!("Failed to open registry key \"{}\"",STEAMREGPATH))
            .get_value("SteamPath")
            .expect(&format!("Failed to get SteamPath value from \"{}\"",STEAMREGPATH))
    } else {
        eprintln!("Unsupported OS");
        steampath = "".to_string();
    }

    steampath
}

#[napi(object)]
pub struct AppInfo {
    pub appid: u32,
    pub gamename: String
}

#[napi]
pub fn get_app_info() -> Vec<AppInfo> {
    let mut appinfo = Vec::new();
    let mut appid: u32 = 0;
    let mut gamename: String = "".to_string();

    if cfg!(target_os="windows") {
        use win32::{RegKey,HKEY_CURRENT_USER};
        use win32::STEAMREGPATH;

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
    } else {
        eprintln!("Unsupported OS");
    }

    appinfo.push(AppInfo {
        appid,
        gamename
    });

    appinfo
}