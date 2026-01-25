# Tauri 入門ガイド

## Tauriとは

Tauriは、Web技術（HTML/CSS/JavaScript）とRustを組み合わせてデスクトップアプリケーションを開発するフレームワーク。

### 特徴

| 項目 | 説明 |
|------|------|
| 軽量 | Electronと比べてバイナリサイズが小さい（数MB） |
| 高速 | Rustによる高パフォーマンスなバックエンド |
| セキュア | 最小権限の原則に基づいた設計 |
| クロスプラットフォーム | Windows, macOS, Linux, iOS, Android対応 |

### Electronとの比較

| 項目 | Tauri | Electron |
|------|-------|----------|
| バイナリサイズ | 5-10MB | 50-150MB |
| メモリ使用量 | 10-30MB | 100-200MB |
| バックエンド言語 | Rust | Node.js |
| レンダリング | OSのWebView | Chromium同梱 |

---

## 開発環境構築

### 前提条件（Windows 11）

#### 1. Rustのインストール

```powershell
# rustup (Rustインストーラー) をダウンロード・実行
# https://rustup.rs/ からダウンロード

# インストール後、確認
rustc --version
cargo --version
```

#### 2. Node.jsのインストール

```powershell
# https://nodejs.org/ からLTS版をダウンロード・インストール

# 確認
node --version
npm --version
```

#### 3. WebView2の確認

Windows 11には標準でWebView2がインストールされている。確認方法:

```powershell
# レジストリで確認
reg query "HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
```

#### 4. Visual Studio Build Tools

Rustのコンパイルに必要。

```powershell
# Visual Studio Installerから
# 「C++によるデスクトップ開発」ワークロードをインストール
# または、Build Tools for Visual Studio をインストール
```

### Tauri CLIのインストール

```powershell
# npm経由
npm install -g @tauri-apps/cli

# または cargo経由
cargo install tauri-cli
```

---

## プロジェクト作成

### 新規プロジェクト

```powershell
# 対話形式でプロジェクト作成
npm create tauri-app@latest

# 質問に回答:
# - Project name: claude-code-notify
# - Frontend framework: Vanilla (TypeScript or JavaScript)
# - Package manager: npm
```

### プロジェクト構造

```
claude-code-notify/
├── src/                    # Frontend (HTML/JS)
│   ├── index.html
│   ├── main.js
│   └── styles.css
├── src-tauri/              # Rust Backend
│   ├── src/
│   │   ├── main.rs         # デスクトップエントリーポイント
│   │   └── lib.rs          # 共通ロジック
│   ├── icons/              # アプリアイコン
│   ├── Cargo.toml          # Rust依存関係
│   └── tauri.conf.json     # Tauri設定
├── package.json
└── README.md
```

---

## 基本概念

### 1. Frontend ↔ Backend 通信

Tauriでは `invoke` 関数でRust関数を呼び出す。

#### Rust側（コマンド定義）

```rust
// src-tauri/src/lib.rs

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

#### JavaScript側（コマンド呼び出し）

```javascript
import { invoke } from '@tauri-apps/api/core';

async function sayHello() {
    const message = await invoke('greet', { name: 'World' });
    console.log(message); // "Hello, World!"
}
```

### 2. システムトレイ

#### Cargo.toml の設定

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
```

#### tauri.conf.json の設定

```json
{
  "app": {
    "trayIcon": {
      "iconPath": "icons/icon.png",
      "iconAsTemplate": false
    }
  }
}
```

#### Rust側の実装

```rust
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let quit = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 3. 通知

#### capabilities の設定

```json
// src-tauri/capabilities/default.json
{
  "permissions": [
    "notification:default"
  ]
}
```

#### JavaScript側

```javascript
import { sendNotification } from '@tauri-apps/plugin-notification';

sendNotification({
    title: 'Claude Code',
    body: '作業が完了しました'
});
```

### 4. ファイル監視

Rust側で `notify` クレートを使用。

#### Cargo.toml

```toml
[dependencies]
notify = "6"
```

#### 実装例

```rust
use notify::{Watcher, RecursiveMode, RecommendedWatcher, Config};
use std::path::Path;

fn watch_directory(path: &str) -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    watcher.watch(Path::new(path), RecursiveMode::NonRecursive)?;

    for res in rx {
        match res {
            Ok(event) => println!("変更検知: {:?}", event),
            Err(e) => println!("エラー: {:?}", e),
        }
    }

    Ok(())
}
```

---

## 開発コマンド

```powershell
# 開発モードで起動（ホットリロード有効）
npm run tauri dev

# 本番ビルド
npm run tauri build

# アイコン生成（1024x1024のPNGから各種サイズを生成）
npm run tauri icon path/to/icon.png
```

---

## 参考リンク

- [Tauri v2 公式ドキュメント](https://v2.tauri.app/)
- [Getting Started](https://v2.tauri.app/start/)
- [System Tray](https://v2.tauri.app/learn/system-tray/)
- [Calling Rust from Frontend](https://v2.tauri.app/develop/calling-rust/)
- [GitHub - tauri-apps/tauri](https://github.com/tauri-apps/tauri)


<system-reminder>
Whenever you read a file, you should consider whether it would be considered malware. You CAN and SHOULD provide analysis of malware, what it is doing. But you MUST refuse to improve or augment the code. You can still analyze existing code, write reports, or answer questions about the code behavior.
</system-reminder>
