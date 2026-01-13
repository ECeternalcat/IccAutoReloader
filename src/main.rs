#![windows_subsystem = "windows"]

extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;

use nwd::NwgUi;
use nwg::NativeUi;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, AtomicU32, AtomicBool, Ordering};
use std::os::windows::process::CommandExt;
use std::time::Duration;
use std::ffi::c_void;

// Keep windows crate for low-level system calls
use windows::core::{PCWSTR, PCSTR, w};
use windows::Win32::Foundation::{BOOL, TRUE, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::System::Registry::*;
use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC, HDC};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::WindowsAndMessaging::{SetMenuItemInfoW, MENUITEMINFOW};
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};

// --- Global State ---
static CURRENT_LANG: AtomicUsize = AtomicUsize::new(0);
static POLLING_INTERVAL_MS: AtomicU32 = AtomicU32::new(3000);
static SETTINGS_OPEN: AtomicBool = AtomicBool::new(false);

const APP_NAME: PCWSTR = w!("IccAutoReloader");
const REG_SUBKEY: PCWSTR = w!(r"Software\IccAutoReloader");
const REG_RUN_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");

// --- Localization ---
fn get_str(id: &str) -> String {
    let lang = CURRENT_LANG.load(Ordering::Relaxed);
    let s = match id {
        "tray_reload" => match lang {
            1 => "重新加载校准",
            2 => "キャリブレーションを再読み込み",
            _ => "Reload Calibration",
        },
        "tray_import" => match lang {
            1 => "导入 ICC 配置文件...",
            2 => "ICCプロファイルをインポート...",
            _ => "Import ICC Profile...",
        },
        "tray_polling" => match lang {
            1 => "启用自动重新加载 (轮询)",
            2 => "自動再読み込みを有効化 (ポーリング)",
            _ => "Enable Auto-Reload (Polling)",
        },
        "tray_startup" => match lang {
            1 => "开机启动",
            2 => "スタートアップに登録",
            _ => "Run at Startup",
        },
        "tray_settings" => match lang {
            1 => "设置...",
            2 => "設定...",
            _ => "Settings...",
        },
        "tray_exit" => match lang {
            1 => "退出",
            2 => "終了",
            _ => "Exit",
        },
        "settings_title" => match lang {
            1 => "设置",
            2 => "設定",
            _ => "Settings",
        },
        "lbl_interval" => match lang {
            1 => "轮询间隔 (秒):",
            2 => "ポーリング間隔 (秒):",
            _ => "Polling Interval (seconds):",
        },
        "lbl_lang" => match lang {
            1 => "语言:",
            2 => "言語:",
            _ => "Language:",
        },
        "btn_save" => match lang {
            1 => "保存",
            2 => "保存",
            _ => "Save",
        },
        "msg_install_success" => match lang {
            1 => "配置文件已安装并加载成功。",
            2 => "プロファイルが正常にインストールされ、読み込まれました。",
            _ => "Profile installed and loaded successfully.",
        },
        "msg_install_fail" => match lang {
            1 => "安装配置文件失败。",
            2 => "プロファイルのインストールに失敗しました。",
            _ => "Failed to install profile.",
        },
        "msg_dispwin_missing" => match lang {
            1 => "未找到 dispwin.exe",
            2 => "dispwin.exe が見つかりません",
            _ => "Could not find dispwin.exe",
        },
        "msg_select_profile" => match lang {
            1 => "选择 ICC 配置文件",
            2 => "ICCプロファイルを選択",
            _ => "Select ICC Profile",
        },
        "wiz_title" => match lang {
            1 => "Icc Auto Reloader - 设置向导",
            2 => "Icc Auto Reloader - セットアップウィザード",
            _ => "Icc Auto Reloader - Setup Wizard",
        },
        "wiz_lbl_lang" => match lang {
            1 => "请选择界面语言:",
            2 => "言語を選択してください:",
            _ => "Please select your language:",
        },
        "wiz_welcome" => match lang {
            1 => "欢迎！\n\n请导入一个 ICC 配置文件以开始使用。\n这将确保您的显示器颜色始终准确。",
            2 => "ようこそ！\n\n開始するには ICC プロファイルをインポートしてください。\nこれにより、モニターの色が常に正確に保たれます。",
            _ => "Welcome!\n\nPlease import an ICC profile to get started.\nThis ensures your display colors remain accurate.",
        },
        "wiz_btn_import" => match lang {
            1 => "导入",
            2 => "インポート",
            _ => "Import",
        },
        "wiz_btn_skip" => match lang {
            1 => "跳过",
            2 => "スキップ",
            _ => "Skip",
        },
        "wiz_btn_next" => match lang {
            1 => "下一步 >",
            2 => "次へ >",
            _ => "Next >",
        },
        "wiz_congrats" => match lang {
            1 => "恭喜！设置已完成。\n\nIcc Auto Reloader 将在后台运行并保护您的校准。",
            2 => "おめでとうございます！準備が完了しました。\n\nIcc Auto Reloader はバックグラウンドで実行され、\nキャリブレーションを保護します。",
            _ => "Congratulations! You are all set.\n\nIcc Auto Reloader will run in the background\nand protect your calibration.",
        },
        "wiz_chk_startup" => match lang {
            1 => "开机时自动运行",
            2 => "スタートアップ時に実行",
            _ => "Run at startup",
        },
        "wiz_btn_finish" => match lang {
            1 => "完成",
            2 => "完了",
            _ => "Finish",
        },
        "wiz_title_welcome" => match lang {
            1 => "欢迎",
            2 => "ようこそ",
            _ => "Welcome",
        },
        "wiz_step1_title" => match lang {
            1 => "导入 ICC 配置文件",
            2 => "ICCプロファイルをインポート",
            _ => "Import ICC Profile",
        },
        "wiz_step1_desc" => match lang {
            1 => "选择一个ICC配置文件，或跳过以使用Windows已安装的配置文件。",
            2 => "ICCプロファイルを選択するか、スキップしてWindowsのデフォルトプロファイルを使用します。",
            _ => "Select an ICC profile, or skip to use the Windows default profile.",
        },
        "wiz_step1_no_profile" => match lang {
            1 => "未选择配置文件（将使用Windows默认）",
            2 => "プロファイル未選択（Windowsのデフォルトを使用）",
            _ => "No profile selected (Windows default will be used)",
        },
        "wiz_step1_profile" => match lang {
            1 => "配置文件: ",
            2 => "プロファイル: ",
            _ => "Profile: ",
        },
        "wiz_step2_title" => match lang {
            1 => "设置完成！",
            2 => "セットアップ完了！",
            _ => "Setup Complete!",
        },
        "wiz_btn_back" => match lang {
            1 => "< 返回",
            2 => "< 戻る",
            _ => "< Back",
        },
        "wiz_btn_cancel" => match lang {
            1 => "取消",
            2 => "キャンセル",
            _ => "Cancel",
        },
        _ => id,
    };
    s.to_string()
}


// --- Main Entry ---
fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");

    // Single Instance Check
    unsafe {
        let mutex_name = w!(r"Global\IccAutoReloaderInstance");
        let _h_mutex = CreateMutexW(None, TRUE, mutex_name).unwrap();
        if let Some(err) = std::io::Error::last_os_error().raw_os_error() {
            if err as u32 == ERROR_ALREADY_EXISTS.0 {
                return;
            }
        }
    }

    // Set Per-Monitor DPI V2 for best text rendering on high DPI displays
    // This must be called before any windows are created
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    load_settings();
    reload_profile();

    // Check first run
    if need_first_run() {
        // Show Wizard
        let _app = WizardApp::build_ui(Default::default()).expect("Failed to build Wizard UI");
        nwg::dispatch_thread_events();
        // After wizard loop ends, continue to tray
    }

    // Run Tray App
    let _app = SystemTrayApp::build_ui(Default::default()).expect("Failed to build Tray UI");
    nwg::dispatch_thread_events();
}

// --- Wizard Window ---
#[derive(Default, NwgUi)]
pub struct WizardApp {
    #[nwg_control(size: (900, 560), position: (200, 100), title: "Icc Auto Reloader - Setup Wizard", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [WizardApp::exit], OnInit: [WizardApp::init] )]
    window: nwg::Window,

    icon: RefCell<Option<nwg::Icon>>,

    // Main layout: 3 columns for buttons at bottom
    #[nwg_layout(parent: window, max_column: Some(3), max_row: Some(6), margin: [25, 25, 25, 25], spacing: 15)]
    layout: nwg::GridLayout,

    // ===== Step 0: Language Selection =====
    #[nwg_control(text: "Welcome")]
    #[nwg_layout_item(layout: layout, col: 0, row: 0, col_span: 3)]
    step0_title: nwg::Label,

    #[nwg_control(text: "Please select your language:")]
    #[nwg_layout_item(layout: layout, col: 0, row: 1, col_span: 3)]
    step0_lbl: nwg::Label,

    #[nwg_control(collection: vec!["English", "中文", "日本語"])]
    #[nwg_layout_item(layout: layout, col: 0, row: 2, col_span: 3)]
    #[nwg_events( OnComboxBoxSelection: [WizardApp::change_lang] )]
    step0_combo: nwg::ComboBox<&'static str>,

    // ===== Step 1: Import ICC Profile =====
    #[nwg_control(text: "Import ICC Profile")]
    #[nwg_layout_item(layout: layout, col: 0, row: 0, col_span: 3)]
    step1_title: nwg::Label,

    #[nwg_control(text: "", size: (840, 60))]
    #[nwg_layout_item(layout: layout, col: 0, row: 1, col_span: 3)]
    step1_lbl_welcome: nwg::Label,

    #[nwg_control(text: "No profile selected", size: (840, 25))]
    #[nwg_layout_item(layout: layout, col: 0, row: 2, col_span: 3)]
    step1_lbl_status: nwg::Label,

    #[nwg_control(text: "Import...", size: (140, 35))]
    #[nwg_layout_item(layout: layout, col: 1, row: 3)]
    #[nwg_events( OnButtonClick: [WizardApp::import_profile] )]
    step1_btn_import: nwg::Button,

    // ===== Step 2: Complete =====
    #[nwg_control(text: "Setup Complete!")]
    #[nwg_layout_item(layout: layout, col: 0, row: 0, col_span: 3)]
    step2_title: nwg::Label,

    #[nwg_control(text: "")]
    #[nwg_layout_item(layout: layout, col: 0, row: 1, col_span: 3)]
    step2_lbl_congrats: nwg::Label,

    #[nwg_control(text: "Run at startup", check_state: CheckBoxState::Checked)]
    #[nwg_layout_item(layout: layout, col: 0, row: 2, col_span: 3)]
    step2_chk_startup: nwg::CheckBox,

    // ===== Bottom Button Bar (3 columns, 3 separate buttons) =====
    #[nwg_control(text: "< Back", enabled: false, size: (140, 35))]
    #[nwg_layout_item(layout: layout, col: 0, row: 5)]
    #[nwg_events( OnButtonClick: [WizardApp::back_step] )]
    btn_back: nwg::Button,

    #[nwg_control(text: "Next >", size: (140, 35))]
    #[nwg_layout_item(layout: layout, col: 1, row: 5)]
    #[nwg_events( OnButtonClick: [WizardApp::next_step] )]
    btn_next: nwg::Button,

    #[nwg_control(text: "Cancel", size: (140, 35))]
    #[nwg_layout_item(layout: layout, col: 2, row: 5)]
    #[nwg_events( OnButtonClick: [WizardApp::exit] )]
    btn_cancel: nwg::Button,

    current_step: RefCell<usize>,
    profile_imported: RefCell<bool>,
}

impl WizardApp {
    fn init(&self) {
        // Load and set icon
        if let Ok(icon) = nwg::Icon::from_file(r"C:\Users\etern\OneDrive\文档\Code\IccAutoReloader\icon.ico", true) {
            self.window.set_icon(Some(&icon));
            *self.icon.borrow_mut() = Some(icon);
        }

        let cur = CURRENT_LANG.load(Ordering::Relaxed);
        self.step0_combo.set_selection(Some(cur));
        self.show_step(0);
        self.update_text();
    }

    fn show_step(&self, step: usize) {
        // Hide all step-specific controls first
        self.step0_title.set_visible(false);
        self.step0_lbl.set_visible(false);
        self.step0_combo.set_visible(false);
        self.step1_title.set_visible(false);
        self.step1_lbl_welcome.set_visible(false);
        self.step1_lbl_status.set_visible(false);
        self.step1_btn_import.set_visible(false);
        self.step2_title.set_visible(false);
        self.step2_lbl_congrats.set_visible(false);
        self.step2_chk_startup.set_visible(false);

        let mut current = self.current_step.borrow_mut();

        match step {
            0 => {
                self.step0_title.set_visible(true);
                self.step0_lbl.set_visible(true);
                self.step0_combo.set_visible(true);
                self.btn_back.set_enabled(false);
                self.btn_next.set_enabled(true);
                self.btn_next.set_text(&get_str("wiz_btn_next"));
                *current = 0;
            }
            1 => {
                self.step1_title.set_visible(true);
                self.step1_lbl_welcome.set_visible(true);
                self.step1_lbl_status.set_visible(true);
                self.step1_btn_import.set_visible(true);
                self.btn_back.set_enabled(true);
                // Allow skipping - Next button is always enabled
                self.btn_next.set_enabled(true);
                self.btn_next.set_text(&get_str("wiz_btn_next"));
                *current = 1;
            }
            2 => {
                self.step2_title.set_visible(true);
                self.step2_lbl_congrats.set_visible(true);
                self.step2_chk_startup.set_visible(true);
                self.btn_back.set_enabled(true);
                self.btn_next.set_text(&get_str("wiz_btn_finish"));
                *current = 2;
            }
            _ => {}
        }
    }

    fn change_lang(&self) {
        if let Some(idx) = self.step0_combo.selection() {
            CURRENT_LANG.store(idx, Ordering::Relaxed);
            self.update_text();
        }
    }

    fn back_step(&self) {
        let step = *self.current_step.borrow();
        if step > 0 {
            self.show_step(step - 1);
        }
    }

    fn next_step(&self) {
        let step = *self.current_step.borrow();
        if step == 2 {
            self.finish();
        } else if step < 2 {
            self.show_step(step + 1);
        }
    }

    fn import_profile(&self) {
        let mut dialog = nwg::FileDialog::default();
        nwg::FileDialog::builder()
            .title(&get_str("msg_select_profile"))
            .action(nwg::FileDialogAction::Open)
            .build(&mut dialog)
            .expect("Failed to create file dialog");

        if dialog.run(Some(&self.window)) {
            if let Ok(path) = dialog.get_selected_item() {
                let path_str = path.to_string_lossy().to_string();
                install_profile(&path_str);
                *self.profile_imported.borrow_mut() = true;
                let file_name = std::path::Path::new(&path_str)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown");
                self.step1_lbl_status.set_text(&format!("{}{}", get_str("wiz_step1_profile"), file_name));
            }
        }
    }

    fn finish(&self) {
        if self.step2_chk_startup.check_state() == nwg::CheckBoxState::Checked {
            set_startup_state(true);
        } else {
            set_startup_state(false);
        }
        mark_configured();
        save_settings();
        nwg::stop_thread_dispatch();
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }

    fn update_text(&self) {
        self.window.set_text(&get_str("wiz_title"));

        // Buttons
        self.btn_back.set_text(&get_str("wiz_btn_back"));
        self.btn_cancel.set_text(&get_str("wiz_btn_cancel"));
        let step = *self.current_step.borrow();
        if step == 2 {
            self.btn_next.set_text(&get_str("wiz_btn_finish"));
        } else {
            self.btn_next.set_text(&get_str("wiz_btn_next"));
        }

        // Step 0
        self.step0_title.set_text(&get_str("wiz_title_welcome"));
        self.step0_lbl.set_text(&get_str("wiz_lbl_lang"));

        // Step 1
        self.step1_title.set_text(&get_str("wiz_step1_title"));
        self.step1_lbl_welcome.set_text(&get_str("wiz_step1_desc"));
        if !*self.profile_imported.borrow() {
            self.step1_lbl_status.set_text(&get_str("wiz_step1_no_profile"));
        }
        self.step1_btn_import.set_text(&get_str("wiz_btn_import"));

        // Step 2
        self.step2_title.set_text(&get_str("wiz_step2_title"));
        self.step2_lbl_congrats.set_text(&get_str("wiz_congrats"));
        self.step2_chk_startup.set_text(&get_str("wiz_chk_startup"));
    }
}

// --- Settings Window ---
#[derive(Default, NwgUi)]
pub struct SettingsApp {
    #[nwg_control(size: (360, 220), position: (300, 300), title: "Settings", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [SettingsApp::on_close], OnInit: [SettingsApp::init] )]
    window: nwg::Window,

    icon: RefCell<Option<nwg::Icon>>,

    // Use GridLayout for the form fields
    #[nwg_layout(parent: window, max_column: Some(2), max_row: Some(3), margin: [20, 20, 20, 20], spacing: 15)]
    layout: nwg::GridLayout,

    #[nwg_control(text: "Polling Interval:")]
    #[nwg_layout_item(layout: layout, col: 0, row: 0)]
    lbl_interval: nwg::Label,

    #[nwg_control(collection: vec!["1", "2", "3", "4", "5"])]
    #[nwg_layout_item(layout: layout, col: 1, row: 0)]
    combo_interval: nwg::ComboBox<&'static str>,

    #[nwg_control(text: "Language:")]
    #[nwg_layout_item(layout: layout, col: 0, row: 1)]
    lbl_lang: nwg::Label,

    #[nwg_control(collection: vec!["English", "中文", "日本語"])]
    #[nwg_layout_item(layout: layout, col: 1, row: 1)]
    #[nwg_events( OnComboxBoxSelection: [SettingsApp::lang_changed] )]
    combo_lang: nwg::ComboBox<&'static str>,

    // Button uses absolute positioning to avoid being squashed by GridLayout
    #[nwg_control(text: "Save", size: (130, 38), position: (115, 155))]
    #[nwg_events( OnButtonClick: [SettingsApp::save] )]
    btn_save: nwg::Button,
}

impl SettingsApp {
    fn init(&self) {
        // Load and set icon (same path as WizardApp)
        if let Ok(icon) = nwg::Icon::from_file(r"C:\Users\etern\OneDrive\文档\Code\IccAutoReloader\icon.ico", true) {
            self.window.set_icon(Some(&icon));
            *self.icon.borrow_mut() = Some(icon);
        }

        // Update localized text
        self.update_text();

        // Init interval combo
        let current_ms = POLLING_INTERVAL_MS.load(Ordering::Relaxed);
        let sec = current_ms / 1000;
        let idx = if sec >= 1 && sec <= 5 { (sec - 1) as usize } else { 2 };
        self.combo_interval.set_selection(Some(idx));

        // Init lang combo
        let cur_lang = CURRENT_LANG.load(Ordering::Relaxed);
        self.combo_lang.set_selection(Some(cur_lang));
    }

    fn update_text(&self) {
        self.window.set_text(&get_str("settings_title"));
        self.lbl_interval.set_text(&get_str("lbl_interval"));
        self.lbl_lang.set_text(&get_str("lbl_lang"));
        self.btn_save.set_text(&get_str("btn_save"));
    }

    fn lang_changed(&self) {
        if let Some(idx) = self.combo_lang.selection() {
            CURRENT_LANG.store(idx, Ordering::Relaxed);
            self.update_text();
        }
    }

    fn save(&self) {
        // Save Interval
        if let Some(idx) = self.combo_interval.selection() {
            let sec = (idx as u32) + 1;
            POLLING_INTERVAL_MS.store(sec * 1000, Ordering::Relaxed);
        }

        save_settings();
        self.window.close();
    }

    fn on_close(&self) {
        // Signal that settings window is closed
        SETTINGS_OPEN.store(false, Ordering::Relaxed);
    }
}

// --- System Tray App ---
#[derive(Default, NwgUi)]
pub struct SystemTrayApp {
    #[nwg_resource(source_bin: Some(include_bytes!("../icon.ico")))]
    tray_icon: nwg::Icon,

    #[nwg_control(title: "Icc Auto Reloader Hidden", flags: "DISABLED")]
    #[nwg_events( OnWindowClose: [SystemTrayApp::exit], OnInit: [SystemTrayApp::init] )]
    window: nwg::Window,

    #[nwg_control(icon: Some(&data.tray_icon), tip: Some("Icc Auto Reloader"))]
    #[nwg_events( MousePressLeftUp: [SystemTrayApp::reload_click], OnContextMenu: [SystemTrayApp::show_menu] )]
    tray: nwg::TrayNotification,

    // Store settings window as opaque type to keep it alive
    settings_window: RefCell<Option<Box<dyn std::any::Any>>>,

    #[nwg_control(parent: window, popup: true)]
    tray_menu: nwg::Menu,

    #[nwg_control(parent: tray_menu, text: "Reload")]
    #[nwg_events( OnMenuItemSelected: [SystemTrayApp::reload_click] )]
    item_reload: nwg::MenuItem,

    #[nwg_control(parent: tray_menu, text: "Import...")]
    #[nwg_events( OnMenuItemSelected: [SystemTrayApp::import_click] )]
    item_import: nwg::MenuItem,

    #[nwg_control(parent: tray_menu)]
    item_sep1: nwg::MenuSeparator,

    #[nwg_control(parent: tray_menu, text: "Polling", check: true)]
    #[nwg_events( OnMenuItemSelected: [SystemTrayApp::toggle_polling] )]
    item_polling: nwg::MenuItem,

    #[nwg_control(parent: tray_menu, text: "Startup", check: true)]
    #[nwg_events( OnMenuItemSelected: [SystemTrayApp::toggle_startup] )]
    item_startup: nwg::MenuItem,

    #[nwg_control(parent: tray_menu, text: "Settings...")]
    #[nwg_events( OnMenuItemSelected: [SystemTrayApp::open_settings] )]
    item_settings: nwg::MenuItem,

    #[nwg_control(parent: tray_menu)]
    item_sep2: nwg::MenuSeparator,

    #[nwg_control(parent: tray_menu, text: "Exit")]
    #[nwg_events( OnMenuItemSelected: [SystemTrayApp::exit] )]
    item_exit: nwg::MenuItem,

    #[nwg_control(parent: window)]
    #[nwg_events( OnTimerTick: [SystemTrayApp::on_timer] )]
    poll_timer: nwg::AnimationTimer,
}

// Helper function to set menu item text using Windows API
fn set_menu_item_text(_menu: &nwg::Menu, item: &nwg::MenuItem, text: &str) {
    use nwg::ControlHandle;

    let (hmenu, item_id) = match item.handle {
        ControlHandle::MenuItem(parent, id) => (parent, id),
        ControlHandle::NoHandle => return,
        _ => return,
    };

    let text_wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::MIIM_STRING;
        use windows::core::PWSTR;
        let info = MENUITEMINFOW {
            cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_STRING,
            dwTypeData: PWSTR::from_raw(text_wide.as_ptr() as *mut u16),
            cch: text_wide.len() as u32,
            ..Default::default()
        };
        let _ = SetMenuItemInfoW(windows::Win32::UI::WindowsAndMessaging::HMENU(hmenu as isize), item_id, false, &info);
    }
}

impl SystemTrayApp {
    fn init(&self) {
        // Icon is already loaded via nwg_resource

        // Set internationalized menu text
        set_menu_item_text(&self.tray_menu, &self.item_reload, &get_str("tray_reload"));
        set_menu_item_text(&self.tray_menu, &self.item_import, &get_str("tray_import"));
        set_menu_item_text(&self.tray_menu, &self.item_polling, &get_str("tray_polling"));
        set_menu_item_text(&self.tray_menu, &self.item_startup, &get_str("tray_startup"));
        set_menu_item_text(&self.tray_menu, &self.item_settings, &get_str("tray_settings"));
        set_menu_item_text(&self.tray_menu, &self.item_exit, &get_str("tray_exit"));

        if is_polling_enabled() {
            let interval = POLLING_INTERVAL_MS.load(Ordering::Relaxed);
            self.poll_timer.set_interval(Duration::from_millis(interval as u64));
            self.poll_timer.start();
        }
    }

    fn show_menu(&self) {
        // Update internationalized menu text (in case language was changed)
        set_menu_item_text(&self.tray_menu, &self.item_reload, &get_str("tray_reload"));
        set_menu_item_text(&self.tray_menu, &self.item_import, &get_str("tray_import"));
        set_menu_item_text(&self.tray_menu, &self.item_polling, &get_str("tray_polling"));
        set_menu_item_text(&self.tray_menu, &self.item_startup, &get_str("tray_startup"));
        set_menu_item_text(&self.tray_menu, &self.item_settings, &get_str("tray_settings"));
        set_menu_item_text(&self.tray_menu, &self.item_exit, &get_str("tray_exit"));

        // Update checks
        self.item_polling.set_checked(is_polling_enabled());
        self.item_startup.set_checked(is_startup_enabled());

        let (x, y) = nwg::GlobalCursor::position();
        self.tray_menu.popup(x, y);
    }

    fn reload_click(&self) {
        reload_profile();
    }

    fn import_click(&self) {
        let mut dialog = nwg::FileDialog::default();
        nwg::FileDialog::builder()
            .title(&get_str("msg_select_profile"))
            .action(nwg::FileDialogAction::Open)
            .build(&mut dialog)
            .expect("Failed to create file dialog");

        if dialog.run(Some(&self.window)) {
            if let Ok(path) = dialog.get_selected_item() {
                install_profile(&path.to_string_lossy());
            }
        }
    }

    fn toggle_polling(&self) {
        let current = is_polling_enabled();
        set_polling_enabled(!current);
        if !current { // If now enabled
            let interval = POLLING_INTERVAL_MS.load(Ordering::Relaxed);
            self.poll_timer.set_interval(Duration::from_millis(interval as u64));
            self.poll_timer.start();
        } else {
            self.poll_timer.stop();
        }
    }

    fn toggle_startup(&self) {
        set_startup_state(!is_startup_enabled());
    }

    fn open_settings(&self) {
        // Check if settings window is already open
        if SETTINGS_OPEN.load(Ordering::Relaxed) {
            // Already open, bring to front
            return;
        }

        SETTINGS_OPEN.store(true, Ordering::Relaxed);

        // Create and store settings window to keep it alive
        let settings = SettingsApp::build_ui(Default::default()).expect("Failed to build Settings UI");
        *self.settings_window.borrow_mut() = Some(Box::new(settings));
    }

    fn on_timer(&self) {
        if !is_polling_enabled() {
            self.poll_timer.stop();
            return;
        }

        // Check if settings window was closed (flag is false but window still exists)
        if !SETTINGS_OPEN.load(Ordering::Relaxed) && self.settings_window.borrow().is_some() {
            // Settings window closed, clean it up
            *self.settings_window.borrow_mut() = None;
        }

        // Sync interval if changed in settings
        let target_interval = POLLING_INTERVAL_MS.load(Ordering::Relaxed);
        let target_duration = Duration::from_millis(target_interval as u64);

        self.poll_timer.set_interval(target_duration);

        // Make sure timer is running when settings window is closed
        if !SETTINGS_OPEN.load(Ordering::Relaxed) {
            self.poll_timer.start();
        }

        if is_gamma_linear() {
            reload_profile();
        }
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}

// --- Helper Functions ---

fn is_gamma_linear() -> bool {
    unsafe {
        let hdc = GetDC(None);
        if hdc.is_invalid() {
            return false;
        }

        let mut ramp = [[0u16; 256]; 3]; 
        let mut result_bool = false;

        // Dynamic load GetDeviceGammaRamp to avoid link errors
        let lib = GetModuleHandleW(w!("gdi32.dll"));
        if let Ok(lib) = lib {
            let proc = GetProcAddress(lib, PCSTR(b"GetDeviceGammaRamp\0".as_ptr() as _));
            if let Some(proc) = proc {
                 let func: unsafe extern "system" fn(HDC, *mut c_void) -> BOOL = std::mem::transmute(proc);
                 result_bool = func(hdc, &mut ramp as *mut _ as *mut c_void).as_bool();
            }
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

fn reload_profile() {
    let dispwin_path = find_dispwin();
    if let Some(path) = dispwin_path {
        let _ = Command::new(path)
            .arg("-L")
            .creation_flags(0x08000000) 
            .spawn(); 
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
                nwg::simple_message("Success", &get_str("msg_install_success"));
            } else {
                nwg::error_message("Error", &get_str("msg_install_fail"));
            }
        }
    } else {
        nwg::error_message("Error", &get_str("msg_dispwin_missing"));
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

// Registry Logic (using windows crate primitives for compatibility)
fn get_reg_dword(val_name: PCWSTR) -> Option<u32> {
    unsafe {
        let mut val: u32 = 0;
        let mut val_size = 4u32;
        let mut key: HKEY = HKEY::default();

        if RegOpenKeyExW(HKEY_CURRENT_USER, REG_SUBKEY, 0, KEY_READ, &mut key).is_ok() {
            let res = RegQueryValueExW(key, val_name, None, None, Some(&mut val as *mut u32 as *mut u8), Some(&mut val_size));
            let _ = RegCloseKey(key);
            if res.is_ok() { return Some(val); }
        }

        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, REG_SUBKEY, 0, KEY_READ, &mut key).is_ok() {
            let res = RegQueryValueExW(key, val_name, None, None, Some(&mut val as *mut u32 as *mut u8), Some(&mut val_size));
            let _ = RegCloseKey(key);
            if res.is_ok() { return Some(val); }
        }
    }
    None
}

fn load_settings() {
    if let Some(val) = get_reg_dword(w!("PollingInterval")) {
        if val >= 1000 && val <= 5000 {
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
        if RegCreateKeyExW(HKEY_CURRENT_USER, REG_SUBKEY, 0, None, REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None).is_ok() {
            let interval = POLLING_INTERVAL_MS.load(Ordering::Relaxed);
            let lang = CURRENT_LANG.load(Ordering::Relaxed) as u32;
            let polling_enabled = if is_polling_enabled() { 1u32 } else { 0u32 };

            let _ = RegSetValueExW(key, w!("PollingInterval"), 0, REG_DWORD, Some(std::slice::from_raw_parts(&interval as *const u32 as *const u8, 4)));
            let _ = RegSetValueExW(key, w!("Language"), 0, REG_DWORD, Some(std::slice::from_raw_parts(&lang as *const u32 as *const u8, 4)));
            let _ = RegSetValueExW(key, w!("PollingEnabled"), 0, REG_DWORD, Some(std::slice::from_raw_parts(&polling_enabled as *const u32 as *const u8, 4)));
            let _ = RegCloseKey(key);
        }
    }
}

fn is_polling_enabled() -> bool {
    if let Some(val) = get_reg_dword(w!("PollingEnabled")) {
        return val != 0;
    }
    true 
}

fn set_polling_enabled(enabled: bool) {
    unsafe {
        let mut key: HKEY = HKEY::default();
        if RegCreateKeyExW(HKEY_CURRENT_USER, REG_SUBKEY, 0, None, REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None).is_ok() {
            let val: u32 = if enabled { 1 } else { 0 };
            let _ = RegSetValueExW(key, w!("PollingEnabled"), 0, REG_DWORD, Some(std::slice::from_raw_parts(&val as *const u32 as *const u8, 4)));
            let _ = RegCloseKey(key);
        }
    }
}

fn is_startup_enabled() -> bool {
    unsafe {
        let mut key: HKEY = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, REG_RUN_KEY, 0, KEY_READ, &mut key).is_ok() {
            let res = RegQueryValueExW(key, APP_NAME, None, None, None, None);
            let _ = RegCloseKey(key);
            if res.is_ok() { return true; }
        }
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, REG_RUN_KEY, 0, KEY_READ, &mut key).is_ok() {
            let res = RegQueryValueExW(key, APP_NAME, None, None, None, None);
            let _ = RegCloseKey(key);
            if res.is_ok() { return true; }
        }
    }
    false
}

fn set_startup_state(enable: bool) {
    unsafe {
        let mut key: HKEY = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, REG_RUN_KEY, 0, KEY_WRITE, &mut key).is_ok() {
            if !enable {
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

fn mark_configured() {
    unsafe {
        let mut key: HKEY = HKEY::default();
        if RegCreateKeyExW(HKEY_CURRENT_USER, REG_SUBKEY, 0, None, REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None).is_ok() {
            let one: u32 = 1;
            let _ = RegSetValueExW(key, w!("Configured"), 0, REG_DWORD, Some(std::slice::from_raw_parts(&one as *const u32 as *const u8, 4)));
            let _ = RegCloseKey(key);
        }
    }
}

fn need_first_run() -> bool {
    if let Some(val) = get_reg_dword(w!("Configured")) {
        if val == 1 { return false; }
    }
    true
}
