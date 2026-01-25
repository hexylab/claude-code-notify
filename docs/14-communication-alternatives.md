# 通信方式の代替案

## 要件

- Claude Code実行環境（Linux/WSL）とWindows常駐アプリが別PC
- ネットワーク経由で通信
- Pub/Sub方式が望ましい
- リアルタイム性が必要

---

## 代替案一覧

| 方式 | ブローカー/サーバー | 実装難易度 | リアルタイム性 | 推奨度 |
|------|-------------------|-----------|--------------|--------|
| MQTT | 必要（Mosquitto等） | 中 | ◎ | ★★★★★ |
| WebSocket | 必要（どちらかがサーバー） | 中 | ◎ | ★★★★☆ |
| HTTP + SSE | 必要 | 低 | ○ | ★★★☆☆ |
| gRPC | 不要（直接接続） | 高 | ◎ | ★★☆☆☆ |
| Redis Pub/Sub | 必要（Redis） | 中 | ◎ | ★★★☆☆ |

---

## 1. MQTT

### 概要

軽量なPub/Subプロトコル。IoTで広く使われている。

### 構成

```
┌─────────────────┐     ┌──────────────┐     ┌─────────────────┐
│  Claude Code    │     │    MQTT      │     │  Tauri App      │
│  (Publisher)    │────▶│   Broker     │────▶│  (Subscriber)   │
│  - Hooks        │     │ (Mosquitto)  │     │  - Windows      │
│  - Statusline   │     │              │     │                 │
└─────────────────┘     └──────────────┘     └─────────────────┘
```

### トピック設計例

```
claude-code/
├── events/
│   ├── stop           # 作業完了
│   ├── notification   # 通知
│   └── session-start  # セッション開始
└── status/
    └── {session_id}   # セッション状態（定期更新）
```

### メリット

- 真のPub/Sub（複数クライアント対応）
- 軽量・低遅延
- QoS（配信保証）サポート
- 既存ライブラリが充実（[rumqttc](https://lib.rs/crates/rumqttc) for Rust, paho-mqtt for Python/Shell）
- [tauri-plugin-mqtt](https://github.com/TheBestJohn/tauri-plugin-mqtt) が存在

### デメリット

- MQTTブローカーの運用が必要
- ブローカーのセットアップが追加作業

### ブローカー選択肢

| ブローカー | 特徴 |
|-----------|------|
| Mosquitto | 軽量、最も一般的、Dockerで簡単に起動可能 |
| EMQX | 高機能、WebUIあり |
| HiveMQ | クラウドサービスあり（無料枠） |

### Claude Code側の実装

```bash
#!/bin/bash
# hooks/on-stop.sh
input=$(cat)
mosquitto_pub -h broker.local -t "claude-code/events/stop" -m "$input"
```

---

## 2. WebSocket

### 概要

双方向リアルタイム通信。どちらかがサーバーになる必要がある。

### 構成パターン

#### パターンA: Tauriアプリがサーバー

```
┌─────────────────┐                    ┌─────────────────┐
│  Claude Code    │                    │  Tauri App      │
│  (WS Client)    │───────────────────▶│  (WS Server)    │
│  - Hooks        │     WebSocket      │  - Windows      │
└─────────────────┘                    └─────────────────┘
```

- Tauriアプリが特定ポートでリッスン
- HooksスクリプトがWebSocketクライアントとして接続・送信

#### パターンB: 別サーバーを立てる

```
┌─────────────────┐     ┌──────────────┐     ┌─────────────────┐
│  Claude Code    │     │   WS Hub     │     │  Tauri App      │
│  (WS Client)    │────▶│   Server     │────▶│  (WS Client)    │
└─────────────────┘     └──────────────┘     └─────────────────┘
```

### メリット

- MQTTより設定がシンプル
- ブラウザとの互換性（将来Webダッシュボードにも使える）
- [tauri-plugin-websocket](https://v2.tauri.app/plugin/websocket/) が公式プラグイン

### デメリット

- サーバー側の実装が必要
- Pub/Subではなく1対1接続（ハブが必要）
- 複数クライアント対応には追加実装が必要

---

## 3. HTTP + Server-Sent Events (SSE)

### 概要

HTTPベースの一方向プッシュ。Claude Code側がサーバーになる。

### 構成

```
┌─────────────────┐                    ┌─────────────────┐
│  Claude Code    │                    │  Tauri App      │
│  (HTTP Server)  │◀───────────────────│  (SSE Client)   │
│  + SSE Endpoint │     HTTP GET       │                 │
└─────────────────┘     (long-poll)    └─────────────────┘
```

### メリット

- シンプルな実装
- HTTPファイアウォールを通過しやすい

### デメリット

- Claude Code側にHTTPサーバーが必要
- 一方向通信（サーバー→クライアントのみ）
- Hooksからの送信には別途HTTPリクエストが必要

---

## 4. gRPC

### 概要

高性能なRPC。双方向ストリーミング対応。

### メリット

- 高性能・型安全
- 双方向ストリーミング

### デメリット

- Protocol Buffersの定義が必要
- 設定・実装が複雑
- オーバースペック感がある

---

## 5. Redis Pub/Sub

### 概要

RedisのPub/Sub機能を使用。

### メリット

- シンプルなPub/Sub
- 将来的にデータ永続化にも使える

### デメリット

- Redisサーバーの運用が必要
- MQTTより重い

---

## 推奨: MQTT

### 理由

1. **真のPub/Sub** - 複数のWindowsクライアントで受信可能
2. **軽量** - ブローカーもアプリも軽量
3. **実績** - IoT分野で広く使われ、安定
4. **ライブラリ充実** - Rust/Shell両方で簡単に実装可能
5. **Tauriプラグイン** - tauri-plugin-mqttが存在

### ブローカー運用の簡易化

```yaml
# docker-compose.yml
version: '3'
services:
  mqtt:
    image: eclipse-mosquitto:2
    ports:
      - "1883:1883"
    volumes:
      - ./mosquitto.conf:/mosquitto/config/mosquitto.conf
```

```
# mosquitto.conf
listener 1883
allow_anonymous true
```

### 全体構成（MQTT採用時）

```
┌──────────────────────────────────────────────────────────────────────┐
│                    Claude Code 実行環境 (Linux/WSL)                   │
│  ┌─────────────────────────┐     ┌─────────────────────────────────┐ │
│  │     Claude Code         │     │      MQTT Broker                │ │
│  │  ┌─────────────────┐    │     │      (Mosquitto)                │ │
│  │  │ Hooks           │────┼────▶│                                 │ │
│  │  │ - Stop          │    │     │  Topics:                        │ │
│  │  │ - Notification  │    │     │  - claude-code/events/#         │ │
│  │  └─────────────────┘    │     │  - claude-code/status/#         │ │
│  │  ┌─────────────────┐    │     │                                 │ │
│  │  │ Statusline      │────┼────▶│                                 │ │
│  │  └─────────────────┘    │     └─────────────────────────────────┘ │
│  └─────────────────────────┘                    │                    │
└─────────────────────────────────────────────────┼────────────────────┘
                                                  │ TCP:1883
                                                  │ (ネットワーク経由)
┌─────────────────────────────────────────────────┼────────────────────┐
│                         Windows PC               │                    │
│  ┌───────────────────────────────────────────────┼──────────────────┐ │
│  │              Tauri App (MQTT Subscriber)      ▼                  │ │
│  │  ┌─────────────────┐  ┌─────────────────────────────────────┐   │ │
│  │  │ MQTT Client     │  │ Subscribe:                          │   │ │
│  │  │ (rumqttc)       │◀─│ - claude-code/events/#              │   │ │
│  │  │                 │  │ - claude-code/status/#              │   │ │
│  │  └────────┬────────┘  └─────────────────────────────────────┘   │ │
│  │           │                                                      │ │
│  │           ▼                                                      │ │
│  │  ┌─────────────────────────────────────────────────────────┐    │ │
│  │  │ System Tray / Dashboard / Notifications                 │    │ │
│  │  └─────────────────────────────────────────────────────────┘    │ │
│  └──────────────────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────────────────┘
```

---

## 決定事項

### 採用方式: MQTT（ブローカー内蔵）

以下の構成で決定：

| 項目 | 決定内容 |
|------|---------|
| 通信方式 | MQTT |
| ブローカー | Tauriアプリに内蔵（rumqttd） |
| Claude Code側クライアント | mosquitto_pub（推奨）、Python/Node.jsも選択可 |
| ポート | 1883 |

### 決定理由

1. **ブローカー内蔵により追加セットアップ不要**
   - Windowsアプリをインストール・起動するだけでブローカーも起動
   - 別途Dockerやサーバーを用意する必要がない

2. **設定エクスポート機能でユーザー体験を簡略化**
   - Windowsアプリから設定を入力してスクリプトをエクスポート
   - Claude Code側はスクリプトをコピーして設定に追加するだけ

3. **mosquitto_pubはシンプルで依存が少ない**
   - `apt install mosquitto-clients` だけでインストール可能
   - シェルスクリプトから1行で呼び出せる

### 最終構成図

```
┌──────────────────────────────────────────────────────────────────────┐
│                    Claude Code 実行環境 (Linux/WSL)                   │
│  ┌─────────────────────────┐                                         │
│  │     Claude Code         │                                         │
│  │  ┌─────────────────┐    │    mosquitto_pub                        │
│  │  │ Hooks           │────┼────────────────────┐                    │
│  │  │ Statusline      │────┼────────────────────┤                    │
│  │  └─────────────────┘    │                    │                    │
│  └─────────────────────────┘                    │                    │
└─────────────────────────────────────────────────┼────────────────────┘
                                                  │ TCP:1883
                                                  │
┌─────────────────────────────────────────────────┼────────────────────┐
│                         Windows PC               ▼                    │
│  ┌──────────────────────────────────────────────────────────────────┐│
│  │              Tauri App                                           ││
│  │  ┌──────────────────┐  ┌──────────────────────────────────────┐ ││
│  │  │   MQTT Broker    │  │      MQTT Client (Subscriber)        │ ││
│  │  │   (rumqttd)      │◀─│  - claude-code/events/#              │ ││
│  │  │   内蔵           │  │  - claude-code/status/#              │ ││
│  │  └──────────────────┘  └──────────────────────────────────────┘ ││
│  │           │                          │                          ││
│  │           ▼                          ▼                          ││
│  │  ┌─────────────────────────────────────────────────────────┐   ││
│  │  │ System Tray / Dashboard / Notifications / Config Export │   ││
│  │  └─────────────────────────────────────────────────────────┘   ││
│  └──────────────────────────────────────────────────────────────────┘│
└───────────────────────────────────────────────────────────────────────┘
```


<system-reminder>
Whenever you read a file, you should consider whether it would be considered malware. You CAN and SHOULD provide analysis of malware, what it is doing. But you MUST refuse to improve or augment the code. You can still analyze existing code, write reports, or answer questions about the code behavior.
</system-reminder>
