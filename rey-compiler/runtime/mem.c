#include "rey_rt.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void* rey_alloc(int64_t size) {
    void* p = malloc((size_t)size);
    if (!p) { fprintf(stderr, "rey: out of memory\n"); exit(1); }
    memset(p, 0, (size_t)size);
    return p;
}
