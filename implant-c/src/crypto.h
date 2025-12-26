/*
 * JamalC2 Implant - Crypto Header (ChaCha20-Poly1305)
 */

#ifndef CRYPTO_H
#define CRYPTO_H

#include <stddef.h>
#include <stdint.h>


// Nonce size for ChaCha20-Poly1305
#define CRYPTO_NONCE_SIZE 12
#define CRYPTO_TAG_SIZE 16
#define CRYPTO_KEY_SIZE 32

// Crypto context
typedef struct {
  uint8_t key[CRYPTO_KEY_SIZE];
} CryptoContext;

// Initialize crypto context from hex string
// Returns: 0 on success, -1 on failure
int crypto_init(CryptoContext *ctx, const char *hex_key);

// Encrypt data (output = nonce + ciphertext + tag)
// Returns: encrypted data length, or -1 on failure
int crypto_encrypt(CryptoContext *ctx, const uint8_t *plaintext,
                   size_t plaintext_len, uint8_t *output, size_t output_size);

// Decrypt data (input = nonce + ciphertext + tag)
// Returns: decrypted data length, or -1 on failure
int crypto_decrypt(CryptoContext *ctx, const uint8_t *ciphertext,
                   size_t ciphertext_len, uint8_t *output, size_t output_size);

#endif // CRYPTO_H
