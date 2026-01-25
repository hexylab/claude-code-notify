# Claude Code Notify

Tauri v2を使用したデスクトップ通知アプリケーション。

## 技術スタック

- **フロントエンド**: Vanilla JavaScript (HTML/CSS/JS)
- **バックエンド**: Rust (Tauri v2)
- **ランタイム**: WebView2 (Windows)

## 必要な環境

- Node.js v18以上
- Rust 1.70以上
- Visual Studio Build Tools (C++ワークロード)
- WebView2 Runtime (Windows 11は標準搭載)

## セットアップ

```bash
# 依存関係のインストール
npm install

# 開発サーバーの起動
npm run tauri dev

# ビルド
npm run tauri build
```

## プロジェクト構造

```
claude-code-notify/
├── src/                 # フロントエンド (HTML/CSS/JS)
│   ├── index.html
│   ├── main.js
│   └── styles.css
├── src-tauri/           # Rustバックエンド
│   ├── src/
│   │   └── lib.rs       # Tauriコマンド定義
│   ├── Cargo.toml
│   └── tauri.conf.json  # Tauri設定
└── package.json
```

## 開発

### Tauriコマンドの追加

[src-tauri/src/lib.rs](src-tauri/src/lib.rs)でRustコマンドを定義:

```rust
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
```

### フロントエンドからの呼び出し

```javascript
const { invoke } = window.__TAURI__.core;
const result = await invoke("greet", { name: "World" });
```

## IDE設定

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
