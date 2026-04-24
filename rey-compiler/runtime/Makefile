CC      = clang
CFLAGS  = -O2 -Wall -Wextra -Wno-unused-parameter -std=c11
AR      = ar
ARFLAGS = rcs

SRCS = mem.c string.c vec.c hashmap.c result.c io.c
OBJS = $(SRCS:.c=.o)
LIB  = librey_rt.a

.PHONY: all clean test

all: $(LIB)

$(LIB): $(OBJS)
	$(AR) $(ARFLAGS) $@ $^

%.o: %.c rey_rt.h
	$(CC) $(CFLAGS) -c $< -o $@

test: $(LIB) test_rt.c
	$(CC) $(CFLAGS) test_rt.c -L. -lrey_rt -o test_rt
	./test_rt

clean:
	rm -f $(OBJS) $(LIB) test_rt test_rt.o
