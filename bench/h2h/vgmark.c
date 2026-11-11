/* Copyright (c) readcon-core developers */
/* Mark the start of a measured region for massif, and sample VmRSS. */
#include <fcntl.h>
#include <pthread.h>
#include <unistd.h>
#include <valgrind/valgrind.h>

static volatile int watch_stop;
static long watch_base;
static long watch_peak;
static pthread_t watch_thread;
static int watch_on;

static long status_kib(const char *key) {
    char buf[8192];
    int fd = open("/proc/self/status", O_RDONLY);
    long value = 0;
    int n;
    int i;
    int k;
    if (fd < 0) {
        return 0;
    }
    n = (int)read(fd, buf, sizeof buf - 1);
    close(fd);
    if (n < 0) {
        return 0;
    }
    buf[n] = 0;
    i = 0;
    while (i < n) {
        k = 0;
        while (key[k] && i + k < n && buf[i + k] == key[k]) {
            k++;
        }
        if (key[k] == 0) {
            i += k;
            while (i < n && (buf[i] == ' ' || buf[i] == '\t')) {
                i++;
            }
            while (i < n && buf[i] >= '0' && buf[i] <= '9') {
                value = value * 10 + (buf[i] - '0');
                i++;
            }
            return value;
        }
        while (i < n && buf[i] != '\n') {
            i++;
        }
        if (i < n) {
            i++;
        }
    }
    return 0;
}

static void *watch_fn(void *unused) {
    (void)unused;
    while (!watch_stop) {
        long rss = status_kib("VmRSS:");
        if (rss > watch_peak) {
            watch_peak = rss;
        }
        usleep(200);
    }
    return 0;
}

void watch_start(void) {
    watch_base = status_kib("VmRSS:");
    watch_peak = watch_base;
    watch_stop = 0;
    watch_on = pthread_create(&watch_thread, 0, watch_fn, 0) == 0;
}

/* Resident increase, in KiB, above the footprint at watch_start. */
long watch_delta_kib(void) {
    long rss;
    watch_stop = 1;
    if (watch_on) {
        pthread_join(watch_thread, 0);
        watch_on = 0;
    }
    rss = status_kib("VmRSS:");
    if (rss > watch_peak) {
        watch_peak = rss;
    }
    if (watch_peak < watch_base) {
        return 0;
    }
    return watch_peak - watch_base;
}

int vg_monitor(const char *command) {
    return (int)VALGRIND_MONITOR_COMMAND(command);
}
