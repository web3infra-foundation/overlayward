#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/stat.h>
#include <errno.h>
#include <fcntl.h>
#include <dirent.h>
#include "include/ow_linux.h"

static int write_file(const char *path, const char *content) {
    int fd = open(path, O_WRONLY);
    if (fd < 0) return -1;
    ssize_t n = write(fd, content, strlen(content));
    close(fd);
    return (n < 0) ? -1 : 0;
}

int ow_cgroup_create(const ow_cgroup_config_t *config) {
    if (!config || !config->path) {
        errno = EINVAL;
        return -1;
    }

    if (mkdir(config->path, 0755) != 0 && errno != EEXIST) {
        return -1;
    }

    char buf[256];

    if (config->memory_max > 0) {
        snprintf(buf, sizeof(buf), "%s/memory.max", config->path);
        char val[32];
        snprintf(val, sizeof(val), "%lu", (unsigned long)config->memory_max);
        if (write_file(buf, val) != 0) return -1;
    }

    if (config->memory_swap_max > 0) {
        snprintf(buf, sizeof(buf), "%s/memory.swap.max", config->path);
        char val[32];
        snprintf(val, sizeof(val), "%lu", (unsigned long)config->memory_swap_max);
        if (write_file(buf, val) != 0) return -1;
    }

    if (config->cpu_quota_us > 0 && config->cpu_period_us > 0) {
        snprintf(buf, sizeof(buf), "%s/cpu.max", config->path);
        char val[64];
        snprintf(val, sizeof(val), "%lu %lu",
                 (unsigned long)config->cpu_quota_us,
                 (unsigned long)config->cpu_period_us);
        if (write_file(buf, val) != 0) return -1;
    }

    return 0;
}

int ow_cgroup_attach(const char *path, pid_t pid) {
    if (!path || pid <= 0) {
        errno = EINVAL;
        return -1;
    }
    char buf[256];
    snprintf(buf, sizeof(buf), "%s/cgroup.procs", path);
    char val[32];
    snprintf(val, sizeof(val), "%d", pid);
    return write_file(buf, val);
}

int ow_cgroup_destroy(const char *path) {
    if (!path) {
        errno = EINVAL;
        return -1;
    }
    return rmdir(path);
}
