pub mod wininfo {
    pub fn get_window_bounds(windowtitle: &str) -> Option<(i32,i32,u32,u32)> {
        #[cfg(target_os = "windows")] {
            use windows::{core::PCWSTR,Win32::{UI::WindowsAndMessaging::{GetWindowRect,FindWindowW},Foundation::{RECT,HWND}}};
            use std::ptr::null_mut;

            unsafe {
                let wide: Vec<u16> = windowtitle.encode_utf16().chain(std::iter::once(0)).collect();
                let hwnd: HWND = FindWindowW(None,PCWSTR(wide.as_ptr())).unwrap_or(HWND(null_mut()));

                if hwnd.0 == null_mut() {
                    return None;
                }
        
                let mut rect = RECT::default();
        
                match GetWindowRect(hwnd,&mut rect) {
                    Ok(_) => Some((rect.top as i32,rect.left as i32,(rect.right - rect.left) as u32,(rect.bottom - rect.top) as u32)),
                    Err(_) => None
                }
            }
        }
    
        #[cfg(target_os = "linux")] {
            use x11::xlib::*;
            use std::{ffi::CStr,mem,ptr};

            unsafe fn find_window(display: *mut Display,window: Window,windowtitle: &str) -> Option<Window> {
                let mut prop: XTextProperty = mem::zeroed();
                
                if XGetWMName(display,window,&mut prop) != 0 && !prop.value.is_null() {
                    let matched = CStr::from_ptr(prop.value as *const i8)
                        .to_str()
                        .map(|name| name == windowtitle)
                        .unwrap_or(false);

                    XFree(prop.value as *mut _);

                    if matched {
                        return Some(window);
                    }
                }

                let mut root_return = 0;
                let mut parent_return = 0;
                let mut children: *mut Window = ptr::null_mut();
                let mut nchildren = 0;

                if XQueryTree(display,window,&mut root_return,&mut parent_return,&mut children,&mut nchildren) == 0 {
                    return None;
                }

                let childwindows = std::slice::from_raw_parts(children,nchildren as usize).to_vec();
                XFree(children as *mut _);

                childwindows.into_iter().find_map(|child| find_window(display,child,windowtitle))
            }

            unsafe {
                let display = XOpenDisplay(ptr::null());
                
                if display.is_null() {
                    return None;
                }

                let root = XDefaultRootWindow(display);
                
                let window = match find_window(display, root,windowtitle) {
                    Some(window) => window,
                    None => {
                        XCloseDisplay(display);
                        return None;
                    }
                };

                let mut x = 0;
                let mut y = 0;
                let mut width = 0;
                let mut height = 0;
                let mut border = 0;
                let mut depth = 0;
                let mut win_root = 0;

                let geo_res = XGetGeometry(
                    display,
                    window,
                    &mut win_root,
                    &mut x,
                    &mut y,
                    &mut width,
                    &mut height,
                    &mut border,
                    &mut depth,
                );

                let mut abs_x = 0;
                let mut abs_y = 0;
                let mut child: Window = 0;

                let trans_res = XTranslateCoordinates(
                    display,
                    window,
                    XDefaultRootWindow(display),
                    0,
                    0,
                    &mut abs_x,
                    &mut abs_y,
                    &mut child,
                );

                XCloseDisplay(display);

                if geo_res != 0 && trans_res != 0 {
                    Some((abs_y,abs_x,width,height))
                } else {
                    None
                }
            }
        }
    }
}