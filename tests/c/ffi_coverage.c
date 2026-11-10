/* Exercise the C ABI: builders, writers, readers, iterators, and null handles. */
#include "readcon-core.h"

#ifdef __cplusplus
using namespace readcon;
#endif

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures = 0;
static unsigned sink = 0;

static void use_status(enum RKRStatus status) { sink += (unsigned)status; }
static void use_u64(uint64_t value) { sink += (unsigned)value; }
static void use_ptr(const void *ptr) { sink += ptr ? 1u : 0u; }

static void free_text(char *text) {
    use_ptr(text);
    rkr_free_string(text);
}

static void touch_frame(const struct RKRConFrame *frame) {
    uintptr_t n = 0;
    struct RKRArrayView view;
    uint32_t bi = 0, bj = 0;
    uint8_t has_order = 0;
    int32_t order = 0;
    char header[256];
    struct CFrame *cframe;
    memset(&view, 0, sizeof(view));
    use_u64(rkr_frame_atom_index_by_id(frame, 0));
    use_u64(rkr_frame_spec_version(frame));
    free_text(rkr_frame_metadata_json(frame));
    use_u64((uint64_t)rkr_frame_energy(frame));
    use_u64((uint64_t)rkr_frame_index_energy(frame));
    free_text(rkr_frame_composition_formula(frame));
    use_u64((uint64_t)rkr_frame_total_mass(frame));
    use_u64((uint64_t)rkr_frame_cell_volume(frame));
    use_u64((uint64_t)rkr_frame_fmax(frame));
    use_u64(rkr_frame_sections_mask(frame));
    use_u64(rkr_frame_index_natoms(frame));
    free_text(rkr_frame_index_projection_json(frame));
    free_text(rkr_frame_potential_type(frame));
    use_u64(rkr_frame_frame_index(frame));
    use_u64((uint64_t)rkr_frame_time(frame));
    use_u64((uint64_t)rkr_frame_timestep(frame));
    use_u64(rkr_frame_neb_bead(frame));
    use_u64(rkr_frame_neb_band(frame));
    use_u64(rkr_frame_bond_count(frame));
    use_status(rkr_frame_bond_at(frame, 0, &bi, &bj, &has_order, &order));
    use_status(rkr_frame_get_header_line(frame, true, 0, header, sizeof(header)));
    use_status(rkr_frame_get_header_line(frame, false, 1, header, sizeof(header)));
    free_text(rkr_frame_get_header_line_cpp(frame, true, 0));
    cframe = rkr_frame_to_c_frame(frame);
    use_ptr(cframe);
    free_c_frame(cframe);
    use_u64(rkr_frame_atom_count(frame));
    use_status(rkr_frame_xyz_view(frame, &view));
    use_status(rkr_frame_velocities_view(frame, &view));
    use_status(rkr_frame_forces_view(frame, &view));
    use_u64(rkr_frame_has_displacements(frame));
    use_status(rkr_frame_displacements_view(frame, &view));
    use_u64(rkr_frame_has_spreads(frame));
    use_status(rkr_frame_spreads_view(frame, &view));
    use_status(rkr_frame_energies_view(frame, &view));
    use_status(rkr_frame_masses_view(frame, &view));
    use_status(rkr_frame_atom_ids_view(frame, &view));
    use_ptr(rkr_frame_xyz_f64(frame, &n));
    use_ptr(rkr_frame_velocities_f64(frame, &n));
}

static void touch_null_frame(void) {
    touch_frame(NULL);
    use_ptr(rkr_frame_binding_rewrite(NULL));
    free_rkr_frame(NULL);
}

static struct RKRConFrame *build_one(void) {
    const double cell[3] = {10.0, 10.0, 10.0};
    const double angles[3] = {90.0, 90.0, 90.0};
    const double vel[3] = {0.1, 0.0, -0.1};
    const double force[3] = {0.0, 0.2, 0.0};
    const double flat_short[3] = {0, 0, 0};
    const double flat[12] = {0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1};
    double xyz[3] = {0, 0, 0};
    double mass = 0;
    bool has_value = false;
    struct RKRConFrameBuilder *builder =
        rkr_frame_new(cell, angles, "comment", NULL, NULL, NULL);
    struct RKRConFrameBuilder *copy;
    struct RKRConFrame *frame;
    if (!builder) {
        fprintf(stderr, "rkr_frame_new returned null\n");
        return NULL;
    }
    use_status(rkr_frame_builder_set_metadata_json(
        builder, "{\"con_spec_version\":3,\"con_spec_minor\":1}"));
    use_status(rkr_frame_builder_set_units(
        builder, "{\"length\":\"angstrom\",\"energy\":\"eV\",\"mass\":\"amu\",\"time\":\"fs\"}"));
    use_status(rkr_frame_builder_set_con_spec_minor(builder, 1));
    use_status(rkr_frame_builder_set_energy(builder, -1.5));
    use_status(rkr_frame_builder_set_frame_index(builder, 3));
    use_status(rkr_frame_builder_set_time(builder, 1.25));
    use_status(rkr_frame_builder_set_timestep(builder, 0.001));
    use_status(rkr_frame_builder_set_neb_bead(builder, 2));
    use_status(rkr_frame_builder_set_neb_band(builder, 1));
    use_status(rkr_frame_add_atom(builder, "Cu", 0, 0, 0, false, 0, 63.5));
    use_status(rkr_frame_builder_set_last_velocity(builder, vel));
    use_status(rkr_frame_builder_set_last_force(builder, force));
    use_status(rkr_frame_add_atom_with_fixed_mask(builder, "H", 1, 0, 0, true, false, false, 1, 1.0));
    use_status(rkr_frame_add_atom_with_velocity(builder, "H", 0, 1, 0, false, 2, 1.0, vel[0], vel[1], vel[2]));
    use_status(rkr_frame_add_atom_full(builder, "Pt", 0, 0, 1, false, false, true, 3, 195.0, vel, force));
    use_u64(rkr_frame_builder_atom_count(builder));
    use_status(rkr_frame_builder_set_atom_position(builder, 0, 0.2, 0.3, 0.4));
    use_status(rkr_frame_builder_set_atom_velocity(builder, 0, vel));
    use_status(rkr_frame_builder_set_atom_force(builder, 0, force));
    use_status(rkr_frame_builder_set_atom_id(builder, 1, 9));
    use_status(rkr_frame_builder_set_atom_mass(builder, 3, 196.0));
    use_status(rkr_frame_builder_set_atom_fixed(builder, 1, true, false, true));
    use_status(rkr_frame_builder_get_atom_position(builder, 0, xyz));
    use_status(rkr_frame_builder_get_atom_velocity(builder, 0, xyz, &has_value));
    use_status(rkr_frame_builder_get_atom_mass(builder, 1, &mass));
    use_status(rkr_frame_builder_set_positions_from_flat(builder, flat_short, 1));
    use_status(rkr_frame_builder_set_positions_from_flat(builder, flat, 12));
    use_ptr(rkr_frame_builder_positions_data(builder));
    use_ptr(rkr_frame_builder_velocities_data(builder));
    use_ptr(rkr_frame_builder_forces_data(builder));
    use_ptr(rkr_frame_builder_masses_data(builder));
    use_ptr(rkr_frame_builder_atom_ids_data(builder));
    use_status(rkr_frame_builder_set_last_displacement(builder, vel));
    use_status(rkr_frame_builder_set_last_spread(builder, force));
    use_status(rkr_frame_builder_set_last_energy(builder, -0.25));
    use_status(rkr_frame_builder_set_forces_from_flat(builder, flat, 12));
    use_status(rkr_frame_builder_set_displacements_from_flat(builder, flat, 12));
    use_status(rkr_frame_builder_set_spreads_from_flat(builder, flat, 12));
    use_status(rkr_frame_builder_set_atom_energies_from_flat(builder, flat, 4));
    use_status(rkr_frame_builder_set_atom_displacement(builder, 0, vel));
    use_status(rkr_frame_builder_set_atom_spread(builder, 0, force));
    use_status(rkr_frame_builder_set_atom_energy(builder, 0, -0.1));
    use_status(rkr_frame_builder_get_atom_force(builder, 0, xyz, &has_value));
    use_status(rkr_frame_builder_get_atom_displacement(builder, 0, xyz, &has_value));
    use_status(rkr_frame_builder_get_atom_spread(builder, 0, xyz, &has_value));
    use_status(rkr_frame_builder_get_atom_energy(builder, 0, &mass, &has_value));
    use_ptr(rkr_frame_builder_displacements_data(builder));
    use_ptr(rkr_frame_builder_spreads_data(builder));
    use_ptr(rkr_frame_builder_atom_energies_data(builder));
    use_status(rkr_frame_builder_clear_atom_force(builder, 0));
    use_status(rkr_frame_builder_clear_atom_velocity(builder, 1));
    use_status(rkr_frame_builder_clear_atom_displacement(builder, 1));
    use_status(rkr_frame_builder_clear_atom_spread(builder, 1));
    use_status(rkr_frame_builder_clear_atom_energy(builder, 1));
    copy = rkr_frame_builder_clone(builder);
    if (copy) {
        free_rkr_frame_builder(copy);
    }
    frame = rkr_frame_builder_build(builder);
    if (!frame) {
        fprintf(stderr, "rkr_frame_builder_build returned null, last=%s\n",
                rkr_last_error_message() ? rkr_last_error_message() : "(none)");
    }
    return frame;
}

static void null_builder(void) {
    double xyz[3] = {0, 0, 0};
    const double vel[3] = {0, 0, 0};
    use_status(rkr_frame_builder_set_last_velocity(NULL, vel));
    use_status(rkr_frame_builder_set_last_force(NULL, vel));
    use_status(rkr_frame_add_atom(NULL, "Cu", 0, 0, 0, false, 0, 1));
    use_u64(rkr_frame_builder_atom_count(NULL));
    use_status(rkr_frame_builder_get_atom_position(NULL, 0, xyz));
    use_ptr(rkr_frame_builder_positions_data(NULL));
    free_rkr_frame_builder(NULL);
    use_ptr(rkr_frame_builder_clone(NULL));
    use_ptr(rkr_frame_builder_build(NULL));
}

static void write_frames(const char *dir, struct RKRConFrame *frame) {
    char path[512];
    struct RKRConFrameWriter *writer;
    const struct RKRConFrame *frames[1];
    frames[0] = frame;
    snprintf(path, sizeof(path), "%s/out.con", dir);
    writer = create_writer_from_path_c(path);
    use_status(rkr_writer_set_canonical(writer, 1));
    use_u64(rkr_writer_is_canonical(writer));
    use_status(rkr_writer_extend(writer, frames, 1));
    use_status(rkr_writer_flush(writer));
    free_rkr_writer(writer);
    use_status(rkr_writer_set_canonical(NULL, 1));
    use_u64(rkr_writer_is_canonical(NULL));
    free_rkr_writer(NULL);
    use_ptr(create_writer_from_path_c(NULL));
    snprintf(path, sizeof(path), "%s/out.gz", dir);
    writer = create_writer_gzip_c(path);
    if (writer) {
        use_status(rkr_writer_extend(writer, frames, 1));
        use_status(rkr_writer_flush(writer));
        free_rkr_writer(writer);
    }
    snprintf(path, sizeof(path), "%s/out.zst", dir);
    writer = create_writer_zstd_c(path);
    if (writer) {
        use_status(rkr_writer_extend(writer, frames, 1));
        free_rkr_writer(writer);
    }
}

static void read_paths(const char *file) {
    uintptr_t n = 0;
    struct RKRConFrame **frames;
    struct RKRConFrame *one;
    struct CConFrameIterator *it;
    char *text;
    size_t len = 0;
    FILE *handle;
    frames = rkr_read_all_frames(file, &n);
    if (frames) {
        if (n > 0) {
            touch_frame(frames[0]);
        }
        free_rkr_frame_array(frames, n);
    }
    frames = rkr_read_all_frames_n_threads(file, &n, 1);
    if (frames) {
        free_rkr_frame_ptr_array(frames, n);
    }
    one = rkr_read_first_frame(file);
    free_rkr_frame(one);
    one = rkr_read_nth_frame(file, 0);
    free_rkr_frame(one);
    use_u64(rkr_count_frames(file));
    use_u64(rkr_count_frames(NULL));
    use_ptr(rkr_read_all_frames(NULL, &n));
    it = read_con_file_iterator(file);
    if (it) {
        one = con_frame_iterator_next(it);
        free_rkr_frame(one);
        use_status(con_frame_iterator_forward(it));
        use_u64(con_frame_iterator_skip(it, 1));
        free_con_frame_iterator(it);
    }
    handle = fopen(file, "rb");
    if (handle) {
        fseek(handle, 0, SEEK_END);
        len = (size_t)ftell(handle);
        fseek(handle, 0, SEEK_SET);
        text = (char *)malloc(len + 1);
        fread(text, 1, len, handle);
        text[len] = 0;
        fclose(handle);
        it = read_con_string_iterator(text);
        if (it) {
            free_con_frame_iterator(it);
        }
        it = read_con_buffer_iterator((const uint8_t *)text, len);
        if (it) {
            one = con_frame_iterator_nth(it, 0);
            free_rkr_frame(one);
            free_con_frame_iterator(it);
        }
        free(text);
    }
    use_ptr(read_con_string_iterator(NULL));
    use_ptr(read_con_file_iterator(NULL));
    free_con_frame_iterator(NULL);
    use_ptr(con_frame_iterator_next(NULL));
}

#ifdef __cplusplus
extern "C" {
#endif
int32_t con_frame_iterator_next_status(struct CConFrameIterator *iterator,
                                       struct RKRConFrame **frame_out);
#ifdef __cplusplus
}
#endif

static void export_one(enum RKRStatus status, RKRDLManagedTensorVersioned *tensor) {
    use_status(status);
    if (status == RKR_STATUS_SUCCESS && tensor) {
        rkr_dlpack_delete(tensor);
    }
}

static void cover_columns(struct RKRConFrame *frame, const char *path) {
    double out[64];
    uint64_t ids[16];
    uint64_t atoms[4];
    uint32_t natoms = 0;
    uint32_t size = 0;
    uint64_t written = 0;
    uintptr_t n = 0;
    uintptr_t need = 0;
    uintptr_t nframes = 0;
    uint8_t blob[4096];
    struct RKRSelectionResult *selected = NULL;
    RKRDLManagedTensorVersioned *tensor = NULL;
    struct RKRDlpackExportOptions opts;
    struct CConFrameIterator *it;
    struct RKRConFrame *got = NULL;
    memset(&opts, 0, sizeof(opts));
    opts.dtype.code = 2;
    opts.dtype.bits = 64;
    opts.dtype.lanes = 1;
    opts.device.device_type = 1;
    use_status(rkr_frame_copy_positions(frame, out, 64));
    use_status(rkr_frame_copy_positions(frame, out, 1));
    use_status(rkr_frame_copy_velocities(frame, out, 64));
    use_status(rkr_frame_copy_forces(frame, out, 64));
    use_status(rkr_frame_copy_displacements(frame, out, 64));
    use_status(rkr_frame_copy_spreads(frame, out, 64));
    use_status(rkr_frame_copy_atom_energies(frame, out, 64));
    use_status(rkr_frame_copy_masses(frame, out, 64));
    use_status(rkr_frame_copy_atom_ids(frame, ids, 16));
    use_status(rkr_frame_copy_positions(NULL, out, 64));
    use_ptr(rkr_frame_forces_f64(frame, &n));
    use_ptr(rkr_frame_displacements_f64(frame, &n));
    use_ptr(rkr_frame_spreads_f64(frame, &n));
    use_u64(rkr_has_chemfiles_support());
    use_u64(rkr_has_parallel_support());
    export_one(rkr_frame_positions_dlpack(frame, &tensor), tensor);
    tensor = NULL;
    export_one(rkr_frame_positions_dlpack_ex(frame, &opts, &tensor), tensor);
    tensor = NULL;
    export_one(rkr_frame_positions_as_dlpack(frame, 1, 0, 0, 1, 0, &tensor), tensor);
    tensor = NULL;
    export_one(rkr_frame_velocities_dlpack(frame, &tensor), tensor);
    tensor = NULL;
    export_one(rkr_frame_velocities_dlpack_ex(frame, NULL, &tensor), tensor);
    tensor = NULL;
    export_one(rkr_frame_forces_dlpack(frame, &tensor), tensor);
    tensor = NULL;
    export_one(rkr_frame_forces_dlpack_ex(frame, NULL, &tensor), tensor);
    tensor = NULL;
    export_one(rkr_frame_displacements_dlpack(frame, &tensor), tensor);
    tensor = NULL;
    export_one(rkr_frame_displacements_dlpack_ex(frame, NULL, &tensor), tensor);
    tensor = NULL;
    export_one(rkr_frame_spreads_dlpack(frame, &tensor), tensor);
    tensor = NULL;
    export_one(rkr_frame_spreads_dlpack_ex(frame, NULL, &tensor), tensor);
    tensor = NULL;
    export_one(rkr_frame_atom_energies_dlpack(frame, &tensor), tensor);
    tensor = NULL;
    export_one(rkr_frame_atom_energies_dlpack_ex(frame, NULL, &tensor), tensor);
    tensor = NULL;
    rkr_dlpack_delete(NULL);
    use_status(rkr_frame_select(frame, "name Cu", &selected));
    use_u64(rkr_selection_result_match_count(selected));
    use_u64(rkr_selection_result_context_size(selected));
    use_status(rkr_selection_result_match_at(selected, 0, atoms, &size));
    use_status(rkr_selection_result_primary_indices(selected, ids, 16, &written));
    rkr_selection_result_free(selected);
    use_status(rkr_frame_select(NULL, "name Cu", &selected));
    rkr_selection_result_free(NULL);
    use_ptr(rkr_read_chemfiles_first(path));
    use_ptr(rkr_read_chemfiles(path, &nframes));
    use_ptr(rkr_read_chemfiles_nth(path, 0));
    use_u64(rkr_chemfiles_nsteps(path));
    use_ptr(rkr_read_chemfiles_range(path, 0, 1, 1, NULL, 0, &nframes));
    use_ptr(rkr_read_chemfiles_memory("Cu 0 0 0\n", "XYZ", &nframes));
    use_status(rkr_pack_rcso(frame, NULL, 0, &need));
    use_status(rkr_pack_rcso(frame, blob, sizeof(blob), &need));
    use_status(rkr_pack_rcso(frame, blob, 1, &need));
    use_status(rkr_unpack_rcso_natoms(blob, sizeof(blob), &natoms));
    use_status(rkr_unpack_rcso_positions(blob, sizeof(blob), out, 4));
    use_status(rkr_unpack_rcso_forces(blob, sizeof(blob), out, 4));
    it = read_con_file_iterator(path);
    if (it) {
        use_u64((uint64_t)con_frame_iterator_next_status(it, &got));
        free_rkr_frame(got);
        use_u64((uint64_t)con_frame_iterator_next_status(NULL, &got));
        free_con_frame_iterator(it);
    }
}

static void stage(const char *name) { fprintf(stderr, "stage %s\n", name); }

int main(int argc, char **argv) {
    const char *root = argc > 1 ? argv[1] : ".";
    const char *scratch = argc > 2 ? argv[2] : ".";
    char path[512];
    struct RKRConFrame *frame;
    struct RKRTrajectory *traj;
    unsigned i;
    stage("abi");
    use_u64(rkr_abi_version_major());
    use_u64(rkr_abi_version_minor());
    use_u64(rkr_abi_layout_revision());
    use_ptr(rkr_abi_stamp());
    use_u64(rkr_con_spec_version());
    use_ptr(rkr_library_version());
    for (i = 0; i < 100; i++) {
        use_u64(rkr_symbol_to_z(rkr_z_to_symbol(i)));
    }
    use_u64(rkr_symbol_to_z(NULL));
    use_u64(rkr_symbol_to_z("nope"));
    use_ptr(rkr_status_message(RKR_STATUS_SUCCESS));
    use_ptr(rkr_status_message(RKR_STATUS_NULL_POINTER));
    use_ptr(rkr_status_message(RKR_STATUS_INVALID_UTF8));
    use_ptr(rkr_status_message(RKR_STATUS_INVALID_JSON));
    use_ptr(rkr_status_message(RKR_STATUS_IO_ERROR));
    use_ptr(rkr_status_message(RKR_STATUS_INDEX_OUT_OF_BOUNDS));
    use_ptr(rkr_status_message(RKR_STATUS_BUFFER_TOO_SMALL));
    use_ptr(rkr_status_message(RKR_STATUS_INTERNAL_ERROR));
    use_ptr(rkr_status_message(RKR_STATUS_SECTION_ABSENT));
    use_ptr(rkr_status_message(RKR_STATUS_VALIDATION_ERROR));
    use_ptr(rkr_status_message(RKR_STATUS_SELECTION_ERROR));
    use_ptr(rkr_status_message(RKR_STATUS_FEATURE_DISABLED));
    use_ptr(rkr_status_message(RKR_STATUS_DEVICE_MISMATCH));
    use_ptr(rkr_status_message(RKR_STATUS_DEVICE_ALLOC_UNSUPPORTED));
    use_ptr(rkr_last_error_message());
    stage("null-frame");
    touch_null_frame();
    stage("null-builder");
    null_builder();
    stage("build");
    frame = build_one();
    if (!frame) {
        fprintf(stderr, "builder returned null\n");
        return 1;
    }
    stage("touch");
    touch_frame(frame);
    snprintf(path, sizeof(path), "%s/resources/test/tiny_cuh2.con", root);
    stage("columns");
    cover_columns(frame, path);
    {
        struct RKRConFrame *rewritten = rkr_frame_binding_rewrite(frame);
        if (rewritten) {
            touch_frame(rewritten);
            free_rkr_frame(rewritten);
        }
    }
    stage("write");
    write_frames(scratch, frame);
    stage("read");
    snprintf(path, sizeof(path), "%s/resources/test/tiny_cuh2.con", root);
    read_paths(path);
    snprintf(path, sizeof(path), "%s/resources/test/tiny_multi_cuh2.con", root);
    read_paths(path);
    traj = rkr_trajectory_open(path, 0);
    if (traj) {
        struct RKRConFrame *got = NULL;
        use_u64(rkr_trajectory_len(traj));
        use_status(rkr_trajectory_read_frame(traj, 0, &got));
        if (got) {
            touch_frame(got);
            free_rkr_frame(got);
        }
        rkr_trajectory_free(traj);
    }
    use_status(rkr_trajectory_read_frame(NULL, 0, NULL));
    use_u64(rkr_trajectory_len(NULL));
    rkr_trajectory_free(NULL);
    free_rkr_frame(frame);
    printf("ffi_coverage sink %u failures %d\n", sink, failures);
    return failures ? 1 : 0;
}
