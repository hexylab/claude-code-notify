# CLAUDE.md

## 言語設定

このリポジトリでは、すべてのやり取りを日本語で行うこと。コードコメント、コミットメッセージ、ドキュメント、ユーザーへの応答など、すべて日本語で統一する。

## プロジェクト概要

Claude Code NotifyはWindows向けのシステムトレイアプリケーション。リモート環境（WSL/SSH）で動作するClaude CodeからMQTT経由で通知を受信し、Windowsデスクトップ通知として表示する。

## 技術スタック

- **フレームワーク**: Tauri v2
- **バックエンド**: Rust (Edition 2021)
- **フロントエンド**: HTML5 + Vanilla JavaScript
- **MQTTブローカー**: rumqttd 0.20（組み込み）
- **MQTTクライアント**: rumqttc 0.25
- **非同期ランタイム**: tokio

## ビルドコマンド

```bash
# 開発モード（ホットリロード付き）
npm run tauri dev

# デバッグログ有効
RUST_LOG=debug npm run tauri dev

# リリースビルド
npm run tauri build

# mqtt-publishツール単体ビルド
cargo build -p mqtt-publish --release

# テスト実行
cd src-tauri && cargo test

# フォーマット・リント
cargo fmt
cargo clippy
```

## アーキテクチャ

```
Claude Code (WSL/SSH)
    ↓ Hooks + Statusline → MQTT publish
    ↓ TCP:1883
Windows PC (Tauri App)
    ├── MQTT Broker (rumqttd)
    ├── MQTT Client (rumqttc)
    ├── State Manager
    └── Notification Manager
    ↓
Windows Toast通知 / トレイツールチップ
```

### MQTTトピック構造

```
claude-code/
├── events/
│   ├── stop                  # タスク完了
│   ├── permission-request    # 承認リクエスト
│   └── notification          # ユーザー入力要求
└── status/
    └── {session_id}          # セッション状態（定期送信）
```

## 主要モジュール（src-tauri/src/）

| モジュール | 役割 |
|-----------|------|
| `lib.rs` | アプリケーション全体の統合、イベントハンドラ、MQTTメッセージルーティング |
| `broker.rs` | MQTTブローカーのライフサイクル管理 |
| `client.rs` | MQTTクライアント（サブスクライバー）、トピック定義 |
| `state.rs` | セッション状態管理、セッション名マッピング（150カタカナ名） |
| `export.rs` | 設定ZIPファイル生成、IPアドレス検出 |
| `templates.rs` | Claude Codeフック用シェルスクリプトテンプレート |
| `tray.rs` | システムトレイ初期化、メニューイベント処理 |

## ワークスペース構成

- `src-tauri/` - Tauriバックエンド（メインアプリ）
- `mqtt-publish/` - スタンドアロンMQTT CLIツール（Windows用）
- `src/` - フロントエンド（HTML/CSS/JS）
- `docs/` - 設計ドキュメント

## 設定ファイル

- `src-tauri/config/rumqttd.toml` - MQTTブローカー設定
- `src-tauri/tauri.conf.json` - Tauriアプリ設定（ウィンドウサイズ480x560px）
- `src-tauri/capabilities/` - Tauriセキュリティ権限

## コーディング規約

- エラー処理: `thiserror`クレートで各モジュールにカスタムエラー型を定義
- Tauriコマンドは `Result<T, String>` を返す
- ログ: `tracing`クレートを使用、`RUST_LOG`環境変数でレベル制御
- セッションID形式: `hostname-ppid`（Claude Codeから受信）
- 並行処理: tokio + MPSCチャネル、RwLockでセッション状態管理

## セッション名管理

セッションIDは`hostname-ppid`形式で受信され、`SessionNameManager`が150種類のカタカナ名にマッピングする。セッションは5分のタイムアウトでクリーンアップされる。
