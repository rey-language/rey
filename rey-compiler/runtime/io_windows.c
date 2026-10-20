#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "rey_rt.h"

void rey_eprint(const char* s) {
    if (!s) return;
    fwrite(s, 1, (size_t)rey_str_len(s), stderr);
    fflush(stderr);
}

void rey_eprintln(const char* s) {
    rey_eprint(s);
    fputc('\n', stderr);
    fflush(stderr);
}

char* rey_read_file(const char* path) {
    if (!path) rey_runtime_error("readFile: cannot open");
    DWORD attributes = GetFileAttributesA(path);
    if (attributes == INVALID_FILE_ATTRIBUTES || (attributes & FILE_ATTRIBUTE_DIRECTORY))
        rey_runtime_error("readFile: cannot open");
    HANDLE file = CreateFileA(path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING,
                              FILE_ATTRIBUTE_NORMAL, NULL);
    if (file == INVALID_HANDLE_VALUE) rey_runtime_error("readFile: cannot open");
    LARGE_INTEGER size;
    if (!GetFileSizeEx(file, &size) || size.QuadPart < 0 || size.QuadPart > INT64_MAX - 1) {
        CloseHandle(file);
        rey_panic_cstr("readFile: failed to read length");
    }
    size_t length = (size_t)size.QuadPart;
    char* buffer = (char*)malloc(length ? length : 1);
    if (!buffer) {
        CloseHandle(file);
        rey_panic_cstr("out of memory");
    }
    size_t offset = 0;
    while (offset < length) {
        DWORD chunk = (DWORD)((length - offset) > 1048576 ? 1048576 : length - offset);
        DWORD received = 0;
        if (!ReadFile(file, buffer + offset, chunk, &received, NULL) || received == 0) {
            free(buffer);
            CloseHandle(file);
            rey_runtime_error("readFile: cannot open");
        }
        offset += received;
    }
    CloseHandle(file);
    char* result = rey_str_new(buffer, (int64_t)length);
    free(buffer);
    return result;
}

void rey_write_file(const char* path, const char* content) {
    if (!path || !content) rey_panic_cstr("writeFile: cannot open for writing");
    HANDLE file = CreateFileA(path, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS,
                              FILE_ATTRIBUTE_NORMAL, NULL);
    if (file == INVALID_HANDLE_VALUE) rey_panic_cstr("writeFile: cannot open for writing");
    size_t length = (size_t)rey_str_len(content), offset = 0;
    while (offset < length) {
        DWORD chunk = (DWORD)((length - offset) > 1048576 ? 1048576 : length - offset);
        DWORD written = 0;
        if (!WriteFile(file, content + offset, chunk, &written, NULL) || written == 0) {
            CloseHandle(file);
            rey_panic_cstr("writeFile: failed to write");
        }
        offset += written;
    }
    if (!CloseHandle(file)) rey_panic_cstr("writeFile: failed to write");
}

void rey_append_file(const char* path, const char* content) {
    if (!path || !content) rey_panic_cstr("appendFile: cannot open");
    HANDLE file = CreateFileA(path, FILE_APPEND_DATA, FILE_SHARE_READ, NULL, OPEN_ALWAYS,
                              FILE_ATTRIBUTE_NORMAL, NULL);
    if (file == INVALID_HANDLE_VALUE) rey_panic_cstr("appendFile: cannot open");
    size_t length = (size_t)rey_str_len(content), offset = 0;
    while (offset < length) {
        DWORD chunk = (DWORD)((length - offset) > 1048576 ? 1048576 : length - offset);
        DWORD written = 0;
        if (!WriteFile(file, content + offset, chunk, &written, NULL) || written == 0) {
            CloseHandle(file);
            rey_panic_cstr("appendFile: failed to write");
        }
        offset += written;
    }
    CloseHandle(file);
}

int64_t rey_file_exists(const char* path) {
    return path && GetFileAttributesA(path) != INVALID_FILE_ATTRIBUTES ? 1 : 0;
}

void rey_delete_file(const char* path) {
    if (path) DeleteFileA(path);
}

void rey_mkdir(const char* path) {
    if (path) CreateDirectoryA(path, NULL);
}

int64_t rey_list_dir(const char* path) {
    int64_t result = rey_vec_new();
    if (!path) return result;
    size_t length = strlen(path);
    char* pattern = (char*)malloc(length + 3);
    if (!pattern) return result;
    memcpy(pattern, path, length);
    if (length && path[length - 1] != '\\' && path[length - 1] != '/') pattern[length++] = '\\';
    pattern[length++] = '*';
    pattern[length] = '\0';
    WIN32_FIND_DATAA entry;
    HANDLE search = FindFirstFileA(pattern, &entry);
    free(pattern);
    if (search == INVALID_HANDLE_VALUE) return result;
    do {
        if (strcmp(entry.cFileName, ".") == 0 || strcmp(entry.cFileName, "..") == 0) continue;
        rey_vec_push(result, (int64_t)(uintptr_t)rey_str_cstr(entry.cFileName));
    } while (FindNextFileA(search, &entry));
    FindClose(search);
    return result;
}

static int g_argc = 0;
static char** g_argv = NULL;

void rey_init(int argc, char** argv) {
    g_argc = argc;
    g_argv = argv;
}

static char* rey_windows_exe_path(const char* fallback) {
    DWORD capacity = 512;
    for (;;) {
        char* buffer = (char*)malloc(capacity);
        if (!buffer) return rey_str_cstr(fallback ? fallback : "rey.exe");
        DWORD length = GetModuleFileNameA(NULL, buffer, capacity);
        if (length == 0) {
            free(buffer);
            return rey_str_cstr(fallback ? fallback : "rey.exe");
        }
        if (length < capacity - 1) {
            buffer[length] = '\0';
            char* result = rey_str_cstr(buffer);
            free(buffer);
            return result;
        }
        free(buffer);
        if (capacity >= 32768) return rey_str_cstr(fallback ? fallback : "rey.exe");
        capacity *= 2;
    }
}

int64_t rey_args(void) {
    int64_t result = rey_vec_new();
    for (int i = 0; i < g_argc; i++) {
        const char* value = g_argv[i] ? g_argv[i] : "";
        char* item = i == 0 ? rey_windows_exe_path(value) : rey_str_cstr(value);
        rey_vec_push(result, (int64_t)(uintptr_t)item);
    }
    return result;
}

typedef struct {
    unsigned char* data;
    size_t len;
    size_t cap;
} ReyWindowsBuffer;

static int rey_windows_append(ReyWindowsBuffer* buffer, const void* data, size_t length) {
    if (length > SIZE_MAX - buffer->len - 1) return 0;
    size_t needed = buffer->len + length + 1;
    if (needed > buffer->cap) {
        size_t capacity = buffer->cap ? buffer->cap : 8192;
        while (capacity < needed) {
            if (capacity > SIZE_MAX / 2) { capacity = needed; break; }
            capacity *= 2;
        }
        unsigned char* grown = (unsigned char*)realloc(buffer->data, capacity);
        if (!grown) return 0;
        buffer->data = grown;
        buffer->cap = capacity;
    }
    if (length) memcpy(buffer->data + buffer->len, data, length);
    buffer->len += length;
    buffer->data[buffer->len] = '\0';
    return 1;
}

static int64_t rey_windows_exec_error(const char* operation, DWORD code) {
    char message[256];
    snprintf(message, sizeof(message), "exec failed: %s (Windows error %lu)", operation, (unsigned long)code);
    return rey_err_str(rey_str_cstr(message));
}

int64_t rey_exec_cmd(const char* cmd) {
    if (!cmd) return rey_err_str(rey_str_cstr("exec failed: null command"));
    const char* shell = getenv("COMSPEC");
    if (!shell || !*shell) shell = "C:\\Windows\\System32\\cmd.exe";
    size_t shell_length = strlen(shell), command_length = strlen(cmd);
    if (shell_length > SIZE_MAX - command_length - 16 || shell_length + command_length + 16 > 32760)
        return rey_err_str(rey_str_cstr("exec failed: command is too long"));
    size_t command_line_size = shell_length + command_length + 16;
    char* command_line = (char*)malloc(command_line_size);
    if (!command_line) return rey_err_str(rey_str_cstr("exec failed: out of memory"));
    snprintf(command_line, command_line_size, "\"%s\" /D /C %s", shell, cmd);

    SECURITY_ATTRIBUTES security = {0};
    security.nLength = sizeof(security);
    security.bInheritHandle = TRUE;
    HANDLE read_pipe = NULL, write_pipe = NULL;
    if (!CreatePipe(&read_pipe, &write_pipe, &security, 0)) {
        DWORD error = GetLastError();
        free(command_line);
        return rey_windows_exec_error("CreatePipe", error);
    }
    SetHandleInformation(read_pipe, HANDLE_FLAG_INHERIT, 0);
    STARTUPINFOA startup = {0};
    startup.cb = sizeof(startup);
    startup.dwFlags = STARTF_USESTDHANDLES;
    startup.hStdInput = GetStdHandle(STD_INPUT_HANDLE);
    startup.hStdOutput = write_pipe;
    startup.hStdError = write_pipe;
    PROCESS_INFORMATION process = {0};
    BOOL started = CreateProcessA(shell, command_line, NULL, NULL, TRUE, CREATE_NO_WINDOW,
                                  NULL, NULL, &startup, &process);
    DWORD start_error = started ? ERROR_SUCCESS : GetLastError();
    free(command_line);
    CloseHandle(write_pipe);
    if (!started) {
        CloseHandle(read_pipe);
        return rey_windows_exec_error("CreateProcess", start_error);
    }

    ReyWindowsBuffer output = {0};
    unsigned char chunk[8192];
    int capture_failed = 0;
    for (;;) {
        DWORD received = 0;
        if (!ReadFile(read_pipe, chunk, sizeof(chunk), &received, NULL)) {
            if (GetLastError() == ERROR_BROKEN_PIPE) break;
            capture_failed = 1;
            break;
        }
        if (received == 0) break;
        if (!capture_failed && !rey_windows_append(&output, chunk, received)) capture_failed = 1;
    }
    CloseHandle(read_pipe);
    WaitForSingleObject(process.hProcess, INFINITE);
    DWORD exit_code = 1;
    GetExitCodeProcess(process.hProcess, &exit_code);
    CloseHandle(process.hThread);
    CloseHandle(process.hProcess);
    if (capture_failed) {
        free(output.data);
        return rey_err_str(rey_str_cstr("exec failed: could not capture child output"));
    }
    if (exit_code == 0) {
        char* result = output.data ? rey_str_cstr((const char*)output.data) : rey_str_empty();
        free(output.data);
        return rey_ok_str(result);
    }

    char status[96];
    snprintf(status, sizeof(status), "command failed (exit code %lu)\noutput:\n", (unsigned long)exit_code);
    ReyWindowsBuffer message = {0};
    int ok = rey_windows_append(&message, status, strlen(status)) &&
             rey_windows_append(&message, output.data ? output.data : (const unsigned char*)"", output.len);
    free(output.data);
    if (!ok) {
        free(message.data);
        return rey_err_str(rey_str_cstr("exec failed: out of memory formatting command error"));
    }
    int64_t result = rey_err_str(rey_str_cstr((const char*)message.data));
    free(message.data);
    return result;
}

void rey_exit(int64_t code) { ExitProcess((UINT)code); }

char* rey_get_env(const char* name) {
    const char* value = name ? getenv(name) : NULL;
    return value ? rey_str_cstr(value) : rey_str_empty();
}

REY_NORETURN void rey_panic_cstr(const char* message) {
    fprintf(stderr, "rey panic: %s\n", message ? message : "(null)");
    ExitProcess(1);
}

REY_NORETURN void rey_runtime_error(const char* message) {
    fprintf(stderr, "error: %s\n", message ? message : "(null)");
    ExitProcess(1);
}
