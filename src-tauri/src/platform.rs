//! 운영체제 의존 기능. Windows가 아닌 곳(개발용 macOS 등)에서는 무난한 기본 동작만 한다.

use tauri::WebviewWindow;

/// 전체 화면 앱·프레젠테이션·잠금 화면 등 팝업을 띄우면 안 되는 상태인지.
pub fn should_suppress_popup() -> bool {
    #[cfg(windows)]
    {
        win::notification_state_busy() || win::foreground_is_fullscreen()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// 다른 앱의 키보드 포커스를 빼앗지 않고 팝업을 보여준다.
pub fn show_without_focus(window: &WebviewWindow) -> tauri::Result<()> {
    #[cfg(windows)]
    {
        let hwnd = window.hwnd()?.0 as windows_sys::Win32::Foundation::HWND;
        unsafe { win::show_no_activate(hwnd) };
        Ok(())
    }
    #[cfg(not(windows))]
    {
        window.show()
    }
}

pub fn hide(window: &WebviewWindow) -> tauri::Result<()> {
    #[cfg(windows)]
    {
        // 표시를 Win32로 직접 했으니 숨김도 같은 경로로 해야 상태가 어긋나지 않는다.
        let hwnd = window.hwnd()?.0 as windows_sys::Win32::Foundation::HWND;
        unsafe { win::hide(hwnd) };
        Ok(())
    }
    #[cfg(not(windows))]
    {
        window.hide()
    }
}

#[cfg(windows)]
mod win {
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow};
    use windows_sys::Win32::UI::Shell::{
        QUERY_USER_NOTIFICATION_STATE, QUNS_BUSY, QUNS_NOT_PRESENT, QUNS_PRESENTATION_MODE,
        QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowRect, HWND_TOPMOST, IsZoomed, SW_HIDE,
        SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, ShowWindow,
    };

    /// Windows가 "지금은 알림을 띄우지 말라"고 하는 상태 (전체 화면 앱, D3D 게임, 프레젠테이션, 잠금).
    pub fn notification_state_busy() -> bool {
        let mut state: QUERY_USER_NOTIFICATION_STATE = 0;
        let hr = unsafe { SHQueryUserNotificationState(&mut state) };
        hr >= 0
            && matches!(state, QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE | QUNS_NOT_PRESENT)
    }

    /// 보조 확인: 전경 창이 모니터 전체(작업 표시줄 포함)를 덮고 있는지. 바탕화면은 제외.
    /// 최대화 창은 제외한다 (작업 표시줄 자동 숨김이면 최대화 창도 모니터 전체를 덮는다).
    pub fn foreground_is_fullscreen() -> bool {
        unsafe {
            let fg = GetForegroundWindow();
            if fg.is_null() || IsZoomed(fg) != 0 {
                return false;
            }
            let mut class = [0u16; 64];
            let len = GetClassNameW(fg, class.as_mut_ptr(), class.len() as i32);
            let class = String::from_utf16_lossy(&class[..len.max(0) as usize]);
            if matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd") {
                return false;
            }
            let mut rect: RECT = std::mem::zeroed();
            if GetWindowRect(fg, &mut rect) == 0 {
                return false;
            }
            let monitor = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(monitor, &mut info) == 0 {
                return false;
            }
            let m = info.rcMonitor;
            rect.left <= m.left && rect.top <= m.top && rect.right >= m.right && rect.bottom >= m.bottom
        }
    }

    pub unsafe fn show_no_activate(hwnd: HWND) {
        unsafe {
            ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }

    pub unsafe fn hide(hwnd: HWND) {
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
}
