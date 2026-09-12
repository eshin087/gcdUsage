//! Native, double-buffered dock and hover card. No resident webview or animation loop.
use super::geometry::{
    corner_at, resize_corner, Corner, DOCK_WIDTH, DOCK_HEIGHT, dock_cell, drag_origin, hover_decision, popup_origin, strip_origin, HoverDecision, Rect,
};
#[path = "duration_picker.rs"]
mod duration_picker;
use crate::{
    dock::{compact, interval},
    models::ColorTheme,
};
use duration_picker::duration_menu;
use std::{
    mem::size_of,
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicIsize, AtomicUsize, Ordering},
        Mutex, OnceLock,
    },
    time::Instant,
};
use tauri::AppHandle;
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::{LibraryLoader::GetModuleHandleW, Registry::*},
    UI::{
        HiDpi::*,
        Input::KeyboardAndMouse::{ReleaseCapture, SetCapture},
        WindowsAndMessaging::*,
    },
};

static HANDLE: AtomicIsize = AtomicIsize::new(0);
static POPUP: AtomicIsize = AtomicIsize::new(0);
static HOVER: AtomicIsize = AtomicIsize::new(-1);
static SCROLL: AtomicUsize = AtomicUsize::new(0);
static APP: OnceLock<AppHandle> = OnceLock::new();
static CELLS: OnceLock<Mutex<Vec<(String, String, bool)>>> = OnceLock::new();
static POINTER: OnceLock<Mutex<Pointer>> = OnceLock::new();
static DURATION: AtomicIsize = AtomicIsize::new(0);
const REPAINT: u32 = WM_APP + 1;
const SETTINGS_CHANGED: u32 = WM_APP + 2;
const HOVER_TIMER: usize = 1;
const HIDE_BUTTON: usize = 4101;

const HISTORY_START: f64 = 114.0;
const HISTORY_ROW: f64 = 38.0;
const HISTORY_FOOTER: f64 = 48.0;
static HISTORY: OnceLock<Mutex<HistoryState>> = OnceLock::new();
static SCROLL_DRAG: AtomicIsize = AtomicIsize::new(0);
#[derive(Clone, Default)]
struct HistoryState {
    generation: usize,
    provider: Option<String>,
    prompts: Vec<crate::dock::DockPrompt>,
    next: Option<crate::dock::DockCursor>,
    more: bool,
    loading: bool,
    failed: bool,
}
struct HistoryView {
    total: usize,
    offset: usize,
    more: bool,
    loading: bool,
    failed: bool,
}
fn history_view(history: &HistoryState, requested: usize, visible: usize) -> (Vec<crate::dock::DockPrompt>, HistoryView) {
    let offset = requested.min(history.prompts.len().saturating_sub(visible));
    let prompts = history.prompts.iter().skip(offset).take(visible).cloned().collect();
    (prompts, HistoryView {total: history.prompts.len(), offset, more: history.more,
        loading: history.loading, failed: history.failed})
}
fn history_state() -> std::sync::MutexGuard<'static, HistoryState> {
    HISTORY.get_or_init(|| Mutex::new(HistoryState::default())).lock().unwrap_or_else(|e| e.into_inner())
}
fn reset_history(provider: Option<&str>, active: bool) {
    let generation = history_state().generation.wrapping_add(1);
    *history_state() = HistoryState {generation, provider:provider.map(str::to_owned), more:active, ..Default::default()};
}
fn load_older() {
    let Some(app) = APP.get().cloned() else { return; };
    let (generation, provider, cursor) = {
        let mut history = history_state();
        if history.loading || !history.more { return; }
        history.loading = true;
        history.failed = false;
        (history.generation, history.provider.clone(), history.next.clone())
    };
    tauri::async_runtime::spawn_blocking(move || {
        let page = crate::app::dock_prompt_page(&app, provider.as_deref(), cursor.as_ref());
        {
            let mut history = history_state();
            if history.generation != generation { return; }
            history.loading = false;
            match page {
                Ok(page) => {
                    history.more = page.next.is_some();
                    history.next = page.next;
                    history.prompts.extend(page.prompts);
                }
                Err(_) => history.failed = true,
            }
        }
        unsafe { if !popup().is_null() { PostMessageW(popup(), REPAINT, 0, 0); } }
    });
}
fn hover_provider(index: isize) -> Option<&'static str> {
    match index { 0..=2 => Some("claude"), 3 => Some("codex"), _ => None }
}
fn visible_rows(height: i32, scale: f64) -> usize {
    let start=(HISTORY_START*scale).round() as i32;
    let row=(HISTORY_ROW*scale).round().max(1.0) as i32;
    ((height-start-(HISTORY_FOOTER*scale).round() as i32)/row).max(0) as usize
}
struct Pointer {
    entered: Instant,
    inside: Instant,
    press: Option<(POINT, RECT, bool)>,
    resize: Option<(POINT, Rect, Corner, u16)>,
}
fn pointer() -> std::sync::MutexGuard<'static, Pointer> {
    POINTER
        .get_or_init(|| {
            Mutex::new(Pointer {
                entered: Instant::now(),
                inside: Instant::now(),
                press: None,
                resize: None,
            })
        })
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | g << 8 | b << 16
}
fn hwnd() -> HWND {
    HANDLE.load(Ordering::Acquire) as HWND
}
fn popup() -> HWND {
    POPUP.load(Ordering::Acquire) as HWND
}
fn locked() -> bool {
    APP.get()
        .map(|a| crate::app::display_settings(a).2)
        .unwrap_or(false)
}
unsafe fn scale(window: HWND) -> f64 {
    GetDpiForWindow(window).max(96) as f64 / 96.0
        * APP
            .get()
            .map(|a| crate::app::display_settings(a).3)
            .unwrap_or(120) as f64
        / 100.0
}
unsafe fn dock_scale(window: HWND) -> f64 {
    let r = window_rect(window);
    if r.right > r.left { (r.right-r.left) as f64/DOCK_WIDTH }
    else { requested_dock_scale(window) }
}
unsafe fn requested_dock_scale(window: HWND) -> f64 {
    let requested = GetDpiForWindow(window).max(96) as f64 / 96.0
        * APP.get().map(crate::app::dock_scale).unwrap_or(100) as f64 / 100.0;
    let work = monitor(window, None).rcWork;
    requested.min((work.right - work.left).max(1) as f64 / DOCK_WIDTH)
        .min((work.bottom - work.top).max(1) as f64 / DOCK_HEIGHT).max(0.01)
}
unsafe fn window_rect(window: HWND) -> RECT {
    let mut r: RECT = std::mem::zeroed();
    GetWindowRect(window, &mut r);
    r
}
fn convert(r: RECT) -> Rect {
    Rect {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
    }
}
fn contains(r: RECT, p: POINT) -> bool {
    p.x >= r.left && p.x < r.right && p.y >= r.top && p.y < r.bottom
}
unsafe fn monitor(window: HWND, point: Option<POINT>) -> MONITORINFO {
    let m = point
        .map(|p| MonitorFromPoint(p, MONITOR_DEFAULTTONEAREST))
        .unwrap_or_else(|| MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST));
    let mut info: MONITORINFO = std::mem::zeroed();
    info.cbSize = size_of::<MONITORINFO>() as u32;
    GetMonitorInfoW(m, &mut info);
    info
}
unsafe fn rounded(window: HWND, w: i32, h: i32, r: i32) {
    let region = CreateRoundRectRgn(0, 0, w + 1, h + 1, r, r);
    if SetWindowRgn(window, region, 1) == 0 {
        DeleteObject(region);
    }
}

pub fn create(app: AppHandle) {
    if APP.set(app.clone()).is_err() {
        return;
    }
    CELLS.get_or_init(|| {
        Mutex::new(super::cells(
            &[],
            chrono::Utc::now().timestamp(),
            crate::app::display_settings(&app).1,
        ))
    });
    std::thread::Builder::new()
        .name("usage-dock".into())
        .spawn(move || unsafe {
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let instance = GetModuleHandleW(null());
            let name = wide("GcdUsageNativeStrip");
            let card_name = wide("GcdUsageHoverCard");
            let mut class: WNDCLASSW = std::mem::zeroed();
            class.style = CS_DROPSHADOW;
            class.lpfnWndProc = Some(wndproc);
            class.hInstance = instance;
            class.lpszClassName = name.as_ptr();
            class.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
            if RegisterClassW(&class) == 0 {
                return;
            }
            class.lpfnWndProc = Some(cardproc);
            class.lpszClassName = card_name.as_ptr();
            if RegisterClassW(&class) == 0 {
                return;
            }
            let dock = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                name.as_ptr(),
                wide("GCD Usage · drag to move · drag a corner to resize").as_ptr(),
                WS_POPUP | WS_CLIPCHILDREN,
                0,
                0,
                DOCK_WIDTH as i32,
                DOCK_HEIGHT as i32,
                null_mut(),
                null_mut(),
                instance,
                null(),
            );
            if dock.is_null() {
                return;
            }
            HANDLE.store(dock as isize, Ordering::Release);
            let card = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                card_name.as_ptr(),
                wide("GCD Usage · recent prompts").as_ptr(),
                WS_POPUP,
                0,
                0,
                560,
                590,
                dock,
                null_mut(),
                instance,
                null(),
            );
            POPUP.store(card as isize, Ordering::Release);
            position_window(dock, crate::strip_position(&app));
            CreateWindowExW(0,wide("BUTTON").as_ptr(),wide("Hide dock").as_ptr(),
                WS_CHILD|WS_VISIBLE|windows_sys::Win32::UI::WindowsAndMessaging::BS_OWNERDRAW as u32,
                0,0,1,1,dock,HIDE_BUTTON as HMENU,instance,null());
            layout_hide_button(dock);
            if !crate::app::dock_hidden(&app) {ShowWindow(dock, SW_SHOWNOACTIVATE);}
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                let dialog = DURATION.load(Ordering::Acquire) as HWND;
                if !dialog.is_null() && IsDialogMessageW(dialog, &msg) != 0 {
                    continue;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })
        .ok();
}
pub fn update(cells: Vec<(String, String, bool)>) {
    *CELLS
        .get_or_init(|| Mutex::new(vec![]))
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = cells;
    unsafe {
        if !hwnd().is_null() {
            PostMessageW(hwnd(), REPAINT, 0, 0);
        }
    }
}
pub fn settings_changed() {
    unsafe {
        if !hwnd().is_null() {
            PostMessageW(hwnd(), SETTINGS_CHANGED, 0, 0);
        }
    }
}
pub fn shutdown() {
    unsafe {
        if !hwnd().is_null() {
            PostMessageW(hwnd(), WM_CLOSE, 0, 0);
        }
    }
}
unsafe fn position_window(window: HWND, saved: Option<(i32, i32)>) {
    let info = monitor(window, saved.map(|(x, y)| POINT { x, y }));
    let s = requested_dock_scale(window);
    let width = ((DOCK_WIDTH * s).round() as i32)
        .min(info.rcWork.right - info.rcWork.left)
        .max(1);
    let height = ((DOCK_HEIGHT * s).round() as i32)
        .min(info.rcWork.bottom - info.rcWork.top)
        .max(1);
    let (x, y) = strip_origin(
        convert(info.rcMonitor),
        convert(info.rcWork),
        (width, height),
        saved,
        false,
        (10.0 * s) as i32,
    );
    SetWindowPos(window, HWND_TOPMOST, x, y, width, height, SWP_NOACTIVATE);
    rounded(window, width, height, (3.0 * s) as i32);
    layout_hide_button(window);
    SetWindowTextW(
        window,
        wide(if locked() {
            "GCD Usage · position locked · click to open"
        } else {
            "GCD Usage · drag to move · drag a corner to resize"
        })
        .as_ptr(),
    );
}
unsafe fn hide_card() {
    reset_history(None, false);
    SCROLL_DRAG.store(0, Ordering::Release);
    if !popup().is_null() {
        ShowWindow(popup(), SW_HIDE);
    }
    HOVER.store(-1, Ordering::Release);
    KillTimer(hwnd(), HOVER_TIMER);
    InvalidateRect(hwnd(), null(), 0);
}
unsafe fn show_card() {
    if popup().is_null() || HOVER.load(Ordering::Acquire) < 0 {
        return;
    }
    let dock = window_rect(hwnd());
    let info = monitor(hwnd(), None);
    let s = scale(hwnd());
    let w = ((1100.0 * s) as i32)
        .min(info.rcWork.right - info.rcWork.left - 8)
        .max(1);
    let h = ((570.0 * s) as i32)
        .min(
            (dock.top - info.rcWork.top - 8)
                .max(info.rcWork.bottom - dock.bottom - 8)
                .max(200),
        )
        .max(1);
    let (x, y) = popup_origin(
        convert(dock),
        convert(info.rcWork),
        (w, h),
        (6.0 * s) as i32,
    );
    SetWindowPos(popup(), HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE);
    rounded(popup(), w, h, (3.0 * s) as i32);
    ShowWindow(popup(), SW_SHOWNOACTIVATE);
    InvalidateRect(popup(), null(), 0);
}
unsafe fn hover_tick() {
    let mut p: POINT = std::mem::zeroed();
    GetCursorPos(&mut p);
    let r = window_rect(hwnd());
    let now = Instant::now();
    let on_hide=contains(window_rect(GetDlgItem(hwnd(),HIDE_BUTTON as i32)),p);
    let over = (IsWindowVisible(hwnd()) != 0 && contains(r, p) && !on_hide
        && corner_at(p.x-r.left,p.y-r.top,r.right-r.left,r.bottom-r.top,(12.*dock_scale(hwnd())).ceil() as i32).is_none())
        .then(|| dock_cell(p.x - r.left, r.right - r.left, dock_scale(hwnd())));
    let visible = !popup().is_null() && IsWindowVisible(popup()) != 0;
    let on_card = visible && contains(window_rect(popup()), p);
    let decision = {
        let mut state = pointer();
        let decision = hover_decision(
            HOVER.load(Ordering::Acquire),
            over,
            on_card,
            visible,
            state.press.is_some() || state.resize.is_some(),
            now.duration_since(state.entered).as_millis(),
            now.duration_since(state.inside).as_millis(),
        );
        if over.is_some() || on_card {
            state.inside = now;
        }
        if matches!(decision, HoverDecision::Switch(_)) {
            state.entered = now;
        }
        decision
    };
    match decision {
        HoverDecision::Switch(index) => {
            HOVER.store(index as isize, Ordering::Release);
            SCROLL.store(0, Ordering::Release);
            reset_history(hover_provider(index as isize), true);
            load_older();
            if !popup().is_null() {
                ShowWindow(popup(), SW_HIDE);
            }
            InvalidateRect(hwnd(), null(), 0);
        }
        HoverDecision::Show => show_card(),
        HoverDecision::Hide => hide_card(),
        HoverDecision::Keep => {}
    }
}
unsafe fn open_dashboard() {
    hide_card();
    if let Some(app) = APP.get() {
        let a = app.clone();
        let _ = app.run_on_main_thread(move || {
            let _ = super::show_dashboard(&a);
        });
    }
}
unsafe fn save_position() {
    let r = window_rect(hwnd());
    position_window(hwnd(), Some((r.left, r.top)));
    let r = window_rect(hwnd());
    if let Some(app) = APP.get() {
        crate::save_strip_position(app, r.left, r.top);
    }
}

struct Palette {
    bg: u32,
    card: u32,
    border: u32,
    text: u32,
    muted: u32,
    orange: u32,
    green: u32,
    purple: u32,
    reset: u32,
}
unsafe fn palette() -> Palette {
    let mut theme = APP
        .get()
        .map(|a| crate::app::display_settings(a).0)
        .unwrap_or_default();
    if theme == ColorTheme::System {
        let mut light = 1u32;
        let mut bytes = 4;
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize").as_ptr(),
            wide("AppsUseLightTheme").as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            &mut light as *mut _ as *mut _,
            &mut bytes,
        );
        theme = if light != 0 {
            ColorTheme::Light
        } else {
            ColorTheme::Slate
        };
    }
    palette_for_theme(theme)
}
fn palette_for_theme(theme: ColorTheme) -> Palette {
    let (bg,card,border,text,muted,accent)=match theme {
        ColorTheme::Light => (0xf8f9f7,0xffffff,0xe4e9e2,0x27332f,0x66726a,0x52785a),
        ColorTheme::Slate => (0x15191f,0x1d232b,0x343e4b,0xe7edf5,0xa1afbe,0xa5c9e2),
        ColorTheme::Midnight => (0x090f20,0x111b31,0x283754,0xe9edff,0xa5b1cf,0xb2befa),
        _ => (0x0a0a08,0x12110c,0x2e2b23,0xf2ecdc,0xb3ac9a,0x6cb07c),
    };
    let color=|hex:u32| rgb((hex>>16)&255,(hex>>8)&255,hex&255);
    Palette {bg:color(bg),card:color(card),border:color(border),text:color(text),
        muted:color(muted),green:color(accent),orange:color(accent),purple:color(accent),
        reset:color(if theme == ColorTheme::Light {0xa34842} else {0xd99a93})}
}

unsafe fn fill(dc: HDC, r: RECT, color: u32) {
    let b = CreateSolidBrush(color);
    FillRect(dc, &r, b);
    DeleteObject(b);
}
unsafe fn round(dc: HDC, r: RECT, color: u32, border: u32, radius: i32) {
    let b = CreateSolidBrush(color);
    let p = CreatePen(PS_SOLID, 1, border);
    let old_b = SelectObject(dc, b);
    let old_p = SelectObject(dc, p);
    RoundRect(dc, r.left, r.top, r.right, r.bottom, radius, radius);
    SelectObject(dc, old_b);
    SelectObject(dc, old_p);
    DeleteObject(b);
    DeleteObject(p);
}
// Process-private font registration; no system installation or registry changes.
// The embedded bytes and OS resource remain valid for this process lifetime.
fn terminal_font_available() -> bool {
    static REGISTERED: OnceLock<bool> = OnceLock::new();
    *REGISTERED.get_or_init(|| unsafe {
        let bytes = include_bytes!("../../assets/JetBrainsMono-Regular.ttf");
        let mut count = 0u32;
        !AddFontMemResourceEx(bytes.as_ptr().cast(), bytes.len() as u32, null(), &mut count).is_null() && count > 0
    })
}
unsafe fn font(size: i32, weight: i32) -> HFONT {
    CreateFontW(
        size,
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        DEFAULT_CHARSET as u32,
        OUT_DEFAULT_PRECIS as u32,
        CLIP_DEFAULT_PRECIS as u32,
        CLEARTYPE_QUALITY as u32,
        DEFAULT_PITCH as u32,
        wide(if terminal_font_available() { "JetBrains Mono" } else { "Consolas" }).as_ptr(),
    )
}
unsafe fn text(dc: HDC, font: HFONT, color: u32, value: &str, mut r: RECT) {
    let old = SelectObject(dc, font);
    SetTextColor(dc, color);
    DrawTextW(
        dc,
        wide(value).as_ptr(),
        -1,
        &mut r,
        DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX | DT_VCENTER,
    );
    SelectObject(dc, old);
}

unsafe fn text_right(dc:HDC, f:HFONT, color:u32, value:&str, mut r:RECT) {
    let old=SelectObject(dc,f);
    SetTextColor(dc,color);
    DrawTextW(dc,wide(value).as_ptr(),-1,&mut r,DT_SINGLELINE|DT_END_ELLIPSIS|DT_NOPREFIX|DT_VCENTER|DT_RIGHT);
    SelectObject(dc,old);
}
unsafe fn text_pair(dc:HDC,f:HFONT,first:u32,second:u32,prefix:&str,suffix:&str,r:RECT) {
    let old=SelectObject(dc,f);
    let w=wide(prefix);
    let mut size:SIZE=std::mem::zeroed();
    GetTextExtentPoint32W(dc,w.as_ptr(),(w.len()-1) as i32,&mut size);
    SelectObject(dc,old);
    text(dc,f,first,prefix,r);
    let left=(r.left+size.cx).min(r.right);
    text(dc,f,second,suffix,RECT {left,..r});
}
unsafe fn paint(window: HWND, card: bool) {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let target = BeginPaint(window, &mut ps);
    let mut bounds: RECT = std::mem::zeroed();
    GetClientRect(window, &mut bounds);
    let dc = CreateCompatibleDC(target);
    let bitmap = CreateCompatibleBitmap(target, bounds.right.max(1), bounds.bottom.max(1));
    let old_bitmap = SelectObject(dc, bitmap);
    let p = palette();
    let s = if card { scale(hwnd()) } else { dock_scale(hwnd()) };
    let (mut summary, previews) = APP.get().map(crate::app::dock_summary).unwrap_or_default();
    // Clone only the visible page; release the history lock before GDI painting.
    let history = if card {
        let (prompts, view) = history_view(&history_state(), SCROLL.load(Ordering::Acquire), visible_rows(bounds.bottom, s));
        summary.prompts = prompts;
        Some(view)
    } else { None };
    let cells = CELLS
        .get()
        .unwrap()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    draw_surface(dc, bounds, p, s, card, &summary, previews, &cells, history.as_ref());
    BitBlt(target, 0, 0, bounds.right, bounds.bottom, dc, 0, 0, SRCCOPY);
    SelectObject(dc, old_bitmap);
    DeleteObject(bitmap);
    DeleteDC(dc);
    EndPaint(window, &ps);
}
unsafe fn draw_surface(
    dc: HDC,
    bounds: RECT,
    p: Palette,
    s: f64,
    card: bool,
    summary: &crate::dock::DockSummary,
    previews: bool,
    cells: &[(String, String, bool)],
    history: Option<&HistoryView>,
) {
    let px = |v: f64| (v * s).round() as i32;
    let fsmall = font(px(if card { -12.0 } else { -13.0 }), 400);
    let fmedium = font(px(if card { -13.0 } else { -14.0 }), 400);
    let flarge = font(px(if card { -20.0 } else { -26.0 }), 400);
    SetBkMode(dc, TRANSPARENT as i32);
    fill(dc, bounds, p.bg);
    round(
        dc,
        RECT {
            left: 0,
            top: 0,
            right: bounds.right,
            bottom: bounds.bottom,
        },
        p.bg,
        p.border,
        px(3.0),
    );
    if card {
        let index = HOVER.load(Ordering::Acquire);
        let provider = match index {
            0 | 1 | 2 => Some("claude"),
            3 => Some("codex"),
            _ => None,
        };
        let accent = match index {
            0 | 1 | 2 => p.orange,
            3 => p.green,
            _ => p.purple,
        };
        let title = match index {
            0 | 1 | 2 => "Claude · recent prompts",
            3 => "Codex / ChatGPT · recent prompts",
            _ => "All activity · recent prompts",
        };
        let r = |y: f64, h: f64| RECT {
            left: px(18.0),
            top: px(y),
            right: bounds.right - px(18.0),
            bottom: px(y + h),
        };
        text(dc, flarge, p.text, title, r(12.0, 28.0));
        let total = provider
            .map(|v| summary.provider_tokens.get(v).copied().unwrap_or(0))
            .unwrap_or(summary.tokens);
        let total_text = if summary.updated_at.is_some() {
            format!(
                "{} logged tokens · last {}",
                compact(total),
                interval(summary.minutes)
            )
        } else {
            "Loading recorded activity…".into()
        };
        text(dc, fmedium, accent, &total_text, r(43.0, 22.0));

        text(dc, fsmall, p.muted, "Row tokens: lifetime · select a prompt for details", r(72.0, 22.0));
        let rows = summary.recent(provider);
        let row_h = px(HISTORY_ROW).max(1);
        let start = px(HISTORY_START);
        let visible = visible_rows(bounds.bottom, s);
        let total_rows = history.map(|h| h.total).unwrap_or(rows.len());
        let max_scroll = total_rows.saturating_sub(visible);
        let offset = history.map(|h| h.offset).unwrap_or_else(|| SCROLL.load(Ordering::Acquire).min(max_scroll));
        SCROLL.store(offset, Ordering::Release);
        let content_width = (bounds.right-px(44.0)).max(1);
        let x = |fraction:f64| px(22.0)+(content_width as f64*fraction) as i32;
        let cell = |left:f64,right:f64,top:i32,bottom:i32| RECT {
            left:x(left),right:x(right)-px(12.0),top,bottom
        };
        for (label,left,right) in [("Project",0.,0.14),("Prompt",0.14,0.59),("Time",0.59,0.72),("Model",0.72,0.91),("Tokens",0.91,1.)] {
            text(dc,fsmall,p.muted,label,cell(left,right,px(94.),start));
        }
        if rows.is_empty() {
            text(dc,fmedium,p.muted,
                if history.is_some_and(|h| h.loading) {"Loading prompts…"}
                else if history.is_some_and(|h| h.failed) {"Unable to load history. Scroll to retry."}
                else {"No recorded prompts yet"},r(135.,28.));
        }
        for (i,row) in rows.iter().skip(if history.is_some() {0} else {offset}).take(visible).enumerate() {
            let top=start+i as i32*row_h;
            let bottom=top+row_h-px(6.);
            fill(dc,RECT {left:px(14.),top,right:bounds.right-px(22.),bottom},p.card);
            let mut tag=cell(0.,0.14,top+px(6.),bottom-px(6.));
            round(dc,tag,p.card,p.border,0);
            tag.left+=px(6.);tag.right-=px(6.);
            text(dc,fsmall,p.muted,if previews {row.project_label()} else {"Hidden"},tag);
            let preview=if !previews {"Prompt preview hidden"} else if row.preview.is_empty() {"No text preview"} else {&row.preview};
            text(dc,fmedium,p.text,preview,cell(0.14,0.59,top,bottom));
            let time=row.timestamp.and_then(|t| chrono::DateTime::from_timestamp(t,0))
                .map(|t|t.with_timezone(&chrono::Local).format("%b %d %H:%M").to_string()).unwrap_or("Unknown".into());
            text(dc,fsmall,p.muted,&time,cell(0.59,0.72,top,bottom));
            text(dc,fsmall,p.muted,&row.model_label(),cell(0.72,0.91,top,bottom));
            text_right(dc,fmedium,p.green,&row.tokens.map(compact).unwrap_or("—".into()),cell(0.91,1.,top,bottom));
        }
        if total_rows>visible || history.is_some_and(|h|h.more) {
            let track=RECT {left:bounds.right-px(12.),top:start,right:bounds.right-px(7.),bottom:bounds.bottom-px(HISTORY_FOOTER)};
            fill(dc,track,p.border);
            let virtual_len=total_rows+if history.is_some_and(|h|h.more) {40} else {0};
            let thumb_h=((track.bottom-track.top) as f64*visible as f64/virtual_len.max(1) as f64).round() as i32;
            let thumb_h=thumb_h.max(px(20.)).min((track.bottom-track.top).max(0));
            let travel=(track.bottom-track.top-thumb_h).max(0);
            let thumb_y=track.top+(travel as f64*offset as f64/virtual_len.saturating_sub(visible).max(1) as f64) as i32;
            fill(dc,RECT {top:thumb_y,bottom:thumb_y+thumb_h,..track},p.green);
        }
        let footer = if history.is_some_and(|h|h.loading) {"Loading older prompts…"}
            else if history.is_some_and(|h|h.failed) {"Unable to load older prompts · scroll to retry"}
            else if history.is_some_and(|h|!h.more) {"All recorded prompts loaded · scroll to browse"}
            else {"Scroll for older prompts · older entries load automatically"};
        text(dc,fsmall,p.muted,footer,RECT {left:px(18.),top:bounds.bottom-px(35.),right:bounds.right-px(18.),bottom:bounds.bottom-px(10.)});
    } else {

        for i in 0..5 {
            let left = bounds.right*i/5;
            let right = bounds.right*(i+1)/5;
            if HOVER.load(Ordering::Acquire)==i as isize {
                fill(dc,RECT {left,top:1,right,bottom:bounds.bottom-1},p.card);
            }
            if i>0 { fill(dc,RECT {left,top:1,right:left+1,bottom:bounds.bottom-1},p.border); }
            let r=|y:f64,h:f64| RECT {left:left+px(22.),right:right-px(18.),top:px(y),bottom:px(y+h)};
            if i<4 {
                if let Some((label,value,_stale))=cells.get(i as usize) {
                    let (main,reset)=value.split_once(" · ").unwrap_or((value.as_str(),""));
                    text(dc,fmedium,p.text,label,r(13.,22.));
                    if let Some((amount,unit))=main.trim().split_once(' ') {
                        if amount.contains('%') { text_pair(dc,flarge,p.green,p.text,amount,&format!(" {unit}"),r(42.,34.)); }
                        else { text(dc,flarge,p.text,"—",r(42.,34.)); }
                    } else { text(dc,flarge,p.text,main.trim(),r(42.,34.)); }
                    let reset=reset.trim();
                    if main.contains('%') {
                        text_pair(dc,fsmall,p.reset,p.reset,"reset ",reset,r(87.,21.));
                    } else {
                        text(dc,fsmall,p.muted,if reset=="sign in" {"sign in needed"} else {reset},r(87.,21.));
                    }
                    let id=["claude:300","claude:10080","claude-fable:10080","codex:10080"][i as usize];
                    let recent=summary.allowance_usage.iter().find(|row|row.window_id==id);
                    let (amount,label)=recent.and_then(|row|row.consumed_percent.map(|value|(value,row.state.as_str())))
                        .map(|(value,state)| {
                            let amount=if value>0.0 && value<0.01 {"<0.01%".into()} else {format!("{}%",format!("{value:.2}").trim_end_matches('0').trim_end_matches('.'))};
                            (amount,format!(" {} / {}",if state=="partial" {"observed"} else {"used"},interval(summary.minutes)))
                        }).unwrap_or_else(||("—".into(),if summary.updated_at.is_some() {format!(" usage / {}",interval(summary.minutes))} else {" usage".into()}));
                    text_pair(dc,fsmall,p.green,p.muted,&amount,&label,r(112.,20.));
                }
            } else {
                let label=if summary.updated_at.is_some() {format!("tokens / {}",interval(summary.minutes).to_uppercase())} else {"tokens".into()};
                text(dc,fmedium,p.text,&label,r(13.,22.));
                let value=if summary.updated_at.is_some() {format!("{}{}",compact(summary.tokens),if summary.unknown_requests>0 {"+"} else {""})} else {"—".into()};
                text(dc,flarge,p.green,&value,r(42.,34.));
            }
        }
    }
    DeleteObject(fsmall);
    DeleteObject(fmedium);
    DeleteObject(flarge);
}

#[cfg(test)]
fn native_gui_counts() -> (u32, u32) {
    #[link(name = "user32")]
    extern "system" { fn GetGuiResources(process: *mut std::ffi::c_void, flags: u32) -> u32; }
    unsafe { (GetGuiResources(-1isize as *mut _, 0), GetGuiResources(-1isize as *mut _, 1)) }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::dock::{DockModel, DockPrompt, DockSummary};
    #[test]
    fn hover_paint_copies_only_the_visible_rows_of_a_long_scroll_session() {
        let history = HistoryState {prompts:(0..10000).map(|i| DockPrompt {
            context:"Synthetic".into(), id:i.to_string(), provider:"claude".into(),
            preview:"Bounded visible row".into(), timestamp:Some(i), tokens:Some(0),
            models:"Synthetic model".into(), status:"completed".into(), browser:false,
        }).collect(),more:true,..Default::default()};
        let (rows,view)=history_view(&history,5000,12);
        assert_eq!(rows.len(),12);assert_eq!(view.total,10000);assert_eq!(view.offset,5000);
        assert_eq!(rows[0].id,"5000");assert_eq!(rows[11].id,"5011");
        let (rows,view)=history_view(&history,usize::MAX,12);
        assert_eq!(view.offset,9988);assert_eq!(rows.last().unwrap().id,"9999");
    }
    #[test]
    fn native_renderer_handles_scaled_history_privacy_and_unknown_data() {
        assert!(terminal_font_available(), "Bundled native font must register");
        unsafe {
            let models = vec![
                DockModel {
                    provider: "claude".into(),
                    model: "Claude Sonnet".into(),
                    effort: Some("high".into()),
                    tokens: 184200,
                    requests: 14,
                },
                DockModel {
                    provider: "codex".into(),
                    model: "GPT coding model".into(),
                    effort: Some("medium".into()),
                    tokens: 72000,
                    requests: 8,
                },
            ];
            let prompts = (0..20)
                .map(|i| DockPrompt {
                    context: "Demo project / Example conversation".into(),
                    id: i.to_string(),
                    provider: if i % 2 == 0 { "claude" } else { "codex" }.into(),
                    preview: [
                        "Build the new search experience",
                        "Review cache behavior and edge cases",
                        "Explain the failing integration test",
                        "Tighten the mobile navigation layout",
                    ][i % 4]
                        .into(),
                    timestamp: Some(1788934000 - i as i64 * 120),
                    tokens: if i == 4 {
                        None
                    } else {
                        Some(12000 + i as u64 * 750)
                    },
                    models: if i % 2 == 0 {
                        "Claude Sonnet · high"
                    } else {
                        "GPT coding model · medium"
                    }
                    .into(),
                    status: "completed".into(),
                    browser: i == 4,
                })
                .collect();
            let summary = DockSummary {
                updated_at: Some(chrono::Utc::now().timestamp()),
                minutes: 60,
                tokens: 256200,
                unknown_requests: 0,
                provider_tokens: std::collections::BTreeMap::from([
                    ("claude".into(), 184200),
                    ("codex".into(), 72000),
                ]),
                models,
                prompts,
                allowance_usage: ["claude:300","claude:10080","claude-fable:10080","codex:10080"].iter().enumerate().map(|(i,id)|
                    crate::storage::RecentAllowanceWindow {
                        provider: if i==3 {crate::models::Provider::Codex} else {crate::models::Provider::Claude},
                        window_id:(*id).into(), label:(*id).into(), consumed_percent:if i==2 {None} else {Some([4.2,1.3,0.0,0.8][i])},
                        state:if i==1 {"partial".into()} else {"observed".into()},
                        observed_seconds:1800, first_reading_at:None,last_reading_at:None,sample_count:16,gap_count:0,reset_count:0,
                    }).collect(),
            };
            let cells = vec![
                ("Claude · 5h".into(), "78% left · 3h 42m".into(), false),
                ("Claude · week".into(), "64% left · 4d 6h".into(), false),
                ("Claude · Fable".into(), "82% left · 5d 2h".into(), false),
                ("Codex · week".into(), "92% left · 5d 2h".into(), false),
            ];
            for (name, card, s, previews, index, small, theme) in [
                ("dock", false, 1.0, true, -1, false, ColorTheme::Black),
                ("dock-light", false, 1.0, true, -1, false, ColorTheme::Light),
                ("dock-slate", false, 1.0, true, -1, false, ColorTheme::Slate),
                ("dock-midnight", false, 1.0, true, -1, false, ColorTheme::Midnight),
                ("claude-hover", true, 1.2, true, 0, false, ColorTheme::Black),
                ("codex-hover", true, 1.2, true, 3, false, ColorTheme::Black),
                ("activity-hover", true, 1.2, true, 4, false, ColorTheme::Black),
                ("private-hover", true, 1.6, false, 0, true, ColorTheme::Black),
                ("small-font", false, 0.8, true, -1, false, ColorTheme::Black),
                ("large-font", false, 1.6, true, -1, false, ColorTheme::Black),
            ] {
                HOVER.store(index, Ordering::Release);
                SCROLL.store(0, Ordering::Release);
                let w = (if card { 1100.0 } else { DOCK_WIDTH } * s) as i32;
                let h = (if card {
                    if small {
                        390.0
                    } else {
                        570.0
                    }
                } else {
                    DOCK_HEIGHT
                } * s) as i32;
                let dc = CreateCompatibleDC(null_mut());
                let mut info: BITMAPINFO = std::mem::zeroed();
                info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
                info.bmiHeader.biWidth = w;
                info.bmiHeader.biHeight = -h;
                info.bmiHeader.biPlanes = 1;
                info.bmiHeader.biBitCount = 32;
                info.bmiHeader.biCompression = BI_RGB;
                let mut bits = null_mut();
                let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
                assert!(!bitmap.is_null());
                let old = SelectObject(dc, bitmap);
                draw_surface(
                    dc,
                    RECT {
                        left: 0,
                        top: 0,
                        right: w,
                        bottom: h,
                    },
                    palette_for_theme(theme),
                    s,
                    card,
                    &summary,
                    previews,
                    &cells,
                    None,
                );
                GdiFlush();
                let data = std::slice::from_raw_parts(bits as *const u8, (w * h * 4) as usize);
                assert!(data.iter().filter(|&&v| v > 100).count() > 1000);
                if let Some(out) = std::env::var_os("GCD_QA_OUTPUT_DIR") {
                    let out = std::path::PathBuf::from(out);
                    std::fs::create_dir_all(&out).unwrap();
                    let mut file = vec![];
                    file.extend_from_slice(b"BM");
                    file.extend_from_slice(&(54 + data.len() as u32).to_le_bytes());
                    file.extend_from_slice(&[0; 4]);
                    file.extend_from_slice(&54u32.to_le_bytes());
                    file.extend_from_slice(&40u32.to_le_bytes());
                    file.extend_from_slice(&w.to_le_bytes());
                    file.extend_from_slice(&(-h).to_le_bytes());
                    file.extend_from_slice(&1u16.to_le_bytes());
                    file.extend_from_slice(&32u16.to_le_bytes());
                    file.extend_from_slice(&[0; 24]);
                    file.extend_from_slice(data);
                    std::fs::write(out.join(format!("{name}.bmp")), file).unwrap();
                }
                SelectObject(dc, old);
                DeleteObject(bitmap);
                DeleteDC(dc);
            }
            HOVER.store(-1, Ordering::Release);
        }
    }
}

unsafe fn scroll_history(window: HWND, delta:isize) {
    let mut bounds:RECT=std::mem::zeroed();GetClientRect(window,&mut bounds);
    let visible=visible_rows(bounds.bottom,scale(hwnd()));
    let (len,more)={let h=history_state();(h.prompts.len(),h.more)};
    let offset=SCROLL.load(Ordering::Acquire).saturating_add_signed(delta).min(len.saturating_sub(visible));
    SCROLL.store(offset,Ordering::Release);
    if more && offset+visible+8>=len {load_older();}
    InvalidateRect(window,null(),0);
}
unsafe fn scroll_to_pointer(window:HWND,y:i32) {
    let mut bounds:RECT=std::mem::zeroed();GetClientRect(window,&mut bounds);
    let s=scale(hwnd());
    let visible=visible_rows(bounds.bottom,s);
    let len=history_state().prompts.len();
    let start=(HISTORY_START*s).round() as i32;
    let end=bounds.bottom-(HISTORY_FOOTER*s).round() as i32;
    let fraction=((y-start) as f64/(end-start).max(1) as f64).clamp(0.,1.);
    SCROLL.store((fraction*len.saturating_sub(visible) as f64).round() as usize,Ordering::Release);
    scroll_history(window,0);
}
unsafe extern "system" fn cardproc(window: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint(window, true);
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,

        REPAINT => {
            InvalidateRect(window,null(),0);
            0
        }
        WM_MOUSEWHEEL => {
            let delta=((w>>16) as u16) as i16;
            scroll_history(window,if delta<0 {3} else {-3});
            0
        }
        WM_LBUTTONDOWN => {
            let x=(l as u16) as i16 as i32;
            let y=((l>>16) as u16) as i16 as i32;
            let mut bounds:RECT=std::mem::zeroed();GetClientRect(window,&mut bounds);
            if x>=bounds.right-(20.*scale(hwnd())) as i32 {
                SCROLL_DRAG.store(1,Ordering::Release);
                SetCapture(window);scroll_to_pointer(window,y);
            }
            0
        }
        WM_MOUSEMOVE if SCROLL_DRAG.load(Ordering::Acquire)!=0 => {
            scroll_to_pointer(window,((l>>16) as u16) as i16 as i32);
            0
        }
        WM_CAPTURECHANGED => {SCROLL_DRAG.store(0,Ordering::Release);0}
        WM_LBUTTONUP => {
            if SCROLL_DRAG.swap(0,Ordering::AcqRel)!=0 {ReleaseCapture();return 0;}
            let x=(l as u16) as i16 as i32;
            let y=((l>>16) as u16) as i16 as i32;
            let s=scale(hwnd());
            let start=(HISTORY_START*s).round() as i32;
            let row_h=(HISTORY_ROW*s).round().max(1.) as i32;
            let mut bounds:RECT=std::mem::zeroed();GetClientRect(window,&mut bounds);
            let visible=visible_rows(bounds.bottom,s);
            if x>= (14.*s) as i32 && x<bounds.right-(22.*s) as i32 &&
                y>=start && y<start+visible as i32*row_h && (y-start)%row_h<row_h-(6.*s).round() as i32 {
                let index=((y-start)/row_h) as usize+SCROLL.load(Ordering::Acquire);
                let id=history_state().prompts.get(index).map(|r|r.id.clone());
                if let (Some(app),Some(id))=(APP.get(),id) {crate::app::activate_prompt(app,id);hide_card();}
            }
            0
        }
        WM_DPICHANGED => {
            show_card();
            0
        }
        _ => DefWindowProcW(window, msg, w, l),
    }
}
unsafe fn layout_hide_button(window: HWND) {
    let s=dock_scale(window);
    let r=window_rect(window);
    SetWindowPos(GetDlgItem(window,HIDE_BUTTON as i32),null_mut(),
        r.right-r.left-(68.*s).round() as i32,(12.*s).round() as i32,
        (50.*s).round() as i32,(24.*s).round() as i32,SWP_NOZORDER|SWP_NOACTIVATE);
}
unsafe fn finish_resize(window: HWND) -> bool {
    let resize=pointer().resize.take();
    if let Some((_,_,_,percent))=resize {
        if let Some(app)=APP.get() {
            let r=window_rect(window);
            if crate::app::set_dock_geometry(app,percent,r.left,r.top).is_err() {
                position_window(window,crate::strip_position(app));
            }
        }
        return true;
    }
    false
}
unsafe extern "system" fn wndproc(window: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND if w & 0xffff == HIDE_BUTTON => {
            hide_card();
            if let Some(app)=APP.get() { let _=crate::app::set_dock_hidden(app,true); }
            0
        }
        WM_DRAWITEM if w == HIDE_BUTTON => {
            let item=&*(l as *const windows_sys::Win32::UI::Controls::DRAWITEMSTRUCT);
            let p=palette(); let s=dock_scale(window); let f=font((-12.*s).round() as i32,400);
            round(item.hDC,item.rcItem,p.bg,p.border,0);
            SetBkMode(item.hDC,TRANSPARENT as i32);
            let mut r=item.rcItem;r.left+=(8.*s).round() as i32;
            text(item.hDC,f,p.muted,"Hide",r);DeleteObject(f);1
        }
        WM_SIZE => {layout_hide_button(window);0}
        WM_SETCURSOR if !locked() => {
            let mut p:POINT=std::mem::zeroed();GetCursorPos(&mut p);let r=window_rect(window);
            if let Some(c)=corner_at(p.x-r.left,p.y-r.top,r.right-r.left,r.bottom-r.top,(12.*dock_scale(window)).ceil() as i32) {
                SetCursor(LoadCursorW(null_mut(),if matches!(c,Corner::TopLeft|Corner::BottomRight) {IDC_SIZENWSE} else {IDC_SIZENESW}));1
            } else {DefWindowProcW(window,msg,w,l)}
        }
        WM_PAINT => {
            paint(window, false);
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
        WM_LBUTTONDOWN => {
            hide_card();
            let mut p: POINT = std::mem::zeroed();
            GetCursorPos(&mut p);
            let r=window_rect(window);
            let corner=corner_at(p.x-r.left,p.y-r.top,r.right-r.left,r.bottom-r.top,(12.*dock_scale(window)).ceil() as i32);
            if let Some(c)=corner.filter(|_|!locked()) {
                let percent=APP.get().map(crate::app::dock_scale).unwrap_or(100);
                pointer().resize=Some((p,convert(r),c,percent));
            } else {pointer().press = Some((p,r,false));}
            SetCapture(window);
            0
        }
        WM_MOUSEMOVE => {
            let mut p: POINT = std::mem::zeroed();
            GetCursorPos(&mut p);
            let resize=pointer().resize;
            if let Some((origin,start,corner,_))=resize {
                let work=convert(monitor(window,None).rcWork);
                let dpi=GetDpiForWindow(window).max(96) as f64/96.;
                let (next,percent)=resize_corner(start,corner,(p.x-origin.x,p.y-origin.y),work,dpi);
                pointer().resize=Some((origin,start,corner,percent));
                SetWindowPos(window,HWND_TOPMOST,next.left,next.top,next.right-next.left,next.bottom-next.top,SWP_NOACTIVATE);
                rounded(window,next.right-next.left,next.bottom-next.top,0);
                InvalidateRect(window,null(),0);
                return 0;
            }
            let movement = {
                let mut state = pointer();
                if let Some((start, r, dragged)) = state.press.as_mut() {
                    if let Some(origin) = drag_origin(
                        (start.x, start.y),
                        (p.x, p.y),
                        (r.left, r.top),
                        locked(),
                        *dragged,
                    ) {
                        *dragged = true;
                        Some(origin)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if let Some((x, y)) = movement {
                SetWindowPos(
                    window,
                    HWND_TOPMOST,
                    x,
                    y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOACTIVATE,
                );
            } else if pointer().press.is_none() {
                SetTimer(window, HOVER_TIMER, 80, None);
                hover_tick();
            }
            0
        }
        WM_LBUTTONUP => {
            if finish_resize(window) {ReleaseCapture();return 0;}
            let press = pointer().press.take();
            ReleaseCapture();
            if press.is_some_and(|(_, _, dragged)| dragged) {
                save_position();
            } else if press.is_some() {
                let x = (l as u16) as i16 as i32;
                let r = window_rect(window);
                if dock_cell(x, r.right - r.left, dock_scale(window)) == 4 {
                    duration_menu()
                } else {
                    open_dashboard()
                }
            }
            0
        }
        WM_CAPTURECHANGED => {
            if finish_resize(window) {return 0;}
            let press = pointer().press.take();
            if press.is_some_and(|(_, _, dragged)| dragged) {
                save_position();
            }
            0
        }
        WM_CONTEXTMENU => {
            duration_menu();
            0
        }
        WM_TIMER if w == HOVER_TIMER => {
            hover_tick();
            0
        }
        WM_DPICHANGED => {
            hide_card();
            let r = &*(l as *const RECT);
            position_window(window, Some((r.left, r.top)));
            InvalidateRect(window, null(), 0);
            0
        }
        WM_DISPLAYCHANGE | WM_SETTINGCHANGE => {
            hide_card();
            let r = window_rect(window);
            position_window(window, Some((r.left, r.top)));
            InvalidateRect(window, null(), 0);
            0
        }
        WM_POWERBROADCAST => {
            if w == 7 || w == 18 {
                if let Some(a) = APP.get() {
                    crate::request_refresh(a);
                }
            }
            1
        }
        SETTINGS_CHANGED => {
            let dialog=DURATION.load(Ordering::Acquire) as HWND;
            if !dialog.is_null() {PostMessageW(dialog,WM_THEMECHANGED,0,0);}
            hide_card();
            position_window(window, APP.get().and_then(crate::strip_position));
            let hidden=APP.get().is_some_and(crate::app::dock_hidden);
            ShowWindow(window,if hidden {SW_HIDE} else {SW_SHOWNOACTIVATE});
            if hidden {KillTimer(window,HOVER_TIMER);}
            RedrawWindow(window,null(),null_mut(),RDW_INVALIDATE|RDW_ALLCHILDREN);
            InvalidateRect(window, null(), 0);
            0
        }
        REPAINT => {
            InvalidateRect(window, null(), 0);
            if !popup().is_null() && IsWindowVisible(popup()) != 0 {
                InvalidateRect(popup(), null(), 0);
            }
            0
        }
        WM_CLOSE => {
            hide_card();
            if !popup().is_null() {
                DestroyWindow(popup());
                POPUP.store(0, Ordering::Release);
            }
            DestroyWindow(window);
            0
        }
        WM_DESTROY => {
            HANDLE.store(0, Ordering::Release);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(window, msg, w, l),
    }
}
