/*
 * JamalC2 Implant - Process Operations Header
 */

#ifndef PROCESS_H
#define PROCESS_H

// Get process list (JSON array)
// Caller must free the returned string
char *process_list(void);

// Kill process by PID
// Returns: 0 on success, -1 on failure
int process_kill(int pid);

#endif // PROCESS_H
