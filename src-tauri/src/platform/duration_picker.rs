use super::*;
use windows_sys::Win32::UI::Controls::{
    EM_SETLIMITTEXT, EM_SETSEL, DRAWITEMSTRUCT, MEASUREITEMSTRUCT,
    ODS_DISABLED, ODS_SELECTED, ODS_FOCUS, ODT_MENU,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
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

const VALUE_ID: i32 = 101;
const UNIT_ID: i32 = 102;
const SUMMARY_ID: i32 = 103;
const PRESETS: [(u32, &str); 4] = [(60,"1 hour"),(360,"6 hours"),(1440,"24 hours"),(10080,"7 days")];
const UNITS: [(u32, &str); 3] = [(1,"Minutes"),(60,"Hours"),(1440,"Days")];
const PICKER_WIDTH: f64 = 600.0;
const PICKER_HEIGHT: f64 = 382.0;
struct PickerState {
    font: HFONT,
    heading: HFONT,
    small: HFONT,
    brush: HBRUSH,
    unit: usize,
    scale: f64,
}
unsafe fn picker(window: HWND) -> *mut PickerState {
    GetWindowLongPtrW(window, GWLP_USERDATA) as *mut PickerState
}
unsafe fn selected_unit(window: HWND) -> usize {
    let state=picker(window);
    if state.is_null() {0} else {(*state).unit}
}
unsafe fn input_minutes(window: HWND) -> Option<u32> {
    let mut value=[0u16;16];
    let n=GetWindowTextW(GetDlgItem(window,VALUE_ID),value.as_mut_ptr(),16);
    crate::dock::duration_minutes(&String::from_utf16_lossy(&value[..n.max(0) as usize]),UNITS[selected_unit(window)].0)
}
fn grouped(value: u32) -> String {
    if value>=1000 {format!("{},{:03}",value/1000,value%1000)} else {value.to_string()}
}
unsafe fn update_summary(window: HWND) {
    let minutes=input_minutes(window);
    let label=match minutes {
        Some(n) if n%1440==0=>format!("{} day{} · {} minutes",n/1440,if n==1440 {""} else {"s"},grouped(n)),
        Some(n) if n%60==0=>format!("{} hour{} · {} minutes",n/60,if n==60 {""} else {"s"},grouped(n)),
        Some(n)=>format!("{} minute{}",grouped(n),if n==1 {""} else {"s"}),
        None=>"Enter a duration from 1 minute to 30 days.".into(),
    };
    SetWindowTextW(GetDlgItem(window,SUMMARY_ID),wide(&label).as_ptr());
    EnableWindow(GetDlgItem(window,1),minutes.is_some() as i32);
    InvalidateRect(window,null(),0);
    for id in [1,201,202,203,204] {InvalidateRect(GetDlgItem(window,id),null(),0);}
}
unsafe fn set_minutes(window: HWND, minutes: u32) {
    let unit=if minutes>1440 && minutes%1440==0 {2} else if minutes%60==0 {1} else {0};
    (*picker(window)).unit=unit;
    SetWindowTextW(GetDlgItem(window,UNIT_ID),wide(UNITS[unit].1).as_ptr());
    SetWindowTextW(GetDlgItem(window,VALUE_ID),wide(&(minutes/UNITS[unit].0).to_string()).as_ptr());
    InvalidateRect(GetDlgItem(window,UNIT_ID),null(),0);
    update_summary(window);
}
unsafe fn layout_picker(window: HWND, s: f64) {
    let state=picker(window);
    if state.is_null() {return;}
    let new_font=font((-16.*s).round() as i32,400);
    let new_heading=font((-22.*s).round() as i32,400);
    let new_small=font((-13.*s).round() as i32,400);
    let old=[(*state).font,(*state).heading,(*state).small];
    (*state).font=new_font;(*state).heading=new_heading;(*state).small=new_small;(*state).scale=s;
    for (id,x,y,w,h) in [
        (10,24.,8.,490.,28.),(2,542.,7.,34.,28.),
        (11,24.,67.,552.,34.),
        (201,24.,126.,138.,38.),(202,162.,126.,138.,38.),
        (203,300.,126.,138.,38.),(204,438.,126.,138.,38.),
        (12,24.,192.,552.,24.),(VALUE_ID,32.,232.,244.,22.),
        (UNIT_ID,300.,224.,276.,38.),(SUMMARY_ID,24.,276.,552.,26.),
        (3,336.,326.,112.,36.),(1,464.,326.,112.,36.),
    ] {
        let child=GetDlgItem(window,id);
        SetWindowPos(child,null_mut(),(x*s).round() as i32,(y*s).round() as i32,
            (w*s).round() as i32,(h*s).round() as i32,SWP_NOZORDER|SWP_NOACTIVATE);
        let f=if id==11 {new_heading} else if id==SUMMARY_ID || id==12 {new_small} else {new_font};
        SendMessageW(child,WM_SETFONT,f as usize,1);
    }
    for f in old {if !f.is_null() {DeleteObject(f);}}
}
unsafe fn custom_duration(minutes: u32) {
    let existing=DURATION.load(Ordering::Acquire) as HWND;
    if !existing.is_null() {SetForegroundWindow(existing);return;}
    let instance=GetModuleHandleW(null());
    let name=wide("GcdUsageDurationPicker");
    let mut class:WNDCLASSW=std::mem::zeroed();
    class.lpfnWndProc=Some(proc);class.hInstance=instance;
    class.lpszClassName=name.as_ptr();class.hCursor=LoadCursorW(null_mut(),IDC_ARROW);
    RegisterClassW(&class);
    let r=window_rect(hwnd());let work=monitor(hwnd(),None).rcWork;
    let s=scale(hwnd()).min((work.right-work.left) as f64/PICKER_WIDTH)
        .min((work.bottom-work.top) as f64/PICKER_HEIGHT).max(0.01);
    let width=(PICKER_WIDTH*s).round() as i32;
    let height=(PICKER_HEIGHT*s).round() as i32;
    let x=(r.right-width).clamp(work.left,(work.right-width).max(work.left));
    let y=(r.top-height-12).clamp(work.top,(work.bottom-height).max(work.top));
    let window=CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_TOPMOST|WS_EX_CONTROLPARENT,
        name.as_ptr(),wide("GCD Usage · activity duration").as_ptr(),WS_POPUP|WS_SYSMENU,
        x,y,width,height,hwnd(),null_mut(),instance,null());
    if window.is_null() {return;}
    let state=Box::new(PickerState {font:null_mut(),heading:null_mut(),small:null_mut(),
        brush:CreateSolidBrush(palette().bg),unit:0,scale:s});
    SetWindowLongPtrW(window,GWLP_USERDATA,Box::into_raw(state) as isize);
    DURATION.store(window as isize,Ordering::Release);
    let button=WS_TABSTOP|BS_OWNERDRAW as u32;
    for (kind,label,id,style) in [
        ("STATIC","gcd_ / Activity duration",10,0),("BUTTON","Close",2,button),
        ("STATIC","Show tokens from the last…",11,0),
        ("BUTTON","1 hour",201,button),("BUTTON","6 hours",202,button),
        ("BUTTON","24 hours",203,button),("BUTTON","7 days",204,button),
        ("STATIC","Custom duration",12,0),
        ("EDIT","",VALUE_ID,WS_TABSTOP|ES_NUMBER as u32|ES_AUTOHSCROLL as u32),
        ("BUTTON","Hours",UNIT_ID,button),
        ("STATIC","",SUMMARY_ID,0),
        ("BUTTON","Cancel",3,button),("BUTTON","Apply",1,button),
    ] {
        let child=CreateWindowExW(0,wide(kind).as_ptr(),wide(label).as_ptr(),
            WS_CHILD|WS_VISIBLE|style,0,0,1,1,window,id as HMENU,instance,null());
        if child.is_null() {DestroyWindow(window);return;}
    }
    layout_picker(window,s);
    SendMessageW(GetDlgItem(window,VALUE_ID),EM_SETLIMITTEXT,5,0);
    set_minutes(window,minutes.clamp(1,43200));
    ShowWindow(window,SW_SHOW);SetForegroundWindow(window);
    windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(GetDlgItem(window,VALUE_ID));
    SendMessageW(GetDlgItem(window,VALUE_ID),EM_SETSEL,0,-1);
}
unsafe fn unit_menu(window: HWND) {
    let menu=CreatePopupMenu();
    if menu.is_null() {return;}
    let mut menu_info:MENUINFO=std::mem::zeroed();
    menu_info.cbSize=size_of::<MENUINFO>() as u32;
    menu_info.fMask=MIM_BACKGROUND;
    menu_info.hbrBack=(*picker(window)).brush;
    SetMenuInfo(menu,&menu_info);
    for (i,(_,label)) in UNITS.iter().enumerate() {
        AppendMenuW(menu,MF_STRING,2100+i,wide(label).as_ptr());
        let mut info:MENUITEMINFOW=std::mem::zeroed();
        info.cbSize=size_of::<MENUITEMINFOW>() as u32;
        info.fMask=MIIM_FTYPE|MIIM_DATA|MIIM_STATE;
        info.fType=MFT_OWNERDRAW;
        info.dwItemData=i;
        info.fState=if selected_unit(window)==i {MFS_CHECKED} else {MFS_UNCHECKED};
        SetMenuItemInfoW(menu,i as u32,1,&info);
    }
    let r=window_rect(GetDlgItem(window,UNIT_ID));
    let choice=TrackPopupMenu(menu,TPM_RETURNCMD|TPM_NONOTIFY|TPM_LEFTALIGN,
        r.left,r.bottom,0,window,null());
    DestroyMenu(menu);
    if (2100..=2102).contains(&choice) {
        let unit=(choice-2100) as usize;
        (*picker(window)).unit=unit;
        SetWindowTextW(GetDlgItem(window,UNIT_ID),wide(UNITS[unit].1).as_ptr());
        InvalidateRect(GetDlgItem(window,UNIT_ID),null(),0);
        update_summary(window);
    }
}
unsafe fn paint_picker(window: HWND) {
    let mut ps:PAINTSTRUCT=std::mem::zeroed();let dc=BeginPaint(window,&mut ps);
    let state=picker(window);
    if !state.is_null() {
        let p=palette();let s=(*state).scale;
        let mut bounds:RECT=std::mem::zeroed();GetClientRect(window,&mut bounds);
        round(dc,bounds,p.bg,p.border,0);
        fill(dc,RECT {left:0,top:(44.*s) as i32,right:bounds.right,bottom:(44.*s) as i32+1},p.border);
        let mut edit=window_rect(GetDlgItem(window,VALUE_ID));
        let mut origin=POINT {x:edit.left,y:edit.top};ScreenToClient(window,&mut origin);
        let inset=(8.*s).round() as i32;
        edit=RECT {left:origin.x-inset,top:origin.y-inset,right:origin.x+edit.right-edit.left+inset,bottom:origin.y+edit.bottom-edit.top+inset};
        let border=if windows_sys::Win32::UI::Input::KeyboardAndMouse::GetFocus()==GetDlgItem(window,VALUE_ID) {p.green} else {p.border};
        let brush=CreateSolidBrush(border);FrameRect(dc,&edit,brush);DeleteObject(brush);
    }
    EndPaint(window,&ps);
}
unsafe fn draw_item(window: HWND, item: &DRAWITEMSTRUCT) {
    let state=picker(window);if state.is_null() {return;}
    let p=palette();let s=(*state).scale;let f=(*state).font;
    let disabled=item.itemState & ODS_DISABLED!=0;
    let pressed=item.itemState & ODS_SELECTED!=0;
    let id=item.CtlID as i32;
    let preset=(201..=204).contains(&id).then(||PRESETS[(id-201) as usize].0);
    let selected=preset.is_some_and(|n|input_minutes(window)==Some(n));
    let primary=id==1 && !disabled;
    let bg=if primary {p.text} else if selected || pressed {p.green} else {p.bg};
    let ink=if primary || selected || pressed {p.bg} else if disabled {p.muted} else {p.text};
    let label=if item.CtlType==ODT_MENU {
        UNITS.get(item.itemData).map(|u|u.1).unwrap_or("")
    } else {match id {
        1=>"Apply",2=>"×",3=>"Cancel",UNIT_ID=>UNITS[selected_unit(window)].1,
        201=>"1 hour",202=>"6 hours",203=>"24 hours",204=>"7 days",_=>"",
    }};
    let menu=item.CtlType==ODT_MENU;
    round(item.hDC,item.rcItem,if menu && pressed {p.green} else {bg},if menu {p.bg} else {p.border},0);
    let mut r=item.rcItem;r.left+=(12.*s) as i32;r.right-=(12.*s) as i32;
    SetBkMode(item.hDC,TRANSPARENT as i32);
    text(item.hDC,f,ink,label,r);
    if id==UNIT_ID && !menu {
        let mut arrow=r;arrow.left=arrow.right-(20.*s) as i32;
        text_right(item.hDC,f,ink,"▾",arrow);
    }
    if item.itemState & ODS_FOCUS!=0 {
        let mut focus=item.rcItem;InflateRect(&mut focus,-3,-3);DrawFocusRect(item.hDC,&focus);
    }
}
unsafe extern "system" fn proc(window: HWND,msg:u32,w:WPARAM,l:LPARAM)->LRESULT {
    match msg {
        WM_ERASEBKGND=>1,
        WM_PAINT=>{paint_picker(window);0}
        WM_NCHITTEST=>{
            let r=window_rect(window);let state=picker(window);
            let x=(l as u16) as i16 as i32-r.left;
            let y=((l>>16) as u16) as i16 as i32-r.top;
            if !state.is_null() && y>=0 && y<(44.*(*state).scale) as i32
                && x<(530.*(*state).scale) as i32 {HTCAPTION as isize}
            else {DefWindowProcW(window,msg,w,l)}
        }
        // DM_GETDEFID: preserve Enter-to-apply with owner-drawn native buttons.
        DM_GETDEFID=>(((DC_HASDEFID as usize)<<16)|1) as isize,
        WM_CTLCOLORSTATIC|WM_CTLCOLOREDIT=>{
            let state=picker(window);if state.is_null() {return DefWindowProcW(window,msg,w,l);}
            let p=palette();let dc=w as HDC;
            SetTextColor(dc,if GetDlgCtrlID(l as HWND)==SUMMARY_ID {p.muted} else {p.text});
            SetBkColor(dc,p.bg);(*state).brush as isize
        }
        WM_MEASUREITEM=>{
            let item=&mut *(l as *mut MEASUREITEMSTRUCT);
            if item.CtlType==ODT_MENU {
                let state=picker(window);let s=if state.is_null() {1.} else {(*state).scale};
                item.itemWidth=(180.*s) as u32;item.itemHeight=(34.*s) as u32;1
            } else {0}
        }
        WM_DRAWITEM=>{draw_item(window,&*(l as *const DRAWITEMSTRUCT));1}
        WM_COMMAND=>{
            let id=(w & 0xffff) as i32;
            match id {
                1=>{
                    if let Some(minutes)=input_minutes(window) {save(minutes);DestroyWindow(window);}
                    else {update_summary(window);}
                }
                2|3=>{DestroyWindow(window);}
                VALUE_ID=>{
                    let notification=(w>>16) as u32;
                    if notification==EN_CHANGE as u32 {update_summary(window);}
                    if notification==EN_SETFOCUS as u32 || notification==EN_KILLFOCUS as u32 {InvalidateRect(window,null(),0);}
                }
                UNIT_ID=>unit_menu(window),
                201..=204=>set_minutes(window,PRESETS[(id-201) as usize].0),
                _=>{}
            }
            0
        }
        WM_THEMECHANGED|WM_SETTINGCHANGE=>{
            let state=picker(window);
            if !state.is_null() {
                let old=(*state).brush;(*state).brush=CreateSolidBrush(palette().bg);DeleteObject(old);
                RedrawWindow(window,null(),null_mut(),RDW_INVALIDATE|RDW_ALLCHILDREN);
            }
            0
        }
        WM_DPICHANGED=>{
            let state=picker(window);if state.is_null() {return 0;}
            let desired=&*(l as *const RECT);
            let work=monitor(window,Some(POINT{x:desired.left,y:desired.top})).rcWork;
            let s=scale(window).min((work.right-work.left) as f64/PICKER_WIDTH)
                .min((work.bottom-work.top) as f64/PICKER_HEIGHT).max(0.01);
            let width=(PICKER_WIDTH*s).round() as i32;let height=(PICKER_HEIGHT*s).round() as i32;
            let x=desired.left.clamp(work.left,(work.right-width).max(work.left));
            let y=desired.top.clamp(work.top,(work.bottom-height).max(work.top));
            SetWindowPos(window,HWND_TOPMOST,x,y,width,height,SWP_NOACTIVATE);
            layout_picker(window,s);InvalidateRect(window,null(),0);0
        }
        WM_CLOSE=>{DestroyWindow(window);0}
        WM_NCDESTROY=>{
            let state=picker(window);SetWindowLongPtrW(window,GWLP_USERDATA,0);
            if !state.is_null() {
                let state=Box::from_raw(state);
                for object in [state.font,state.heading,state.small,state.brush] {
                    if !object.is_null() {DeleteObject(object);}
                }
            }
            DURATION.store(0,Ordering::Release);
            DefWindowProcW(window,msg,w,l)
        }
        _=>DefWindowProcW(window,msg,w,l),
    }
}
