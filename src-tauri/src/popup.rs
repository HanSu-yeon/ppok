use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::error::Result;
use crate::platform;
use crate::settings::Corner;
use crate::state::PopupPayload;

pub const LABEL: &str = "popup";
/// 카드(340×156) + 그림자 여백. 실제 카드 크기는 popup.css에서 정한다.
const WIDTH: f64 = 372.0;
const HEIGHT: f64 = 196.0;
/// 작업 영역 가장자리로부터의 여백(논리 px). 그림자 여백이 따로 있어 작게 둔다.
const EDGE: f64 = 4.0;

pub fn create(app: &AppHandle) -> Result<WebviewWindow> {
    let win = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("popup.html".into()))
        .title("뽁")
        .inner_size(WIDTH, HEIGHT)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .skip_taskbar(true)
        .focused(false)
        .focusable(false)
        .visible(false)
        .build()?;
    Ok(win)
}

fn window(app: &AppHandle) -> Result<WebviewWindow> {
    match app.get_webview_window(LABEL) {
        Some(w) => Ok(w),
        None => create(app),
    }
}

/// 주 모니터 작업 영역(작업 표시줄 제외)의 지정한 모서리에 맞춘다.
fn place(win: &WebviewWindow, corner: Corner) -> Result<()> {
    let Some(monitor) = win.primary_monitor()?.or(win.current_monitor()?) else {
        return Ok(());
    };
    let scale = monitor.scale_factor();
    win.set_size(LogicalSize::new(WIDTH, HEIGHT))?;
    let area = monitor.work_area();
    let w = (WIDTH * scale).round() as i32;
    let h = (HEIGHT * scale).round() as i32;
    let edge = (EDGE * scale).round() as i32;
    let left = area.position.x + edge;
    let right = area.position.x + area.size.width as i32 - w - edge;
    let top = area.position.y + edge;
    let bottom = area.position.y + area.size.height as i32 - h - edge;
    let (x, y) = match corner {
        Corner::BottomLeft => (left, bottom),
        Corner::TopRight => (right, top),
        Corner::TopLeft => (left, top),
        Corner::BottomRight | Corner::Random => (right, bottom),
    };
    win.set_position(PhysicalPosition::new(x, y))?;
    Ok(())
}

pub fn show(app: &AppHandle, payload: &PopupPayload) -> Result<()> {
    let win = window(app)?;
    place(&win, payload.corner)?;
    win.emit_to(LABEL, "popup:word", payload)?;
    platform::show_without_focus(&win)?;
    Ok(())
}

pub fn hide(app: &AppHandle) -> Result<()> {
    if let Some(win) = app.get_webview_window(LABEL) {
        platform::hide(&win)?;
    }
    Ok(())
}
