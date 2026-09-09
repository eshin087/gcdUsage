//! Native, double-buffered dock and hover card. No resident webview or animation loop.
use super::geometry::{
    dock_cell, drag_origin, hover_decision, popup_origin, strip_origin, HoverDecision, Rect,
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
struct Pointer {
    entered: Instant,
    inside: Instant,
    press: Option<(POINT, RECT, bool)>,
}
fn pointer() -> std::sync::MutexGuard<'static, Pointer> {
    POINTER
        .get_or_init(|| {
            Mutex::new(Pointer {
                entered: Instant::now(),
                inside: Instant::now(),
                press: None,
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
    let requested = GetDpiForWindow(window).max(96) as f64 / 96.0
        * APP.get().map(crate::app::dock_scale).unwrap_or(100) as f64 / 100.0;
    let work = monitor(window, None).rcWork;
    requested.min((work.right - work.left).max(1) as f64 / 800.0)
        .min((work.bottom - work.top).max(1) as f64 / 56.0).max(0.01)
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
                wide("GCD Usage · drag anywhere · click to open").as_ptr(),
                WS_POPUP,
                0,
                0,
                800,
                84,
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
            ShowWindow(dock, SW_SHOWNOACTIVATE);
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
    let s = dock_scale(window);
    let width = ((800.0 * s).round() as i32)
        .min(info.rcWork.right - info.rcWork.left)
        .max(1);
    let height = ((56.0 * s).round() as i32)
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
    SetWindowTextW(
        window,
        wide(if locked() {
            "GCD Usage · position locked · click to open"
        } else {
            "GCD Usage · drag anywhere · click to open"
        })
        .as_ptr(),
    );
}
unsafe fn hide_card() {
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
    let w = ((560.0 * s) as i32)
        .min(info.rcWork.right - info.rcWork.left - 8)
        .max(1);
    let h = ((790.0 * s) as i32)
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
    let over = contains(r, p).then(|| dock_cell(p.x - r.left, r.right - r.left, dock_scale(hwnd())));
    let visible = !popup().is_null() && IsWindowVisible(popup()) != 0;
    let on_card = visible && contains(window_rect(popup()), p);
    let decision = {
        let mut state = pointer();
        let decision = hover_decision(
            HOVER.load(Ordering::Acquire),
            over,
            on_card,
            visible,
            state.press.is_some(),
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
    if theme == ColorTheme::Light {
        return Palette {
            bg: rgb(244, 246, 250),
            card: rgb(255, 255, 255),
            border: rgb(212, 219, 229),
            text: rgb(23, 31, 46),
            muted: rgb(88, 101, 118),
            orange: rgb(161, 74, 29),
            green: rgb(28, 116, 93),
            purple: rgb(104, 70, 173),
        };
    }
    if theme == ColorTheme::Black {
        return Palette {
            bg: rgb(0, 0, 0), card: rgb(7, 16, 11), border: rgb(36, 67, 49),
            text: rgb(220, 239, 225), muted: rgb(155, 184, 164),
            orange: rgb(102, 255, 153), green: rgb(102, 255, 153), purple: rgb(102, 255, 153),
        };
    }
    let (bg, card) = match theme {
        ColorTheme::Slate => (rgb(18, 23, 30), rgb(26, 34, 45)),
        ColorTheme::Midnight => (rgb(9, 13, 30), rgb(17, 24, 47)),
        _ => (rgb(0, 0, 0), rgb(13, 16, 21)),
    };
    Palette {
        bg,
        card,
        border: rgb(43, 50, 61),
        text: rgb(239, 244, 251),
        muted: rgb(151, 164, 183),
        orange: rgb(242, 168, 120),
        green: rgb(108, 221, 180),
        purple: rgb(181, 159, 255),
    }
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
        wide("Consolas").as_ptr(),
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
    let (summary, previews) = APP.get().map(crate::app::dock_summary).unwrap_or_default();
    let cells = CELLS
        .get()
        .unwrap()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    draw_surface(dc, bounds, p, s, card, &summary, previews, &cells);
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
) {
    let px = |v: f64| (v * s).round() as i32;
    let fsmall = font(px(-10.5), 400);
    let fmedium = font(px(-12.0), 400);
    let flarge = font(px(-19.0), 400);
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
        text(
            dc,
            fsmall,
            p.muted,
            &summary.model_summary(provider, 3),
            r(69.0, 20.0),
        );
        let age = summary
            .updated_at
            .map(|t| (chrono::Utc::now().timestamp() - t).max(0));
        let note = if age.is_some_and(|n| n > 120) {
            "History may be stale · open dashboard to refresh"
        } else {
            "Latest prompts · lifetime prompt tokens · model / reasoning"
        };
        text(dc, fsmall, p.muted, note, r(91.0, 20.0));
        let rows = summary.recent(provider);
        let row_h = px(62.0).max(1);
        let start = px(120.0);
        let visible = ((bounds.bottom - start - px(40.0)) / row_h).max(1) as usize;
        let max_scroll = rows.len().saturating_sub(visible);
        let offset = SCROLL.load(Ordering::Acquire).min(max_scroll);
        SCROLL.store(offset, Ordering::Release);
        if rows.is_empty() {
            text(
                dc,
                fmedium,
                p.muted,
                if summary.updated_at.is_some() {
                    "No recorded prompts yet"
                } else {
                    "History is loading…"
                },
                r(145.0, 28.0),
            );
        }
        for (i, row) in rows.iter().skip(offset).take(visible).enumerate() {
            let top = start + i as i32 * row_h;
            let row_bounds = RECT {
                left: px(12.0),
                top,
                right: bounds.right - px(12.0),
                bottom: top + row_h - px(3.0),
            };
            round(dc, row_bounds, p.card, p.card, px(2.0));
            text(
                dc,
                fsmall,
                accent,
                if previews {
                    &row.context
                } else {
                    "Context hidden"
                },
                RECT {
                    left: px(20.0),
                    top: top + px(2.0),
                    right: bounds.right - px(20.0),
                    bottom: top + px(19.0),
                },
            );
            let preview = if previews {
                if row.preview.is_empty() {
                    "No text preview"
                } else {
                    &row.preview
                }
            } else {
                "Prompt preview hidden"
            };
            let token = row.tokens.map(compact).unwrap_or_else(|| "—".into());
            text(
                dc,
                fmedium,
                p.text,
                preview,
                RECT {
                    left: px(20.0),
                    top: top + px(20.0),
                    right: bounds.right - px(90.0),
                    bottom: top + px(40.0),
                },
            );
            text(
                dc,
                fmedium,
                accent,
                &token,
                RECT {
                    left: bounds.right - px(81.0),
                    top: top + px(20.0),
                    right: bounds.right - px(17.0),
                    bottom: top + px(40.0),
                },
            );
            let time = row
                .timestamp
                .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
                .map(|t| {
                    t.with_timezone(&chrono::Local)
                        .format("%b %d %H:%M")
                        .to_string()
                })
                .unwrap_or_else(|| "Unknown time".into());
            let detail = format!(
                "{} · {} · {}{}",
                time,
                row.models,
                row.status,
                if row.browser { " · browser" } else { "" }
            );
            text(
                dc,
                fsmall,
                p.muted,
                &detail,
                RECT {
                    left: px(20.0),
                    top: top + px(41.0),
                    right: bounds.right - px(20.0),
                    bottom: top + px(58.0),
                },
            );
        }
        let footer = if rows.len() > visible {
            format!(
                "{}–{} of {} · scroll for more · browser tokens unknown",
                offset + 1,
                (offset + visible).min(rows.len()),
                rows.len()
            )
        } else if summary.unknown_requests > 0 {
            format!(
                "{} requests lack tokens · all computers / accounts",
                summary.unknown_requests
            )
        } else {
            "All computers / accounts · browser tokens unknown".into()
        };
        text(
            dc,
            fsmall,
            p.muted,
            &footer,
            RECT {
                left: px(18.0),
                top: bounds.bottom - px(31.0),
                right: bounds.right - px(18.0),
                bottom: bounds.bottom - px(8.0),
            },
        );
    } else {
        let gap = px(5.0);
        let inset = px(7.0);
        let width = (bounds.right - inset * 2 - gap * 4) / 5;
        for i in 0..5 {
            let left = inset + i as i32 * (width + gap);
            let accent = if i < 3 {
                p.orange
            } else if i == 3 {
                p.green
            } else {
                p.purple
            };
            let hovered = HOVER.load(Ordering::Acquire) == i as isize;
            round(
                dc,
                RECT {
                    left,
                    top: px(3.0),
                    right: left + width,
                    bottom: bounds.bottom - px(3.0),
                },
                p.bg,
                if hovered { accent } else { p.border },
                px(2.0),
            );
            let r = |y: f64, h: f64| RECT {
                left: left + px(7.0),
                top: px(y),
                right: left + width - px(6.0),
                bottom: px(y + h),
            };
            if i < 4 {
                if let Some((label, value, stale)) = cells.get(i) {
                    let (main, reset) = value.split_once(" · ").unwrap_or((value.as_str(), ""));
                    text(dc, fsmall, accent, label, r(4.0, 13.0));
                    text(
                        dc,
                        flarge,
                        if *stale { p.muted } else { p.text },
                        main,
                        r(17.0, 20.0),
                    );
                    text(
                        dc,
                        fsmall,
                        p.muted,
                        &format!("rst {reset}"),
                        r(37.0, 13.0),
                    );
                }
            } else {
                let label = if summary.updated_at.is_some() {
                    format!(
                        "[{}] tokens / menu",
                        interval(summary.minutes).to_uppercase()
                    )
                } else {
                    "RECORDED ACTIVITY".into()
                };
                text(dc, fsmall, accent, &label, r(4.0, 13.0));
                let value = if summary.updated_at.is_some() {
                    format!(
                        "{}{}",
                        compact(summary.tokens),
                        if summary.unknown_requests > 0 {
                            "+"
                        } else {
                            ""
                        }
                    )
                } else {
                    "—".into()
                };
                text(dc, flarge, p.text, &value, r(17.0, 20.0));
                text(
                    dc,
                    fsmall,
                    p.muted,
                    &summary.model_summary(None, 2),
                    r(37.0, 13.0),
                );
            }
        }
    }
    DeleteObject(fsmall);
    DeleteObject(fmedium);
    DeleteObject(flarge);
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::dock::{DockModel, DockPrompt, DockSummary};
    #[test]
    fn native_renderer_handles_scaled_history_privacy_and_unknown_data() {
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
            };
            let cells = vec![
                ("Claude · 5h".into(), "78% left · 3h 42m".into(), false),
                ("Claude · week".into(), "64% left · 4d 6h".into(), false),
                ("Claude · Fable".into(), "82% left · 5d 2h".into(), false),
                ("Codex · week".into(), "92% left · 5d 2h".into(), false),
            ];
            for (name, card, s, previews, index, small) in [
                ("dock", false, 1.0, true, -1, false),
                ("claude-hover", true, 1.2, true, 0, false),
                ("codex-hover", true, 1.2, true, 3, false),
                ("activity-hover", true, 1.2, true, 4, false),
                ("private-hover", true, 1.6, false, 0, true),
                ("small-font", false, 0.8, true, -1, false),
                ("large-font", false, 1.6, true, -1, false),
            ] {
                HOVER.store(index, Ordering::Release);
                SCROLL.store(0, Ordering::Release);
                let w = (if card { 560.0 } else { 800.0 } * s) as i32;
                let h = (if card {
                    if small {
                        390.0
                    } else {
                        790.0
                    }
                } else {
                    56.0
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
                    palette(),
                    s,
                    card,
                    &summary,
                    previews,
                    &cells,
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
unsafe extern "system" fn cardproc(window: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint(window, true);
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
        WM_MOUSEWHEEL => {
            let delta = ((w >> 16) as u16) as i16;
            if delta < 0 {
                SCROLL.fetch_add(1, Ordering::AcqRel);
            } else {
                let _ = SCROLL.fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                    Some(n.saturating_sub(1))
                });
            }
            InvalidateRect(window, null(), 0);
            0
        }
        WM_LBUTTONUP => {
            let y = ((l >> 16) as u16) as i16 as i32;
            let s = scale(hwnd());
            let start = (120.0 * s).round() as i32;
            let mut bounds: RECT = std::mem::zeroed();
            GetClientRect(window, &mut bounds);
            let visible = ((bounds.bottom - start - (40.0 * s).round() as i32)
                / (62.0 * s).round().max(1.0) as i32)
                .max(0);
            if y >= start && y < start + visible * (62.0 * s).round().max(1.0) as i32 {
                let index = ((y - start) / (62.0 * s).round().max(1.0) as i32) as usize
                    + SCROLL.load(Ordering::Acquire);
                let provider = match HOVER.load(Ordering::Acquire) {
                    0 | 1 | 2 => Some("claude"),
                    3 => Some("codex"),
                    _ => None,
                };
                if let Some(app) = APP.get() {
                    let summary = crate::app::dock_summary(app).0;
                    if let Some(row) = summary.recent(provider).get(index) {
                        crate::app::activate_prompt(app, row.id.clone());
                        hide_card();
                    }
                }
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
unsafe extern "system" fn wndproc(window: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
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
            pointer().press = Some((p, window_rect(window), false));
            SetCapture(window);
            0
        }
        WM_MOUSEMOVE => {
            let mut p: POINT = std::mem::zeroed();
            GetCursorPos(&mut p);
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
            hide_card();
            position_window(window, APP.get().and_then(crate::strip_position));
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
