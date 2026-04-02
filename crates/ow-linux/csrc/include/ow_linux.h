#ifndef OW_LINUX_H
#define OW_LINUX_H

#include <sys/types.h>
#include <stdint.h>

// ── Namespace ──

typedef struct {
    int clone_flags;
    int userns_fd;
} ow_ns_config_t;

typedef struct {
    pid_t init_pid;
    int ns_fds[7];
} ow_ns_result_t;

int ow_ns_create(const ow_ns_config_t *config, ow_ns_result_t *result);
int ow_ns_enter(const ow_ns_result_t *ns, int ns_type);
int ow_ns_destroy(const ow_ns_result_t *ns);

// ── Cgroup v2 ──

typedef struct {
    const char *path;
    uint64_t cpu_quota_us;
    uint64_t cpu_period_us;
    uint64_t memory_max;
    uint64_t memory_swap_max;
} ow_cgroup_config_t;

int ow_cgroup_create(const ow_cgroup_config_t *config);
int ow_cgroup_attach(const char *path, pid_t pid);
int ow_cgroup_destroy(const char *path);

// ── Mount ──

int ow_pivot_root(const char *new_root, const char *put_old);
int ow_bind_mount(const char *src, const char *dst, int readonly);
int ow_tmpfs_mount(const char *dst, uint64_t size);

#endif // OW_LINUX_H
