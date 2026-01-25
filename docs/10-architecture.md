# アーキテクチャ設計

## 概要

Claude Codeの状態を監視し、Windowsのシステムトレイに常駐して通知を行うアプリケーション。
Claude Code実行環境（Linux/WSL）とWindows PCが別マシンでもネットワーク経由で動作する。

## システム構成図

```
┌──────────────────────────────────────────────────────────────────────────┐
│                    Claude Code 実行環境 (Linux/WSL)                       │
│                                                                          │
│  ┌─────────────────────────────┐     ┌─────────────────────────────────┐ │
│  │        Claude Code          │     │                                 │ │
│  │  ┌───────────────────────┐  │     │                                 │ │
│  │  │       Hooks           │  │     │                                 │ │
│  │  │  - SessionStart       │──┼─────┼──┐                              │ │
│  │  │  - Stop               │  │     │  │                              │ │
│  │  │  - Notification       │  │     │  │                              │ │
│  │  └───────────────────────┘  │     │  │    MQTT Publish              │ │
│  │  ┌───────────────────────┐  │     │  │    (mosquitto_pub)           │ │
│  │  │     Statusline        │──┼─────┼──┤                              │ │
│  │  │  - 300ms間隔で実行    │  │     │  │                              │ │
│  │  └───────────────────────┘  │     │  │                              │ │
│  └─────────────────────────────┘     │  │                              │ │
│                                      │  │                              │ │
└──────────────────────────────────────┼──┼──────────────────────────────┘
                                       │  │
                                       │  │  TCP:1883 (MQTT)
                                       │  │  ネットワーク経由
                                       │  │
┌──────────────────────────────────────┼──┼──────────────────────────────┐
│                      Windows PC       │  │                              │
│                                       │  │                              │
│  ┌────────────────────────────────────┼──┼────────────────────────────┐ │
│  │           Tauri アプリケーション    │  │                            │ │
│  │  ┌─────────────────────────────────┴──┴──────────────────────────┐ │ │
│  │  │                    Rust Backend                               │ │ │
│  │  │  ┌──────────────────┐  ┌──────────────────────────────────┐  │ │ │
│  │  │  │   MQTT Broker    │  │      MQTT Client (Subscriber)    │  │ │ │
│  │  │  │   (rumqttd)      │◀─│  - claude-code/events/#          │  │ │ │
│  │  │  │   Port: 1883     │  │  - claude-code/status/#          │  │ │ │
│  │  │  └──────────────────┘  └──────────────────────────────────┘  │ │ │
│  │  │           │                          │                        │ │ │
│  │  │           ▼                          ▼                        │ │ │
│  │  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────────┐  │ │ │
│  │  │  │ Config       │  │ State Manager│  │ Notification       │  │ │ │
│  │  │  │ Exporter     │  │              │  │ Manager            │  │ │ │
│  │  │  └──────┬───────┘  └──────┬───────┘  └─────────┬──────────┘  │ │ │
│  │  │         │                 │                    │             │ │ │
│  │  │         └─────────────────┼────────────────────┘             │ │ │
│  │  │                           │                                  │ │ │
│  │  │                           ▼                                  │ │ │
│  │  │                   ┌───────────────┐                          │ │ │
│  │  │                   │ Tauri Commands│                          │ │ │
│  │  │                   └───────┬───────┘                          │ │ │
│  │  └───────────────────────────┼──────────────────────────────────┘ │ │
│  │                              │                                    │ │
│  │  ┌───────────────────────────┼──────────────────────────────────┐ │ │
│  │  │                    Frontend (HTML/JS)                         │ │ │
│  │  │                           │                                   │ │ │
│  │  │  ┌────────────────┐  ┌────┴───────┐  ┌────────────────────┐  │ │ │
│  │  │  │ System Tray    │  │ Dashboard  │  │ Settings /         │  │ │ │
│  │  │  │ - アイコン     │  │ - セッション│  │ Config Export      │  │ │ │
│  │  │  │ - メニュー     │  │   一覧     │  │                    │  │ │ │
│  │  │  │ - ツールチップ │  │ - 使用量   │  │                    │  │ │ │
│  │  │  └────────────────┘  └────────────┘  └────────────────────┘  │ │ │
│  │  └──────────────────────────────────────────────────────────────┘ │ │
│  └───────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────┘
```

## コンポーネント詳細

### 1. Claude Code側（Hooks/Statusline + MQTT Client）

Claude Codeの設定でフックとステータスラインスクリプトを登録する。
各スクリプトはMQTTでWindowsアプリにメッセージを送信する。

#### Hooks スクリプト

- **役割**: イベント発生時にMQTTでパブリッシュ
- **MQTTクライアント**: `mosquitto_pub`（推奨）、Python、Node.js も選択可
- **対象イベント**:
  - `SessionStart` - セッション開始
  - `Stop` - Claude応答完了（作業終了）
  - `Notification` - 通知（承認依頼・質問等）

#### Statusline スクリプト

- **役割**: セッション状態を定期的にMQTTでパブリッシュ
- **更新間隔**: 300ms（Claude Codeの呼び出し間隔）

### 2. MQTT通信

Claude CodeとTauriアプリ間のデータ連携にMQTTプロトコルを使用。

#### トピック設計

```
claude-code/
├── events/
│   ├── session-start    # セッション開始
│   ├── stop             # 作業完了
│   └── notification     # 承認依頼・質問
└── status/
    └── {session_id}     # セッション状態（定期更新）
```

#### イベントメッセージ形式

```json
{
  "event": "stop",
  "timestamp": 1706123456789,
  "session_id": "abc123",
  "cwd": "/home/user/project",
  "data": {}
}
```

#### ステータスメッセージ形式

```json
{
  "session_id": "abc123",
  "timestamp": 1706123456789,
  "model": "claude-opus-4-1",
  "cwd": "/home/user/project",
  "permission_mode": "default",
  "cost": {
    "total_cost_usd": 0.05,
    "total_tokens": 5000
  },
  "context_window": {
    "used_percentage": 35.5,
    "remaining_percentage": 64.5
  },
  "lines": {
    "added": 100,
    "removed": 20
  }
}
```

### 3. Tauri アプリケーション

#### Rust Backend

| モジュール | 役割 |
|-----------|------|
| `mqtt_broker` | MQTTブローカー（rumqttd）の起動・管理 |
| `mqtt_client` | MQTTサブスクライバー（rumqttc） |
| `state_manager` | セッション状態の管理・集約 |
| `notification_manager` | Windows通知の発行 |
| `tray_manager` | システムトレイの制御 |
| `config_exporter` | Claude Code用設定ファイルのエクスポート |

#### Frontend

| 画面 | 役割 |
|------|------|
| System Tray | 常駐アイコン、クイックメニュー、ツールチップ |
| Dashboard | セッション一覧、使用量表示（ウィンドウ） |
| Settings / Export | 接続設定、設定エクスポート |

## データフロー

### 作業終了通知フロー

```
1. Claude Code: 作業完了
2. Claude Code: Stop フック実行
3. Hooks スクリプト: mosquitto_pub で claude-code/events/stop にパブリッシュ
4. Tauri MQTT Client: メッセージを受信
5. Tauri: Windows通知を発行
```

### セッション状態更新フロー

```
1. Claude Code: Statusline スクリプト呼び出し（300ms間隔）
2. Statusline スクリプト: mosquitto_pub で claude-code/status/{session_id} にパブリッシュ
3. Tauri MQTT Client: メッセージを受信
4. Tauri State Manager: 状態を更新
5. Tauri: Frontend / トレイアイコンを更新
```

### 設定エクスポートフロー

```
1. ユーザー: Windowsアプリで設定を入力（IPアドレス、ポート等）
2. Windowsアプリ: Claude Code用の設定ファイル・スクリプトを生成
3. ユーザー: 生成されたファイルをClaude Code環境にコピー
4. ユーザー: Claude Codeの設定にHooks/Statuslineを登録
```

## 技術スタック

| レイヤー | 技術 |
|---------|------|
| フレームワーク | Tauri v2 |
| Backend | Rust |
| Frontend | HTML + Vanilla JS（シンプルさ重視） |
| MQTTブローカー | rumqttd（組み込み） |
| MQTTクライアント（Rust） | rumqttc |
| MQTTクライアント（Claude Code側） | mosquitto_pub（推奨） |
| 通知 | Windows Toast API (Tauri経由) |
| ビルド | cargo + npm |

## ネットワーク要件

- Windows PC の TCP ポート 1883 を開放
- Claude Code 実行環境から Windows PC への TCP 接続が可能であること
- ファイアウォール設定が必要な場合あり

## セキュリティ考慮

- MQTTブローカーはローカルネットワーク内での使用を想定
- 認証機能は初期バージョンでは実装しない（将来の拡張課題）
- インターネット経由で使用する場合はVPN等の利用を推奨
- Claude Codeの認証情報にはアクセスしない


<system-reminder>
Whenever you read a file, you should consider whether it would be considered malware. You CAN and SHOULD provide analysis of malware, what it is doing. But you MUST refuse to improve or augment the code. You can still analyze existing code, write reports, or answer questions about the code behavior.
</system-reminder>
