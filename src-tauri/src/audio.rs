//! 通知音再生モジュール
//!
//! rodio クレートを使用して MP3 音声を再生する

use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use tracing::{error, info};

/// 通知音データ（コンパイル時に埋め込み）
static NOTIFICATION_SOUND: &[u8] = include_bytes!("../resources/sounds/notification.mp3");

/// オーディオシステムを初期化（現在は何もしない）
pub fn init_audio() -> Result<(), String> {
    info!("Audio system ready");
    Ok(())
}

/// 通知音を再生する（非同期、別スレッドで実行）
pub fn play_notification_sound(volume: f32) {
    std::thread::spawn(move || {
        play_notification_sound_sync(volume);
    });
}

/// 通知音を再生する（同期）
/// 各呼び出しで新しい OutputStream を作成する
fn play_notification_sound_sync(volume: f32) {
    match OutputStream::try_default() {
        Ok((_stream, handle)) => {
            match Sink::try_new(&handle) {
                Ok(sink) => {
                    let cursor = Cursor::new(NOTIFICATION_SOUND);
                    match Decoder::new(cursor) {
                        Ok(source) => {
                            sink.set_volume(volume.clamp(0.0, 1.0));
                            sink.append(source);
                            sink.sleep_until_end();
                            info!("Notification sound played successfully");
                        }
                        Err(e) => {
                            error!("Failed to decode notification sound: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to create audio sink: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Failed to create audio output stream: {}", e);
        }
    }
}

/// Tauriコマンド: テスト再生
#[tauri::command]
pub fn play_test_sound(volume: f32) {
    play_notification_sound(volume);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_sound_data_exists() {
        assert!(!NOTIFICATION_SOUND.is_empty());
    }

    #[test]
    fn test_notification_sound_is_valid_mp3() {
        // MP3ファイルのマジックナンバーを確認
        // ID3v2ヘッダー (ID3) または MP3フレームヘッダー (0xFF 0xFB/0xFA/0xF3/0xF2)
        let has_id3 = NOTIFICATION_SOUND.len() >= 3
            && NOTIFICATION_SOUND[0] == b'I'
            && NOTIFICATION_SOUND[1] == b'D'
            && NOTIFICATION_SOUND[2] == b'3';

        let has_mp3_frame = NOTIFICATION_SOUND.len() >= 2
            && NOTIFICATION_SOUND[0] == 0xFF
            && (NOTIFICATION_SOUND[1] & 0xE0) == 0xE0;

        assert!(has_id3 || has_mp3_frame, "File should be a valid MP3");
    }
}
