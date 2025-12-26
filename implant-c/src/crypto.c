/*
 * JamalC2 Implant - ChaCha20-Poly1305 Implementation
 * Standard implementation compatible with libsodium/ring/chacha20poly1305 crate
 */

#include <stdlib.h>
#include <string.h>
#include <windows.h>


#include "crypto.h"
#include "utils.h"

// ============================================================================
// ChaCha20 Implementation
// ============================================================================

#define ROTL32(v, n) (((v) << (n)) | ((v) >> (32 - (n))))

#define QUARTERROUND(a, b, c, d)                                               \
  a += b;                                                                      \
  d ^= a;                                                                      \
  d = ROTL32(d, 16);                                                           \
  c += d;                                                                      \
  b ^= c;                                                                      \
  b = ROTL32(b, 12);                                                           \
  a += b;                                                                      \
  d ^= a;                                                                      \
  d = ROTL32(d, 8);                                                            \
  c += d;                                                                      \
  b ^= c;                                                                      \
  b = ROTL32(b, 7);

static void chacha20_block(uint32_t output[16], const uint32_t input[16]) {
  uint32_t x[16];
  int i;

  for (i = 0; i < 16; i++) {
    x[i] = input[i];
  }

  for (i = 0; i < 10; i++) {
    // Column rounds
    QUARTERROUND(x[0], x[4], x[8], x[12]);
    QUARTERROUND(x[1], x[5], x[9], x[13]);
    QUARTERROUND(x[2], x[6], x[10], x[14]);
    QUARTERROUND(x[3], x[7], x[11], x[15]);
    // Diagonal rounds
    QUARTERROUND(x[0], x[5], x[10], x[15]);
    QUARTERROUND(x[1], x[6], x[11], x[12]);
    QUARTERROUND(x[2], x[7], x[8], x[13]);
    QUARTERROUND(x[3], x[4], x[9], x[14]);
  }

  for (i = 0; i < 16; i++) {
    output[i] = x[i] + input[i];
  }
}

static void chacha20_init_state(uint32_t state[16], const uint8_t key[32],
                                const uint8_t nonce[12], uint32_t counter) {
  // Constants "expand 32-byte k"
  state[0] = 0x61707865;
  state[1] = 0x3320646e;
  state[2] = 0x79622d32;
  state[3] = 0x6b206574;

  // Key (little-endian)
  for (int i = 0; i < 8; i++) {
    state[4 + i] = ((uint32_t)key[i * 4]) | ((uint32_t)key[i * 4 + 1] << 8) |
                   ((uint32_t)key[i * 4 + 2] << 16) |
                   ((uint32_t)key[i * 4 + 3] << 24);
  }

  // Counter
  state[12] = counter;

  // Nonce (little-endian)
  state[13] = ((uint32_t)nonce[0]) | ((uint32_t)nonce[1] << 8) |
              ((uint32_t)nonce[2] << 16) | ((uint32_t)nonce[3] << 24);
  state[14] = ((uint32_t)nonce[4]) | ((uint32_t)nonce[5] << 8) |
              ((uint32_t)nonce[6] << 16) | ((uint32_t)nonce[7] << 24);
  state[15] = ((uint32_t)nonce[8]) | ((uint32_t)nonce[9] << 8) |
              ((uint32_t)nonce[10] << 16) | ((uint32_t)nonce[11] << 24);
}

static void chacha20_crypt(const uint8_t key[32], const uint8_t nonce[12],
                           uint32_t counter, const uint8_t *in, uint8_t *out,
                           size_t len) {
  uint32_t state[16];
  uint32_t block[16];
  uint8_t keystream[64];

  chacha20_init_state(state, key, nonce, counter);

  size_t offset = 0;
  while (offset < len) {
    chacha20_block(block, state);
    state[12]++; // Increment counter

    // Convert block to bytes (little-endian)
    for (int i = 0; i < 16; i++) {
      keystream[i * 4] = (uint8_t)(block[i]);
      keystream[i * 4 + 1] = (uint8_t)(block[i] >> 8);
      keystream[i * 4 + 2] = (uint8_t)(block[i] >> 16);
      keystream[i * 4 + 3] = (uint8_t)(block[i] >> 24);
    }

    size_t block_len = (len - offset < 64) ? (len - offset) : 64;
    for (size_t i = 0; i < block_len; i++) {
      out[offset + i] = in[offset + i] ^ keystream[i];
    }
    offset += block_len;
  }
}

// ============================================================================
// Poly1305 Implementation (RFC 8439)
// ============================================================================

// We use 64-bit arithmetic with careful handling to avoid overflow

typedef struct {
  uint32_t r[5];   // r (clamped)
  uint32_t h[5];   // accumulator
  uint32_t pad[4]; // s = r * key
} poly1305_state;

static void poly1305_init(poly1305_state *st, const uint8_t key[32]) {
  // r (first 16 bytes, clamped)
  uint32_t t0 = ((uint32_t)key[0]) | ((uint32_t)key[1] << 8) |
                ((uint32_t)key[2] << 16) | ((uint32_t)key[3] << 24);
  uint32_t t1 = ((uint32_t)key[4]) | ((uint32_t)key[5] << 8) |
                ((uint32_t)key[6] << 16) | ((uint32_t)key[7] << 24);
  uint32_t t2 = ((uint32_t)key[8]) | ((uint32_t)key[9] << 8) |
                ((uint32_t)key[10] << 16) | ((uint32_t)key[11] << 24);
  uint32_t t3 = ((uint32_t)key[12]) | ((uint32_t)key[13] << 8) |
                ((uint32_t)key[14] << 16) | ((uint32_t)key[15] << 24);

  // Clamp r
  st->r[0] = t0 & 0x3ffffff;
  st->r[1] = ((t0 >> 26) | (t1 << 6)) & 0x3ffff03;
  st->r[2] = ((t1 >> 20) | (t2 << 12)) & 0x3ffc0ff;
  st->r[3] = ((t2 >> 14) | (t3 << 18)) & 0x3f03fff;
  st->r[4] = (t3 >> 8) & 0xfffff;

  // h = 0
  st->h[0] = st->h[1] = st->h[2] = st->h[3] = st->h[4] = 0;

  // s (last 16 bytes, not clamped)
  st->pad[0] = ((uint32_t)key[16]) | ((uint32_t)key[17] << 8) |
               ((uint32_t)key[18] << 16) | ((uint32_t)key[19] << 24);
  st->pad[1] = ((uint32_t)key[20]) | ((uint32_t)key[21] << 8) |
               ((uint32_t)key[22] << 16) | ((uint32_t)key[23] << 24);
  st->pad[2] = ((uint32_t)key[24]) | ((uint32_t)key[25] << 8) |
               ((uint32_t)key[26] << 16) | ((uint32_t)key[27] << 24);
  st->pad[3] = ((uint32_t)key[28]) | ((uint32_t)key[29] << 8) |
               ((uint32_t)key[30] << 16) | ((uint32_t)key[31] << 24);
}

static void poly1305_blocks(poly1305_state *st, const uint8_t *m, size_t len,
                            uint32_t hibit) {
  uint32_t r0 = st->r[0], r1 = st->r[1], r2 = st->r[2], r3 = st->r[3],
           r4 = st->r[4];
  uint32_t s1 = r1 * 5, s2 = r2 * 5, s3 = r3 * 5, s4 = r4 * 5;
  uint32_t h0 = st->h[0], h1 = st->h[1], h2 = st->h[2], h3 = st->h[3],
           h4 = st->h[4];

  while (len >= 16) {
    // h += m[i]
    uint32_t t0 = ((uint32_t)m[0]) | ((uint32_t)m[1] << 8) |
                  ((uint32_t)m[2] << 16) | ((uint32_t)m[3] << 24);
    uint32_t t1 = ((uint32_t)m[4]) | ((uint32_t)m[5] << 8) |
                  ((uint32_t)m[6] << 16) | ((uint32_t)m[7] << 24);
    uint32_t t2 = ((uint32_t)m[8]) | ((uint32_t)m[9] << 8) |
                  ((uint32_t)m[10] << 16) | ((uint32_t)m[11] << 24);
    uint32_t t3 = ((uint32_t)m[12]) | ((uint32_t)m[13] << 8) |
                  ((uint32_t)m[14] << 16) | ((uint32_t)m[15] << 24);

    h0 += t0 & 0x3ffffff;
    h1 += ((t0 >> 26) | (t1 << 6)) & 0x3ffffff;
    h2 += ((t1 >> 20) | (t2 << 12)) & 0x3ffffff;
    h3 += ((t2 >> 14) | (t3 << 18)) & 0x3ffffff;
    h4 += (t3 >> 8) | hibit;

    // h *= r
    uint64_t d0 = ((uint64_t)h0 * r0) + ((uint64_t)h1 * s4) +
                  ((uint64_t)h2 * s3) + ((uint64_t)h3 * s2) +
                  ((uint64_t)h4 * s1);
    uint64_t d1 = ((uint64_t)h0 * r1) + ((uint64_t)h1 * r0) +
                  ((uint64_t)h2 * s4) + ((uint64_t)h3 * s3) +
                  ((uint64_t)h4 * s2);
    uint64_t d2 = ((uint64_t)h0 * r2) + ((uint64_t)h1 * r1) +
                  ((uint64_t)h2 * r0) + ((uint64_t)h3 * s4) +
                  ((uint64_t)h4 * s3);
    uint64_t d3 = ((uint64_t)h0 * r3) + ((uint64_t)h1 * r2) +
                  ((uint64_t)h2 * r1) + ((uint64_t)h3 * r0) +
                  ((uint64_t)h4 * s4);
    uint64_t d4 = ((uint64_t)h0 * r4) + ((uint64_t)h1 * r3) +
                  ((uint64_t)h2 * r2) + ((uint64_t)h3 * r1) +
                  ((uint64_t)h4 * r0);

    // Carry
    uint32_t c;
    c = (uint32_t)(d0 >> 26);
    h0 = (uint32_t)d0 & 0x3ffffff;
    d1 += c;
    c = (uint32_t)(d1 >> 26);
    h1 = (uint32_t)d1 & 0x3ffffff;
    d2 += c;
    c = (uint32_t)(d2 >> 26);
    h2 = (uint32_t)d2 & 0x3ffffff;
    d3 += c;
    c = (uint32_t)(d3 >> 26);
    h3 = (uint32_t)d3 & 0x3ffffff;
    d4 += c;
    c = (uint32_t)(d4 >> 26);
    h4 = (uint32_t)d4 & 0x3ffffff;
    h0 += c * 5;
    c = h0 >> 26;
    h0 &= 0x3ffffff;
    h1 += c;

    m += 16;
    len -= 16;
  }

  st->h[0] = h0;
  st->h[1] = h1;
  st->h[2] = h2;
  st->h[3] = h3;
  st->h[4] = h4;
}

static void poly1305_finish(poly1305_state *st, uint8_t tag[16]) {
  uint32_t h0 = st->h[0], h1 = st->h[1], h2 = st->h[2], h3 = st->h[3],
           h4 = st->h[4];
  uint32_t c, g0, g1, g2, g3, g4, mask;

  // Final carry
  c = h1 >> 26;
  h1 &= 0x3ffffff;
  h2 += c;
  c = h2 >> 26;
  h2 &= 0x3ffffff;
  h3 += c;
  c = h3 >> 26;
  h3 &= 0x3ffffff;
  h4 += c;
  c = h4 >> 26;
  h4 &= 0x3ffffff;
  h0 += c * 5;
  c = h0 >> 26;
  h0 &= 0x3ffffff;
  h1 += c;

  // Compute h + -p
  g0 = h0 + 5;
  c = g0 >> 26;
  g0 &= 0x3ffffff;
  g1 = h1 + c;
  c = g1 >> 26;
  g1 &= 0x3ffffff;
  g2 = h2 + c;
  c = g2 >> 26;
  g2 &= 0x3ffffff;
  g3 = h3 + c;
  c = g3 >> 26;
  g3 &= 0x3ffffff;
  g4 = h4 + c - (1 << 26);

  // Select h if h < p, or h - p if h >= p
  mask = (g4 >> 31) - 1;
  g0 &= mask;
  g1 &= mask;
  g2 &= mask;
  g3 &= mask;
  g4 &= mask;
  mask = ~mask;
  h0 = (h0 & mask) | g0;
  h1 = (h1 & mask) | g1;
  h2 = (h2 & mask) | g2;
  h3 = (h3 & mask) | g3;
  h4 = (h4 & mask) | g4;

  // h = h % (2^128)
  h0 = (h0) | (h1 << 26);
  h1 = (h1 >> 6) | (h2 << 20);
  h2 = (h2 >> 12) | (h3 << 14);
  h3 = (h3 >> 18) | (h4 << 8);

  // h += s
  uint64_t f;
  f = (uint64_t)h0 + st->pad[0];
  h0 = (uint32_t)f;
  f = (uint64_t)h1 + st->pad[1] + (f >> 32);
  h1 = (uint32_t)f;
  f = (uint64_t)h2 + st->pad[2] + (f >> 32);
  h2 = (uint32_t)f;
  f = (uint64_t)h3 + st->pad[3] + (f >> 32);
  h3 = (uint32_t)f;

  // Output
  tag[0] = (uint8_t)h0;
  tag[1] = (uint8_t)(h0 >> 8);
  tag[2] = (uint8_t)(h0 >> 16);
  tag[3] = (uint8_t)(h0 >> 24);
  tag[4] = (uint8_t)h1;
  tag[5] = (uint8_t)(h1 >> 8);
  tag[6] = (uint8_t)(h1 >> 16);
  tag[7] = (uint8_t)(h1 >> 24);
  tag[8] = (uint8_t)h2;
  tag[9] = (uint8_t)(h2 >> 8);
  tag[10] = (uint8_t)(h2 >> 16);
  tag[11] = (uint8_t)(h2 >> 24);
  tag[12] = (uint8_t)h3;
  tag[13] = (uint8_t)(h3 >> 8);
  tag[14] = (uint8_t)(h3 >> 16);
  tag[15] = (uint8_t)(h3 >> 24);
}

static void poly1305_mac(const uint8_t key[32], const uint8_t *m, size_t len,
                         uint8_t tag[16]) {
  poly1305_state st;
  poly1305_init(&st, key);

  // Process full blocks
  size_t blocks = len & ~15;
  if (blocks > 0) {
    poly1305_blocks(&st, m, blocks, 1 << 24);
  }

  // Process remaining bytes
  if (len & 15) {
    uint8_t final_block[16] = {0};
    size_t rem = len & 15;
    memcpy(final_block, m + blocks, rem);
    final_block[rem] = 1;
    poly1305_blocks(&st, final_block, 16, 0);
  }

  poly1305_finish(&st, tag);
}

// ============================================================================
// ChaCha20-Poly1305 AEAD (RFC 8439)
// ============================================================================

// Generate Poly1305 key from ChaCha20 key and nonce
static void poly1305_key_gen(const uint8_t key[32], const uint8_t nonce[12],
                             uint8_t poly_key[32]) {
  uint32_t state[16];
  uint32_t block[16];

  chacha20_init_state(state, key, nonce, 0);
  chacha20_block(block, state);

  // First 32 bytes of keystream
  for (int i = 0; i < 8; i++) {
    poly_key[i * 4] = (uint8_t)block[i];
    poly_key[i * 4 + 1] = (uint8_t)(block[i] >> 8);
    poly_key[i * 4 + 2] = (uint8_t)(block[i] >> 16);
    poly_key[i * 4 + 3] = (uint8_t)(block[i] >> 24);
  }
}

// Pad16: pad data to 16-byte boundary and update MAC
static void poly1305_pad16(poly1305_state *st, size_t len) {
  if (len & 15) {
    uint8_t zero[16] = {0};
    size_t pad = 16 - (len & 15);
    poly1305_blocks(st, zero, 16, 0);
  }
}

// ============================================================================
// Public API
// ============================================================================

int crypto_init(CryptoContext *ctx, const char *hex_key) {
  if (!ctx || !hex_key || strlen(hex_key) != 64) {
    return -1;
  }

  if (hex_to_bytes(hex_key, ctx->key, CRYPTO_KEY_SIZE) != CRYPTO_KEY_SIZE) {
    return -1;
  }

  return 0;
}

int crypto_encrypt(CryptoContext *ctx, const uint8_t *plaintext,
                   size_t plaintext_len, uint8_t *output, size_t output_size) {

  size_t required_size = CRYPTO_NONCE_SIZE + plaintext_len + CRYPTO_TAG_SIZE;
  if (output_size < required_size) {
    return -1;
  }

  // Generate random nonce
  uint8_t *nonce = output;
  random_bytes(nonce, CRYPTO_NONCE_SIZE);

  // Encrypt (counter starts at 1 for encryption)
  uint8_t *ciphertext = output + CRYPTO_NONCE_SIZE;
  chacha20_crypt(ctx->key, nonce, 1, plaintext, ciphertext, plaintext_len);

  // Generate Poly1305 key
  uint8_t poly_key[32];
  poly1305_key_gen(ctx->key, nonce, poly_key);

  // Build Poly1305 input: AAD (none) || pad || ciphertext || pad || len_aad ||
  // len_ct For us: just ciphertext || pad || 0 || len_ct
  size_t aad_len = 0;
  uint8_t len_block[16];

  // Little-endian lengths
  memset(len_block, 0, 16);
  len_block[0] = (uint8_t)(aad_len);
  len_block[1] = (uint8_t)(aad_len >> 8);
  len_block[2] = (uint8_t)(aad_len >> 16);
  len_block[3] = (uint8_t)(aad_len >> 24);
  len_block[8] = (uint8_t)(plaintext_len);
  len_block[9] = (uint8_t)(plaintext_len >> 8);
  len_block[10] = (uint8_t)(plaintext_len >> 16);
  len_block[11] = (uint8_t)(plaintext_len >> 24);

  // Compute tag
  poly1305_state st;
  poly1305_init(&st, poly_key);

  // AAD (none, but need to pad)
  // poly1305_pad16(&st, 0);  // No AAD

  // Ciphertext
  size_t ct_blocks = plaintext_len & ~15;
  if (ct_blocks > 0) {
    poly1305_blocks(&st, ciphertext, ct_blocks, 1 << 24);
  }
  if (plaintext_len & 15) {
    uint8_t final_block[16] = {0};
    memcpy(final_block, ciphertext + ct_blocks, plaintext_len & 15);
    poly1305_blocks(&st, final_block, 16, 1 << 24);
  }

  // Length block
  poly1305_blocks(&st, len_block, 16, 1 << 24);

  // Finalize
  uint8_t *tag = output + CRYPTO_NONCE_SIZE + plaintext_len;
  poly1305_finish(&st, tag);

  return (int)required_size;
}

int crypto_decrypt(CryptoContext *ctx, const uint8_t *input, size_t input_len,
                   uint8_t *output, size_t output_size) {

  if (input_len < CRYPTO_NONCE_SIZE + CRYPTO_TAG_SIZE) {
    return -1;
  }

  size_t plaintext_len = input_len - CRYPTO_NONCE_SIZE - CRYPTO_TAG_SIZE;
  if (output_size < plaintext_len) {
    return -1;
  }

  const uint8_t *nonce = input;
  const uint8_t *ciphertext = input + CRYPTO_NONCE_SIZE;
  const uint8_t *tag = input + input_len - CRYPTO_TAG_SIZE;

  // Generate Poly1305 key
  uint8_t poly_key[32];
  poly1305_key_gen(ctx->key, nonce, poly_key);

  // Verify tag
  size_t aad_len = 0;
  uint8_t len_block[16];
  memset(len_block, 0, 16);
  len_block[0] = (uint8_t)(aad_len);
  len_block[1] = (uint8_t)(aad_len >> 8);
  len_block[2] = (uint8_t)(aad_len >> 16);
  len_block[3] = (uint8_t)(aad_len >> 24);
  len_block[8] = (uint8_t)(plaintext_len);
  len_block[9] = (uint8_t)(plaintext_len >> 8);
  len_block[10] = (uint8_t)(plaintext_len >> 16);
  len_block[11] = (uint8_t)(plaintext_len >> 24);

  poly1305_state st;
  poly1305_init(&st, poly_key);

  size_t ct_blocks = plaintext_len & ~15;
  if (ct_blocks > 0) {
    poly1305_blocks(&st, ciphertext, ct_blocks, 1 << 24);
  }
  if (plaintext_len & 15) {
    uint8_t final_block[16] = {0};
    memcpy(final_block, ciphertext + ct_blocks, plaintext_len & 15);
    poly1305_blocks(&st, final_block, 16, 1 << 24);
  }

  poly1305_blocks(&st, len_block, 16, 1 << 24);

  uint8_t computed_tag[16];
  poly1305_finish(&st, computed_tag);

  // Constant-time comparison
  uint8_t diff = 0;
  for (int i = 0; i < 16; i++) {
    diff |= tag[i] ^ computed_tag[i];
  }
  if (diff != 0) {
    return -1; // Authentication failed
  }

  // Decrypt
  chacha20_crypt(ctx->key, nonce, 1, ciphertext, output, plaintext_len);

  return (int)plaintext_len;
}
