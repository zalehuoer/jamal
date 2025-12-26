/*
 * JamalC2 Implant - Utility Functions Implementation
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <windows.h>


#include "utils.h"

// Base64 encoding table
static const char BASE64_TABLE[] =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

char *base64_encode(const uint8_t *data, size_t len) {
  size_t output_len = 4 * ((len + 2) / 3) + 1;
  char *output = safe_malloc(output_len);

  size_t i = 0, j = 0;
  while (i < len) {
    uint32_t octet_a = i < len ? data[i++] : 0;
    uint32_t octet_b = i < len ? data[i++] : 0;
    uint32_t octet_c = i < len ? data[i++] : 0;

    uint32_t triple = (octet_a << 16) + (octet_b << 8) + octet_c;

    output[j++] = BASE64_TABLE[(triple >> 18) & 0x3F];
    output[j++] = BASE64_TABLE[(triple >> 12) & 0x3F];
    output[j++] = BASE64_TABLE[(triple >> 6) & 0x3F];
    output[j++] = BASE64_TABLE[triple & 0x3F];
  }

  // Padding
  size_t mod = len % 3;
  if (mod == 1) {
    output[j - 1] = '=';
    output[j - 2] = '=';
  } else if (mod == 2) {
    output[j - 1] = '=';
  }

  output[j] = '\0';
  return output;
}

// Base64 decode table
static int base64_decode_char(char c) {
  if (c >= 'A' && c <= 'Z')
    return c - 'A';
  if (c >= 'a' && c <= 'z')
    return c - 'a' + 26;
  if (c >= '0' && c <= '9')
    return c - '0' + 52;
  if (c == '+')
    return 62;
  if (c == '/')
    return 63;
  return -1;
}

int base64_decode(const char *input, uint8_t **output) {
  size_t len = strlen(input);
  if (len % 4 != 0)
    return -1;

  size_t output_len = len / 4 * 3;
  if (input[len - 1] == '=')
    output_len--;
  if (input[len - 2] == '=')
    output_len--;

  *output = safe_malloc(output_len);

  size_t i = 0, j = 0;
  while (i < len) {
    int a = base64_decode_char(input[i++]);
    int b = base64_decode_char(input[i++]);
    int c = input[i] == '=' ? 0 : base64_decode_char(input[i]);
    i++;
    int d = input[i] == '=' ? 0 : base64_decode_char(input[i]);
    i++;

    if (a < 0 || b < 0) {
      free(*output);
      *output = NULL;
      return -1;
    }

    uint32_t triple = (a << 18) + (b << 12) + (c << 6) + d;

    if (j < output_len)
      (*output)[j++] = (triple >> 16) & 0xFF;
    if (j < output_len)
      (*output)[j++] = (triple >> 8) & 0xFF;
    if (j < output_len)
      (*output)[j++] = triple & 0xFF;
  }

  return (int)output_len;
}

int hex_to_bytes(const char *hex, uint8_t *output, size_t output_size) {
  size_t hex_len = strlen(hex);
  if (hex_len % 2 != 0)
    return -1;

  size_t byte_len = hex_len / 2;
  if (byte_len > output_size)
    return -1;

  for (size_t i = 0; i < byte_len; i++) {
    unsigned int byte;
    if (sscanf(hex + i * 2, "%02x", &byte) != 1) {
      return -1;
    }
    output[i] = (uint8_t)byte;
  }

  return (int)byte_len;
}

void sleep_with_jitter(int base_seconds, int jitter_percent) {
  static int seeded = 0;
  if (!seeded) {
    srand((unsigned int)time(NULL));
    seeded = 1;
  }

  int jitter = (base_seconds * jitter_percent) / 100;
  int actual = base_seconds + (rand() % (2 * jitter + 1)) - jitter;
  if (actual < 1)
    actual = 1;

  Sleep((DWORD)(actual * 1000));
}

void random_bytes(uint8_t *output, size_t len) {
  // Use Windows CryptGenRandom for secure random
  HCRYPTPROV hProv;
  if (CryptAcquireContext(&hProv, NULL, NULL, PROV_RSA_FULL,
                          CRYPT_VERIFYCONTEXT)) {
    CryptGenRandom(hProv, (DWORD)len, output);
    CryptReleaseContext(hProv, 0);
  } else {
    // Fallback to rand (not secure)
    for (size_t i = 0; i < len; i++) {
      output[i] = (uint8_t)rand();
    }
  }
}

char *safe_strdup(const char *s) {
  if (!s)
    return NULL;
  size_t len = strlen(s) + 1;
  char *dup = safe_malloc(len);
  memcpy(dup, s, len);
  return dup;
}

void *safe_malloc(size_t size) {
  void *ptr = malloc(size);
  if (!ptr && size > 0) {
    // Fatal error
    ExitProcess(1);
  }
  if (ptr) {
    memset(ptr, 0, size);
  }
  return ptr;
}

void *safe_realloc(void *ptr, size_t size) {
  void *new_ptr = realloc(ptr, size);
  if (!new_ptr && size > 0) {
    ExitProcess(1);
  }
  return new_ptr;
}
