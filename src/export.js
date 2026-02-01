const { invoke } = window.__TAURI__.core;
const { save } = window.__TAURI__.dialog;
const { writeFile } = window.__TAURI__.fs;

document.addEventListener('DOMContentLoaded', () => {
    const hostInput = document.getElementById('host');
    const detectIpBtn = document.getElementById('detect-ip');
    const exportLinuxBtn = document.getElementById('export-linux-btn');
    const exportWindowsBtn = document.getElementById('export-windows-btn');
    const statusDiv = document.getElementById('status');
    const ipStatusDiv = document.getElementById('ip-status');

    // Detect IP on load
    detectIp();

    detectIpBtn.addEventListener('click', detectIp);
    exportLinuxBtn.addEventListener('click', () => exportConfig('linux_wsl'));
    exportWindowsBtn.addEventListener('click', () => exportConfig('windows'));

    async function detectIp() {
        try {
            detectIpBtn.disabled = true;
            detectIpBtn.querySelector('.btn-text').textContent = '検出中...';

            const ip = await invoke('detect_ip');
            hostInput.value = ip;
            showIpStatus('検出しました: ' + ip, 'success');
        } catch (error) {
            console.error('IP detection failed:', error);
            showIpStatus('検出に失敗しました', 'error');
        } finally {
            detectIpBtn.disabled = false;
            detectIpBtn.querySelector('.btn-text').textContent = '自動検出';
        }
    }

    function showIpStatus(message, type) {
        ipStatusDiv.textContent = message;
        ipStatusDiv.className = 'ip-status ' + type;

        if (type === 'success') {
            setTimeout(() => {
                ipStatusDiv.classList.add('hidden');
            }, 3000);
        }
    }

    async function exportConfig(platform) {
        const host = hostInput.value.trim();
        const port = 1883;

        if (!host) {
            showStatus('IPアドレスを入力してください', 'error');
            return;
        }

        const btn = platform === 'windows' ? exportWindowsBtn : exportLinuxBtn;
        const originalContent = btn.innerHTML;
        const platformName = platform === 'windows' ? 'Windows' : 'Linux/WSL';
        const defaultFileName = platform === 'windows'
            ? 'claude-code-notify-windows-setup.zip'
            : 'claude-code-notify-linux-setup.zip';

        try {
            btn.disabled = true;
            btn.innerHTML = '<span class="btn-content">エクスポート中...</span>';

            // Generate ZIP data using the new v2 command
            const zipData = await invoke('generate_config_zip_v2', {
                options: { host, port, platform }
            });

            // Show save dialog
            const filePath = await save({
                defaultPath: defaultFileName,
                filters: [{ name: 'ZIP Archive', extensions: ['zip'] }]
            });

            if (filePath) {
                // Write file
                await writeFile(filePath, new Uint8Array(zipData));
                showStatus(`${platformName}用設定をエクスポートしました: ${filePath}`, 'success');
            } else {
                showStatus('エクスポートがキャンセルされました', 'info');
            }
        } catch (error) {
            console.error('Export failed:', error);
            showStatus('エクスポートに失敗しました: ' + error, 'error');
        } finally {
            btn.disabled = false;
            btn.innerHTML = originalContent;
        }
    }

    function showStatus(message, type) {
        statusDiv.textContent = message;
        statusDiv.className = 'status ' + type;
        statusDiv.classList.remove('hidden');

        // Auto-hide after 5 seconds for success/info
        if (type === 'success' || type === 'info') {
            setTimeout(() => {
                statusDiv.classList.add('hidden');
            }, 5000);
        }
    }
});
