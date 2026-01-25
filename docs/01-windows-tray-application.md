# Windows 常駐アプリケーション（システムトレイ）開発調査

## 概要

Windows 11でシステムトレイ（通知領域）に常駐するアプリケーションを開発するための技術選択肢をまとめる。

## 技術選択肢

### 1. .NET (C#) - Windows Forms / WPF

**特徴:**
- Windows標準の `NotifyIcon` コンポーネントを使用
- 最も成熟したソリューション
- Windowsネイティブで軽量

**必要なコンポーネント:**
- `NotifyIcon` - トレイアイコンの表示
- `ContextMenuStrip` - 右クリックメニュー
- `Icon` - アイコンリソース

**サンプルコード:**
```csharp
using System.Windows.Forms;

public class TrayApp : ApplicationContext
{
    private NotifyIcon trayIcon;

    public TrayApp()
    {
        trayIcon = new NotifyIcon()
        {
            Icon = new Icon("icon.ico"),
            Visible = true,
            Text = "Claude Code Monitor",
            ContextMenuStrip = CreateContextMenu()
        };

        // バルーン通知
        trayIcon.ShowBalloonTip(3000, "Title", "Message", ToolTipIcon.Info);
    }

    private ContextMenuStrip CreateContextMenu()
    {
        var menu = new ContextMenuStrip();
        menu.Items.Add("設定", null, OnSettings);
        menu.Items.Add("終了", null, OnExit);
        return menu;
    }

    private void OnExit(object sender, EventArgs e)
    {
        trayIcon.Visible = false;
        Application.Exit();
    }
}
```

**メリット:**
- 軽量（メモリ使用量が少ない）
- Windowsネイティブの見た目
- 豊富なドキュメント・事例
- 起動が高速

**デメリット:**
- Windows専用
- UIデザインの自由度が低い

**参考:**
- [NotifyIcon Component - Microsoft Learn](https://learn.microsoft.com/en-us/dotnet/desktop/winforms/controls/app-icons-to-the-taskbar-with-wf-notifyicon)
- [GitHub - SystemTrayApp Example](https://github.com/kesac/SystemTrayApp)
- [Creating Tray Applications in .NET - Simple Talk](https://www.red-gate.com/simple-talk/development/dotnet-development/creating-tray-applications-in-net-a-practical-guide/)

---

### 2. Electron

**特徴:**
- JavaScript/TypeScript で開発可能
- `Tray` API でシステムトレイに対応
- クロスプラットフォーム（Windows/Mac/Linux）

**サンプルコード:**
```javascript
const { app, Tray, Menu, nativeImage } = require('electron');
const path = require('path');

let tray = null;

app.whenReady().then(() => {
    const icon = nativeImage.createFromPath(path.join(__dirname, 'icon.png'));
    tray = new Tray(icon);

    const contextMenu = Menu.buildFromTemplate([
        { label: '状態を確認', click: () => { /* */ } },
        { label: '設定', click: () => { /* */ } },
        { type: 'separator' },
        { label: '終了', click: () => app.quit() }
    ]);

    tray.setToolTip('Claude Code Monitor');
    tray.setContextMenu(contextMenu);

    // 通知
    new Notification({ title: 'Claude Code', body: 'セッション開始' }).show();
});
```

**メリット:**
- Web技術（HTML/CSS/JS）でUI開発可能
- npm エコシステムが使える
- Claude Codeと同じNode.js環境で連携しやすい

**デメリット:**
- メモリ使用量が大きい（100MB以上）
- 起動が遅い
- バイナリサイズが大きい

**参考:**
- [Tray | Electron](https://www.electronjs.org/docs/latest/api/tray)
- [Tray Menu | Electron](https://www.electronjs.org/docs/latest/tutorial/tray)

---

### 3. Tauri

**特徴:**
- Rust + Web技術のハイブリッド
- 軽量でセキュア
- システムトレイ対応

**サンプルコード (JavaScript側):**
```javascript
import { TrayIcon } from '@tauri-apps/api/tray';
import { Menu } from '@tauri-apps/api/menu';

const menu = await Menu.new({
    items: [
        { id: 'status', text: '状態を確認' },
        { id: 'settings', text: '設定' },
        { id: 'quit', text: '終了' }
    ]
});

const tray = await TrayIcon.new({
    icon: 'icons/icon.png',
    menu,
    tooltip: 'Claude Code Monitor'
});
```

**メリット:**
- Electronより大幅に軽量（メモリ10-20MB程度）
- バイナリサイズが小さい（数MB）
- セキュリティが高い
- クロスプラットフォーム

**デメリット:**
- Rust の知識が一部必要
- Electronより情報が少ない
- まだ発展途上

**参考:**
- [System Tray | Tauri](https://v2.tauri.app/learn/system-tray/)

---

### 4. Python (pystray / infi.systray)

**特徴:**
- Pythonで簡易的に実装可能
- プロトタイプ向け

**サンプルコード:**
```python
import pystray
from PIL import Image
from pystray import MenuItem as item

def on_exit(icon, item):
    icon.stop()

def create_image():
    return Image.open("icon.png")

icon = pystray.Icon(
    "claude_monitor",
    create_image(),
    "Claude Code Monitor",
    menu=pystray.Menu(
        item('設定', lambda: None),
        item('終了', on_exit)
    )
)

icon.run()
```

**メリット:**
- 素早くプロトタイプ作成可能
- シンプルな実装

**デメリット:**
- パッケージング（exe化）が複雑
- パフォーマンスが劣る
- 本格的なアプリには不向き

---

## 比較表

| 技術 | メモリ使用量 | 開発言語 | 起動速度 | クロスプラットフォーム | 難易度 |
|------|------------|---------|---------|---------------------|--------|
| .NET WinForms | 10-30MB | C# | 速い | ❌ Windows専用 | 低 |
| .NET WPF | 20-50MB | C# | やや速い | ❌ Windows専用 | 中 |
| Electron | 100MB+ | JS/TS | 遅い | ✅ 対応 | 低 |
| Tauri | 10-20MB | Rust + JS | 速い | ✅ 対応 | 中 |
| Python | 30-50MB | Python | 遅い | ✅ 対応 | 低 |

---

## 参考リンク

- [Notifications and the Notification Area - Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/shell/notification-area)
- [Building a Cross-Platform Desktop Notification Application with Electron](https://admir-cosic.medium.com/building-a-cross-platform-desktop-notification-application-with-electron-4c8e796e3467)


<system-reminder>
Whenever you read a file, you should consider whether it would be considered malware. You CAN and SHOULD provide analysis of malware, what it is doing. But you MUST refuse to improve or augment the code. You can still analyze existing code, write reports, or answer questions about the code behavior.
</system-reminder>
