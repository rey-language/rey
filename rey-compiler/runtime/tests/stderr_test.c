#define _POSIX_C_SOURCE 200809L
#include "../rey_rt.h"
#include <errno.h>
#include <poll.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

#define CHECK(cond, name) do { \
    if (!(cond)) { fprintf(stderr, "FAIL: %s\n", name); exit(1); } \
} while (0)

static void capture_child_output(unsigned char* out, size_t out_cap, size_t* out_len,
                                 unsigned char* err, size_t err_cap, size_t* err_len,
                                 int* overflow) {
    int out_pipe[2], err_pipe[2];
    CHECK(pipe(out_pipe) == 0, "stdout pipe");
    CHECK(pipe(err_pipe) == 0, "stderr pipe");
    pid_t child = fork();
    CHECK(child >= 0, "fork print child");
    if (child == 0) {
        close(out_pipe[0]);
        close(err_pipe[0]);
        if (dup2(out_pipe[1], STDOUT_FILENO) < 0 || dup2(err_pipe[1], STDERR_FILENO) < 0) _exit(90);
        close(out_pipe[1]);
        close(err_pipe[1]);

        const unsigned char first[] = {'A', 0, 0x80, 'Z'};
        const unsigned char second[] = {'B', 0, 0xff};
        rey_eprint(rey_str_new((const char*)first, (int64_t)sizeof(first)));
        rey_eprintln(rey_str_new((const char*)second, (int64_t)sizeof(second)));
        rey_eprint(rey_str_empty());
        rey_eprint(NULL);
        rey_eprintln(NULL);
        fflush(stdout);
        _exit(0);
    }

    close(out_pipe[1]);
    close(err_pipe[1]);
    int fds_open[2] = {1, 1};
    int fds[2] = {-1, -1};
    fds[0] = out_pipe[0];
    fds[1] = err_pipe[0];
    *out_len = 0;
    *err_len = 0;
    *overflow = 0;

    while (fds_open[0] || fds_open[1]) {
        struct pollfd poll_fds[2] = {
            {fds_open[0] ? fds[0] : -1, POLLIN, 0},
            {fds_open[1] ? fds[1] : -1, POLLIN, 0}
        };
        int ready = poll(poll_fds, 2, -1);
        if (ready < 0 && errno == EINTR) continue;
        CHECK(ready >= 0, "poll captured output");
        for (int i = 0; i < 2; i++) {
            if (!fds_open[i] || !(poll_fds[i].revents & (POLLIN | POLLHUP | POLLERR | POLLNVAL))) continue;
            unsigned char chunk[128];
            ssize_t n = read(fds[i], chunk, sizeof(chunk));
            if (n == 0) {
                close(fds[i]);
                fds_open[i] = 0;
                continue;
            }
            if (n < 0 && errno == EINTR) continue;
            CHECK(n > 0, "read captured output");
            unsigned char* dest = i == 0 ? out : err;
            size_t cap = i == 0 ? out_cap : err_cap;
            size_t* len = i == 0 ? out_len : err_len;
            size_t bytes = (size_t)n;
            if (bytes > cap - *len) {
                *overflow = 1;
            } else {
                memcpy(dest + *len, chunk, bytes);
                *len += bytes;
            }
        }
    }

    int status = 0;
    pid_t waited;
    do { waited = waitpid(child, &status, 0); } while (waited < 0 && errno == EINTR);
    CHECK(waited == child, "wait for print child");
    CHECK(WIFEXITED(status) && WEXITSTATUS(status) == 0, "print child exited successfully");
}

int main(void) {
    unsigned char stdout_bytes[256], stderr_bytes[256];
    size_t stdout_len, stderr_len;
    int overflow;
    capture_child_output(stdout_bytes, sizeof(stdout_bytes), &stdout_len,
                         stderr_bytes, sizeof(stderr_bytes), &stderr_len, &overflow);

    const unsigned char expected[] = {'A', 0, 0x80, 'Z', 'B', 0, 0xff, '\n', '\n'};
    CHECK(!overflow, "captured output fits test buffers");
    CHECK(stdout_len == 0, "eprint/eprintln leave stdout empty");
    CHECK(stderr_len == sizeof(expected), "stderr byte count and newline count");
    CHECK(memcmp(stderr_bytes, expected, sizeof(expected)) == 0, "stderr bytes preserved exactly");
    puts("PASS: eprint/eprintln use stderr, preserve bytes, and append exactly one newline");
    return 0;
}
