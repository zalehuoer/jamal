# JamalC2 架构升级任务清单

## 阶段 1：C/C++ Implant 重写（预计 6-8 天）— ✅ 90% 完成

### ✅ 1.1 项目结构搭建
- [x] 创建 `implant-c/` 目录
- [x] CMakeLists.txt / build.bat
- [x] 头文件结构

### ✅ 1.2 HTTP 客户端
- [x] WinHTTP 封装
- [x] HTTPS 支持
- [x] Header 伪装

### ✅ 1.3 ChaCha20-Poly1305 加密
- [x] 加密函数（纯 C 实现）
- [x] 解密函数
- [x] 与 Rust Server 兼容验证

### ✅ 1.4 通信协议
- [x] JSON 解析
- [x] 请求/响应格式
- [x] 协议兼容测试

### ✅ 1.5 命令处理器
- [x] Shell 命令执行
- [x] 文件上传/下载
- [x] 文件删除
- [x] 进程列表
- [x] 系统信息收集

### ✅ 1.6 集成测试
- [x] 与 Windows Server 联调
- [x] 全功能测试

### ⏳ 1.7 Shellcode 输出验证
- [ ] Donut 兼容测试
- [ ] Loader 测试

---

## 阶段 2：Linux Web Server（预计 5-7 天）— ⏳ 待开始

### ⏳ 2.1 核心逻辑提取
- [ ] 创建 server/core/ crate
- [ ] 提取 listener、database、builder 等

### ⏳ 2.2 Web API 开发
- [ ] Axum REST API
- [ ] WebSocket 支持（实时日志）

### ⏳ 2.3 前端适配
- [ ] 复用 React 组件
- [ ] API 调用适配

### ⏳ 2.4 部署脚本
- [ ] Linux 安装脚本
- [ ] Docker 支持

### ⏳ 2.5 跨平台测试
- [ ] Windows Server 测试
- [ ] Linux Server 测试

---

## 阶段 3：Builder 多格式输出升级（预计 2-3 天）— ⏳ 待开始

### ⏳ 3.1 C Implant 编译集成
- [ ] 调用 C 编译器
- [ ] 配置注入

### ⏳ 3.2 多格式输出
- [ ] EXE 格式
- [ ] Shellcode (bin) 格式
- [ ] C Array 格式

### ⏳ 3.3 UI 更新
- [ ] 输出格式选择
- [ ] Windows/Linux Builder 适配

---

## 进度总览

| 阶段 | 状态 | 预计时间 | 完成度 |
|------|------|----------|--------|
| 阶段 1: C/C++ Implant | ✅ 进行中 | 6-8 天 | **90%** |
| 阶段 2: Linux Web Server | ⏳ 待开始 | 5-7 天 | 0% |
| 阶段 3: Builder 升级 | ⏳ 待开始 | 2-3 天 | 0% |
| **总计** | - | **13-18 天** | **30%** |

---

## 更新日志

### 2025-12-26
- ✅ 修复文件管理功能（目录浏览、上传、下载、删除）
- ✅ 修复 JSON 反转义导致的路径问题
- ✅ 修复命令编号不一致问题（Upload/Download）
- ✅ 实现文件删除功能（CMD_DELETE）
- ✅ 添加下载任务追踪以保留原文件名
- ✅ 修复中文文件名乱码问题
- ✅ 修复 Windows 驱动器列表显示
