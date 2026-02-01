//! トレイアイコン点滅モジュール
//!
//! 通知があった際にトレイアイコンを点滅させる機能を提供する。
//! 通常アイコンと赤いドット付きアイコンを交互に表示して点滅効果を出す。

use image::{Rgba, RgbaImage};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{image::Image, AppHandle};
use tracing::{error, info};

/// 通常のトレイアイコンデータ
static NORMAL_ICON: &[u8] = include_bytes!("../icons/icon.png");

/// 点滅状態を管理する構造体
pub struct TrayFlasher {
    is_flashing: Arc<AtomicBool>,
    notification_icon: Vec<u8>,
}

impl TrayFlasher {
    /// 新しい TrayFlasher を作成
    pub fn new() -> Self {
        let notification_icon = create_notification_icon().unwrap_or_else(|e| {
            error!("Failed to create notification icon: {}", e);
            NORMAL_ICON.to_vec()
        });

        Self {
            is_flashing: Arc::new(AtomicBool::new(false)),
            notification_icon,
        }
    }

    /// トレイアイコンの点滅を開始（stop_flashが呼ばれるまで無限に点滅）
    pub fn start_flash(&self, app: &AppHandle) {
        // 既に点滅中なら何もしない
        if self.is_flashing.swap(true, Ordering::SeqCst) {
            return;
        }

        let is_flashing = self.is_flashing.clone();
        let notification_icon = self.notification_icon.clone();
        let app_handle = app.clone();

        std::thread::spawn(move || {
            let mut show_notification = true;

            while is_flashing.load(Ordering::SeqCst) {
                let icon_data = if show_notification {
                    &notification_icon
                } else {
                    NORMAL_ICON
                };

                if let Some(tray) = app_handle.tray_by_id("main-tray") {
                    match Image::from_bytes(icon_data) {
                        Ok(icon) => {
                            if let Err(e) = tray.set_icon(Some(icon)) {
                                error!("Failed to set tray icon: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to create icon from bytes: {}", e);
                        }
                    }
                }

                show_notification = !show_notification;
                std::thread::sleep(Duration::from_millis(500));
            }

            // 点滅終了後は通常アイコンに戻す
            if let Some(tray) = app_handle.tray_by_id("main-tray") {
                if let Ok(icon) = Image::from_bytes(NORMAL_ICON) {
                    let _ = tray.set_icon(Some(icon));
                }
            }

            info!("Tray icon flash stopped");
        });

        info!("Tray icon flash started (infinite until stopped)");
    }

    /// トレイアイコンの点滅を停止し、通常アイコンに戻す
    pub fn stop_flash(&self, app: &AppHandle) {
        self.is_flashing.store(false, Ordering::SeqCst);

        if let Some(tray) = app.tray_by_id("main-tray") {
            if let Ok(icon) = Image::from_bytes(NORMAL_ICON) {
                let _ = tray.set_icon(Some(icon));
            }
        }
    }

    /// 点滅中かどうかを確認
    #[allow(dead_code)]
    pub fn is_flashing(&self) -> bool {
        self.is_flashing.load(Ordering::SeqCst)
    }
}

/// 赤いドット付きの通知アイコンを動的に生成
fn create_notification_icon() -> Result<Vec<u8>, String> {
    // 元のアイコンを読み込む
    let img = image::load_from_memory(NORMAL_ICON)
        .map_err(|e| format!("Failed to load icon: {}", e))?;

    let mut rgba_img: RgbaImage = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();

    // 赤いドットのパラメータ
    let dot_radius = (width.min(height) / 4) as i32; // アイコンサイズの1/4
    let dot_center_x = (width as i32) - dot_radius - 1;
    let dot_center_y = dot_radius + 1;

    // 赤いドットを描画（アンチエイリアス付き円）
    draw_filled_circle(&mut rgba_img, dot_center_x, dot_center_y, dot_radius, Rgba([220, 53, 69, 255]));

    // PNGにエンコード
    let mut buffer = std::io::Cursor::new(Vec::new());
    rgba_img
        .write_to(&mut buffer, image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode icon: {}", e))?;

    Ok(buffer.into_inner())
}

/// 塗りつぶし円を描画
fn draw_filled_circle(img: &mut RgbaImage, cx: i32, cy: i32, radius: i32, color: Rgba<u8>) {
    let (width, height) = img.dimensions();

    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                continue;
            }

            let dx = x - cx;
            let dy = y - cy;
            let distance_sq = dx * dx + dy * dy;
            let radius_sq = radius * radius;

            if distance_sq <= radius_sq {
                // アンチエイリアス: 端に近いほど透明度を下げる
                let distance = (distance_sq as f32).sqrt();
                let r = radius as f32;

                if distance > r - 1.0 {
                    // 端付近はブレンド
                    let alpha = (r - distance).max(0.0);
                    let existing = img.get_pixel(x as u32, y as u32);
                    let blended = blend_pixels(existing, &color, alpha);
                    img.put_pixel(x as u32, y as u32, blended);
                } else {
                    img.put_pixel(x as u32, y as u32, color);
                }
            }
        }
    }
}

/// ピクセルをブレンド
fn blend_pixels(bg: &Rgba<u8>, fg: &Rgba<u8>, alpha: f32) -> Rgba<u8> {
    let a = alpha * (fg[3] as f32 / 255.0);
    let inv_a = 1.0 - a;

    Rgba([
        ((fg[0] as f32 * a) + (bg[0] as f32 * inv_a)) as u8,
        ((fg[1] as f32 * a) + (bg[1] as f32 * inv_a)) as u8,
        ((fg[2] as f32 * a) + (bg[2] as f32 * inv_a)) as u8,
        ((fg[3] as f32 * a) + (bg[3] as f32 * inv_a)) as u8,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_notification_icon() {
        let result = create_notification_icon();
        assert!(result.is_ok(), "Should create notification icon successfully");
        let icon_data = result.unwrap();
        assert!(!icon_data.is_empty(), "Icon data should not be empty");
    }
}
