#define _GNU_SOURCE
#include <sched.h>
#include <signal.h>
#include <unistd.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <errno.h>
#include <string.h>
#include <fcntl.h>
#include <stdio.h>
#include <linux/sched.h>
#include "include/ow_linux.h"

static long sys_clone3(struct clone_args *args, size_t size) {
    return syscall(SYS_clone3, args, size);
}

int ow_ns_create(const ow_ns_config_t *config, ow_ns_result_t *result) {
    if (!config || !result) {
        errno = EINVAL;
        return -1;
    }

    memset(result, 0, sizeof(*result));
    for (int i = 0; i < 7; i++) result->ns_fds[i] = -1;

    struct clone_args args = {0};
    args.flags = config->clone_flags & (
        CLONE_NEWPID | CLONE_NEWNET | CLONE_NEWNS |
        CLONE_NEWUTS | CLONE_NEWIPC | CLONE_NEWCGROUP
    );
    args.exit_signal = SIGCHLD;

    pid_t pid = (pid_t)sys_clone3(&args, sizeof(args));
    if (pid < 0) {
        return -1;
    }

    if (pid == 0) {
        pause();
        _exit(0);
    }

    result->init_pid = pid;

    const char *ns_names[] = {"pid", "net", "mnt", "uts", "ipc", "user", "cgroup"};
    char path[256];
    for (int i = 0; i < 7; i++) {
        snprintf(path, sizeof(path), "/proc/%d/ns/%s", pid, ns_names[i]);
        int fd = open(path, O_RDONLY);
        result->ns_fds[i] = fd;
    }

    return 0;
}

int ow_ns_enter(const ow_ns_result_t *ns, int ns_type) {
    if (!ns) {
        errno = EINVAL;
        return -1;
    }

    int idx = -1;
    int flags[] = {CLONE_NEWPID, CLONE_NEWNET, CLONE_NEWNS,
                   CLONE_NEWUTS, CLONE_NEWIPC, CLONE_NEWUSER, CLONE_NEWCGROUP};
    for (int i = 0; i < 7; i++) {
        if (flags[i] == ns_type) { idx = i; break; }
    }
    if (idx < 0 || ns->ns_fds[idx] < 0) {
        errno = EINVAL;
        return -1;
    }

    return setns(ns->ns_fds[idx], ns_type);
}

int ow_ns_destroy(const ow_ns_result_t *ns) {
    if (!ns) {
        errno = EINVAL;
        return -1;
    }

    for (int i = 0; i < 7; i++) {
        if (ns->ns_fds[i] >= 0) {
            close(ns->ns_fds[i]);
        }
    }

    if (ns->init_pid > 0) {
        kill(ns->init_pid, SIGKILL);
        waitpid(ns->init_pid, NULL, 0);
    }

    return 0;
}
