//! A native GDI window: no resident webview, browser process, or animation loop.
use std::{
    mem::size_of,
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicIsize, Ordering},
        Mutex, OnceLock,
    },
};
use tauri::AppHandle;
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::{LibraryLoader::GetModuleHandleW, Registry::*},
    UI::{HiDpi::*, WindowsAndMessaging::*},
};

static HANDLE: AtomicIsize = AtomicIsize::new(0);
static APP: OnceLock<AppHandle> = OnceLock::new();
static CELLS: OnceLock<Mutex<Vec<(String, String, bool)>>> = OnceLock::new();
const REPAINT: u32 = WM_APP + 1;
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | g << 8 | b << 16
}

pub fn create(app: AppHandle) {
    if APP.set(app.clone()).is_err() {
        return;
    }
    CELLS.get_or_init(|| Mutex::new(super::cells(&[], chrono::Utc::now().timestamp())));
    std::thread::Builder::new()
        .name("usage-strip".into())
        .spawn(move || unsafe {
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let instance = GetModuleHandleW(null());
            let name = wide("GcdUsageNativeStrip");
            let mut class: WNDCLASSW = std::mem::zeroed();
            class.lpfnWndProc = Some(wndproc);
            class.hInstance = instance;
            class.lpszClassName = name.as_ptr();
            class.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
            if RegisterClassW(&class) == 0 {
                return;
            }
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                name.as_ptr(),
                wide("GCD Usage · drag the left grip, click to open").as_ptr(),
                WS_POPUP,
                0,
                0,
                500,
                56,
                null_mut(),
                null_mut(),
                instance,
                null(),
            );
            if hwnd.is_null() {
                return;
            }
            HANDLE.store(hwnd as isize, Ordering::Release);
            let position = crate::strip_position(&app);
            position_window(hwnd, position);
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })
        .ok();
}
pub fn update(cells: Vec<(String, String, bool)>) {
    *CELLS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = cells;
    let hwnd = HANDLE.load(Ordering::Acquire) as HWND;
    if !hwnd.is_null() {
        unsafe {
            PostMessageW(hwnd, REPAINT, 0, 0);
        }
    }
}
pub fn shutdown() {
    let hwnd = HANDLE.load(Ordering::Acquire) as HWND;
    if !hwnd.is_null() {
        unsafe {
            PostMessageW(hwnd, WM_CLOSE, 0, 0);
        }
    }
}

unsafe fn position_window(hwnd: HWND, saved: Option<(i32, i32)>) {
    let mut rect: RECT = std::mem::zeroed();
    GetWindowRect(hwnd, &mut rect);
    let pt = saved.map(|(x, y)| POINT { x, y }).unwrap_or(POINT {
        x: rect.left,
        y: rect.top,
    });
    let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
    let mut info: MONITORINFO = std::mem::zeroed();
    info.cbSize = size_of::<MONITORINFO>() as u32;
    if GetMonitorInfoW(monitor, &mut info) == 0 {
        return;
    }
    let scale = GetDpiForWindow(hwnd).max(96) as f64 / 96.0;
    let width = (500.0 * scale) as i32;
    let height = (56.0 * scale) as i32;
    let margin = (8.0 * scale) as i32;
    let (x, y) = saved.unwrap_or((
        info.rcWork.right - width - margin,
        info.rcWork.bottom - height - margin,
    ));
    let max_x = (info.rcWork.right - width).max(info.rcWork.left);
    let max_y = (info.rcWork.bottom - height).max(info.rcWork.top);
    SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        x.clamp(info.rcWork.left, max_x),
        y.clamp(info.rcWork.top, max_y),
        width,
        height,
        SWP_NOACTIVATE,
    );
}
unsafe fn light_theme() -> bool {
    let mut value = 1u32;
    let mut bytes = 4u32;
    RegGetValueW(
        HKEY_CURRENT_USER,
        wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize").as_ptr(),
        wide("AppsUseLightTheme").as_ptr(),
        RRF_RT_REG_DWORD,
        null_mut(),
        &mut value as *mut _ as *mut _,
        &mut bytes,
    );
    value != 0
}
unsafe fn paint(hwnd: HWND) {
    let mut paint: PAINTSTRUCT = std::mem::zeroed();
    let dc = BeginPaint(hwnd, &mut paint);
    let mut bounds: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut bounds);
    let light = light_theme();
    let background = if light {
        rgb(247, 248, 245)
    } else {
        rgb(27, 32, 32)
    };
    let foreground = if light {
        rgb(30, 45, 42)
    } else {
        rgb(232, 241, 233)
    };
    let muted = if light {
        rgb(105, 119, 111)
    } else {
        rgb(151, 168, 155)
    };
    let brush = CreateSolidBrush(background);
    FillRect(dc, &bounds, brush);
    DeleteObject(brush);
    let accent = CreateSolidBrush(rgb(89, 158, 124));
    let edge = RECT {
        left: 0,
        top: 0,
        right: 3,
        bottom: bounds.bottom,
    };
    FillRect(dc, &edge, accent);
    DeleteObject(accent);
    SetBkMode(dc, TRANSPARENT as i32);
    let dpi = GetDpiForWindow(hwnd).max(96) as i32;
    let label_font = CreateFontW(
        -11 * dpi / 96,
        0,
        0,
        0,
        500,
        0,
        0,
        0,
        DEFAULT_CHARSET as u32,
        OUT_DEFAULT_PRECIS as u32,
        CLIP_DEFAULT_PRECIS as u32,
        CLEARTYPE_QUALITY as u32,
        DEFAULT_PITCH as u32,
        wide("Segoe UI").as_ptr(),
    );
    let value_font = CreateFontW(
        -14 * dpi / 96,
        0,
        0,
        0,
        600,
        0,
        0,
        0,
        DEFAULT_CHARSET as u32,
        OUT_DEFAULT_PRECIS as u32,
        CLIP_DEFAULT_PRECIS as u32,
        CLEARTYPE_QUALITY as u32,
        DEFAULT_PITCH as u32,
        wide("Segoe UI").as_ptr(),
    );
    let old = SelectObject(dc, label_font);
    SetTextColor(dc, muted);
    let mut grip = RECT {
        left: 8 * dpi / 96,
        top: 0,
        right: 25 * dpi / 96,
        bottom: bounds.bottom,
    };
    let grip_text = wide("⠿");
    DrawTextW(
        dc,
        grip_text.as_ptr(),
        -1,
        &mut grip,
        DT_SINGLELINE | DT_VCENTER | DT_CENTER,
    );
    let cells = CELLS
        .get()
        .unwrap()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let start = 32 * dpi / 96;
    let cell_width = (bounds.right - start) / 3;
    for (i, (label, value, stale)) in cells.iter().enumerate() {
        SelectObject(dc, label_font);
        SetTextColor(dc, muted);
        let left = start + i as i32 * cell_width;
        let mut label_rect = RECT {
            left,
            top: 7 * dpi / 96,
            right: left + cell_width - 8 * dpi / 96,
            bottom: 24 * dpi / 96,
        };
        DrawTextW(
            dc,
            wide(label).as_ptr(),
            -1,
            &mut label_rect,
            DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        SelectObject(dc, value_font);
        SetTextColor(dc, if *stale { muted } else { foreground });
        let mut value_rect = RECT {
            left,
            top: 26 * dpi / 96,
            right: left + cell_width - 8 * dpi / 96,
            bottom: 49 * dpi / 96,
        };
        DrawTextW(
            dc,
            wide(value).as_ptr(),
            -1,
            &mut value_rect,
            DT_SINGLELINE | DT_END_ELLIPSIS,
        );
    }
    SelectObject(dc, old);
    DeleteObject(label_font);
    DeleteObject(value_font);
    EndPaint(hwnd, &paint);
}
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint(hwnd);
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
        WM_NCHITTEST => {
            let mut rect: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut rect);
            let x = (lparam as u32 & 0xffff) as i16 as i32;
            if x - rect.left < 28 * GetDpiForWindow(hwnd).max(96) as i32 / 96 {
                HTCAPTION as isize
            } else {
                HTCLIENT as isize
            }
        }
        WM_LBUTTONUP | WM_CONTEXTMENU => {
            if let Some(app) = APP.get() {
                let a = app.clone();
                let _ = app.run_on_main_thread(move || {
                    let _ = super::show_dashboard(&a);
                });
            }
            0
        }
        WM_EXITSIZEMOVE => {
            let mut rect: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut rect);
            if let Some(app) = APP.get() {
                crate::save_strip_position(app, rect.left, rect.top);
            }
            0
        }
        WM_DPICHANGED => {
            let rect = &*(lparam as *const RECT);
            position_window(hwnd, Some((rect.left, rect.top)));
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_DISPLAYCHANGE | WM_SETTINGCHANGE => {
            let mut rect: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut rect);
            position_window(hwnd, Some((rect.left, rect.top)));
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_POWERBROADCAST => {
            if wparam == 7 || wparam == 18 {
                if let Some(app) = APP.get() {
                    crate::request_refresh(app);
                }
            }
            1
        }
        REPAINT => {
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            HANDLE.store(0, Ordering::Release);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
