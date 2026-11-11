// Time the eOn client's CON and convel path.
// A read loads every frame with readcon::read_all_frames and copies each
// frame into Matter through eonc::io::con2matter, which is the client load.
// A write uses matter2con or matter2convel.

#include "eon/ConFileIO.h"
#include "eon/Matter.h"
#include "eon/Parameters.h"
#include "eon/Potential.h"

#include <chrono>
#include <cstdint>
#include <dlfcn.h>
#include <fstream>
#include <iostream>
#include <string>
#include <vector>

namespace {

long status_kb(const char *key) {
    std::ifstream in("/proc/self/status");
    std::string line;
    const std::string prefix = std::string(key) + ":";
    while (std::getline(in, line)) {
        if (line.compare(0, prefix.size(), prefix) == 0) {
            return std::stol(line.substr(prefix.size()));
        }
    }
    return 0;
}

eonc::Matter make_matter() {
    eonc::Parameters params;
    eonc::ParametersLoadAccess::potential_options(params).potential = eonc::PotType::LJ;
    auto pot = eonc::helpers::sharePotential(
        eonc::helpers::makePotential(eonc::PotType::LJ, params));
    return eonc::Matter(pot, params);
}

void load_all(eonc::Matter &matter, const std::string &path) {
    auto frames = readcon::read_all_frames(path);
    if (frames.empty()) {
        throw std::runtime_error("no frames");
    }
    for (const auto &frame : frames) {
        if (!eonc::io::io_ok(eonc::io::con2matter(matter, frame, nullptr))) {
            throw std::runtime_error("con2matter");
        }
    }
}

double seconds_since(std::chrono::steady_clock::time_point start) {
    return std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count();
}

void print_samples(const std::vector<double> &samples) {
    std::cout << "{\"samples\":[";
    for (size_t i = 0; i < samples.size(); ++i) {
        if (i) {
            std::cout << ',';
        }
        std::cout.setf(std::ios::fixed);
        std::cout.precision(9);
        std::cout << samples[i];
    }
    std::cout << "],\"hwm_kib\":" << status_kb("VmHWM") << "}\n";
}

} // namespace

int main(int argc, char **argv) {
    if (argc < 3) {
        return 2;
    }
    const std::string mode = argv[1];
    const std::string path = argv[2];
    try {
        if (mode == "read") {
            const int repeats = std::stoi(argv[3]);
            {
                auto warm = make_matter();
                load_all(warm, path);
            }
            std::vector<double> samples;
            for (int i = 0; i < repeats; ++i) {
                auto matter = make_matter();
                const auto t0 = std::chrono::steady_clock::now();
                load_all(matter, path);
                samples.push_back(seconds_since(t0));
            }
            print_samples(samples);
            return 0;
        }
        if (mode == "once") {
            auto matter = make_matter();
            load_all(matter, path);
            std::cout << "{\"hwm_kib\":" << status_kb("VmHWM") << "}\n";
            return 0;
        }
        if (mode == "write-con" || mode == "write-convel") {
            const std::string out = argv[3];
            const int repeats = std::stoi(argv[4]);
            auto matter = make_matter();
            load_all(matter, path);
            auto write_one = [&]() {
                if (mode == "write-convel") {
                    if (!eonc::io::io_ok(eonc::io::matter2convel(matter, out))) {
                        throw std::runtime_error("matter2convel");
                    }
                } else if (!eonc::io::io_ok(eonc::io::matter2con(matter, out))) {
                    throw std::runtime_error("matter2con");
                }
            };
            write_one();
            std::vector<double> samples;
            for (int i = 0; i < repeats; ++i) {
                const auto t0 = std::chrono::steady_clock::now();
                write_one();
                samples.push_back(seconds_since(t0));
            }
            print_samples(samples);
            return 0;
        }
        if (mode == "write-once") {
            const std::string out = argv[3];
            const std::string kind = argv[4];
            const std::string library = argv[5];
            const std::string marker = argv[6];
            auto matter = make_matter();
            load_all(matter, path);
            void *handle = dlopen(library.c_str(), RTLD_NOW);
            if (handle == nullptr) {
                throw std::runtime_error("vgmark");
            }
            auto start = reinterpret_cast<void (*)()>(dlsym(handle, "watch_start"));
            auto stop = reinterpret_cast<long (*)()>(dlsym(handle, "watch_delta_kib"));
            auto monitor = reinterpret_cast<int (*)(const char *)>(dlsym(handle, "vg_monitor"));
            if (start == nullptr || stop == nullptr || monitor == nullptr) {
                throw std::runtime_error("vgmark symbol");
            }
            const std::string command = "detailed_snapshot " + marker;
            start();
            if (monitor(command.c_str()) != 0) {
                throw std::runtime_error("snapshot");
            }
            if (kind == "convel") {
                if (!eonc::io::io_ok(eonc::io::matter2convel(matter, out))) {
                    throw std::runtime_error("matter2convel");
                }
            } else if (!eonc::io::io_ok(eonc::io::matter2con(matter, out))) {
                throw std::runtime_error("matter2con");
            }
            std::cout << "{\"rss_kib\":" << stop() << "}\n";
            return 0;
        }
    } catch (const std::exception &err) {
        std::cerr << err.what() << '\n';
        return 1;
    }
    return 2;
}
