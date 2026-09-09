use super::*;
use windows_sys::Win32::UI::Controls::{EM_SETLIMITTEXT, EM_SETSEL};
pub(super) unsafe fn duration_menu() {
    hide_card();
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return;
    }
    let current = APP
        .get()
        .map(|a| crate::app::dock_summary(a).0.minutes)
        .unwrap_or(60);
    for (minutes, label) in [
        (30, "Last 30 minutes"),
        (60, "Last hour"),
        (360, "Last 6 hours"),
        (1440, "Last 24 hours"),
        (10080, "Last 7 days"),
        (43200, "Last 30 days"),
    ] {
        AppendMenuW(
            menu,
            MF_STRING | if minutes == current { MF_CHECKED } else { 0 },
            minutes as usize,
            wide(label).as_ptr(),
        );
    }
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    AppendMenuW(menu, MF_STRING, 50001, wide("Custom duration…").as_ptr());
    AppendMenuW(menu, MF_STRING, 50002, wide("Open dashboard").as_ptr());
    let size_menu = CreatePopupMenu();
    if !size_menu.is_null() {
        let current_scale = APP.get().map(crate::app::dock_scale).unwrap_or(100);
        for scale in [80u16, 100, 120, 140, 160] {
            AppendMenuW(size_menu, MF_STRING | if scale == current_scale { MF_CHECKED } else { 0 },
                60000 + scale as usize, wide(&format!("{scale}%")).as_ptr());
        }
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        AppendMenuW(menu, MF_POPUP, size_menu as usize, wide("Dock size").as_ptr());
    }
    let mut p: POINT = std::mem::zeroed();
    GetCursorPos(&mut p);
    SetForegroundWindow(hwnd());
    let choice = TrackPopupMenu(
        menu,
        TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
        p.x,
        p.y,
        0,
        hwnd(),
        null(),
    );
    DestroyMenu(menu);
    PostMessageW(hwnd(), WM_NULL, 0, 0);
    match choice {
        50001 => custom_duration(current),
        50002 => open_dashboard(),
        60080..=60160 => {
            if let Some(app) = APP.get() {
                if let Err(error) = crate::app::set_dock_scale(app, (choice - 60000) as u16) {
                    MessageBoxW(hwnd(), wide(&error).as_ptr(), wide("Size not saved").as_ptr(), MB_OK | MB_ICONERROR);
                }
            }
        },
        1..=43200 => save(choice as u32),
        _ => {}
    }
}
unsafe fn save(minutes: u32) {
    if let Some(app) = APP.get() {
        let a = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            if let Err(e) = crate::app::set_dock_minutes(&a, minutes) {
                unsafe {
                    MessageBoxW(
                        hwnd(),
                        wide(&e).as_ptr(),
                        wide("Duration not saved").as_ptr(),
                        MB_OK | MB_ICONERROR,
                    );
                }
            }
        });
    }
}
unsafe fn custom_duration(minutes: u32) {
    let existing = DURATION.load(Ordering::Acquire) as HWND;
    if !existing.is_null() {
        SetForegroundWindow(existing);
        return;
    }
    let instance = GetModuleHandleW(null());
    let name = wide("GcdUsageDurationPicker");
    let mut class: WNDCLASSW = std::mem::zeroed();
    class.lpfnWndProc = Some(proc);
    class.hInstance = instance;
    class.lpszClassName = name.as_ptr();
    class.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
    class.hbrBackground = (COLOR_WINDOW + 1) as HBRUSH;
    RegisterClassW(&class);
    let s = scale(hwnd());
    let r = window_rect(hwnd());
    let work = monitor(hwnd(), None).rcWork;
    let width = (390.0 * s) as i32;
    let height = (190.0 * s) as i32;
    let x = (r.right - width).clamp(work.left, (work.right - width).max(work.left));
    let y = (r.top - height - 12).clamp(work.top, (work.bottom - height).max(work.top));
    let window = CreateWindowExW(
        WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_CONTROLPARENT,
        name.as_ptr(),
        wide("GCD Usage · activity duration").as_ptr(),
        WS_POPUP | WS_CAPTION | WS_SYSMENU,
        x,
        y,
        width,
        height,
        hwnd(),
        null_mut(),
        instance,
        null(),
    );
    if window.is_null() {
        return;
    }
    DURATION.store(window as isize, Ordering::Release);
    let f = font((-16.0 * s) as i32, 400);
    SetWindowLongPtrW(window, GWLP_USERDATA, f as isize);
    for (kind, text, id, left, top, w, h, style) in [
        (
            "STATIC",
            "Show tokens from the last…",
            10,
            18.,
            12.,
            340.,
            24.,
            0,
        ),
        (
            "EDIT",
            "",
            101,
            18.,
            44.,
            180.,
            31.,
            WS_BORDER | WS_TABSTOP | ES_NUMBER as u32 | ES_AUTOHSCROLL as u32,
        ),
        ("STATIC", "minutes (1–43,200)", 11, 207., 48., 160., 24., 0),
        (
            "BUTTON",
            "Apply",
            1,
            18.,
            97.,
            120.,
            34.,
            WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
        ),
        ("BUTTON", "Cancel", 2, 151., 97., 120., 34., WS_TABSTOP),
    ] {
        let child = CreateWindowExW(
            0,
            wide(kind).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            (left * s) as i32,
            (top * s) as i32,
            (w * s) as i32,
            (h * s) as i32,
            window,
            id as HMENU,
            instance,
            null(),
        );
        SendMessageW(child, WM_SETFONT, f as usize, 1);
    }
    let edit = GetDlgItem(window, 101);
    SetWindowTextW(edit, wide(&minutes.max(1).to_string()).as_ptr());
    SendMessageW(edit, EM_SETLIMITTEXT, 5, 0);
    ShowWindow(window, SW_SHOW);
    SetForegroundWindow(window);
    windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(edit);
    SendMessageW(edit, EM_SETSEL, 0, -1);
}
unsafe extern "system" fn proc(window: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND if w & 0xffff == 1 => {
            let mut value = [0u16; 16];
            let n = GetWindowTextW(GetDlgItem(window, 101), value.as_mut_ptr(), 16);
            let minutes = String::from_utf16_lossy(&value[..n.max(0) as usize])
                .parse::<u32>()
                .ok()
                .filter(|n| (1..=43200).contains(n));
            if let Some(minutes) = minutes {
                save(minutes);
                DestroyWindow(window);
            } else {
                MessageBoxW(
                    window,
                    wide("Enter a whole number from 1 to 43,200 minutes.").as_ptr(),
                    wide("Choose a duration").as_ptr(),
                    MB_OK | MB_ICONINFORMATION,
                );
            }
            0
        }
        WM_COMMAND if w & 0xffff == 2 => {
            DestroyWindow(window);
            0
        }
        WM_CLOSE => {
            DestroyWindow(window);
            0
        }
        WM_DESTROY => {
            let f = GetWindowLongPtrW(window, GWLP_USERDATA) as HFONT;
            if !f.is_null() {
                DeleteObject(f);
            }
            DURATION.store(0, Ordering::Release);
            0
        }
        _ => DefWindowProcW(window, msg, w, l),
    }
}
