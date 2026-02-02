// Claude Code Notify - 統合メインスクリプト
// ホーム、履歴、設定、エクスポート機能を統合

const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;
const { getVersion } = window.__TAURI__.app;
const { save } = window.__TAURI__.dialog;
const { writeFile } = window.__TAURI__.fs;
const { listen } = window.__TAURI__.event;

// ===== グローバル状態 =====
let currentTab = 'home';

// ===== DOM要素 =====
const elements = {};

// ===== 初期化 =====
document.addEventListener('DOMContentLoaded', async () => {
    initElements();
    initTabNavigation();
    initHomeTab();
    initHistoryTab();
    initSettingsTab();
    initExportTab();
    initFooter();
    initTauriEvents();

    // バージョン表示
    displayVersion();
});

function initElements() {
    // 共通
    elements.appVersion = document.getElementById('app-version');
    elements.minimizeBtn = document.getElementById('minimize-btn');

    // タブ
    elements.tabBtns = document.querySelectorAll('.tab-btn');
    elements.tabContents = document.querySelectorAll('.tab-content');
    elements.unreadBadge = document.getElementById('unread-badge');

    // ホーム
    elements.brokerStatus = document.getElementById('broker-status');

    // 履歴
    elements.sessionFilter = document.getElementById('session-filter');
    elements.markAllReadBtn = document.getElementById('mark-all-read');
    elements.clearHistoryBtn = document.getElementById('clear-history');
    elements.historyList = document.getElementById('history-list');
    elements.historyEmpty = document.getElementById('history-empty');

    // 設定
    elements.toastEnabled = document.getElementById('toast-enabled');
    elements.soundEnabled = document.getElementById('sound-enabled');
    elements.flashEnabled = document.getElementById('flash-enabled');
    elements.badgeEnabled = document.getElementById('badge-enabled');
    elements.trayFlashEnabled = document.getElementById('tray-flash-enabled');
    elements.volumeSlider = document.getElementById('volume');
    elements.volumeDisplay = document.getElementById('volume-display');
    elements.testSoundBtn = document.getElementById('test-sound');
    elements.saveBtn = document.getElementById('save-btn');
    elements.saveStatus = document.getElementById('save-status');

    // エクスポート
    elements.hostInput = document.getElementById('host');
    elements.detectIpBtn = document.getElementById('detect-ip');
    elements.exportLinuxBtn = document.getElementById('export-linux-btn');
    elements.exportWindowsBtn = document.getElementById('export-windows-btn');
    elements.exportStatus = document.getElementById('export-status');
    elements.ipStatus = document.getElementById('ip-status');
}

// ===== バージョン表示 =====
async function displayVersion() {
    try {
        const version = await getVersion();
        elements.appVersion.textContent = `v${version}`;
    } catch (error) {
        console.error('Failed to get version:', error);
        elements.appVersion.textContent = 'v?.?.?';
    }
}

// ===== タブナビゲーション =====
function initTabNavigation() {
    elements.tabBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            const tabId = btn.dataset.tab;
            switchTab(tabId);
        });
    });
}

function switchTab(tabId) {
    currentTab = tabId;

    // タブボタンの状態更新
    elements.tabBtns.forEach(btn => {
        btn.classList.toggle('active', btn.dataset.tab === tabId);
    });

    // タブコンテンツの表示切替
    elements.tabContents.forEach(content => {
        content.classList.toggle('active', content.id === `tab-${tabId}`);
    });

    // タブ固有の初期化
    if (tabId === 'history') {
        loadHistory();
    } else if (tabId === 'settings') {
        loadSettings();
    } else if (tabId === 'export') {
        detectIp();
    }
}

// ===== ホームタブ =====
function initHomeTab() {
    checkBrokerStatus();
    setInterval(checkBrokerStatus, 5000);
}

async function checkBrokerStatus() {
    try {
        const isRunning = await invoke('get_broker_status');
        updateStatusDisplay(isRunning);
    } catch (error) {
        console.error('Failed to check broker status:', error);
        updateStatusDisplay(false);
    }
}

function updateStatusDisplay(isConnected) {
    const statusText = elements.brokerStatus.querySelector('.status-text');

    if (isConnected) {
        elements.brokerStatus.classList.add('connected');
        elements.brokerStatus.classList.remove('disconnected');
        statusText.textContent = '稼働中';
    } else {
        elements.brokerStatus.classList.add('disconnected');
        elements.brokerStatus.classList.remove('connected');
        statusText.textContent = '停止中';
    }
}

// ===== 履歴タブ =====
function initHistoryTab() {
    elements.sessionFilter.addEventListener('change', loadHistory);
    elements.markAllReadBtn.addEventListener('click', markAllRead);
    elements.clearHistoryBtn.addEventListener('click', clearHistory);
}

async function loadHistory() {
    try {
        const filterSession = elements.sessionFilter.value || null;
        const entries = await invoke('get_notification_history', { filterSession });

        renderHistory(entries);
        updateUnreadBadge();
        updateSessionFilter(entries);
    } catch (error) {
        console.error('Failed to load history:', error);
        // バックエンドがまだ実装されていない場合は空表示
        renderHistory([]);
    }
}

function renderHistory(entries) {
    elements.historyList.innerHTML = '';

    if (entries.length === 0) {
        elements.historyEmpty.classList.remove('hidden');
        elements.historyList.style.display = 'none';
        return;
    }

    elements.historyEmpty.classList.add('hidden');
    elements.historyList.style.display = 'flex';

    entries.forEach(entry => {
        const item = createHistoryItem(entry);
        elements.historyList.appendChild(item);
    });
}

function createHistoryItem(entry) {
    const item = document.createElement('div');
    item.className = `history-item ${entry.read ? '' : 'unread'}`;
    item.dataset.id = entry.id;

    const iconClass = getEventIconClass(entry.event_type);
    const icon = getEventIcon(entry.event_type);
    const typeName = getEventTypeName(entry.event_type);
    const time = formatTime(entry.timestamp);
    const project = extractProjectName(entry.cwd);

    item.innerHTML = `
        <div class="history-icon ${iconClass}">${icon}</div>
        <div class="history-info">
            <div class="history-meta">
                <span class="history-type">${typeName}</span>
                <span class="history-time">${time}</span>
            </div>
            <div class="history-session">${entry.session_name}</div>
            <div class="history-project">${project}</div>
        </div>
    `;

    item.addEventListener('click', () => markAsRead(entry.id));

    return item;
}

function getEventIconClass(eventType) {
    switch (eventType) {
        case 'Stop': return 'stop';
        case 'PermissionRequest': return 'permission';
        case 'Notification': return 'notification';
        default: return '';
    }
}

function getEventIcon(eventType) {
    switch (eventType) {
        case 'Stop': return '✓';
        case 'PermissionRequest': return '⚠';
        case 'Notification': return '💬';
        default: return '•';
    }
}

function getEventTypeName(eventType) {
    switch (eventType) {
        case 'Stop': return 'タスク完了';
        case 'PermissionRequest': return '承認待ち';
        case 'Notification': return '通知';
        default: return eventType;
    }
}

function formatTime(timestamp) {
    const date = new Date(timestamp);
    const now = new Date();
    const diff = now - date;

    // 今日なら時刻のみ
    if (diff < 24 * 60 * 60 * 1000 && date.getDate() === now.getDate()) {
        return date.toLocaleTimeString('ja-JP', { hour: '2-digit', minute: '2-digit' });
    }

    // 昨日以前なら日付も
    return date.toLocaleDateString('ja-JP', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
}

function extractProjectName(cwd) {
    if (!cwd) return '';
    const parts = cwd.split('/');
    return parts[parts.length - 1] || cwd;
}

async function markAsRead(id) {
    try {
        await invoke('mark_notification_read', { id });
        loadHistory();
    } catch (error) {
        console.error('Failed to mark as read:', error);
    }
}

async function markAllRead() {
    try {
        await invoke('mark_all_notifications_read');
        loadHistory();
    } catch (error) {
        console.error('Failed to mark all as read:', error);
    }
}

async function clearHistory() {
    try {
        await invoke('clear_notification_history');
        loadHistory();
    } catch (error) {
        console.error('Failed to clear history:', error);
    }
}

async function updateUnreadBadge() {
    try {
        const count = await invoke('get_unread_count');
        if (count > 0) {
            elements.unreadBadge.textContent = count > 99 ? '99+' : count;
            elements.unreadBadge.classList.remove('hidden');
        } else {
            elements.unreadBadge.classList.add('hidden');
        }
    } catch (error) {
        console.error('Failed to get unread count:', error);
        elements.unreadBadge.classList.add('hidden');
    }
}

function updateSessionFilter(entries) {
    const sessions = new Set();
    entries.forEach(e => sessions.add(e.session_name));

    const currentValue = elements.sessionFilter.value;
    elements.sessionFilter.innerHTML = '<option value="">すべて</option>';

    sessions.forEach(session => {
        const option = document.createElement('option');
        option.value = session;
        option.textContent = session;
        elements.sessionFilter.appendChild(option);
    });

    elements.sessionFilter.value = currentValue;
}

// ===== 設定タブ =====
function initSettingsTab() {
    elements.volumeSlider.addEventListener('input', () => {
        elements.volumeDisplay.textContent = elements.volumeSlider.value;
    });

    elements.testSoundBtn.addEventListener('click', playTestSound);
    elements.saveBtn.addEventListener('click', saveSettings);
}

async function loadSettings() {
    try {
        const settings = await invoke('get_settings');

        elements.toastEnabled.checked = settings.toast_notification_enabled;
        elements.soundEnabled.checked = settings.sound_enabled;
        elements.flashEnabled.checked = settings.taskbar_flash_enabled;
        elements.badgeEnabled.checked = settings.taskbar_badge_enabled;
        elements.trayFlashEnabled.checked = settings.tray_flash_enabled ?? true;

        const volumePercent = Math.round(settings.sound_volume * 100);
        elements.volumeSlider.value = volumePercent;
        elements.volumeDisplay.textContent = volumePercent;
    } catch (error) {
        console.error('Failed to load settings:', error);
        showSettingsStatus('設定の読み込みに失敗しました', 'error');
    }
}

async function playTestSound() {
    try {
        const volume = parseFloat(elements.volumeSlider.value) / 100;
        await invoke('play_test_sound', { volume });
    } catch (error) {
        console.error('Failed to play test sound:', error);
        showSettingsStatus('テスト再生に失敗しました', 'error');
    }
}

async function saveSettings() {
    try {
        const settings = {
            toast_notification_enabled: elements.toastEnabled.checked,
            sound_enabled: elements.soundEnabled.checked,
            taskbar_flash_enabled: elements.flashEnabled.checked,
            taskbar_badge_enabled: elements.badgeEnabled.checked,
            tray_flash_enabled: elements.trayFlashEnabled.checked,
            sound_volume: parseFloat(elements.volumeSlider.value) / 100
        };

        await invoke('save_settings_command', { settings });
        showSettingsStatus('設定を保存しました', 'success');
    } catch (error) {
        console.error('Failed to save settings:', error);
        showSettingsStatus('設定の保存に失敗しました', 'error');
    }
}

function showSettingsStatus(message, type) {
    elements.saveStatus.textContent = message;
    elements.saveStatus.className = 'save-status ' + type;

    setTimeout(() => {
        elements.saveStatus.textContent = '';
        elements.saveStatus.className = 'save-status';
    }, 3000);
}

// ===== エクスポートタブ =====
function initExportTab() {
    elements.detectIpBtn.addEventListener('click', detectIp);
    elements.exportLinuxBtn.addEventListener('click', () => exportConfig('linux_wsl'));
    elements.exportWindowsBtn.addEventListener('click', () => exportConfig('windows'));
}

async function detectIp() {
    try {
        elements.detectIpBtn.disabled = true;
        elements.detectIpBtn.querySelector('.btn-text').textContent = '検出中...';

        const ip = await invoke('detect_ip');
        elements.hostInput.value = ip;
        showIpStatus('検出しました: ' + ip, 'success');
    } catch (error) {
        console.error('IP detection failed:', error);
        showIpStatus('検出に失敗しました', 'error');
    } finally {
        elements.detectIpBtn.disabled = false;
        elements.detectIpBtn.querySelector('.btn-text').textContent = '自動検出';
    }
}

function showIpStatus(message, type) {
    elements.ipStatus.textContent = message;
    elements.ipStatus.className = 'ip-status ' + type;

    if (type === 'success') {
        setTimeout(() => {
            elements.ipStatus.classList.add('hidden');
        }, 3000);
    }
}

async function exportConfig(platform) {
    const host = elements.hostInput.value.trim();
    const port = 1883;

    if (!host) {
        showExportStatus('IPアドレスを入力してください', 'error');
        return;
    }

    const btn = platform === 'windows' ? elements.exportWindowsBtn : elements.exportLinuxBtn;
    const originalContent = btn.innerHTML;
    const platformName = platform === 'windows' ? 'Windows' : 'Linux/WSL';
    const defaultFileName = platform === 'windows'
        ? 'claude-code-notify-windows-setup.zip'
        : 'claude-code-notify-linux-setup.zip';

    try {
        btn.disabled = true;
        btn.innerHTML = '<span class="btn-content">エクスポート中...</span>';

        const zipData = await invoke('generate_config_zip_v2', {
            options: { host, port, platform }
        });

        const filePath = await save({
            defaultPath: defaultFileName,
            filters: [{ name: 'ZIP Archive', extensions: ['zip'] }]
        });

        if (filePath) {
            await writeFile(filePath, new Uint8Array(zipData));
            showExportStatus(`${platformName}用設定をエクスポートしました`, 'success');
        } else {
            showExportStatus('エクスポートがキャンセルされました', 'info');
        }
    } catch (error) {
        console.error('Export failed:', error);
        showExportStatus('エクスポートに失敗しました: ' + error, 'error');
    } finally {
        btn.disabled = false;
        btn.innerHTML = originalContent;
    }
}

function showExportStatus(message, type) {
    elements.exportStatus.textContent = message;
    elements.exportStatus.className = 'status ' + type;
    elements.exportStatus.classList.remove('hidden');

    if (type === 'success' || type === 'info') {
        setTimeout(() => {
            elements.exportStatus.classList.add('hidden');
        }, 5000);
    }
}

// ===== フッター =====
function initFooter() {
    elements.minimizeBtn.addEventListener('click', minimizeToTray);
}

async function minimizeToTray() {
    try {
        const currentWindow = getCurrentWindow();
        await currentWindow.hide();
    } catch (error) {
        console.error('Failed to minimize:', error);
    }
}

// ===== Tauriイベント =====
function initTauriEvents() {
    // トレイメニューからのタブ切り替え
    listen('switch-tab', (event) => {
        switchTab(event.payload);
    });

    // 通知追加イベント
    listen('notification-added', () => {
        if (currentTab === 'history') {
            loadHistory();
        }
        updateUnreadBadge();
    });
}
