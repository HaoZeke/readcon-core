/* Copyright (c) readcon developers */
#include "readcon-core.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static double now_s(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
}

static long vm_hwm_kib(void) {
    FILE *file = fopen("/proc/self/status", "r");
    char line[256];
    long value = 0;
    if (file == NULL) {
        return 0;
    }
    while (fgets(line, sizeof line, file) != NULL) {
        if (sscanf(line, "VmHWM: %ld", &value) == 1) {
            break;
        }
    }
    fclose(file);
    return value;
}

static uint64_t xor_f64(uint64_t acc, const double *values, size_t count) {
    size_t i;
    for (i = 0; i < count; i++) {
        uint64_t bits;
        memcpy(&bits, &values[i], sizeof bits);
        acc ^= bits;
    }
    return acc;
}

static int write_dump(const char *path, uint64_t n_frames, uint64_t n_atoms,
                      const double *pos, const double *lengths, const double *angles,
                      const int32_t *types) {
    FILE *file = fopen(path, "wb");
    uint64_t counts[2];
    if (file == NULL) {
        return 1;
    }
    counts[0] = n_frames;
    counts[1] = n_atoms;
    fwrite("H2H1", 1, 4, file);
    fwrite(counts, sizeof counts, 1, file);
    fwrite(pos, sizeof(double), (size_t)n_frames * (size_t)n_atoms * 3, file);
    fwrite(lengths, sizeof(double), (size_t)n_frames * 3, file);
    fwrite(angles, sizeof(double), (size_t)n_frames * 3, file);
    fwrite(types, sizeof(int32_t), (size_t)n_frames * (size_t)n_atoms, file);
    fclose(file);
    return 0;
}

static int materialize(struct RKRConFrame **frames, uintptr_t n_frames, double **pos,
                       double **lengths, double **angles, int32_t **types,
                       uintptr_t *n_atoms_out) {
    uintptr_t n_atoms = 0;
    uintptr_t frame_index;
    if (n_frames > 0) {
        n_atoms = rkr_frame_atom_count(frames[0]);
    }
    *n_atoms_out = n_atoms;
    *pos = calloc((size_t)n_frames * (size_t)n_atoms * 3, sizeof(double));
    *lengths = calloc((size_t)n_frames * 3, sizeof(double));
    *angles = calloc((size_t)n_frames * 3, sizeof(double));
    *types = calloc((size_t)n_frames * (size_t)n_atoms, sizeof(int32_t));
    if (*pos == NULL || *lengths == NULL || *angles == NULL || *types == NULL) {
        return 1;
    }
    for (frame_index = 0; frame_index < n_frames; frame_index++) {
        struct CFrame *view;
        uintptr_t atom;
        double *frame_pos = *pos + frame_index * n_atoms * 3;
        if (rkr_frame_copy_positions(frames[frame_index], frame_pos, n_atoms * 3) !=
            RKR_STATUS_SUCCESS) {
            return 2;
        }
        view = rkr_frame_to_c_frame(frames[frame_index]);
        if (view == NULL) {
            return 3;
        }
        memcpy(*lengths + frame_index * 3, view->cell, 3 * sizeof(double));
        memcpy(*angles + frame_index * 3, view->angles, 3 * sizeof(double));
        for (atom = 0; atom < view->num_atoms; atom++) {
            (*types)[frame_index * n_atoms + atom] = (int32_t)view->atoms[atom].atomic_number;
        }
        free_c_frame(view);
    }
    return 0;
}

int main(int argc, char **argv) {
    const char *op;
    const char *path;
    const char *out;
    uintptr_t threads;
    uintptr_t index;
    double t0;
    double elapsed;
    if (argc != 6) {
        fprintf(stderr, "usage: time_read op path out threads index\n");
        return 2;
    }
    op = argv[1];
    path = argv[2];
    out = argv[3];
    threads = (uintptr_t)strtoul(argv[4], NULL, 10);
    index = (uintptr_t)strtoul(argv[5], NULL, 10);
    if (strcmp(op, "read") == 0) {
        struct RKRConFrame **frames;
        uintptr_t n_frames = 0;
        uintptr_t n_atoms = 0;
        double *pos = NULL;
        double *lengths = NULL;
        double *angles = NULL;
        int32_t *types = NULL;
        int rc;
        t0 = now_s();
        frames = rkr_read_all_frames_n_threads(path, &n_frames, threads);
        if (frames == NULL) {
            fprintf(stderr, "read failed\n");
            return 1;
        }
        rc = materialize(frames, n_frames, &pos, &lengths, &angles, &types, &n_atoms);
        elapsed = now_s() - t0;
        if (rc != 0) {
            fprintf(stderr, "materialize %d\n", rc);
            return 1;
        }
        printf("elapsed_s %.17g\n", elapsed);
        printf("n_frames %llu\n", (unsigned long long)n_frames);
        printf("n_atoms %llu\n", (unsigned long long)n_atoms);
        printf("VmHWM_KiB %ld\n", vm_hwm_kib());
        if (write_dump(out, n_frames, n_atoms, pos, lengths, angles, types) != 0) {
            return 1;
        }
        free_rkr_frame_array(frames, n_frames);
        free(pos);
        free(lengths);
        free(angles);
        free(types);
        return 0;
    }
    if (strcmp(op, "random") == 0) {
        struct RKRConFrame *frame;
        struct RKRConFrame *frames[1];
        uintptr_t n_atoms = 0;
        double *pos = NULL;
        double *lengths = NULL;
        double *angles = NULL;
        int32_t *types = NULL;
        t0 = now_s();
        frame = rkr_read_nth_frame(path, index);
        if (frame == NULL) {
            fprintf(stderr, "nth failed\n");
            return 1;
        }
        frames[0] = frame;
        if (materialize(frames, 1, &pos, &lengths, &angles, &types, &n_atoms) != 0) {
            return 1;
        }
        elapsed = now_s() - t0;
        printf("elapsed_s %.17g\n", elapsed);
        printf("n_frames 1\n");
        printf("n_atoms %llu\n", (unsigned long long)n_atoms);
        printf("VmHWM_KiB %ld\n", vm_hwm_kib());
        if (write_dump(out, 1, n_atoms, pos, lengths, angles, types) != 0) {
            return 1;
        }
        free_rkr_frame(frame);
        free(pos);
        free(lengths);
        free(angles);
        free(types);
        return 0;
    }
    if (strcmp(op, "stream") == 0) {
        struct CConFrameIterator *iterator;
        uint64_t n_frames = 0;
        uint64_t n_atoms = 0;
        uint64_t pos_xor = 0;
        t0 = now_s();
        iterator = read_con_file_iterator(path);
        if (iterator == NULL) {
            fprintf(stderr, "iterator failed\n");
            return 1;
        }
        for (;;) {
            struct RKRConFrame *frame = con_frame_iterator_next(iterator);
            struct CFrame *view;
            uintptr_t atom;
            if (frame == NULL) {
                break;
            }
            view = rkr_frame_to_c_frame(frame);
            if (view == NULL) {
                return 1;
            }
            if (n_frames == 0) {
                n_atoms = view->num_atoms;
            }
            for (atom = 0; atom < view->num_atoms; atom++) {
                double coords[3] = {view->atoms[atom].x, view->atoms[atom].y, view->atoms[atom].z};
                pos_xor = xor_f64(pos_xor, coords, 3);
            }
            free_c_frame(view);
            free_rkr_frame(frame);
            n_frames += 1;
        }
        elapsed = now_s() - t0;
        free_con_frame_iterator(iterator);
        printf("elapsed_s %.17g\n", elapsed);
        printf("n_frames %llu\n", (unsigned long long)n_frames);
        printf("n_atoms %llu\n", (unsigned long long)n_atoms);
        printf("pos_xor %llu\n", (unsigned long long)pos_xor);
        printf("VmHWM_KiB %ld\n", vm_hwm_kib());
        return 0;
    }
    fprintf(stderr, "unknown op\n");
    return 2;
}
