const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;
const { getVersion } = window.__TAURI__.app;

document.addEventListener('DOMContentLoaded', () => {
    const brokerStatus = document.getElementById('broker-status');
    const openExportBtn = document.getElementById('open-export');
    const minimizeBtn = document.getElementById('minimize-btn');
    const appVersion = document.getElementById('app-version');

    // Display app version
    displayVersion();

    // Check broker status on load
    checkBrokerStatus();

    async function displayVersion() {
        try {
            const version = await getVersion();
            appVersion.textContent = `v${version}`;
        } catch (error) {
            console.error('Failed to get version:', error);
            appVersion.textContent = 'v?.?.?';
        }
    }

    // Periodically check broker status
    setInterval(checkBrokerStatus, 5000);

    // Open export window
    openExportBtn.addEventListener('click', openExportWindow);

    // Minimize to tray
    minimizeBtn.addEventListener('click', minimizeToTray);

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
        const statusText = brokerStatus.querySelector('.status-text');

        if (isConnected) {
            brokerStatus.classList.add('connected');
            brokerStatus.classList.remove('disconnected');
            statusText.textContent = '稼働中';
        } else {
            brokerStatus.classList.add('disconnected');
            brokerStatus.classList.remove('connected');
            statusText.textContent = '停止中';
        }
    }

    async function openExportWindow() {
        try {
            // Navigate to export page
            window.location.href = 'export.html';
        } catch (error) {
            console.error('Failed to open export:', error);
        }
    }

    async function minimizeToTray() {
        try {
            const currentWindow = getCurrentWindow();
            await currentWindow.hide();
        } catch (error) {
            console.error('Failed to minimize:', error);
        }
    }
});
