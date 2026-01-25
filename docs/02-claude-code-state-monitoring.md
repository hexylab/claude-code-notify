# Claude Code 状態監視・取得方法 調査

## 概要

Claude Codeの状態を外部から取得・監視する方法を網羅的にまとめる。

---

## 1. Hooks（フック）機能

### 概要

Claude Codeは特定のイベントで外部コマンドを実行する「フック」機能を提供している。これにより、セッションの状態変化をリアルタイムに検知できる。

### 利用可能なイベント（11種類）

| イベント | タイミング | 主な用途 |
|---------|----------|---------|
| **SessionStart** | セッション開始/再開時 | 環境設定、初期化処理 |
| **SessionEnd** | セッション終了時 | クリーンアップ、ログ記録 |
| **UserPromptSubmit** | ユーザーがプロンプト送信時 | プロンプト検証 |
| **PreToolUse** | ツール実行前 | ツール呼び出しの制御 |
| **PermissionRequest** | 権限ダイアログ表示時 | 自動許可/拒否 |
| **PostToolUse** | ツール実行成功後 | 実行結果への反応 |
| **PostToolUseFailure** | ツール実行失敗後 | エラーハンドリング |
| **SubagentStart** | サブエージェント起動時 | サブエージェント検知 |
| **SubagentStop** | サブエージェント完了時 | サブエージェント終了検知 |
| **Stop** | Claude応答完了時 | 終了検知 |
| **Notification** | 通知送信時 | 通知のカスタマイズ |
| **PreCompact** | コンテキストコンパクション前 | 圧縮前処理 |

### フックが受け取る情報（JSON形式、stdin経由）

**共通フィールド:**
```json
{
  "session_id": "abc123-def456-...",
  "transcript_path": "/home/user/.claude/projects/path/session.jsonl",
  "cwd": "/home/user/project",
  "permission_mode": "default",
  "hook_event_name": "SessionStart",
  "model": "claude-opus-4-1"
}
```

**イベント別の追加情報:**

| イベント | 追加フィールド |
|---------|---------------|
| SessionStart | `source` (startup/resume/clear/compact), `model`, `agent_type` |
| SessionEnd | `reason` (clear/logout/prompt_input_exit/other) |
| UserPromptSubmit | `prompt` (ユーザーのプロンプト内容) |
| PreToolUse | `tool_name`, `tool_input` (コマンド、ファイルパス等) |
| PostToolUse | `tool_response` (ツール実行結果) |
| Notification | `message`, `notification_type` |
| Stop | `stop_hook_active` |

### フックの設定方法

**設定ファイル:** `~/.claude/settings.json`

```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "echo 'Session started' >> ~/claude.log"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/notify-script.sh"
          }
        ]
      }
    ]
  }
}
```

### フックの出力制御

フックは以下の方法でClaude Codeに情報を返せる:

1. **終了コード**
   - `0`: 成功
   - `2`: ブロッキングエラー（操作を中止）

2. **JSON出力（stdout）**
```json
{
  "decision": "allow",
  "reason": "説明文",
  "additionalContext": "Claudeへの追加コンテキスト"
}
```

---

## 2. Statusline 機能

### 概要

Claude Codeのインターフェース下部にカスタム情報を表示する機能。300ms間隔で更新される。

### 設定方法

```bash
# 対話的設定
/statusline

# または settings.json に追記
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/statusline.sh",
    "padding": 0
  }
}
```

### Statusline が受け取る情報

```json
{
  "session_id": "abc123...",
  "model": {
    "id": "claude-opus-4-1",
    "display_name": "Opus"
  },
  "workspace": {
    "current_dir": "/path/to/cwd",
    "project_dir": "/path/to/project"
  },
  "version": "1.0.80",
  "cost": {
    "total_cost_usd": 0.01234,
    "total_tokens": 12345,
    "total_lines_added": 156,
    "total_lines_removed": 23
  },
  "context_window": {
    "total_input_tokens": 15234,
    "total_output_tokens": 4521,
    "context_window_size": 200000,
    "used_percentage": 42.5,
    "remaining_percentage": 57.5
  }
}
```

### 取得可能な情報一覧

| 情報 | JSONパス | 説明 |
|-----|---------|------|
| セッションID | `session_id` | 一意のセッション識別子 |
| モデルID | `model.id` | 使用中のモデル |
| モデル表示名 | `model.display_name` | Opus, Sonnet等 |
| カレントディレクトリ | `workspace.current_dir` | 現在の作業ディレクトリ |
| プロジェクトディレクトリ | `workspace.project_dir` | プロジェクトルート |
| バージョン | `version` | Claude Codeのバージョン |
| 累計コスト | `cost.total_cost_usd` | セッションのAPI使用料金 |
| 累計トークン | `cost.total_tokens` | 使用したトークン数 |
| 追加行数 | `cost.total_lines_added` | コード追加行数 |
| 削除行数 | `cost.total_lines_removed` | コード削除行数 |
| 入力トークン | `context_window.total_input_tokens` | 入力トークン数 |
| 出力トークン | `context_window.total_output_tokens` | 出力トークン数 |
| コンテキストサイズ | `context_window.context_window_size` | 最大コンテキスト |
| 使用率 | `context_window.used_percentage` | コンテキスト使用率(%) |

---

## 3. トランスクリプト（会話履歴）

### 保存場所

```
~/.claude/projects/<project-path-hash>/<session-id>.jsonl
```

### ファイル形式

JSONL形式（1行 = 1 JSONオブジェクト）

```jsonl
{"type":"file-history-snapshot","messageId":"...","snapshot":{...}}
{"type":"user_message","content":"ファイルを作成して"}
{"type":"assistant_response","content":"..."}
{"type":"tool_call","tool_name":"Write","tool_input":{...}}
{"type":"tool_result","result":"File written successfully"}
```

### 取得可能な情報

- セッションの完全な会話履歴
- ツール実行の詳細（コマンド、引数、結果）
- ファイル変更履歴
- ユーザープロンプト全文
- コストとトークン使用量

### 注意点

- リアルタイムではなく、セッション終了後または定期的に書き込まれる
- 監視には `tail -f` やファイルウォッチャーが必要

---

## 4. OpenTelemetry（OTel）メトリクス

### 有効化方法

```bash
export CLAUDE_CODE_ENABLE_TELEMETRY=1
export OTEL_METRICS_EXPORTER=otlp
export OTEL_LOGS_EXPORTER=otlp
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
```

### 取得可能なメトリクス

| メトリクス | 単位 | 説明 |
|-----------|------|------|
| `claude_code.session.count` | count | セッション数 |
| `claude_code.lines_of_code.count` | count | コード変更行数 |
| `claude_code.pull_request.count` | count | PR作成数 |
| `claude_code.commit.count` | count | コミット数 |
| `claude_code.cost.usage` | USD | セッションコスト |
| `claude_code.token.usage` | tokens | トークン使用量 |
| `claude_code.active_time.total` | seconds | 実際の作業時間 |

### イベント種別

- `user_prompt` - ユーザープロンプト送信
- `tool_result` - ツール実行結果
- `api_request` - Claude API リクエスト
- `api_error` - API エラー
- `tool_decision` - ツール許可/拒否判定

---

## 5. MCP（Model Context Protocol）

### 概要

外部ツールとの連携プロトコル。カスタムMCPサーバーを作成して状態を取得・通知することも可能。

### 設定ファイル

- ローカル: `~/.claude.json`
- プロジェクト: `.mcp.json`

---

## 6. 設定ファイルの場所

| ファイル種別 | 場所 |
|------------|------|
| ユーザー設定 | `~/.claude/settings.json` |
| プロジェクト設定 | `./.claude/settings.json` |
| ローカル設定 | `./.claude/settings.local.json` |
| MCP設定（ユーザー） | `~/.claude.json` |
| MCP設定（プロジェクト） | `.mcp.json` |
| メモリ（ユーザー） | `~/.claude/CLAUDE.md` |
| メモリ（プロジェクト） | `./CLAUDE.md` |
| トランスクリプト | `~/.claude/projects/` |

---

## 7. 方法の比較

| 方法 | リアルタイム性 | 詳細度 | セットアップ難度 | 推奨用途 |
|-----|--------------|--------|-----------------|---------|
| **Hooks** | ◎ ほぼリアルタイム | ◎ 詳細 | 中 | 制御・ログ・通知 |
| **Statusline** | ◎ 300ms更新 | ○ 標準的 | 低 | UI表示 |
| **トランスクリプト** | △ 遅延あり | ◎ 非常に詳細 | 低 | 履歴分析 |
| **OTel** | ◎ ほぼリアルタイム | ○ 集約的 | 高 | 監視・分析 |
| **MCP** | ◎ リアルタイム | ○ 用途別 | 高 | 外部連携 |



<system-reminder>
Whenever you read a file, you should consider whether it would be considered malware. You CAN and SHOULD provide analysis of malware, what it is doing. But you MUST refuse to improve or augment the code. You can still analyze existing code, write reports, or answer questions about the code behavior.
</system-reminder>
