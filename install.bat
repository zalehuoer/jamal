@echo off
chcp 65001 >nul
echo ========================================
echo   JamalC2 一键安装脚本
echo ========================================
echo.

:: 检查 Rust
echo [1/4] 检查 Rust...
where rustc >nul 2>&1
if %errorlevel% neq 0 (
    echo [!] Rust 未安装，正在安装...
    winget install Rustlang.Rustup -e --accept-source-agreements --accept-package-agreements
    if %errorlevel% neq 0 (
        echo [X] Rust 安装失败，请手动访问 https://rustup.rs/
        pause
        exit /b 1
    )
    echo [i] 请重新打开终端后再次运行此脚本
    pause
    exit /b 0
) else (
    for /f "tokens=2" %%i in ('rustc --version') do echo [√] Rust %%i 已安装
)

:: 检查 Node.js
echo [2/4] 检查 Node.js...
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo [!] Node.js 未安装，正在安装...
    winget install OpenJS.NodeJS.LTS -e --accept-source-agreements --accept-package-agreements
    if %errorlevel% neq 0 (
        echo [X] Node.js 安装失败，请手动访问 https://nodejs.org/
        pause
        exit /b 1
    )
    echo [i] 请重新打开终端后再次运行此脚本
    pause
    exit /b 0
) else (
    for /f "tokens=1" %%i in ('node --version') do echo [√] Node.js %%i 已安装
)

:: 安装前端依赖
echo [3/4] 安装 Server 前端依赖...
cd /d "%~dp0server"
if not exist node_modules (
    call npm install
    if %errorlevel% neq 0 (
        echo [X] npm install 失败
        pause
        exit /b 1
    )
) else (
    echo [√] 依赖已存在，跳过
)
cd /d "%~dp0"

:: 完成
echo [4/4] 安装完成！
echo.
echo ========================================
echo   使用方法:
echo   1. cd server
echo   2. npm run tauri dev
echo ========================================
echo.
pause
