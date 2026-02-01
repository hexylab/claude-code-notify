//! タスクバー制御モジュール（Windows専用）
//!
//! タスクバーボタンの点滅とバッジ（オーバーレイアイコン）表示を制御する

#[cfg(windows)]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
#[cfg(windows)]
use tracing::{error, info, warn};
#[cfg(windows)]
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{COLORREF, HWND},
        Graphics::Gdi::{
            CreateBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush, DeleteDC,
            DeleteObject, DrawTextW, Ellipse, GetDC, ReleaseDC, SelectObject, SetBkMode,
            SetTextColor, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, DEFAULT_QUALITY,
            DT_CENTER, DT_SINGLELINE, DT_VCENTER, FF_DONTCARE, FW_BOLD, HBRUSH,
            OUT_DEFAULT_PRECIS, TRANSPARENT,
        },
        System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED},
        UI::{
            Shell::{ITaskbarList3, TaskbarList},
            WindowsAndMessaging::{
                CreateIconIndirect, DestroyIcon, FlashWindowEx,
                FLASHWINFO, FLASHW_ALL, FLASHW_STOP, FLASHW_TIMERNOFG, HICON, ICONINFO,
            },
        },
    },
};

/// RGB to COLORREF (0x00BBGGRR)
#[cfg(windows)]
fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

/// タスクバーシステムを初期化（COM初期化のみ）
#[cfg(windows)]
pub fn init_taskbar() -> Result<(), String> {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if hr.is_err() {
            // Already initialized is OK
            warn!("COM already initialized or failed: {:?}", hr);
        }
        info!("Taskbar system initialized");
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn init_taskbar() -> Result<(), String> {
    Ok(())
}

/// ITaskbarList3 インスタンスを作成
#[cfg(windows)]
fn get_taskbar_list() -> Option<ITaskbarList3> {
    unsafe {
        match CoCreateInstance::<_, ITaskbarList3>(&TaskbarList, None, CLSCTX_INPROC_SERVER) {
            Ok(taskbar) => {
                if let Err(e) = taskbar.HrInit() {
                    error!("Failed to initialize TaskbarList: {}", e);
                    return None;
                }
                Some(taskbar)
            }
            Err(e) => {
                error!("Failed to create TaskbarList: {}", e);
                None
            }
        }
    }
}

/// WebviewWindow から HWND を取得
#[cfg(windows)]
pub fn get_hwnd(window: &tauri::WebviewWindow) -> Option<HWND> {
    match window.window_handle() {
        Ok(handle) => match handle.as_raw() {
            RawWindowHandle::Win32(win32_handle) => {
                Some(HWND(win32_handle.hwnd.get() as *mut std::ffi::c_void))
            }
            _ => {
                warn!("Not a Win32 window handle");
                None
            }
        },
        Err(e) => {
            error!("Failed to get window handle: {}", e);
            None
        }
    }
}

#[cfg(not(windows))]
pub fn get_hwnd(_window: &tauri::WebviewWindow) -> Option<()> {
    None
}

/// タスクバーボタンを点滅させる
#[cfg(windows)]
pub fn flash_taskbar(hwnd: HWND, count: u32) {
    let flash_info = FLASHWINFO {
        cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
        hwnd,
        dwFlags: if count == 0 {
            FLASHW_STOP
        } else {
            FLASHW_ALL | FLASHW_TIMERNOFG
        },
        uCount: count,
        dwTimeout: 0,
    };
    unsafe {
        let _ = FlashWindowEx(&flash_info);
    }
    info!("Taskbar flash triggered (count: {})", count);
}

#[cfg(not(windows))]
pub fn flash_taskbar(_hwnd: (), _count: u32) {}

/// タスクバーの点滅を停止
#[cfg(windows)]
pub fn stop_flash(hwnd: HWND) {
    flash_taskbar(hwnd, 0);
}

#[cfg(not(windows))]
pub fn stop_flash(_hwnd: ()) {}

/// オーバーレイバッジを設定（未確認メッセージ数を表示）
#[cfg(windows)]
pub fn set_overlay_badge(hwnd: HWND, count: u32) -> Result<(), String> {
    if let Some(taskbar) = get_taskbar_list() {
        unsafe {
            if count == 0 {
                // バッジをクリア（null HICONを渡す）
                taskbar
                    .SetOverlayIcon(hwnd, HICON::default(), PCWSTR::null())
                    .map_err(|e| format!("Failed to clear overlay icon: {}", e))?;
                info!("Overlay badge cleared");
            } else {
                // 数字付きアイコンを動的生成して設定
                let icon = create_badge_icon(count)?;
                let description: Vec<u16> = format!("{}件の通知\0", count)
                    .encode_utf16()
                    .collect();
                taskbar
                    .SetOverlayIcon(hwnd, icon, PCWSTR(description.as_ptr()))
                    .map_err(|e| format!("Failed to set overlay icon: {}", e))?;
                // アイコンを破棄
                let _ = DestroyIcon(icon);
                info!("Overlay badge set to {}", count);
            }
        }
    } else {
        return Err("Failed to get taskbar list".to_string());
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn set_overlay_badge(_hwnd: (), _count: u32) -> Result<(), String> {
    Ok(())
}

/// オーバーレイバッジをクリア
#[cfg(windows)]
pub fn clear_overlay_badge(hwnd: HWND) -> Result<(), String> {
    set_overlay_badge(hwnd, 0)
}

#[cfg(not(windows))]
pub fn clear_overlay_badge(_hwnd: ()) -> Result<(), String> {
    Ok(())
}

/// バッジアイコンを動的に生成（赤丸に白文字で数字）
#[cfg(windows)]
fn create_badge_icon(count: u32) -> Result<HICON, String> {
    let display_text = if count > 9 {
        "9+".to_string()
    } else {
        count.to_string()
    };

    unsafe {
        // アイコンサイズ（16x16）
        let size: i32 = 16;

        // デスクトップDCを取得
        let screen_dc = GetDC(None);
        if screen_dc.is_invalid() {
            return Err("Failed to get screen DC".to_string());
        }

        // 互換DCを作成
        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        if mem_dc.is_invalid() {
            let _ = ReleaseDC(None, screen_dc);
            return Err("Failed to create compatible DC".to_string());
        }

        // カラービットマップを作成
        let color_bitmap = CreateBitmap(size, size, 1, 32, None);
        if color_bitmap.is_invalid() {
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(None, screen_dc);
            return Err("Failed to create color bitmap".to_string());
        }

        // マスクビットマップを作成
        let mask_bitmap = CreateBitmap(size, size, 1, 1, None);
        if mask_bitmap.is_invalid() {
            let _ = DeleteObject(color_bitmap.into());
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(None, screen_dc);
            return Err("Failed to create mask bitmap".to_string());
        }

        // カラービットマップを選択
        let old_bitmap = SelectObject(mem_dc, color_bitmap.into());

        // 赤いブラシで円を描画
        let red_brush: HBRUSH = CreateSolidBrush(rgb(220, 53, 69));
        let old_brush = SelectObject(mem_dc, red_brush.into());

        // 円を描画
        let _ = Ellipse(mem_dc, 0, 0, size, size);

        // テキスト設定
        SetBkMode(mem_dc, TRANSPARENT);
        SetTextColor(mem_dc, rgb(255, 255, 255));

        // フォントを作成
        let font_name: Vec<u16> = "Arial\0".encode_utf16().collect();
        let font = CreateFontW(
            12,                    // 高さ
            0,                     // 幅（0=自動）
            0,                     // 傾斜角度
            0,                     // 方向
            FW_BOLD.0 as i32,      // 太さ
            0,                     // イタリック
            0,                     // 下線
            0,                     // 打ち消し線
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            DEFAULT_QUALITY,
            (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
            PCWSTR(font_name.as_ptr()),
        );

        let old_font = SelectObject(mem_dc, font.into());

        // テキストを中央に描画
        let mut text: Vec<u16> = display_text.encode_utf16().collect();

        // テキストサイズを計算して中央揃え
        let mut rect = windows::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: size,
            bottom: size,
        };
        DrawTextW(
            mem_dc,
            &mut text,
            &mut rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );

        // オブジェクトを復元
        SelectObject(mem_dc, old_font);
        SelectObject(mem_dc, old_brush);
        SelectObject(mem_dc, old_bitmap);

        // リソースを解放
        let _ = DeleteObject(font.into());
        let _ = DeleteObject(red_brush.into());
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(None, screen_dc);

        // アイコンを作成
        let icon_info = ICONINFO {
            fIcon: windows::Win32::Foundation::TRUE,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask_bitmap,
            hbmColor: color_bitmap,
        };

        let icon = CreateIconIndirect(&icon_info)
            .map_err(|e| format!("Failed to create icon: {}", e))?;

        // ビットマップを解放
        let _ = DeleteObject(color_bitmap.into());
        let _ = DeleteObject(mask_bitmap.into());

        Ok(icon)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_non_windows_functions_compile() {
        #[cfg(not(windows))]
        {
            let _ = super::init_taskbar();
            super::flash_taskbar((), 0);
            super::stop_flash(());
            let _ = super::set_overlay_badge((), 0);
            let _ = super::clear_overlay_badge(());
        }
    }
}
