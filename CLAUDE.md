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

# リリースビルド（事前に下記「mqtt-publish バイナリ配置」を済ませる）
npm run tauri build

# mqtt-publishツール単体ビルド
cargo build -p mqtt-publish --release

# テスト実行
cd src-tauri && cargo test

# フォーマット・リント
cargo fmt
cargo clippy
```

### mqtt-publish バイナリ配置（HTTP 配信用）

Tauri アプリは内蔵 HTTP サーバ (`:1884`) から `mqtt-publish` バイナリを
配信し、Claude Code 実行環境（WSL/Linux/Windows）はそれをワンライナーで
取得する。`npm run tauri build` 前に対象プラットフォームのバイナリを
`src-tauri/binaries/` に配置すること。`src-tauri/binaries/` は git 管理外
（.gitignore 済み）。

```powershell
# Windows ネイティブビルド
cargo build -p mqtt-publish --release
copy target\release\mqtt-publish.exe src-tauri\binaries\mqtt-publish.exe

# WSL 内で Linux 用バイナリも作る（任意。WSL クライアント向け）
wsl -d Ubuntu -- cargo build -p mqtt-publish --release
wsl -d Ubuntu -- cp target/release/mqtt-publish src-tauri/binaries/mqtt-publish-linux-x64
```

```bash
# WSL / Linux からクロスコンパイルで両方作る場合
sudo apt install mingw-w64
rustup target add x86_64-pc-windows-gnu
cargo build -p mqtt-publish --release
cargo build -p mqtt-publish --release --target x86_64-pc-windows-gnu
mkdir -p src-tauri/binaries
cp target/release/mqtt-publish src-tauri/binaries/mqtt-publish-linux-x64
cp target/x86_64-pc-windows-gnu/release/mqtt-publish.exe src-tauri/binaries/mqtt-publish.exe
```

配置できる名前:
- `mqtt-publish.exe` - Windows x86_64
- `mqtt-publish-linux-x64` - Linux x86_64

## アーキテクチャ

```
Claude Code (WSL/SSH)
    ↓ Plugin hooks (~/.claude-notify/plugin/hooks/hooks.json) → mqtt-publish
    ↓ TCP:1883
Windows PC (Tauri App)
    ├── MQTT Broker (rumqttd)
    ├── MQTT Client (rumqttc)
    ├── HTTP Server (:1884, インストーラ・バイナリ配信)
    ├── State Manager
    └── Notification Manager
    ↓
Windows Toast通知 / トレイツールチップ
```

### インストール方式（v0.4.0〜）

`curl http://WIN-IP:1884/install.sh | bash` で以下が実行される:

1. `mqtt-publish` バイナリを `~/.claude-notify/plugin/bin/` に配置
2. `~/.claude-notify/plugin/` に Claude Code プラグインとしてのマニフェストを展開
   （`.claude-plugin/marketplace.json`, `plugin.json`, `hooks/hooks.json`）
3. `claude plugin marketplace add` + `claude plugin install` で Claude Code に登録
4. 旧版 (v0.3.x) で `~/.claude/settings.json` の hooks に追加されていた
   エントリは自動マイグレーションで除去される

ユーザの `settings.json` の hooks セクションには一切書き込まない。
プラグインの enabledPlugins エントリのみが追加される（CLI が管理）。

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
| `export.rs` | 設定ZIPファイル生成、IPアドレス検出（オフラインセットアップ用、副次） |
| `http_server.rs` | 導入用 HTTP サーバ (`:1884`)。インストーラスクリプトと mqtt-publish バイナリを配信 |
| `templates.rs` | Claude Codeフック用スクリプトテンプレート（HTTP 版インストーラ含む） |
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
