// Settings page JavaScript
const { invoke } = window.__TAURI__.core;

// DOM elements
let toastEnabled;
let soundEnabled;
let flashEnabled;
let badgeEnabled;
let trayFlashEnabled;
let volumeSlider;
let volumeDisplay;
let testSoundBtn;
let saveBtn;
let saveStatus;

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', async () => {
    // Get DOM elements
    toastEnabled = document.getElementById('toast-enabled');
    soundEnabled = document.getElementById('sound-enabled');
    flashEnabled = document.getElementById('flash-enabled');
    badgeEnabled = document.getElementById('badge-enabled');
    trayFlashEnabled = document.getElementById('tray-flash-enabled');
    volumeSlider = document.getElementById('volume');
    volumeDisplay = document.getElementById('volume-display');
    testSoundBtn = document.getElementById('test-sound');
    saveBtn = document.getElementById('save-btn');
    saveStatus = document.getElementById('save-status');

    // Load current settings
    await loadSettings();

    // Setup event listeners
    setupEventListeners();
});

async function loadSettings() {
    try {
        const settings = await invoke('get_settings');

        // Apply settings to UI
        toastEnabled.checked = settings.toast_notification_enabled;
        soundEnabled.checked = settings.sound_enabled;
        flashEnabled.checked = settings.taskbar_flash_enabled;
        badgeEnabled.checked = settings.taskbar_badge_enabled;
        trayFlashEnabled.checked = settings.tray_flash_enabled ?? true;

        // Volume is stored as 0.0-1.0, display as 0-100
        const volumePercent = Math.round(settings.sound_volume * 100);
        volumeSlider.value = volumePercent;
        volumeDisplay.textContent = volumePercent;
    } catch (error) {
        console.error('Failed to load settings:', error);
        showStatus('設定の読み込みに失敗しました', 'error');
    }
}

function setupEventListeners() {
    // Volume slider
    volumeSlider.addEventListener('input', () => {
        volumeDisplay.textContent = volumeSlider.value;
    });

    // Test sound button
    testSoundBtn.addEventListener('click', async () => {
        try {
            const volume = parseFloat(volumeSlider.value) / 100;
            await invoke('play_test_sound', { volume });
        } catch (error) {
            console.error('Failed to play test sound:', error);
            showStatus('テスト再生に失敗しました', 'error');
        }
    });

    // Save button
    saveBtn.addEventListener('click', async () => {
        await saveSettings();
    });
}

async function saveSettings() {
    try {
        const settings = {
            toast_notification_enabled: toastEnabled.checked,
            sound_enabled: soundEnabled.checked,
            taskbar_flash_enabled: flashEnabled.checked,
            taskbar_badge_enabled: badgeEnabled.checked,
            tray_flash_enabled: trayFlashEnabled.checked,
            sound_volume: parseFloat(volumeSlider.value) / 100
        };

        await invoke('save_settings_command', { settings });
        showStatus('設定を保存しました', 'success');
    } catch (error) {
        console.error('Failed to save settings:', error);
        showStatus('設定の保存に失敗しました', 'error');
    }
}

function showStatus(message, type) {
    saveStatus.textContent = message;
    saveStatus.className = 'save-status ' + type;

    // Clear status after 3 seconds
    setTimeout(() => {
        saveStatus.textContent = '';
        saveStatus.className = 'save-status';
    }, 3000);
}
