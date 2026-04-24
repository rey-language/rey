#include "rey_rt.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>
#include <dirent.h>

// --- file I/O ---

char* rey_read_file(const char* path) {
    FILE* f = fopen(path, "rb");
    if (!f) return rey_str_empty();
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    rewind(f);
    char* buf = (char*)rey_alloc(sz + 1);
    fread(buf, 1, (size_t)sz, f);
    fclose(f);
    char* s = rey_str_new(buf, sz);
    free(buf);
    return s;
}

void rey_write_file(const char* path, const char* content) {
    FILE* f = fopen(path, "wb");
    if (!f) rey_panic_cstr("writeFile: cannot open for writing");
    int64_t len = rey_str_len(content);
    fwrite(content, 1, (size_t)len, f);
    fclose(f);
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
        if (de->d_name[0] == '.') continue;
        rey_vec_push(v, (int64_t)(uintptr_t)rey_str_cstr(de->d_name));
    }
    closedir(d);
    return v;
}

// --- process ---

static int    g_argc = 0;
static char** g_argv = NULL;

void rey_init(int argc, char** argv) { g_argc = argc; g_argv = argv; }

int64_t rey_args(void) {
    int64_t v = rey_vec_new();
    for (int i = 0; i < g_argc; i++)
        rey_vec_push(v, (int64_t)(uintptr_t)rey_str_cstr(g_argv[i]));
    return v;
}

// Execute a shell command via popen. Returns Result<String,String> (i64).
int64_t rey_exec_cmd(const char* cmd) {
    FILE* fp = popen(cmd, "r");
    if (!fp) return rey_err_str(rey_str_cstr("exec failed: popen"));

    char buf[4096];
    char* out = rey_str_empty();
    while (fgets(buf, sizeof(buf), fp)) {
        char* chunk = rey_str_cstr(buf);
        out = rey_str_concat(out, chunk);
    }
    int status = pclose(fp);
    if (status != 0) return rey_err_str(out);
    return rey_ok_str(out);
}

void rey_exit(int64_t code) { exit((int)code); }

char* rey_get_env(const char* name) {
    const char* v = getenv(name);
    return v ? rey_str_cstr(v) : rey_str_empty();
}

void rey_panic_cstr(const char* msg) {
    fprintf(stderr, "rey panic: %s\n", msg);
    exit(1);
}
