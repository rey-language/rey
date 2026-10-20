#include "rey_rt.h"
#include <errno.h>
#include <poll.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <sys/stat.h>
#include <unistd.h>
#include <dirent.h>
#if defined(__APPLE__)
#include <mach-o/dyld.h>
#endif

void rey_eprint(const char* s) {
    if (!s) return;
    int64_t len = rey_str_len(s);
    fwrite(s, 1, (size_t)len, stderr);
    fflush(stderr);
}

void rey_eprintln(const char* s) {
    rey_eprint(s);
    fputc('\n', stderr);
    fflush(stderr);
}

// --- file I/O ---

char* rey_read_file(const char* path) {
    if (!path) rey_runtime_error("readFile: cannot open");
    FILE* f = fopen(path, "rb");
    if (!f) rey_runtime_error("readFile: cannot open");
    struct stat st;
    if (fstat(fileno(f), &st) != 0 || !S_ISREG(st.st_mode)) {
        fclose(f);
        rey_runtime_error("readFile: cannot open");
    }
    if (st.st_size < 0 || (uint64_t)st.st_size > (uint64_t)INT64_MAX - 1) {
        fclose(f);
        rey_panic_cstr("readFile: failed to read length");
    }
    int64_t size = (int64_t)st.st_size;
    char* buf = (char*)rey_alloc(size + 1);
    int64_t got = 0;
    while (got < size) {
        size_t count = fread(buf + got, 1, (size_t)(size - got), f);
        if (count == 0) {
            free(buf);
            fclose(f);
            rey_runtime_error("readFile: cannot open");
        }
        got += (int64_t)count;
    }
    fclose(f);
    char* text = rey_str_new(buf, size);
    free(buf);
    return text;
}

void rey_write_file(const char* path, const char* content) {
    if (!path || !content) rey_panic_cstr("writeFile: cannot open for writing");
    FILE* f = fopen(path, "wb");
    if (!f) rey_panic_cstr("writeFile: cannot open for writing");
    int64_t len = rey_str_len(content);
    int64_t put = 0;
    while (put < len) {
        size_t count = fwrite(content + put, 1, (size_t)(len - put), f);
        if (count == 0) {
            fclose(f);
            rey_panic_cstr("writeFile: failed to write");
        }
        put += (int64_t)count;
    }
    if (fclose(f) != 0) rey_panic_cstr("writeFile: failed to write");
}

void rey_append_file(const char* path, const char* content) {
    FILE* f = fopen(path, "ab");
    if (!f) rey_panic_cstr("appendFile: cannot open");
    int64_t len = rey_str_len(content);
    fwrite(content, 1, (size_t)len, f);
    fclose(f);
}

int64_t rey_file_exists(const char* path) {
    struct stat st;
    return stat(path, &st) == 0 ? 1 : 0;
}

void rey_delete_file(const char* path) { remove(path); }

void rey_mkdir(const char* path) { mkdir(path, 0755); }

int64_t rey_list_dir(const char* path) {
    int64_t v = rey_vec_new();
    DIR* d = opendir(path);
    if (!d) return v;
    struct dirent* de;
    while ((de = readdir(d))) {
        if (strcmp(de->d_name, ".") == 0 || strcmp(de->d_name, "..") == 0) continue;
        rey_vec_push(v, (int64_t)(uintptr_t)rey_str_cstr(de->d_name));
    }
    closedir(d);
    return v;
}

// --- process ---

static int    g_argc = 0;
static char** g_argv = NULL;

void rey_init(int argc, char** argv) { g_argc = argc; g_argv = argv; }

static char* get_real_exe_path(const char* fallback) {
#if defined(__APPLE__)
    char buf[1024];
    uint32_t size = sizeof(buf);
    if (_NSGetExecutablePath(buf, &size) == 0) {
        char real[1024];
        if (realpath(buf, real) != NULL) {
            return strdup(real);
        }
        return strdup(buf);
    }
#elif defined(__linux__)
    char buf[1024];
    ssize_t len = readlink("/proc/self/exe", buf, sizeof(buf) - 1);
    if (len > 0) {
        buf[len] = '\0';
        return strdup(buf);
    }
#endif
    return strdup(fallback);
}

int64_t rey_args(void) {
    int64_t v = rey_vec_new();
    for (int i = 0; i < g_argc; i++) {
        if (i == 0 && g_argv != NULL && g_argv[0] != NULL) {
            char* real_exe = get_real_exe_path(g_argv[0]);
            rey_vec_push(v, (int64_t)(uintptr_t)rey_str_cstr(real_exe));
            free(real_exe);
        } else {
            rey_vec_push(v, (int64_t)(uintptr_t)rey_str_cstr(g_argv[i]));
        }
    }
    return v;
}

// Run through sh -c; success is stdout, failure includes status and both streams.
typedef struct {
    unsigned char* data;
    size_t len;
    size_t cap;
} ReyIoBuffer;

static int rey_io_append(ReyIoBuffer* b, const unsigned char* data, size_t len) {
    if (len > SIZE_MAX - b->len) return 0;
    size_t needed = b->len + len;
    if (needed > b->cap) {
        size_t cap = b->cap ? b->cap : 8192;
        while (cap < needed) {
            if (cap > SIZE_MAX / 2) { cap = needed; break; }
            cap *= 2;
        }
        unsigned char* grown = (unsigned char*)realloc(b->data, cap);
        if (!grown) return 0;
        b->data = grown;
        b->cap = cap;
    }
    if (len) memcpy(b->data + b->len, data, len);
    b->len = needed;
    return 1;
}

static void rey_io_close(int* fd) {
    if (*fd >= 0) {
        close(*fd);
        *fd = -1;
    }
}

static int64_t rey_exec_error(const char* operation, int error) {
    char message[256];
    snprintf(message, sizeof(message), "exec failed: %s: %s", operation, strerror(error));
    return rey_err_str(rey_str_cstr(message));
}

static int rey_io_append_text(ReyIoBuffer* b, const char* text) {
    return rey_io_append(b, (const unsigned char*)text, strlen(text));
}

static int64_t rey_exec_status_error(int status, const ReyIoBuffer* out, const ReyIoBuffer* err) {
    char status_text[128];
    if (WIFEXITED(status)) {
        snprintf(status_text, sizeof(status_text), "command failed (exit status %d)\n", WEXITSTATUS(status));
    } else if (WIFSIGNALED(status)) {
        snprintf(status_text, sizeof(status_text), "command terminated by signal %d\n", WTERMSIG(status));
    } else {
        snprintf(status_text, sizeof(status_text), "command failed (wait status %d)\n", status);
    }

    ReyIoBuffer message = {0};
    int ok = rey_io_append_text(&message, status_text) &&
             rey_io_append_text(&message, "stdout:\n") &&
             rey_io_append(&message, out->data, out->len) &&
             rey_io_append_text(&message, "\nstderr:\n") &&
             rey_io_append(&message, err->data, err->len);
    if (!ok || message.len > (size_t)(INT64_MAX - REY_STR_HDR - 1)) {
        free(message.data);
        return rey_err_str(rey_str_cstr("exec failed: out of memory formatting child result"));
    }
    char* error = rey_str_new((const char*)message.data, (int64_t)message.len);
    free(message.data);
    return rey_err_str(error);
}

static void rey_exec_forward_stderr(const ReyIoBuffer* err) {
    size_t written = 0;
    while (written < err->len) {
        size_t n = fwrite(err->data + written, 1, err->len - written, stderr);
        if (!n) break;
        written += n;
    }
    fflush(stderr);
}

static void rey_exec_child_error(int fd, const char* message, size_t len) {
    while (len) {
        ssize_t n = write(fd, message, len);
        if (n > 0) { message += n; len -= (size_t)n; }
        else if (n < 0 && errno == EINTR) continue;
        else break;
    }
    _exit(127);
}

int64_t rey_exec_cmd(const char* cmd) {
    if (!cmd) return rey_err_str(rey_str_cstr("exec failed: null command"));

    int out_pipe[2] = {-1, -1};
    int err_pipe[2] = {-1, -1};
    if (pipe(out_pipe) < 0) return rey_exec_error("pipe stdout", errno);
    if (pipe(err_pipe) < 0) {
        int error = errno;
        rey_io_close(&out_pipe[0]);
        rey_io_close(&out_pipe[1]);
        return rey_exec_error("pipe stderr", error);
    }

    pid_t child = fork();
    if (child < 0) {
        int error = errno;
        rey_io_close(&out_pipe[0]); rey_io_close(&out_pipe[1]);
        rey_io_close(&err_pipe[0]); rey_io_close(&err_pipe[1]);
        return rey_exec_error("fork", error);
    }
    if (child == 0) {
        close(out_pipe[0]);
        close(err_pipe[0]);
        if (dup2(out_pipe[1], STDOUT_FILENO) < 0)
            rey_exec_child_error(err_pipe[1], "exec failed: redirect stdout\n", sizeof("exec failed: redirect stdout\n") - 1);
        if (dup2(err_pipe[1], STDERR_FILENO) < 0)
            rey_exec_child_error(err_pipe[1], "exec failed: redirect stderr\n", sizeof("exec failed: redirect stderr\n") - 1);
        close(out_pipe[1]);
        close(err_pipe[1]);
        execl("/bin/sh", "sh", "-c", cmd, (char*)NULL);
        rey_exec_child_error(STDERR_FILENO, "exec failed: could not start /bin/sh\n", sizeof("exec failed: could not start /bin/sh\n") - 1);
    }

    rey_io_close(&out_pipe[1]);
    rey_io_close(&err_pipe[1]);
    ReyIoBuffer out = {0}, err = {0};
    int capture_error = 0;
    int out_of_memory = 0;

    while (out_pipe[0] >= 0 || err_pipe[0] >= 0) {
        struct pollfd fds[2] = {
            {out_pipe[0], POLLIN, 0},
            {err_pipe[0], POLLIN, 0}
        };
        int ready = poll(fds, 2, -1);
        if (ready < 0) {
            if (errno == EINTR) continue;
            capture_error = errno;
            kill(child, SIGTERM);
            kill(child, SIGKILL);
            break;
        }

        for (int i = 0; i < 2; i++) {
            int* fd = i == 0 ? &out_pipe[0] : &err_pipe[0];
            if (*fd < 0 || !(fds[i].revents & (POLLIN | POLLHUP | POLLERR | POLLNVAL))) continue;
            if (fds[i].revents & POLLNVAL) {
                capture_error = EBADF;
                rey_io_close(fd);
                kill(child, SIGTERM);
                kill(child, SIGKILL);
                continue;
            }

            unsigned char chunk[8192];
            ssize_t n = read(*fd, chunk, sizeof(chunk));
            if (n > 0) {
                ReyIoBuffer* dest = i == 0 ? &out : &err;
                if (!out_of_memory && !rey_io_append(dest, chunk, (size_t)n)) out_of_memory = 1;
            } else if (n == 0) {
                rey_io_close(fd);
            } else if (errno != EINTR && errno != EAGAIN && errno != EWOULDBLOCK) {
                capture_error = errno;
                rey_io_close(fd);
                kill(child, SIGTERM);
                kill(child, SIGKILL);
            }
        }
    }

    rey_io_close(&out_pipe[0]);
    rey_io_close(&err_pipe[0]);
    int status = 0;
    pid_t waited;
    do { waited = waitpid(child, &status, 0); } while (waited < 0 && errno == EINTR);
    if (waited < 0) {
        int error = errno;
        free(out.data); free(err.data);
        return rey_exec_error("waitpid", error);
    }
    if (capture_error) {
        free(out.data); free(err.data);
        return rey_exec_error("capture child output", capture_error);
    }
    if (out_of_memory) {
        free(out.data); free(err.data);
        return rey_err_str(rey_str_cstr("exec failed: out of memory capturing child output"));
    }
    if (WIFEXITED(status) && WEXITSTATUS(status) == 0) {
        if (out.len > (size_t)(INT64_MAX - REY_STR_HDR - 1)) {
            free(out.data); free(err.data);
            return rey_err_str(rey_str_cstr("exec failed: stdout exceeds string limit"));
        }
        rey_exec_forward_stderr(&err);
        char* result = rey_str_new((const char*)out.data, (int64_t)out.len);
        free(out.data); free(err.data);
        return rey_ok_str(result);
    }

    int64_t result = rey_exec_status_error(status, &out, &err);
    free(out.data); free(err.data);
    return result;
}

void rey_exit(int64_t code) { exit((int)code); }

char* rey_get_env(const char* name) {
    const char* v = getenv(name);
    return v ? rey_str_cstr(v) : rey_str_empty();
}

REY_NORETURN void rey_panic_cstr(const char* msg) {
    fprintf(stderr, "rey panic: %s\n", msg);
    exit(1);
}

REY_NORETURN void rey_runtime_error(const char* msg) {
    fprintf(stderr, "error: %s\n", msg ? msg : "(null)");
    exit(1);
}
