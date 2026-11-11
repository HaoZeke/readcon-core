/* Copyright (c) readcon-core developers */
// Read every frame of a CON file through eOn's ConFileIO.
// Matter::con2matter(ConFrame) is the per-frame path. The file read is
// readcon::read_all_frames, the same call ConFileIO uses.

#include "eon/ConFileIO.h"
#include "eon/Matter.h"
#include "eon/Parameters.h"
#include "eon/Potential.h"

#include <readcon-core.hpp>

#include <chrono>
#include <cmath>
#ifdef READCON_EON_TEST_SEAM
#include <cstdlib>
#endif
#include <format>
#include <fstream>
#include <iostream>
#include <string>

namespace {

int hwm_kib() {
#ifdef READCON_EON_TEST_SEAM
  const char *over = std::getenv("READCON_EON_STATUS");
  std::ifstream status(over ? over : "/proc/self/status");
#else
  std::ifstream status("/proc/self/status");
#endif
  std::string line;
  while (std::getline(status, line)) {
    if (line.rfind("VmHWM:", 0) == 0) {
      return std::stoi(line.substr(line.find_first_of("0123456789")));
    }
  }
  return 0;
}

double expected(std::size_t frame, std::size_t atom, int axis) {
  const double raw =
      std::fmod(static_cast<double>(frame) * 17.0 +
                    static_cast<double>(atom) * 3.0 + static_cast<double>(axis),
                10000.0) /
      1000.0;
  return std::round(raw * 1e8) / 1e8;
}

// Test builds pass -DREADCON_EON_TEST_SEAM. The normal bench binary does
// not, so it has no environment variable and cannot be steered. The seam
// exists because con2matter returns ReadError only from its catch, and a
// frame read_all_frames accepts does not throw there. The checksum loop
// and the check loop share the counter.
#ifdef READCON_EON_TEST_SEAM
eonc::io::IoStatus con2matter_or_fail(eonc::Matter &matter, const readcon::ConFrame &frame) {
  static int calls = 0;
  ++calls;
  if (const char *fail = std::getenv("READCON_EON_FAIL_CALL")) {
    char *end = nullptr;
    const long which = std::strtol(fail, &end, 10);
    if (end != fail && which == static_cast<long>(calls)) {
      return eonc::io::IoStatus::ReadError;
    }
  }
  return eonc::io::con2matter(matter, frame, nullptr);
}
#endif

} // namespace

int main(int argc, char **argv) {
  if (argc != 2) {
    std::cerr << "usage: eon_allframes FILE\n";
    return 2;
  }
  eonc::Parameters parameters;
  auto potential =
      eonc::helpers::sharePotential(eonc::helpers::makePotential(eonc::PotType::LJ, parameters));
  eonc::Matter matter(potential, parameters);
  const int before = hwm_kib();
  const auto t0 = std::chrono::steady_clock::now();
  const auto frames = readcon::read_all_frames(argv[1]);
  double checksum = 0.0;
  for (const auto &frame : frames) {
#ifdef READCON_EON_TEST_SEAM
    const auto status = con2matter_or_fail(matter, frame);
#else
    const auto status = eonc::io::con2matter(matter, frame, nullptr);
#endif
    if (!eonc::io::io_ok(status)) {
      std::cerr << "con2matter failed\n";
      return 1;
    }
    const auto &positions = matter.getPositions();
    checksum += positions.sum();
  }
  const auto t1 = std::chrono::steady_clock::now();
  const int after = hwm_kib();
  const double seconds = std::chrono::duration<double>(t1 - t0).count();

  int matched = 0;
  std::size_t index = 0;
  for (const auto &frame : frames) {
#ifdef READCON_EON_TEST_SEAM
    const auto status = con2matter_or_fail(matter, frame);
#else
    const auto status = eonc::io::con2matter(matter, frame, nullptr);
#endif
    if (!eonc::io::io_ok(status)) {
      break;
    }
    const auto &positions = matter.getPositions();
    bool ok = positions.rows() > 0;
    for (Eigen::Index atom = 0; atom < positions.rows() && ok; ++atom) {
      for (int axis = 0; axis < 3; ++axis) {
        const double got = positions(atom, axis);
        if (std::abs(got - expected(index, static_cast<std::size_t>(atom), axis)) > 1e-5) {
          ok = false;
        }
      }
    }
    if (ok) {
      ++matched;
    }
    ++index;
  }
  std::cout << std::format("{:.17g} {} {} {} {} {:.17g}\n", seconds, before, after, matched,
                           frames.size(), checksum);
  return 0;
}
