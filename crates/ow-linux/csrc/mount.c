#define _GNU_SOURCE
#include <sys/mount.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include "include/ow_linux.h"

int ow_pivot_root(const char *new_root, const char *put_old) {
    if (!new_root || !put_old) {
        errno = EINVAL;
        return -1;
    }
    return syscall(SYS_pivot_root, new_root, put_old);
}

int ow_bind_mount(const char *src, const char *dst, int readonly) {
    if (!src || !dst) {
        errno = EINVAL;
        return -1;
    }

    mkdir(dst, 0755);

    unsigned long flags = MS_BIND | MS_REC;
    if (mount(src, dst, NULL, flags, NULL) != 0) {
        return -1;
    }

    if (readonly) {
        flags |= MS_REMOUNT | MS_RDONLY;
        if (mount(NULL, dst, NULL, flags, NULL) != 0) {
            return -1;
        }
    }

    return 0;
}

int ow_tmpfs_mount(const char *dst, uint64_t size) {
    if (!dst) {
        errno = EINVAL;
        return -1;
    }

    mkdir(dst, 0755);

    char opts[64];
    if (size > 0) {
        snprintf(opts, sizeof(opts), "size=%lu", (unsigned long)size);
    } else {
        opts[0] = '\0';
    }

    return mount("tmpfs", dst, "tmpfs", 0, size > 0 ? opts : NULL);
}
