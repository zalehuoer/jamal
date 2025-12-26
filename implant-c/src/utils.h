/*
 * JamalC2 Implant - Utility Functions Header
 */

#ifndef UTILS_H
#define UTILS_H

#include <stddef.h>
#include <stdint.h>

// Debug print macro - disabled in release builds
#ifdef NDEBUG
#define DEBUG_PRINT(fmt, ...) ((void)0)
#else
#include <stdio.h>
#define DEBUG_PRINT(fmt, ...) printf(fmt, ##__VA_ARGS__)
#endif

// Base64 encode
// Caller must free the returned string
char *base64_encode(const uint8_t *data, size_t len);

// Base64 decode
// Returns decoded length, caller must free output
int base64_decode(const char *input, uint8_t **output);

// Hex string to bytes
// Returns: number of bytes written, or -1 on failure
int hex_to_bytes(const char *hex, uint8_t *output, size_t output_size);

// Sleep with jitter
void sleep_with_jitter(int base_seconds, int jitter_percent);

// Generate random bytes
void random_bytes(uint8_t *output, size_t len);

// Safe string duplicate
char *safe_strdup(const char *s);

// Safe memory allocation
void *safe_malloc(size_t size);

// Safe memory reallocation
void *safe_realloc(void *ptr, size_t size);

#endif // UTILS_H
