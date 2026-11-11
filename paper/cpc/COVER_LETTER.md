Dear Editor,

Please consider the enclosed manuscript, "readcon-core: a versioned geometry checkpoint for resumed saddle searches", for Computer Physics Communications as a Computer Programs in Physics paper.
We submit it with the program archive and the raw timing logs.

Saddle-search codes in the eOn family stop and resume from eOn configuration (CON) files.
CON had no specification, so each reader decided which files are legal and what each column means.
The manuscript states CON 3.1 as a versioned text grammar.
Each coordinate row carries a mask of fixed components and an atom identifier, and a velocity section names its time unit.
readcon-core is the reference reader and writer.
It is written in Rust, with C, C++, Fortran 2008, Julia and Python interfaces, under an open-source licence approved for the journal's Program Library.

Two corpora of legal and illegal files fix the verdict of a conforming reader, and every Python and C read path returns that verdict.
On a platinum slab, a climbing-image band resumed from CON 3.1 repeats the uninterrupted band.
On one CON file, ASE get_positions takes 10.97 s and eOn loadcons takes 14.62 s.
The ratio is the ratio of those type-7 medians, 1.332.
chemfiles, OVITO, MDAnalysis and pymatgen do not read CON.
The ASE median is 61.68 times the all-frames tip median in reader-exchange-public.log.
Each timing names its raw log.
Those logs and the program come from one release, readcon-core TODO(Rohit): version and commit.
The developer repository is https://github.com/lode-org/readcon-core.
The concept record of the software is https://doi.org/10.5281/zenodo.18753567.
TODO(Rohit): version digital object identifier (DOI) of this release.
The program archive contains the source, a README with installation steps, the CON 3.1 specification, both corpora and a test run with its output.

That archive and the manuscript have one author, and I am the corresponding author.
This work received no specific grant, and I have no competing interests.
The manuscript carries the generative-AI declaration that the journal requires.
The manuscript is not under consideration elsewhere.

TODO(Rohit): preprint identifier, if any.
TODO(Rohit): suggested reviewers, if any.

Rohit Goswami TODO(Rohit): affiliation lines as they should print.
TODO(Rohit): email and telephone number.
