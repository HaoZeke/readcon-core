# Coverage table

Generated from a `cargo llvm-cov --branch` JSON export.
Hit, total, and percent are that export's file summary.
OPEN means lines or branches are under 98 percent.
A file with no branches is not failed for branches.

Measured commit: `ebecc8c8e8e3a9556e7a19f28c3c0ec470920236`

Command:

```
cargo llvm-cov --workspace --all-targets \
  --features parallel,zstd,grammar --branch --no-fail-fast \
  --json --output-path coverage.json
```

| File | Lines hit | Lines total | Lines % | Branches hit | Branches total | Branches % | Status |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| src/array.rs | 378 | 378 | 100.00 | 18 | 18 | 100.00 | met |
| src/chemfiles_import.rs | 151 | 151 | 100.00 | 2 | 2 | 100.00 | met |
| src/chemfiles_selection.rs | 98 | 98 | 100.00 | 0 | 0 | n/a | met |
| src/compression.rs | 424 | 426 | 99.53 | 46 | 46 | 100.00 | met |
| src/conbin.rs | 2208 | 2314 | 95.42 | 172 | 184 | 93.48 | OPEN |
| src/convert.rs | 176 | 178 | 98.88 | 8 | 8 | 100.00 | met |
| src/distrib.rs | 69 | 69 | 100.00 | 14 | 14 | 100.00 | met |
| src/error.rs | 127 | 127 | 100.00 | 0 | 0 | n/a | met |
| src/extxyz_scan.rs | 320 | 320 | 100.00 | 68 | 68 | 100.00 | met |
| src/ffi.rs | 6696 | 6919 | 96.78 | 512 | 664 | 77.11 | OPEN |
| src/frame_index.rs | 636 | 637 | 99.84 | 56 | 60 | 93.33 | OPEN |
| src/grammar.rs | 247 | 248 | 99.60 | 0 | 0 | n/a | met |
| src/helpers.rs | 216 | 216 | 100.00 | 0 | 0 | n/a | met |
| src/hostlabel.rs | 105 | 105 | 100.00 | 26 | 26 | 100.00 | met |
| src/index_proj.rs | 613 | 614 | 99.84 | 56 | 56 | 100.00 | met |
| src/iterators.rs | 3028 | 3070 | 98.63 | 549 | 588 | 93.37 | OPEN |
| src/lib.rs | 9 | 9 | 100.00 | 0 | 0 | n/a | met |
| src/main.rs | 84 | 84 | 100.00 | 24 | 24 | 100.00 | met |
| src/parser.rs | 3444 | 3509 | 98.15 | 572 | 622 | 91.96 | OPEN |
| src/pdb_scan.rs | 202 | 202 | 100.00 | 36 | 36 | 100.00 | met |
| src/rcso.rs | 269 | 269 | 100.00 | 36 | 36 | 100.00 | met |
| src/storage_dtype.rs | 815 | 815 | 100.00 | 32 | 32 | 100.00 | met |
| src/stream.rs | 220 | 224 | 98.21 | 38 | 42 | 90.48 | OPEN |
| src/trajectory.rs | 40 | 40 | 100.00 | 2 | 2 | 100.00 | met |
| src/types.rs | 3316 | 3323 | 99.79 | 639 | 648 | 98.61 | met |
| src/units.rs | 664 | 664 | 100.00 | 104 | 104 | 100.00 | met |
| src/writer.rs | 1137 | 1161 | 97.93 | 163 | 168 | 97.02 | OPEN |

OPEN files: 7. Measured files: 27.

## src/conbin.rs

Status: OPEN. Summary misses 106 lines and 12 branches.

Uncovered lines:

181-183, 188, 210-213, 333, 571-573, 600-601, 1581, 1860, 1867, 2801, 2860-2861, 2863-2866, 2871-2872, 2875-2881, 2884-2888, 2891-2892, 2896, 2899-2931, 2935, 2939-2947, 2951-2952, 3014-3016

Uncovered branches (line:column, side not taken):

- 180:12 true
- 181:16 false
- 181:16 true
- 187:8 true
- 209:8 true
- 211:16 false
- 211:16 true
- 565:20 false
- 566:28 false
- 598:12 false
- 1352:51 true
- 1866:13 false

The summary counts each instantiation. The locations above are the merged map, where a side is listed only when the best copy still leaves it at zero.

## src/ffi.rs

Status: OPEN. Summary misses 223 lines and 152 branches.

Uncovered lines:

88, 106, 169, 207, 295, 311-313, 449-450, 461, 664, 670-671, 715, 759, 763-767, 769, 771-774, 777, 779, 781, 783-785, 787, 790, 825, 864, 884, 892-893, 913, 954-955, 1040-1042, 1044-1045, 1087, 1204, 1212-1213, 1259-1260, 1273, 1340, 1364, 1420, 1445, 1450, 1481, 1488, 1495, 1656, 1851, 1870, 1889, 1909, 1929, 1947, 1967, 1985, 2006, 2023, 2039, 2055, 2071, 2087, 2107, 2126, 2146, 2166, 2185, 2237-2238, 2240, 2265-2266, 2268, 2296-2297, 2299, 2327-2328, 2330, 2356-2357, 2359, 2380, 2716, 2754, 2758, 2794, 2798, 2838, 2842, 2880, 2884, 2922, 2926, 2959, 2992, 3049, 3068, 3087, 3106, 3125, 3144, 3163, 3284, 3288, 3311, 3315, 3319, 3357, 3379, 3383, 3859, 3879, 3900, 3925, 3946, 3971, 3975, 3998, 4001, 4011-4012, 4053, 4785-4787, 4789, 4798, 4826, 4833-4834, 4837, 4850, 4870, 4876, 4881, 4883, 4889, 4894-4895, 4898, 4901, 5003, 5050, 5100, 5150, 5189-5190, 5195-5196, 5225, 5229, 5231, 5233, 5297, 5322, 5324, 5327, 5390, 5393, 5412, 5415, 5434, 5437, 5453, 5479, 5482, 5499, 5519, 5522, 5526-5528, 5530, 5532-5534, 5552, 5577, 5581, 5615, 5636, 5639, 5668, 5671, 5674, 6457, 6463, 8135, 8140, 8144, 8206

Uncovered branches (line:column, side not taken):

- 429:12 true
- 430:12 true
- 431:12 true
- 432:12 true
- 445:16 false
- 714:26 true
- 763:8 false
- 763:8 true
- 763:30 false
- 763:30 true
- 767:8 false
- 767:8 true
- 863:24 false
- 1018:28 true
- 1086:12 true
- 1203:8 true
- 1444:8 true
- 1444:35 false
- 1444:35 true
- 1449:13 false
- 1480:8 true
- 1485:8 true
- 1708:36 true
- 1726:36 true
- 1745:36 true
- 1764:36 true
- 1863:36 true
- 1882:36 true
- 1902:36 true
- 1922:36 true
- 2100:36 true
- 2119:36 true
- 2139:36 true
- 2159:36 true
- 2178:36 true
- 2197:36 true
- 2224:36 true
- 2224:57 true
- 2252:36 true
- 2252:57 true
- 2283:36 true
- 2283:57 true
- 2314:36 true
- 2314:57 true
- 2345:36 true
- 2345:59 true
- 2371:36 true
- 2674:42 false
- 2674:67 false
- 2711:36 true
- 2749:36 true
- 2757:8 true
- 2789:36 true
- 2797:8 true
- 2833:36 true
- 2841:8 true
- 2875:36 true
- 2883:8 true
- 2917:36 true
- 2925:8 true
- 2954:36 true
- 2987:36 true
- 3048:8 true
- 3067:8 true
- 3086:8 true
- 3105:8 true
- 3124:8 true
- 3143:8 true
- 3162:8 true
- 3244:26 true
- 3278:36 true
- 3305:36 true
- 3351:36 true
- 3373:36 true
- 3373:53 true
- 3966:32 true
- 3993:32 true
- 4000:22 true
- 4051:16 false
- 4441:34 true
- 4537:8 false
- 4557:8 true
- 4624:8 true
- 4671:8 true
- 4772:34 true
- 4782:12 false
- 4820:34 true
- 4824:24 false
- 4826:15 false
- 4826:15 true
- 4864:34 true
- 4869:8 true
- 4875:20 true
- 4880:21 false
- 4880:34 false
- 4880:58 false
- 4881:13 false
- 4881:13 true
- 4882:8 true
- 4888:8 true
- 4891:29 false
- 4891:71 false
- 4894:15 false
- 4894:15 true
- 4894:57 false
- 4894:57 true
- 4900:8 true
- 4946:34 true
- 4983:34 true
- 5030:34 true
- 5080:34 true
- 5130:34 true
- 5177:34 true
- 5187:8 false
- 5219:34 true
- 5219:57 true
- 5278:35 true
- 5295:12 false
- 5318:8 false
- 5323:8 true
- 5326:8 true
- 5389:9 false
- 5408:28 true
- 5411:9 false
- 5433:9 false
- 5452:9 false
- 5475:28 true
- 5478:9 false
- 5481:23 true
- 5485:22 false
- 5492:18 false
- 5515:28 true
- 5515:50 true
- 5518:9 false
- 5521:9 false
- 5550:16 false
- 5606:25 true
- 5630:25 true
- 5638:8 true
- 5662:25 true
- 5670:9 false
- 5673:8 true
- 6456:16 true
- 6461:16 false
- 8134:12 true
- 8139:12 true
- 8143:12 true
- 8205:12 true

The summary counts each instantiation. The locations above are the merged map, where a side is listed only when the best copy still leaves it at zero.

## src/frame_index.rs

Status: OPEN. Summary misses 1 line and 4 branches.

Uncovered lines:

none

Uncovered branches (line:column, side not taken):

- 342:8 true
- 382:19 false

The summary counts each instantiation. The locations above are the merged map, where a side is listed only when the best copy still leaves it at zero.

## src/iterators.rs

Status: OPEN. Summary misses 42 lines and 39 branches.

Uncovered lines:

1268, 1367, 1925, 1956, 2047, 2139, 2152, 2234, 2648, 2691, 2798, 3242-3243, 3462, 4041, 4315-4317, 4558-4559, 4579

Uncovered branches (line:column, side not taken):

- 126:20 true
- 571:35 false
- 846:77 false
- 864:57 false
- 913:40 true
- 963:16 false
- 983:23 false
- 983:39 false
- 1022:16 false
- 1146:17 false
- 1146:33 false
- 1146:46 false
- 1487:8 false
- 1494:8 false
- 1921:12 false
- 1976:8 false
- 2034:27 true
- 2090:8 true
- 2117:29 false
- 2137:8 false
- 2148:12 false
- 2782:20 false
- 2786:12 false
- 2794:24 false
- 3123:8 true
- 3156:8 true
- 3159:8 true
- 3336:21 false
- 3336:45 false
- 3422:29 false
- 3422:43 false
- 3460:12 false
- 3760:17 false
- 3760:41 false
- 3760:71 false
- 4039:20 false
- 4314:12 true

The summary counts each instantiation. The locations above are the merged map, where a side is listed only when the best copy still leaves it at zero.

## src/parser.rs

Status: OPEN. Summary misses 65 lines and 50 branches.

Uncovered lines:

723, 1281, 1912, 1916, 1924, 1975, 1981, 1996, 2033, 2075, 2096, 2100, 2134, 2139, 2146, 2152, 2160, 2208, 2347, 2352, 2354, 2365, 2371, 2378, 2432, 2436, 2443, 2456

Uncovered branches (line:column, side not taken):

- 83:13 false
- 86:12 true
- 102:27 true
- 127:9 false
- 133:9 false
- 155:19 false
- 155:31 false
- 155:45 false
- 260:8 true
- 1299:20 true
- 1905:8 true
- 1911:12 true
- 1941:23 false
- 1974:16 true
- 1980:45 true
- 1985:20 false
- 2044:16 false
- 2067:8 true
- 2095:16 true
- 2098:16 false
- 2126:8 true
- 2140:12 true
- 2143:12 true
- 2155:16 true
- 2158:16 false
- 2189:37 false
- 2205:16 false
- 2223:24 true
- 2230:24 true
- 2237:24 true
- 2244:24 true
- 2251:24 true
- 2258:24 true
- 2265:24 true
- 2271:24 false
- 2272:32 false
- 2339:8 true
- 2353:12 true
- 2376:16 false
- 2425:8 true
- 2454:16 false
- 2889:42 false
- 4517:71 false

The summary counts each instantiation. The locations above are the merged map, where a side is listed only when the best copy still leaves it at zero.

## src/stream.rs

Status: OPEN. Summary misses 4 lines and 4 branches.

Uncovered lines:

110

Uncovered branches (line:column, side not taken):

- 109:20 true
- 179:27 true

The summary counts each instantiation. The locations above are the merged map, where a side is listed only when the best copy still leaves it at zero.

## src/writer.rs

Status: OPEN. Summary misses 24 lines and 5 branches.

Uncovered lines:

none

Uncovered branches (line:column, side not taken):

none

The summary counts each instantiation. The locations above are the merged map, where a side is listed only when the best copy still leaves it at zero.
