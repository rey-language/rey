ifeq ($(filter undefined default,$(origin CC)), $(origin CC))
CC := /usr/bin/clang
endif
ifeq ($(filter undefined default,$(origin AR)), $(origin AR))
AR := /usr/bin/ar
endif
ifeq ($(filter undefined default,$(origin ARFLAGS)), $(origin ARFLAGS))
ARFLAGS := rcs
endif
ifeq ($(origin CFLAGS),undefined)
CFLAGS := -O3 -flto -Wall -Wextra -Wno-unused-parameter -std=c11
endif

# Objects go to OUT, never beside these sources. BUILD_DIR is the older alias.
ifeq ($(origin OUT),undefined)
  ifeq ($(origin BUILD_DIR),undefined)
    OUT := ../../build/runtime
  else
    OUT := $(BUILD_DIR)
  endif
else ifneq ($(origin BUILD_DIR),undefined)
  ifneq ($(OUT),$(BUILD_DIR))
    $(error OUT ($(OUT)) and BUILD_DIR ($(BUILD_DIR)) disagree)
  endif
endif
BUILD_DIR := $(OUT)
LIB = $(OUT)/librey_rt.a
SRCS = mem.c string.c vec.c hashmap.c result.c enum.c io.c value.c math.c

.PHONY: all librey_rt.a test clean

all: rey_rt.h $(SRCS)
	@set -eu; \
	mkdir -p "$(OUT)"; \
	runtime_dir=$$(pwd -P); \
	build_dir=$$(cd "$(OUT)" && pwd -P); \
	case "$$build_dir/" in "$$runtime_dir/"*) \
		echo "error: OUT must be outside the runtime source directory" >&2; \
		rmdir "$$build_dir" 2>/dev/null || true; \
		exit 2 ;; \
	esac; \
	for source in $(SRCS); do \
		object="$(OUT)/$${source%.c}.o"; \
		$(CC) $(CFLAGS) -I. -c "$$source" -o "$$object"; \
	done
	$(AR) $(ARFLAGS) "$(LIB)" \
		"$(OUT)/mem.o" \
		"$(OUT)/string.o" \
		"$(OUT)/vec.o" \
		"$(OUT)/hashmap.o" \
		"$(OUT)/result.o" \
		"$(OUT)/enum.o" \
		"$(OUT)/io.o" \
		"$(OUT)/value.o" \
		"$(OUT)/math.o"

librey_rt.a: all

test: all test_rt.c
	$(CC) $(CFLAGS) -I. test_rt.c "$(LIB)" -lm -o "$(OUT)/test_rt"
	"$(OUT)/test_rt"

clean:
	@set -eu; \
	if [ ! -d "$(OUT)" ]; then exit 0; fi; \
	runtime_dir=$$(pwd -P); \
	build_dir=$$(cd "$(OUT)" && pwd -P); \
	case "$$build_dir/" in "$$runtime_dir/"*) \
		echo "error: refusing to clean inside the runtime source directory" >&2; exit 2 ;; \
	esac; \
	rm -f \
		"$$build_dir/librey_rt.a" \
		"$$build_dir/mem.o" \
		"$$build_dir/string.o" \
		"$$build_dir/vec.o" \
		"$$build_dir/hashmap.o" \
		"$$build_dir/result.o" \
		"$$build_dir/enum.o" \
		"$$build_dir/io.o" \
		"$$build_dir/value.o" \
		"$$build_dir/math.o" \
		"$$build_dir/test_rt"
