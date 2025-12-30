/*
 * JamalC2 Implant - Sleep Obfuscation Header
 * 睡眠混淆 - 在睡眠期间加密内存以规避扫描
 */

#ifndef SLEEP_OBF_H
#define SLEEP_OBF_H

#include <windows.h>

/**
 * 带混淆的睡眠函数
 * 睡眠期间会加密当前模块的代码区域，唤醒后解密
 * @param dwMilliseconds 睡眠时间（毫秒）
 */
void obfuscated_sleep(DWORD dwMilliseconds);

/**
 * 初始化睡眠混淆模块
 * @return 0 成功, -1 失败
 */
int sleep_obf_init(void);

/**
 * 设置要保护的密钥（可选）
 * 在睡眠期间会加密此密钥的备份
 */
void sleep_obf_set_key(const unsigned char *key, unsigned long long size);

#endif // SLEEP_OBF_H
