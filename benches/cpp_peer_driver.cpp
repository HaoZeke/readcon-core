// One head-to-head process: read every frame and print seconds, frames and
// the sum of each x coordinate rounded to 0.01 angstrom.
//
// readcon-soa, readcon-frames, readcon-soa-all and readcon-frames-vel read a
// CON file through readcon::read_all_frames. chemfiles and chemfiles-vel read
// the peer trajectory. The rounded sum is the same number when the peer
// holds the same coordinates.

#include "readcon-core.hpp"

#include <chemfiles.hpp>

#include <chrono>
#include <cmath>
#include <cstdint>
#include <iostream>
#include <string>

namespace {

long long cents_of(double x) { return std::llround(x * 100.0); }

long long sum_readcon(const std::filesystem::path &path, std::size_t *frames) {
    auto loaded = readcon::read_all_frames(path, 1);
    *frames = loaded.size();
    long long cents = 0;
    for (const auto &frame : loaded) {
        const auto view = frame.xyz_view();
        const auto *data = static_cast<const double *>(view.data);
        const auto cols = view.cols == 0 ? 1u : view.cols;
        for (std::uintptr_t i = 0; i < view.n; ++i)
            cents += cents_of(data[i * cols]);
        // Touch velocities so a velocity file is not read as coordinates only.
        const auto vel = frame.velocities_view();
        if (vel.data != nullptr && vel.n > 0) {
            const auto *v = static_cast<const double *>(vel.data);
            volatile double sink = v[0];
            (void)sink;
        }
    }
    return cents;
}

long long sum_chemfiles(const std::string &path, std::size_t *frames) {
    chemfiles::Trajectory traj(path);
    *frames = traj.nsteps();
    long long cents = 0;
    for (std::size_t step = 0; step < traj.nsteps(); ++step) {
        const auto frame = traj.read_step(step);
        const auto positions = frame.positions();
        for (std::size_t i = 0; i < frame.size(); ++i)
            cents += cents_of(positions[i][0]);
    }
    return cents;
}

} // namespace

int main(int argc, char **argv) {
    if (argc != 3) {
        std::cerr << "usage: cpp_peer_driver MODE PATH\n";
        return 2;
    }
    const std::string mode = argv[1];
    const std::string path = argv[2];
    const auto start = std::chrono::steady_clock::now();
    std::size_t frames = 0;
    long long cents = 0;
    try {
        if (mode == "chemfiles" || mode == "chemfiles-vel")
            cents = sum_chemfiles(path, &frames);
        else if (mode == "readcon-soa" || mode == "readcon-frames" ||
                 mode == "readcon-soa-all" || mode == "readcon-frames-vel")
            cents = sum_readcon(path, &frames);
        else {
            std::cerr << "unknown mode " << mode << "\n";
            return 2;
        }
    } catch (const std::exception &err) {
        std::cerr << err.what() << "\n";
        return 1;
    }
    const auto seconds = std::chrono::duration<double>(
                             std::chrono::steady_clock::now() - start)
                             .count();
    std::cout << "seconds " << seconds << "\nframes " << frames
              << "\nchecksum " << (cents / 100) << "."
              << (cents < 0 ? -cents : cents) % 100 << "\n";
    return 0;
}
