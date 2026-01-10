#![windows_subsystem = "windows"]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Once, atomic::{AtomicUsize, AtomicU32, Ordering}};
use std::io::{self, Write};
use std::fs::OpenOptions;
use std::ffi::c_void;
use std::os::windows::process::CommandExt;

use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::System::LibraryLoader::*, 
    Win32::UI::Shell::*,
    Win32::UI::WindowsAndMessaging::*,
    Win32::System::Threading::*,
    Win32::System::Registry::*,
    Win32::UI::Controls::*, 
    Win32::System::Com::*,
    Win32::Graphics::Gdi::*,
};

const WM_APP_TRAY_ICON: u32 = WM_APP + 1;
const WM_APP_REGISTRY_CHANGED: u32 = WM_APP + 2;
const WM_APP_START_POLLING: u32 = WM_APP + 3;
const WM_APP_STOP_POLLING: u32 = WM_APP + 4;

const ID_TRAY_ICON: u32 = 1;
const IDM_RELOAD: usize = 101;
const IDM_IMPORT: usize = 102;
const IDM_STARTUP: usize = 103;
const IDM_POLLING: usize = 104; 
const IDM_SETTINGS: usize = 105; 
const IDM_EXIT: usize = 106;

const IDC_COMBO_INTERVAL: i32 = 201;
const IDC_COMBO_LANG: i32 = 202;
const IDC_BTN_OK: i32 = 203;

const ID_TIMER_POLL: usize = 1000;
const ID_TIMER_RELOAD: usize = 999;

const RELOAD_DELAY_MS: u32 = 2000;

static CURRENT_LANG: AtomicUsize = AtomicUsize::new(0); 
static POLLING_INTERVAL_MS: AtomicU32 = AtomicU32::new(3000); 

static REGISTER_CLASS: Once = Once::new();
const WINDOW_CLASS: PCWSTR = w!("IccAutoReloaderWindow");
const SETTINGS_WINDOW_CLASS: PCWSTR = w!("IccSettingsWindow");
const REG_SUBKEY: PCWSTR = w!(r"Software\IccAutoReloader");
const REG_RUN_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const APP_NAME: PCWSTR = w!("IccAutoReloader");

fn main() -> Result<()> {
    log_debug("Application Starting...");

    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
    }

    load_settings();

    unsafe {
        let mutex_name = w!(r"Global\IccAutoReloaderInstance");
        let _h_mutex = CreateMutexW(None, TRUE, mutex_name)?;
        if let Some(err) = io::Error::last_os_error().raw_os_error() {
            if err as u32 == ERROR_ALREADY_EXISTS.0 {
                return Ok(());
            }
        }
    }

    unsafe {
        let iccex = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES,
        };
        InitCommonControlsEx(&iccex);
        
        let instance = GetModuleHandleW(None)?;

        REGISTER_CLASS.call_once(|| {
            let wc = WNDCLASSW {
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                hInstance: instance.into(),
                lpszClassName: WINDOW_CLASS,
                lpfnWndProc: Some(wnd_proc),
                ..Default::default()
            };
            RegisterClassW(&wc);

            let wc_settings = WNDCLASSW {
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                hInstance: instance.into(),
                lpszClassName: SETTINGS_WINDOW_CLASS,
                lpfnWndProc: Some(settings_wnd_proc),
                hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as isize),
                ..Default::default()
            };
            RegisterClassW(&wc_settings);
        });

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            WINDOW_CLASS,
            APP_NAME,
            WS_OVERLAPPEDWINDOW,
            0, 0, 0, 0,
            None,
            None,
            instance,
            None,
        );

        if hwnd.0 == 0 {
            return Err(Error::from_win32());
        }

        add_tray_icon(hwnd)?;

        if is_polling_enabled() {
            let interval = POLLING_INTERVAL_MS.load(Ordering::Relaxed);
            SetTimer(hwnd, ID_TIMER_POLL, interval, None);
            log_debug(&format!("Polling enabled at startup. Interval: {}ms", interval));
        }

        check_first_run(hwnd);
        reload_profile();

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).into() {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    Ok(())
}

fn get_str(id: &str) -> PCWSTR {
    let lang = CURRENT_LANG.load(Ordering::Relaxed);
    match id {
        "tray_reload" => match lang {
            1 => w!("重新加载校准"),
            2 => w!("キャリブレーションを再読み込み"),
            _ => w!("Reload Calibration"),
        },
        "tray_import" => match lang {
            1 => w!("导入 ICC 配置文件..."),
            2 => w!("ICCプロファイルをインポート..."),
            _ => w!("Import ICC Profile..."),
        },
        "tray_polling" => match lang {
            1 => w!("启用自动重新加载 (轮询)"),
            2 => w!("自動再読み込みを有効化 (ポーリング)"),
            _ => w!("Enable Auto-Reload (Polling)"),
        },
        "tray_startup" => match lang {
            1 => w!("开机启动"),
            2 => w!("スタートアップに登録"),
            _ => w!("Run at Startup"),
        },
        "tray_settings" => match lang {
            1 => w!("设置..."),
            2 => w!("設定..."),
            _ => w!("Settings..."),
        },
        "tray_exit" => match lang {
            1 => w!("退出"),
            2 => w!("終了"),
            _ => w!("Exit"),
        },
        "settings_title" => match lang {
            1 => w!("设置"),
            2 => w!("設定"),
            _ => w!("Settings"),
        },
        "lbl_interval" => match lang {
            1 => w!("轮询间隔 (秒):"),
            2 => w!("ポーリング間隔 (秒):"),
            _ => w!("Polling Interval (seconds):"),
        },
        "lbl_lang" => match lang {
            1 => w!("语言:"),
            2 => w!("言語:"),
            _ => w!("Language:"),
        },
        "btn_ok" => match lang {
            1 => w!("保存"),
            2 => w!("保存"),
            _ => w!("Save"),
        },
        "msg_install_success" => match lang {
            1 => w!("配置文件已安装并加载成功。"),
            2 => w!("プロファイルが正常にインストールされ、読み込まれました。"),
            _ => w!("Profile installed and loaded successfully."),
        },
        "msg_install_fail" => match lang {
            1 => w!("安装配置文件失败。"),
            2 => w!("プロファイルのインストールに失敗しました。"),
            _ => w!("Failed to install profile."),
        },
        "msg_dispwin_missing" => match lang {
            1 => w!("未找到 dispwin.exe"),
            2 => w!("dispwin.exe が見つかりません"),
            _ => w!("Could not find dispwin.exe"),
        },
        "msg_file_dialog_error" => match lang {
            1 => w!("无法打开文件对话框。"),
            2 => w!("ファイルダイアログを開けませんでした。"),
            _ => w!("Could not open file dialog."),
        },
        "msg_select_profile" => match lang {
            1 => w!("选择 ICC 配置文件"),
            2 => w!("ICCプロファイルを選択"),
            _ => w!("Select ICC Profile"),
        },
        "msg_first_run" => match lang {
            1 => w!("欢迎使用 Icc Auto Reloader！\n\n是否立即导入 ICC 配置文件？\n(选择“否”以使用现有系统校准)"),
            2 => w!("Icc Auto Reloader へようこそ！\n\n今すぐ ICC プロファイルをインポートしますか？\n(「いいえ」を選択すると、既存のシステムキャリブレーションを使用します)"),
            _ => w!("Welcome to Icc Auto Reloader!\n\nDo you want to import an ICC profile now?\n(Select 'No' to use existing system calibration)"),
        },
        _ => w!("?"),
    }
}

unsafe extern "system" fn wnd_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_DESTROY => {
            remove_tray_icon(window);
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            reload_profile();
            LRESULT(0)
        }
        WM_POWERBROADCAST => {
            if wparam.0 == 18 || wparam.0 == 7 {
                reload_profile();
            }
            LRESULT(1) 
        }
        WM_TIMER => {
            if wparam.0 == ID_TIMER_POLL {
                if is_gamma_linear() {
                    log_debug("Poll: Detected linear gamma. Reloading.");
                    reload_profile();
                }
            }
            LRESULT(0)
        }
        WM_APP_TRAY_ICON => {
            match lparam.0 as u32 {
                0x0205 => { show_context_menu(window); }
                0x0203 => { reload_profile(); }
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = wparam.0 & 0xFFFF; 
            match id {
                IDM_RELOAD => { reload_profile(); }
                IDM_IMPORT => { import_profile_dialog(window); }
                IDM_STARTUP => { toggle_startup(); }
                IDM_POLLING => { toggle_polling(window); }
                IDM_SETTINGS => { show_settings_dialog(window); }
                IDM_EXIT => { DestroyWindow(window); }
                _ => {}
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}

fn show_settings_dialog(owner: HWND) {
    unsafe {
        let h_inst = GetModuleHandleW(None).unwrap_or_default();
        let width = 300;
        let height = 220;
        
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = (screen_w - width) / 2;
        let y = (screen_h - height) / 2;

        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            SETTINGS_WINDOW_CLASS,
            get_str("settings_title"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX, 
            x, y, width, height,
            owner, 
            None,
            h_inst,
            None
        );
    }
}

unsafe extern "system" fn settings_wnd_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_CREATE => {
            let h_inst = GetModuleHandleW(None).unwrap_or_default();
            
            CreateWindowExW(WINDOW_EX_STYLE::default(), w!("STATIC"), get_str("lbl_interval"),
                WS_CHILD | WS_VISIBLE, 20, 20, 240, 20, window, None, h_inst, None);
            
            let h_combo_int = CreateWindowExW(WINDOW_EX_STYLE::default(), w!("COMBOBOX"), w!(""),
                WS_CHILD | WS_VISIBLE | WS_BORDER | WINDOW_STYLE(CBS_DROPDOWNLIST as u32), 
                20, 45, 240, 200, window, HMENU(IDC_COMBO_INTERVAL as isize), h_inst, None);
            
            for i in 1..=10 {
                let s = format!("{}", i);
                let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
                SendMessageW(h_combo_int, CB_ADDSTRING, WPARAM(0), LPARAM(wide.as_ptr() as isize));
            }
            
            let current_sec = POLLING_INTERVAL_MS.load(Ordering::Relaxed) / 1000;
            let idx = if current_sec >= 1 && current_sec <= 10 { current_sec - 1 } else { 2 };
            SendMessageW(h_combo_int, CB_SETCURSEL, WPARAM(idx as usize), LPARAM(0));

            CreateWindowExW(WINDOW_EX_STYLE::default(), w!("STATIC"), get_str("lbl_lang"),
                WS_CHILD | WS_VISIBLE, 20, 80, 240, 20, window, None, h_inst, None);

            let h_combo_lang = CreateWindowExW(WINDOW_EX_STYLE::default(), w!("COMBOBOX"), w!(""),
                WS_CHILD | WS_VISIBLE | WS_BORDER | WINDOW_STYLE(CBS_DROPDOWNLIST as u32), 
                20, 105, 240, 200, window, HMENU(IDC_COMBO_LANG as isize), h_inst, None);
            
            let langs = [w!("English"), w!("中文"), w!("日本語")];
            for l in langs {
                SendMessageW(h_combo_lang, CB_ADDSTRING, WPARAM(0), LPARAM(l.as_ptr() as isize));
            }
            let cur_lang = CURRENT_LANG.load(Ordering::Relaxed);
            SendMessageW(h_combo_lang, CB_SETCURSEL, WPARAM(cur_lang), LPARAM(0));

            CreateWindowExW(WINDOW_EX_STYLE::default(), w!("BUTTON"), get_str("btn_ok"),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32), 
                100, 145, 80, 30, window, HMENU(IDC_BTN_OK as isize), h_inst, None);

            ShowWindow(window, SW_SHOW);
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = wparam.0 & 0xFFFF; 
            if id as i32 == IDC_BTN_OK {
                let h_combo_int = GetDlgItem(window, IDC_COMBO_INTERVAL);
                let h_combo_lang = GetDlgItem(window, IDC_COMBO_LANG);
                
                let idx_int = SendMessageW(h_combo_int, CB_GETCURSEL, WPARAM(0), LPARAM(0));
                let idx_lang = SendMessageW(h_combo_lang, CB_GETCURSEL, WPARAM(0), LPARAM(0));
                
                if idx_int.0 != -1 {
                    let sec = (idx_int.0 as u32) + 1;
                    POLLING_INTERVAL_MS.store(sec * 1000, Ordering::Relaxed);
                    
                    let h_main = FindWindowW(WINDOW_CLASS, APP_NAME);
                    if h_main.0 != 0 && is_polling_enabled() {
                        SetTimer(h_main, ID_TIMER_POLL, sec * 1000, None);
                    }
                }
                
                if idx_lang.0 != -1 {
                    CURRENT_LANG.store(idx_lang.0 as usize, Ordering::Relaxed);
                }
                
                save_settings();
                DestroyWindow(window);
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(window, message, wparam, lparam),
    }
}

// Registry Helper: Try HKCU, then HKLM
fn get_reg_dword(val_name: PCWSTR) -> Option<u32> {
    unsafe {
        let mut val: u32 = 0;
        let mut val_size = 4u32;
        let mut key: HKEY = HKEY::default();

        // Try HKCU
        if RegOpenKeyExW(HKEY_CURRENT_USER, REG_SUBKEY, 0, KEY_READ, &mut key).is_ok() {
            let res = RegQueryValueExW(key, val_name, None, None, Some(&mut val as *mut u32 as *mut u8), Some(&mut val_size));
            RegCloseKey(key);
            if res.is_ok() {
                return Some(val);
            }
        }

        // Try HKLM
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, REG_SUBKEY, 0, KEY_READ, &mut key).is_ok() {
            let res = RegQueryValueExW(key, val_name, None, None, Some(&mut val as *mut u32 as *mut u8), Some(&mut val_size));
            RegCloseKey(key);
            if res.is_ok() {
                return Some(val);
            }
        }
    }
    None
}

fn load_settings() {
    if let Some(val) = get_reg_dword(w!("PollingInterval")) {
        if val >= 1000 && val <= 10000 {
            POLLING_INTERVAL_MS.store(val, Ordering::Relaxed);
        }
    }
    if let Some(val) = get_reg_dword(w!("Language")) {
        if val <= 2 {
            CURRENT_LANG.store(val as usize, Ordering::Relaxed);
        }
    }
}

fn save_settings() {
    unsafe {
        let mut key: HKEY = HKEY::default();
        // Save only to HKCU
        if RegCreateKeyExW(HKEY_CURRENT_USER, REG_SUBKEY, 0, None, REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None).is_ok() {
            let interval = POLLING_INTERVAL_MS.load(Ordering::Relaxed);
            let lang = CURRENT_LANG.load(Ordering::Relaxed) as u32;
            
            let _ = RegSetValueExW(key, w!("PollingInterval"), 0, REG_DWORD, Some(std::slice::from_raw_parts(&interval as *const u32 as *const u8, 4)));
            let _ = RegSetValueExW(key, w!("Language"), 0, REG_DWORD, Some(std::slice::from_raw_parts(&lang as *const u32 as *const u8, 4)));
            RegCloseKey(key);
        }
    }
}

fn is_polling_enabled() -> bool {
    if let Some(val) = get_reg_dword(w!("PollingEnabled")) {
        return val != 0;
    }
    true // Default true
}

fn toggle_polling(window: HWND) {
    let enabled = is_polling_enabled();
    let new_state = !enabled;

    unsafe {
        let mut key: HKEY = HKEY::default();
        if RegCreateKeyExW(HKEY_CURRENT_USER, REG_SUBKEY, 0, None, REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None).is_ok() {
            let val: u32 = if new_state { 1 } else { 0 };
            let _ = RegSetValueExW(key, w!("PollingEnabled"), 0, REG_DWORD, Some(std::slice::from_raw_parts(&val as *const u32 as *const u8, 4)));
            RegCloseKey(key);
        }

        if new_state {
            let interval = POLLING_INTERVAL_MS.load(Ordering::Relaxed);
            log_debug("Polling enabled by user.");
            SetTimer(window, ID_TIMER_POLL, interval, None);
        } else {
            log_debug("Polling disabled by user.");
            KillTimer(window, ID_TIMER_POLL);
        }
    }
}

fn is_startup_enabled() -> bool {
    unsafe {
        let mut key: HKEY = HKEY::default();
        // Check HKCU
        if RegOpenKeyExW(HKEY_CURRENT_USER, REG_RUN_KEY, 0, KEY_READ, &mut key).is_ok() {
            let res = RegQueryValueExW(key, APP_NAME, None, None, None, None);
            RegCloseKey(key);
            if res.is_ok() { return true; }
        }
        // Check HKLM
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, REG_RUN_KEY, 0, KEY_READ, &mut key).is_ok() {
            let res = RegQueryValueExW(key, APP_NAME, None, None, None, None);
            RegCloseKey(key);
            if res.is_ok() { return true; }
        }
    }
    false
}

fn toggle_startup() {
    unsafe {
        let enabled = is_startup_enabled();
        // Only toggle HKCU
        let mut key: HKEY = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, REG_RUN_KEY, 0, KEY_WRITE, &mut key).is_ok() {
            if enabled {
                let _ = RegDeleteValueW(key, APP_NAME);
            } else {
                if let Ok(exe_path) = std::env::current_exe() {
                    let path_str = exe_path.to_string_lossy();
                    let path_wide: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();
                    let _ = RegSetValueExW(
                        key, 
                        APP_NAME, 
                        0, 
                        REG_SZ, 
                        Some(std::slice::from_raw_parts(path_wide.as_ptr() as *const u8, path_wide.len() * 2))
                    );
                }
            }
            let _ = RegCloseKey(key);
        }
    }
}

fn check_first_run(window: HWND) {
    if let Some(val) = get_reg_dword(w!("Configured")) {
        if val == 1 { return; }
    }

    // First run (or configured=0)
    unsafe {
        let msg = get_str("msg_first_run");
        let response = MessageBoxW(
            window,
            msg,
            APP_NAME,
            MB_YESNO | MB_ICONQUESTION
        );
        
        if response == IDYES {
            import_profile_dialog(window);
        }
        
        // Save Configured=1 to HKCU
        let mut key: HKEY = HKEY::default();
        if RegCreateKeyExW(HKEY_CURRENT_USER, REG_SUBKEY, 0, None, REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None).is_ok() {
            let one: u32 = 1;
            let _ = RegSetValueExW(key, w!("Configured"), 0, REG_DWORD, Some(std::slice::from_raw_parts(&one as *const u32 as *const u8, 4)));
            RegCloseKey(key);
        }
    }
}

fn is_gamma_linear() -> bool {
    unsafe {
        let hdc = GetDC(None);
        if hdc.is_invalid() {
            return false;
        }

        let mut ramp = [[0u16; 256]; 3]; 
        let mut result_bool = false;

        if let Ok(lib) = LoadLibraryW(w!("gdi32.dll")) {
            if let Some(proc) = GetProcAddress(lib, s!("GetDeviceGammaRamp")) {
                 let func: unsafe extern "system" fn(HDC, *mut c_void) -> BOOL = std::mem::transmute(proc);
                 result_bool = func(hdc, &mut ramp as *mut _ as *mut c_void).as_bool();
            }
            let _ = FreeLibrary(lib);
        }

        ReleaseDC(None, hdc);

        if !result_bool {
            return false;
        }

        let tolerance = 256; 

        for channel in 0..3 {
            for i in 0..256 {
                let expected = (i as u32 * 65535 / 255) as i32;
                let actual = ramp[channel][i] as i32;
                if (actual - expected).abs() > tolerance {
                    return false; 
                }
            }
        }

        true 
    }
}

fn log_debug(msg: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("debug.log") {
        let _ = writeln!(file, "{}", msg);
    }
}

fn reload_profile() {
    let dispwin_path = find_dispwin();
    if let Some(path) = dispwin_path {
        let _ = Command::new(path)
            .arg("-L")
            .creation_flags(0x08000000)
            .spawn(); 
    }
}

fn import_profile_dialog(window: HWND) {
    unsafe {
        let dialog_res: Result<IFileOpenDialog> = CoCreateInstance(&FileOpenDialog, None, CLSCTX_ALL);
        if let Ok(dialog) = dialog_res {
            let _ = dialog.SetTitle(get_str("msg_select_profile"));
            
            if dialog.Show(window).is_ok() {
                if let Ok(result) = dialog.GetResult() {
                    if let Ok(path) = result.GetDisplayName(SIGDN_FILESYSPATH) {
                        if let Ok(path_str) = path.to_string() {
                            install_profile(&path_str);
                        }
                    }
                }
            }
        } else {
             MessageBoxW(window, get_str("msg_file_dialog_error"), APP_NAME, MB_OK | MB_ICONERROR);
        }
    }
}

fn install_profile(path_str: &str) {
    let dispwin_path = find_dispwin();
    if let Some(exe) = dispwin_path {
        let status = Command::new(&exe)
            .arg("-I")
            .arg(path_str)
            .creation_flags(0x08000000)
            .status();
            
        if let Ok(s) = status {
            if s.success() {
                reload_profile();
                unsafe {
                    MessageBoxW(None, get_str("msg_install_success"), APP_NAME, MB_OK | MB_ICONINFORMATION);
                }
            } else {
                 unsafe {
                    MessageBoxW(None, get_str("msg_install_fail"), APP_NAME, MB_OK | MB_ICONERROR);
                }
            }
        }
    } else {
        unsafe {
            MessageBoxW(None, get_str("msg_dispwin_missing"), APP_NAME, MB_OK | MB_ICONERROR);
        }
    }
}

fn find_dispwin() -> Option<PathBuf> {
    let candidates = [
        Path::new("bin").join("dispwin.exe"),
        std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("bin").join("dispwin.exe"))).unwrap_or_default(),
    ];

    for c in &candidates {
        if c.exists() && c.file_name().is_some() {
            return Some(c.clone());
        }
    }
    None
}

unsafe fn add_tray_icon(window: HWND) -> Result<()> {
    let mut nid = NOTIFYICONDATAW::default();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = window;
    nid.uID = ID_TRAY_ICON;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_APP_TRAY_ICON;
    nid.hIcon = LoadIconW(None, IDI_APPLICATION)?;
    
    let tip = "Icc Auto Reloader\0";
    let tip_wide: Vec<u16> = tip.encode_utf16().collect();
    if tip_wide.len() <= nid.szTip.len() {
        nid.szTip[..tip_wide.len()].copy_from_slice(&tip_wide);
    }

    Shell_NotifyIconW(NIM_ADD, &nid);
    Ok(())
}

unsafe fn remove_tray_icon(window: HWND) {
    let mut nid = NOTIFYICONDATAW::default();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = window;
    nid.uID = ID_TRAY_ICON;
    Shell_NotifyIconW(NIM_DELETE, &nid);
}

unsafe fn show_context_menu(window: HWND) {
    let mut point = POINT::default();
    GetCursorPos(&mut point);

    let hmenu = CreatePopupMenu().unwrap_or_default();
    
    let startup_enabled = is_startup_enabled();
    let startup_flags = if startup_enabled { MF_CHECKED | MF_STRING } else { MF_STRING };

    let polling_enabled = is_polling_enabled();
    let polling_flags = if polling_enabled { MF_CHECKED | MF_STRING } else { MF_STRING };

    AppendMenuW(hmenu, MF_STRING, IDM_RELOAD, get_str("tray_reload"));
    AppendMenuW(hmenu, MF_STRING, IDM_IMPORT, get_str("tray_import"));
    AppendMenuW(hmenu, MF_SEPARATOR, 0, None);
    AppendMenuW(hmenu, polling_flags, IDM_POLLING, get_str("tray_polling"));
    AppendMenuW(hmenu, startup_flags, IDM_STARTUP, get_str("tray_startup"));
    AppendMenuW(hmenu, MF_STRING, IDM_SETTINGS, get_str("tray_settings"));
    AppendMenuW(hmenu, MF_SEPARATOR, 0, None);
    AppendMenuW(hmenu, MF_STRING, IDM_EXIT, get_str("tray_exit"));

    SetForegroundWindow(window); 
    TrackPopupMenu(
        hmenu,
        TPM_BOTTOMALIGN | TPM_LEFTALIGN,
        point.x,
        point.y,
        0,
        window,
        None,
    );
    DestroyMenu(hmenu);
}
