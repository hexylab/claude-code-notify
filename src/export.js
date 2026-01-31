const { invoke } = window.__TAURI__.core;
const { save } = window.__TAURI__.dialog;
const { writeFile } = window.__TAURI__.fs;

document.addEventListener('DOMContentLoaded', () => {
    const hostInput = document.getElementById('host');
    const detectIpBtn = document.getElementById('detect-ip');
    const exportBtn = document.getElementById('export-btn');
    const statusDiv = document.getElementById('status');
    const ipStatusDiv = document.getElementById('ip-status');

    // Detect IP on load
    detectIp();

    detectIpBtn.addEventListener('click', detectIp);
    exportBtn.addEventListener('click', exportConfig);

    async function detectIp() {
        try {
            detectIpBtn.disabled = true;
            detectIpBtn.textContent = '検出中...';

            const ip = await invoke('detect_ip');
            hostInput.value = ip;
            showIpStatus('検出しました: ' + ip, 'success');
        } catch (error) {
            console.error('IP detection failed:', error);
            showIpStatus('検出に失敗しました', 'error');
        } finally {
            detectIpBtn.disabled = false;
            detectIpBtn.textContent = '自動検出';
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

    async function exportConfig() {
        const host = hostInput.value.trim();
        const port = 1883;

        if (!host) {
            showStatus('IPアドレスを入力してください', 'error');
            return;
        }

        try {
            exportBtn.disabled = true;
            exportBtn.textContent = 'エクスポート中...';

            // Generate ZIP data
            const zipData = await invoke('generate_config_zip', { host, port });

            // Show save dialog
            const filePath = await save({
                defaultPath: 'claude-code-notify-setup.zip',
                filters: [{ name: 'ZIP Archive', extensions: ['zip'] }]
            });

            if (filePath) {
                // Write file
                await writeFile(filePath, new Uint8Array(zipData));
                showStatus('エクスポートが完了しました: ' + filePath, 'success');
            } else {
                showStatus('エクスポートがキャンセルされました', 'info');
            }
        } catch (error) {
            console.error('Export failed:', error);
            showStatus('エクスポートに失敗しました: ' + error, 'error');
        } finally {
            exportBtn.disabled = false;
            exportBtn.textContent = 'ZIPとしてエクスポート';
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
