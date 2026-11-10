# Reproduction

Commands run from `paper/cpc`.
Each manuscript input names the script that writes it.
An output is written by that command, not by hand.

## Manuscript inputs

| Input | Script | Command |
|---|---|---|
| `src/readcon-url.tex` | `source file` | `tracked in git` |
| `src/affilfix.tex` | `source file` | `tracked in git` |
| `src/commit.tex` | `source file` | `tracked in git` |
| `src/cargo_version.tex` | `cargo_version_tex.py` | `python3 scripts/cargo_version_tex.py` |
| `src/release.tex` | `gen_release.py` | `python3 scripts/gen_release.py` |
| `src/figures/generated/sizes_macros.tex` | `file_sizes.py` | `python3 scripts/file_sizes.py --table` |
| `src/figures/generated/case_macros.tex` | `gen_case_macros.py` | `python3 scripts/gen_case_macros.py` |
| `src/figures/generated/corpus_recount_macros.tex` | `spec_corpus_recount.py` | `python3 scripts/spec_corpus_recount.py --tables` |
| `src/figures/generated/corpus31_macros.tex` | `corpus31_results.py` | `python3 scripts/corpus31_results.py --tables` |
| `src/figures/generated/corpus_paths_macros.tex` | `corpus_paths.py` | `python3 scripts/corpus_paths.py --tables` |
| `src/figures/generated/every_path_macros.tex` | `every_path_count.py` | `python3 scripts/every_path_count.py --tables` |
| `src/figures/generated/clauses_macros.tex` | `clause_coverage.py` | `python3 scripts/clause_coverage.py` |
| `src/figures/generated/mutant_macros.tex` | `mutant_differential.py` | `python3 scripts/mutant_differential.py --tables` |
| `src/figures/generated/corpus_macros.tex` | `corpus_crosscheck.py` | `python3 scripts/corpus_crosscheck.py --tables` |
| `src/figures/generated/h2h_macros.tex` | `h2h_tables.py` | `python3 scripts/h2h_tables.py` |
| `src/figures/generated/bf_macros.tex` | `binary_formats_tables.py` | `python3 scripts/binary_formats_tables.py` |
| `src/figures/generated/ordering_macros.tex` | `ordering_tests.py` | `python3 scripts/ordering_tests.py` |
| `src/figures/generated/spec_macros.tex` | `gen_spec_supp.py` | `python3 scripts/gen_spec_supp.py --as-is` |
| `src/figures/generated/e5_macros.tex` | `e5_prefactor.py` | `python3 evidence/e5_prefactor.py --tables` |
| `src/figures/generated/velocity_case.tex` | `velocity_case.py` | `python3 scripts/velocity_case.py --tables` |
| `src/figures/generated/worked_macros.tex` | `worked_example.py` | `python3 scripts/worked_example.py --tables` |
| `src/figures/generated/test_run_macros.tex` | `test_run.py` | `python3 scripts/test_run.py --tables` |
| `src/figures/generated/si_macros.tex` | `sinum.py` | `python3 scripts/sinum.py` |
| `src/figures/generated/dlpack_macros.tex` | `bench_dlpack.py` | `python3 scripts/bench_dlpack.py --from-log` |
| `src/figures/generated/host_macros.tex` | `host_from_log.py` | `python3 scripts/host_from_log.py` |
| `src/figures/generated/db_macros.tex` | `db_from_log.py` | `python3 scripts/db_from_log.py` |
| `src/authors.tex` | `source file` | `tracked in git` |
| `src/figures/generated/survey_listing.tex` | `survey_listing.py` | `python3 scripts/survey_listing.py --tables` |
| `src/figures/generated/versions_table.tex` | `gen_spec_supp.py` | `python3 scripts/gen_spec_supp.py --as-is` |
| `src/figures/generated/restart_main.tex` | `gen_cloud_tables.py` | `python3 scripts/gen_cloud_tables.py` |
| `src/figures/generated/e5_routes.tex` | `e5_prefactor.py` | `python3 evidence/e5_prefactor.py --tables` |
| `src/figures/generated/sizes_table.tex` | `file_sizes.py` | `python3 scripts/file_sizes.py --table` |
| `src/figures/generated/host_table.tex` | `host_from_log.py` | `python3 scripts/host_from_log.py` |
| `src/figures/generated/mpi_macros.tex` | `mpi_from_log.py` | `python3 scripts/mpi_from_log.py` |
| `src/figures/generated/mpi_host_table.tex` | `mpi_from_log.py` | `python3 scripts/mpi_from_log.py` |
| `src/figures/generated/anchors_macros.tex` | `src_anchors.py` | `python3 scripts/src_anchors.py` |
| `src/figures/generated/spec_normative.tex` | `gen_spec_supp.py` | `python3 scripts/gen_spec_supp.py --as-is` |
| `src/figures/generated/corpus_table.tex` | `spec_corpus_recount.py` | `python3 scripts/spec_corpus_recount.py --tables` |
| `src/figures/generated/drift_table.tex` | `gen_failure_tables.py` | `python3 scripts/gen_failure_tables.py` |
| `src/figures/generated/survey_table.tex` | `gen_failure_tables.py` | `python3 scripts/gen_failure_tables.py` |
| `src/figures/generated/test_run.tex` | `test_run.py` | `python3 scripts/test_run.py --tables` |
| `src/figures/generated/corpus31_table.tex` | `corpus31_results.py` | `python3 scripts/corpus31_results.py --tables` |
| `src/figures/generated/corpus_paths_table.tex` | `corpus_paths.py` | `python3 scripts/corpus_paths.py --tables` |
| `src/figures/generated/corpus_crosscheck.tex` | `corpus_crosscheck.py` | `python3 scripts/corpus_crosscheck.py --tables` |
| `src/figures/generated/versionsafety_table.tex` | `gen_failure_tables.py` | `python3 scripts/gen_failure_tables.py` |
| `src/figures/generated/survey_compare.tex` | `gen_failure_tables.py` | `python3 scripts/gen_failure_tables.py` |
| `src/figures/generated/restart_kernels.tex` | `gen_cloud_tables.py` | `python3 scripts/gen_cloud_tables.py` |
| `src/figures/generated/hcn_table.tex` | `gen_tables.py` | `python3 scripts/gen_tables.py` |
| `src/figures/generated/format_context.tex` | `host_from_log.py` | `python3 scripts/host_from_log.py` |
| `src/figures/generated/version_map.tex` | `gen_version_map.py` | `python3 scripts/gen_version_map.py` |
| `src/figures/generated/bf_ratios.tex` | `binary_formats_tables.py` | `python3 scripts/binary_formats_tables.py` |
| `src/figures/generated/bf_table.tex` | `binary_formats_tables.py` | `python3 scripts/binary_formats_tables.py` |
| `src/figures/generated/dlpack_table.tex` | `bench_dlpack.py` | `python3 scripts/bench_dlpack.py --from-log` |
| `src/figures/generated/h2h_cpp.tex` | `h2h_tables.py` | `python3 scripts/h2h_tables.py` |
| `src/figures/generated/h2h_python.tex` | `h2h_tables.py` | `python3 scripts/h2h_tables.py` |
| `src/figures/generated/ordering_table.tex` | `ordering_tests.py` | `python3 scripts/ordering_tests.py` |
| `src/figures/generated/r0174_cachegrind.tex` | `r0174_tables.py` | `python3 scripts/r0174_tables.py` |
| `src/figures/generated/r0174_cpp.tex` | `r0174_tables.py` | `python3 scripts/r0174_tables.py` |
| `src/figures/generated/r0174_macros.tex` | `r0174_tables.py` | `python3 scripts/r0174_tables.py` |
| `src/figures/generated/r0174_table.tex` | `r0174_tables.py` | `python3 scripts/r0174_tables.py` |
| `src/figures/generated/toc.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/structures.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/architecture.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/dlpack.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/restart_formats.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/restart.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/timing.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/ratios.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/memory.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/mpi_host.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/fairness_ratios.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/layers.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/roofline.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/ase_mask.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/mpi_strong.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/mpi_weak.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/db_ingest_throughput.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/db_ingest_bandwidth.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/db_query_latency.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/db_disk_footprint.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/db_dedup.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/frame_anatomy.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/error_classes.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/roundtrip_fidelity.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/restart_diff.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/dedup_graph.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/neb_multiples.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/velocity_mislabel.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/layout.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/matrix.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/pipeline.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/threads.pdf` | `plot scripts via regen_paper.py` | `python3 scripts/regen_figures.py` |
| `src/figures/generated/survey_frame.con` | `survey_listing.py` | `python3 scripts/survey_listing.py --tables` |

## Outputs that had no command

| Output | Command |
|---|---|
| `logs/NUMBERS.tsv` | `python3 scripts/numbers_from_log.py logs/reader-exchange.log logs/NUMBERS.tsv` |
| `records.tar.xz and records.sha256` | `python3 scripts/pack_records.py --write` |
| `src/rg_main.pdf and src/rg_supp.pdf` | `bash scripts/build_tex.sh` |
| `variants/rg_main-b.pdf and variants/rg_supp-b.pdf` | `bash scripts/build_tex.sh  # then the public-release build named in variants/NUMBERS.txt` |
| `data/cloud-runs-2/05/bands/*.npz` | `python3 data/cloud-runs-2/cloud-run/run_05.py` |
| `the tables and figures together` | `python3 scripts/regen_paper.py` |
| `the same targets through Snakemake` | `snakemake -c1 all` |

## Timing hostname

A timing receipt keeps a hostname key.
The value is the literal bench.
The manuscript check accepts that value and no other.
A missing key is a failure.
A value other than bench is a failure.
Any other host, hostname, or nodename value is a failure.
