/* TNG through the utility reader of the TNG library, for files that store the
 * positions and the box every STRIDE frames, as the example files of the TNG
 * repository do: tng_util_box_shape_read and tng_util_pos_read, then every
 * stored frame from the arrays they return (mode all), or, one stored frame at
 * a time, tng_util_box_shape_read_range and tng_util_pos_read_range from that
 * frame to itself, the range call of tng_io_read_pos_util of the TNG tests
 * (mode frames).  Positions and box are binary32 nm.  The reader takes no
 * thread count.
 */
#include "bf_driver.h"

#include "tng/tng_io.h"

int main(int argc, char **argv) {
    BfRun run;
    bf_args(argc, argv, &run);
    bf_single_thread(&run);

    double t0 = bf_now();
    tng_trajectory_t traj = NULL;
    if (tng_util_trajectory_open(run.path, 'r', &traj) != TNG_SUCCESS) bf_die("tng_util_trajectory_open failed");
    int64_t particles = 0, frames = 0;
    if (tng_num_particles_get(traj, &particles) != TNG_SUCCESS || particles < 1 ||
        tng_num_frames_get(traj, &frames) != TNG_SUCCESS || frames < 1) {
        bf_die("tng_num_particles_get or tng_num_frames_get failed");
    }
    float *positions = NULL, *shape = NULL;
    if (run.mode == BF_FRAMES_MODE) {
        int64_t stride = 0;
        if (tng_data_get_stride_length(traj, TNG_TRAJ_POSITIONS, 0, &stride) != TNG_SUCCESS || stride < 1) {
            bf_die("tng_data_get_stride_length failed");
        }
        for (int64_t frame = 0; frame < frames; frame += stride) {
            int64_t pos_stride = 0, box_stride = 0;
            if (tng_util_box_shape_read_range(traj, frame, frame, &shape, &box_stride) != TNG_SUCCESS ||
                tng_util_pos_read_range(traj, frame, frame, &positions, &pos_stride) != TNG_SUCCESS) {
                bf_die("tng_util_box_shape_read_range or tng_util_pos_read_range failed");
            }
            if (pos_stride != stride || box_stride != stride) bf_die("positions and box differ in stride");
            double lengths[3] = {shape[0], shape[4], shape[8]};
            bf_frame_xyz(&run, positions, particles, lengths, 10.0);
            free(positions);
            free(shape);
            positions = NULL;
            shape = NULL;
        }
    } else {
        int64_t stride = 0, box_stride = 0;
        if (tng_util_box_shape_read(traj, &shape, &box_stride) != TNG_SUCCESS) bf_die("tng_util_box_shape_read failed");
        if (tng_util_pos_read(traj, &positions, &stride) != TNG_SUCCESS) bf_die("tng_util_pos_read failed");
        if (stride < 1 || box_stride != stride) bf_die("positions and box differ in stride");
        int64_t stored = (frames - 1) / stride + 1;
        for (int64_t f = 0; f < stored; f++) {
            const float *s = shape + 9 * f;
            double lengths[3] = {s[0], s[4], s[8]};
            bf_frame_xyz(&run, positions + 3 * particles * f, particles, lengths, 10.0);
        }
    }
    free(positions);
    free(shape);
    if (tng_util_trajectory_close(&traj) != TNG_SUCCESS) bf_die("tng_util_trajectory_close failed");
    bf_report(&run, bf_now() - t0);
    return 0;
}
