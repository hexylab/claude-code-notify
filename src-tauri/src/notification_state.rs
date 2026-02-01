//! 未確認通知の状態管理モジュール
//!
//! 未読の通知数を追跡し、バッジ表示やリセットを管理する

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::info;

/// 通知状態を管理する構造体
#[derive(Debug, Clone)]
pub struct NotificationState {
    /// 未読通知カウント
    unread_count: Arc<AtomicU32>,
}

impl NotificationState {
    /// 新しい NotificationState を作成
    pub fn new() -> Self {
        Self {
            unread_count: Arc::new(AtomicU32::new(0)),
        }
    }

    /// 未読カウントを1増加し、新しい値を返す
    pub fn increment(&self) -> u32 {
        let new_count = self.unread_count.fetch_add(1, Ordering::SeqCst) + 1;
        info!("Notification count incremented to {}", new_count);
        new_count
    }

    /// 現在の未読カウントを取得
    pub fn get(&self) -> u32 {
        self.unread_count.load(Ordering::SeqCst)
    }

    /// 未読カウントをリセット（0に戻す）
    pub fn reset(&self) {
        self.unread_count.store(0, Ordering::SeqCst);
        info!("Notification count reset to 0");
    }
}

impl Default for NotificationState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_new_state_is_zero() {
        let state = NotificationState::new();
        assert_eq!(state.get(), 0);
    }

    #[test]
    fn test_increment() {
        let state = NotificationState::new();
        assert_eq!(state.increment(), 1);
        assert_eq!(state.increment(), 2);
        assert_eq!(state.increment(), 3);
        assert_eq!(state.get(), 3);
    }

    #[test]
    fn test_reset() {
        let state = NotificationState::new();
        state.increment();
        state.increment();
        assert_eq!(state.get(), 2);
        state.reset();
        assert_eq!(state.get(), 0);
    }

    #[test]
    fn test_clone_shares_state() {
        let state1 = NotificationState::new();
        let state2 = state1.clone();

        state1.increment();
        assert_eq!(state2.get(), 1);

        state2.increment();
        assert_eq!(state1.get(), 2);

        state1.reset();
        assert_eq!(state2.get(), 0);
    }

    #[test]
    fn test_thread_safety() {
        let state = NotificationState::new();
        let mut handles = vec![];

        // 10スレッドから同時にインクリメント
        for _ in 0..10 {
            let state_clone = state.clone();
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    state_clone.increment();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // 10スレッド × 100回 = 1000
        assert_eq!(state.get(), 1000);
    }

    #[test]
    fn test_default() {
        let state = NotificationState::default();
        assert_eq!(state.get(), 0);
    }
}
