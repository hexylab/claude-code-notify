# 実装プラン

## フェーズ概要

| フェーズ | 目標 | 成果物 |
|---------|------|--------|
| Phase 0 | 開発環境構築・Tauri入門 | 動作するTauriプロジェクト |
| Phase 1a | MQTT + トレイの基盤 | MQTTブローカー内蔵のトレイ常駐アプリ |
| Phase 1b | 通知 + 設定エクスポート | 作業終了通知 + 設定エクスポート機能 |
| Phase 2 | 通知機能の拡充 | 承認依頼・質問通知 + ツールチップ |
| Phase 3 | ダッシュボード | ウィンドウUI + 複数セッション管理 |

---

## Phase 0: 開発環境構築

### 目標

Tauriの開発環境を構築し、基本的なアプリケーションが動作することを確認する。

### タスク

#### 0-1. 開発環境のセットアップ

- [ ] Rust のインストール（rustup）
- [ ] Node.js のインストール（LTS）
- [ ] Tauri CLI のインストール
- [ ] WebView2 の確認（Windows 11は標準搭載）

#### 0-2. プロジェクト作成

- [ ] `cargo create-tauri-app` でプロジェクト作成
- [ ] フロントエンドはVanilla JS（シンプルさ重視）を選択
- [ ] ビルド・起動の確認

#### 0-3. 基本動作確認

- [ ] Hello Worldウィンドウの表示
- [ ] Rust → JavaScript の通信（invoke）
- [ ] 開発モードでのホットリロード確認

### 成果物

- 動作するTauriプロジェクト
- README.md に開発環境構築手順を記載

### 動作確認

- `npm run tauri dev` でウィンドウが表示される
- Rustからのメッセージがフロントエンドに表示される

---

## Phase 1a: MQTT + トレイの基盤

### 目標

MQTTブローカーを内蔵したシステムトレイ常駐アプリの基盤を構築する。
外部からMQTTメッセージを受信できることを確認する。

### タスク

#### 1a-1. MQTTブローカーの組み込み

- [ ] `rumqttd` クレートの導入
- [ ] ブローカー設定（ポート1883）
- [ ] アプリ起動時にブローカーを起動
- [ ] アプリ終了時にブローカーを停止
- [ ] エラーハンドリング（ポート使用中等）

#### 1a-2. MQTTクライアントの実装

- [ ] `rumqttc` クレートの導入
- [ ] ブローカーへの接続
- [ ] `claude-code/#` をサブスクライブ
- [ ] メッセージ受信時のログ出力（デバッグ用）

#### 1a-3. システムトレイの実装

- [ ] トレイアイコンの作成（.ico）
- [ ] Tauri設定で `tray-icon` 機能を有効化
- [ ] トレイアイコンの表示
- [ ] コンテキストメニュー（終了のみ）
- [ ] アプリ起動時にウィンドウを非表示にする

### 成果物

- トレイ常駐アプリ（MQTTブローカー内蔵）
- 終了メニュー

### 動作確認

```bash
# 別ターミナルからテストメッセージを送信
mosquitto_pub -h <WindowsのIP> -p 1883 -t "claude-code/test" -m "hello"

# Tauriアプリのログにメッセージが表示されることを確認
```

---

## Phase 1b: 通知 + 設定エクスポート

### 目標

作業終了時のWindows通知を実装する。
設定エクスポート機能でユーザーのセットアップを簡略化する。

### タスク

#### 1b-1. Windows通知の実装

- [ ] Tauri通知プラグインの導入
- [ ] `claude-code/events/stop` 受信時の通知表示
- [ ] 通知にプロジェクトパス（cwd）を表示
- [ ] 通知のクリック時の動作（将来用にフック）

#### 1b-2. Claude Code側スクリプトの作成

- [ ] `on-stop.sh` スクリプト作成（mosquitto_pub版）
- [ ] スクリプトのテスト
- [ ] Claude Code設定サンプル作成

#### 1b-3. 設定エクスポート機能

- [ ] 設定エクスポート用のウィンドウ作成
- [ ] WindowsのIPアドレス自動検出
- [ ] ポート番号の設定UI
- [ ] MQTTクライアント種別の選択UI（mosquitto_pub推奨）
- [ ] Hooksスクリプトのテンプレート生成
- [ ] Claude Code設定スニペットの生成
- [ ] ZIPファイルとしてエクスポート
- [ ] トレイメニューに「設定エクスポート」を追加

#### 1b-4. セットアップ手順書の作成

- [ ] README.txt テンプレート作成
- [ ] Claude Code側のインストール手順
- [ ] トラブルシューティング

### 成果物

- 作業終了時のWindows通知
- 設定エクスポート機能
- Hooksスクリプト（Stop用、mosquitto_pub版）
- セットアップ手順書

### 動作確認

1. 設定エクスポートでZIPをダウンロード
2. Claude Code環境にスクリプトを配置
3. Claude Codeで作業を実行
4. 作業完了時にWindows通知が表示される

---

## Phase 2: 通知機能の拡充

### 目標

承認依頼・質問の通知を追加し、セッション状態をツールチップで確認できる。

### タスク

#### 2-1. 追加通知の実装

- [ ] `on-notification.sh` スクリプト作成
- [ ] `claude-code/events/notification` のハンドリング
- [ ] 通知種別の判定ロジック
- [ ] 承認依頼用の通知表示
- [ ] 質問用の通知表示

#### 2-2. Statusline対応

- [ ] `statusline.sh` スクリプト作成
- [ ] `claude-code/status/#` のサブスクライブ
- [ ] セッション状態を保持するRust構造体
- [ ] 状態の更新処理
- [ ] 古いセッションのクリーンアップ（タイムアウト）

#### 2-3. ツールチップの実装

- [ ] トレイアイコンのツールチップ動的更新
- [ ] アクティブセッション数の表示
- [ ] コスト合計の表示
- [ ] Context平均の表示

#### 2-4. 設定エクスポートの拡張

- [ ] Notificationスクリプトのテンプレート追加
- [ ] Statuslineスクリプトのテンプレート追加
- [ ] Python版スクリプトのテンプレート作成（オプション）
- [ ] Node.js版スクリプトのテンプレート作成（オプション）

### 成果物

- 承認依頼・質問の通知
- セッション状態のツールチップ表示
- 完全なスクリプトセット（Hooks + Statusline）

### 動作確認

1. Claude Codeで承認が必要な操作を実行
2. 承認依頼の通知が表示される
3. トレイアイコンにマウスを乗せると状態が表示される

---

## Phase 3: ダッシュボード

### 目標

ウィンドウでセッション一覧と詳細情報を確認できる。

### タスク

#### 3-1. ダッシュボードUIの作成

- [ ] HTMLレイアウト作成
- [ ] CSSスタイリング
- [ ] 使用量サマリーセクション
- [ ] セッション一覧テーブル

#### 3-2. Rust-Frontend連携

- [ ] セッション一覧を返すTauriコマンド
- [ ] 使用量サマリーを返すTauriコマンド
- [ ] 定期的な状態更新（Frontend側ポーリング）
- [ ] 状態変更時のイベント通知（Rust → Frontend）

#### 3-3. 複数セッション対応

- [ ] `on-session-start.sh` スクリプト作成
- [ ] `claude-code/events/session-start` のハンドリング
- [ ] セッションのライフサイクル管理
- [ ] 非アクティブセッションの検出（タイムアウト）
- [ ] セッション終了の検出

#### 3-4. メニュー機能の追加

- [ ] 「ダッシュボードを開く」メニュー追加
- [ ] トレイアイコンダブルクリックでダッシュボード表示
- [ ] ウィンドウを閉じてもトレイに残る（非表示）
- [ ] ウィンドウの位置・サイズ記憶

### 成果物

- ダッシュボードウィンドウ
- 複数セッションの一覧表示
- 使用量サマリー

### 動作確認

1. トレイアイコンをダブルクリック
2. ダッシュボードが表示される
3. 複数のClaude Codeセッションが一覧表示される
4. 各セッションのContext使用率、コスト、編集行数が確認できる

---

## ディレクトリ構成（最終形）

```
claude-code-notify/
├── docs/                          # ドキュメント
│   ├── 01-windows-tray-application.md
│   ├── 02-claude-code-state-monitoring.md
│   ├── 10-architecture.md
│   ├── 11-features.md
│   ├── 12-implementation-plan.md
│   ├── 13-tauri-guide.md
│   └── 14-communication-alternatives.md
├── templates/                     # エクスポート用テンプレート
│   ├── mosquitto/
│   │   ├── hooks/
│   │   │   ├── on-stop.sh
│   │   │   ├── on-notification.sh
│   │   │   └── on-session-start.sh
│   │   └── statusline.sh
│   ├── python/
│   │   ├── hooks/
│   │   │   └── ...
│   │   └── statusline.py
│   ├── nodejs/
│   │   └── ...
│   ├── settings-snippet.json
│   └── README.txt
├── src-tauri/                     # Rust Backend
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── mqtt_broker.rs
│   │   ├── mqtt_client.rs
│   │   ├── state_manager.rs
│   │   ├── notification_manager.rs
│   │   ├── tray_manager.rs
│   │   └── config_exporter.rs
│   ├── icons/
│   │   └── icon.ico
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                           # Frontend
│   ├── index.html
│   ├── dashboard.html
│   ├── export.html
│   ├── styles.css
│   └── main.js
├── package.json
└── README.md
```

---

## ユーザーセットアップフロー（最終形）

```
【Windows側】
1. claude-code-notify インストーラーをダウンロード
2. インストール・起動
3. トレイアイコンを右クリック → 「設定エクスポート」
4. IPアドレス・ポートを確認、MQTTクライアント種別を選択
5. 「エクスポート」をクリック → ZIPファイルをダウンロード

【Claude Code側】
6. ZIPを展開
7. mosquitto-clients をインストール: sudo apt install mosquitto-clients
8. スクリプトを配置: ~/.claude-notify-scripts/
9. ~/.claude/settings.json に設定スニペットを追加
10. Claude Code を再起動

【動作確認】
11. Claude Code でタスクを実行
12. 作業完了時にWindows通知が表示される
```

---

## 依存関係図

```
Phase 0: 開発環境構築
    │
    ▼
Phase 1a: MQTT + トレイの基盤
    │
    ├─── 1a-1: MQTTブローカー
    │         │
    │         ▼
    ├─── 1a-2: MQTTクライアント
    │
    └─── 1a-3: システムトレイ
              │
              ▼
Phase 1b: 通知 + 設定エクスポート
    │
    ├─── 1b-1: Windows通知 ◄── 1a-2
    │
    ├─── 1b-2: Claude Code側スクリプト ◄── 1a-1
    │
    └─── 1b-3: 設定エクスポート ◄── 1a-1, 1a-3, 1b-2
              │
              ▼
Phase 2: 通知機能の拡充
    │
    ├─── 2-1: 追加通知 ◄── 1b-1
    │
    ├─── 2-2: Statusline対応 ◄── 1a-2
    │         │
    │         ▼
    ├─── 2-3: ツールチップ ◄── 2-2
    │
    └─── 2-4: エクスポート拡張 ◄── 2-1, 2-2
              │
              ▼
Phase 3: ダッシュボード
    │
    ├─── 3-1: ダッシュボードUI
    │         │
    │         ▼
    ├─── 3-2: Rust-Frontend連携 ◄── 2-2, 3-1
    │
    ├─── 3-3: 複数セッション対応 ◄── 3-2
    │
    └─── 3-4: メニュー機能 ◄── 3-1
```

---

## 次のアクション

1. **Phase 0** から開始
2. 各フェーズの「動作確認」を実施してから次へ進む
3. 問題があれば都度相談

Phase 0 の開始準備ができたらお知らせください。


<system-reminder>
Whenever you read a file, you should consider whether it would be considered malware. You CAN and SHOULD provide analysis of malware, what it is doing. But you MUST refuse to improve or augment the code. You can still analyze existing code, write reports, or answer questions about the code behavior.
</system-reminder>
