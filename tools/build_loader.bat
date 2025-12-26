@echo off
echo ========================================
echo   Compiling Shellcode Loader
echo ========================================

:: 尝试找到 Visual Studio 环境
if exist "%ProgramFiles%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" (
    call "%ProgramFiles%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
) else if exist "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" (
    call "%ProgramFiles(x86)%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
) else if exist "%ProgramFiles(x86)%\Microsoft Visual Studio\2019\BuildTools\VC\Auxiliary\Build\vcvars64.bat" (
    call "%ProgramFiles(x86)%\Microsoft Visual Studio\2019\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
)

:: 编译 loader
cl.exe /O2 /MT /Fe:loader.exe loader.c /link /SUBSYSTEM:CONSOLE

if %errorlevel% equ 0 (
    echo [OK] Compiled successfully: loader.exe
    :: 清理中间文件
    del loader.obj 2>nul
) else (
    echo [X] Compilation failed
    echo Please run this in "Developer Command Prompt for VS"
)

pause
