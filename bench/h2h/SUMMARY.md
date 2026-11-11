# Head to head on one host

The host is Intel Xeon Processor, x86_64, 8 CPUs, memory_kib 49342768.
Every arm runs under taskset -c 0.
Each arm is one Snakemake rule and one asv class with time and track methods.
The timeout on each class is 600 s.
Samples are serial, six repeats, and the median and interquartile range use the type 7 quantile.
A row is correct only when the atom count, the cell, and the coordinates match within 1e-12.
The eon client is upstream commit 579738a9387e8d0ea55496f45dd057812cfab892.

## Same file

This table reads one con file.
The Atomic Simulation Environment (ASE) row uses the eon format.
The in-process Python read and the fresh-process Python read are separate rows.

| arm | median s | iqr s | correct |
| --- | --- | --- | --- |
| C read, 1000000 atoms | 0.080 | 0.005 | yes |
| Fortran read, 1000000 atoms | 0.100 | 0.003 | yes |
| Python in-process read, 1000000 atoms | 0.089 | 0.010 | yes |
| Python fresh-process read, 1000000 atoms | 0.121 | 0.009 | yes |
| eon C++ read, 1000000 atoms | 0.276 | 0.013 | yes |
| eon Python read, 1000000 atoms | 3.151 | 0.057 | yes |
| ASE eon read, 1000000 atoms | 1.402 | 0.065 | yes |
| C write, 1000000 atoms | 0.198 | 0.009 | yes |
| Fortran write, 1000000 atoms | 0.205 | 0.009 | yes |
| Python write, 1000000 atoms | 0.222 | 0.010 | yes |
| C read, 10000 frames of 100 atoms | 0.080 | 0.004 | yes |
| Fortran read, 10000 frames of 100 atoms | 0.116 | 0.003 | yes |
| Python read, 10000 frames of 100 atoms | 0.088 | 0.017 | yes |
| C read, 10 atoms | 0.0000047 | 0.0000009 | yes |
| Fortran read, 10 atoms | 0.0000049 | 0.0000031 | yes |
| Python read, 10 atoms | 0.0000060 | 0.0000070 | yes |
| eon C++ read, 10 atoms | 0.0000254 | 0.0000025 | yes |

The C reader peaks at 140 MiB on the million atom file.
The fresh Python process peaks at 165 MiB on that file.
The in-process Python peak is 268 MiB because that process has already read the smaller files.
The fresh-process split is parse 0.100 s, convert 0.024 s, copy 0.000189 s.
The Python write split is copy 0.002 s, convert 0.024 s, file format 0.169 s.
Python read of one million atoms is faster than the eon client and faster than eon Python.
The 10 atom C read is 4.7 us.
The eon client takes 25.4 us on that file.

## Nearest native format

This table reads different files.
It is a separate panel from the con table.
NumPy reads an npy coordinate array, which has no cell.
ASE and chemfiles read extended xyz.
`OVITO` reads extended xyz.
pymatgen reads a POSCAR.
MDAnalysis reads a protein data bank file with a crystal record.

| arm | median s | iqr s | atoms | cell | coordinates | correct |
| --- | --- | --- | --- | --- | --- | --- |
| NumPy npy | 0.007 | 0.010 | yes | no, cell is missing | yes, max abs 0 | no |
| ASE extxyz | 1.342 | 0.036 | yes | yes | yes, max abs 0 | yes |
| `OVITO` extxyz | 0.131 | 0.007 | yes | yes | yes, max abs 0 | yes |
| chemfiles extxyz | 0.338 | 0.034 | yes | yes | yes, max abs 0 | yes |
| MDAnalysis crystal record | 7.426 | 0.072 | yes | yes | yes, max abs 0 | yes |
| pymatgen poscar | 14.864 | 3.866 | yes | yes | yes, max abs 7e-15 | yes |

NumPy has no cell, so that row fails the cell check.
The other native rows match the atom count, the cell, and the coordinates.
`RCSO` is an in memory broadcast blob, and conbin is absent from this version, so neither is in these tables.

## Compressed CON

The host line for this session is Intel Xeon Processor, x86_64, 8 CPUs, memory_kib 49342816.
The kernel is 6.12.113+.
readcon is 0.18.1 built with zstd, rustc is 1.91.0, ASE is 3.29.0, gzip is 1.12, and Valgrind is 3.22.0.
`compression_from_extension` returns gzip and zstd.
Each comparison is one Snakemake rule and one asv class, one session, one warmup and six serial repeats, type 7.
The 100000 frames are one repeated copper grid.
Plain, gzip and zstd sizes are 63488890, 492026 and 181243 bytes for that trajectory, and 39589039, 2644663 and 752245 bytes for the million-atom file.
ASE xyz read of the trajectory does not finish.
ASE xyz on the million-atom file returns cell lengths of 0.

| arm | file | median s | iqr s | rss MiB | massif MiB | correct |
| --- | --- | --- | --- | --- | --- | --- |
| readcon Rust gzip read | 100000 frames | 0.412 | 0.00264 | 539.3 | 530.5 | yes |
| readcon Rust zstd read | 100000 frames | 0.414 | 0.00763 | 539.7 | 530.5 | yes |
| readcon Python gzip read | 100000 frames | 0.439 | 0.0114 | 579.8 | 547.7 | yes |
| readcon Python zstd read | 100000 frames | 0.435 | 0.0175 | 580.1 | 547.7 | yes |
| ASE eon gzip read | 100000 frames | 10.6 | 0.0354 | 377.5 | 168.1 | yes |
| gzip -dc pipe read | 100000 frames | 0.643 | 0.00362 | 535.9 | 530.5 | yes |
| readcon Rust gzip read | 1000000 atoms | 0.100 | 0.00502 | 125.3 | 131.2 | yes |
| readcon Rust zstd read | 1000000 atoms | 0.104 | 0.0101 | 133.3 | 131.2 | yes |
| readcon Python gzip read | 1000000 atoms | 0.113 | 0.00658 | 571.3 | 130.2 | yes |
| readcon Python zstd read | 1000000 atoms | 0.108 | 0.00670 | 571.5 | 138.9 | yes |
| ASE eon gzip read | 1000000 atoms | 1.47 | 0.0446 | 429.8 | 121.8 | yes |
| ASE xyz gzip read | 1000000 atoms | 127 | 0.120 | 376.5 | 74.9 | no, cell lengths are 0 |
| gzip -dc pipe read | 1000000 atoms | 0.165 | 0.00179 | 125.0 | 131.2 | yes |
| readcon Rust gzip write | 100000 frames | 0.561 | 0.00380 | 0.57 | 0 | yes |
| readcon Rust zstd write | 100000 frames | 0.471 | 0.00208 | 3.41 | 3.55 | yes |
| readcon Python gzip write | 100000 frames | 3.36 | 0.0788 | 2045 | 721.4 | yes |
| readcon Python zstd write | 100000 frames | 3.45 | 0.170 | 2048 | 724.6 | yes |
| ASE eon gzip write | 100000 frames | 14.2 | 0.138 | 22.5 | 12.8 | yes |
| ASE xyz gzip write | 100000 frames | 6.04 | 0.120 | 22.5 | 12.7 | no, cell is missing |
| readcon Rust gzip write | 1000000 atoms | 0.699 | 0.00222 | 23.2 | 32.3 | yes |
| readcon Rust zstd write | 1000000 atoms | 0.209 | 0.00115 | 26.2 | 35.5 | yes |
| readcon Python gzip write | 1000000 atoms | 1.23 | 0.00964 | 1022 | 577.3 | yes |
| readcon Python zstd write | 1000000 atoms | 0.718 | 0.00474 | 1022 | 577.3 | yes |
| ASE eon gzip write | 1000000 atoms | 6.14 | 0.0186 | 341.6 | 239.0 | yes |
| ASE xyz gzip write | 1000000 atoms | 5.41 | 0.0196 | 31.6 | 12.7 | no, cell is missing |

Write RSS and massif are the increase above the loaded input.
The Rust gzip write does not raise the heap above the loaded frames.

## readcon-db

readcon-db is 0.2.0.
The append loads the 100000 frames first.
ASE's SQLite store is `with connect`.
The random read uses the same 1000 indices in every arm.
readcon-db occupies 30867584 bytes, the ASE database 153169920 bytes, and the trajectory file 115236273 bytes.

| arm | work | median s | iqr s | rss MiB | massif MiB | correct |
| --- | --- | --- | --- | --- | --- | --- |
| readcon-db | append | 5.51 | 0.0611 | 692.2 | 722.0 | yes |
| ASE db | append | 9.44 | 0.0628 | 71.2 | 14.6 | yes |
| ASE trajectory | append | 3.45 | 0.0388 | 50.0 | 36.7 | yes |
| readcon-db | 1000 frames | 4.53 | 0.138 | 67.9 | 8.11 | yes |
| ASE db | 1000 frames | 0.297 | 0.00450 | 97.6 | 33.8 | yes |
| ASE trajectory | 1000 frames | 0.0468 | 0.000624 | 81.3 | 19.7 | yes |
