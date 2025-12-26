/*
 * JamalC2 Implant - File Operations Header
 */

#ifndef FILES_H
#define FILES_H

#include <stddef.h>
#include <stdint.h>


// Read file and return base64 encoded content
// Caller must free the returned string
char *files_read_base64(const char *path);

// Write base64 decoded content to file
// Returns: 0 on success, -1 on failure
int files_write_base64(const char *path, const char *base64_content);

// Check if file exists
int files_exists(const char *path);

// Get file size
int64_t files_size(const char *path);

// List directory contents (JSON array)
// Caller must free the returned string
char *files_list_dir(const char *path);

#endif // FILES_H
