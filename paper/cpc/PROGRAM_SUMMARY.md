# PROGRAM SUMMARY

Program title: readcon-core

CPC Library link to program files: (to be added by Technical Editor)

Developer's repository link: https://github.com/lode-org/readcon-core

Code Ocean capsule: (to be added by Technical Editor)

Licensing provisions: MIT

Programming language: Rust, with C, C++, Fortran 2008, Julia and Python interfaces

Supplementary material: the CON 3.1 specification, the two conformance corpora, and the scripts and raw records behind each table and figure

Journal reference of previous version: none; this is the first CPC deposit

Program version and commit: TODO(Rohit): name the release (v0.18.0 at d5e77f99, or a 0.18.1 or other tag) that the archive and the DOI below describe

Software DOI: TODO(Rohit): version DOI from Zenodo or Mendeley Data.
The concept record is https://doi.org/10.5281/zenodo.18753567.

Nature of problem: Rare-event campaigns stop and resume saddle searches from a stored geometry.
A resumed surface search reads which Cartesian components of each atom are fixed, because a held component removes a mode from the harmonic prefactor and can move the saddle.
It reads an identifier on each row, because events and band images are matched by atom identity.
It reads the time unit of the velocities, because a misread unit scales the kinetic energy by the square of the conversion factor.
A restart that repeats the interrupted run to roundoff also reads back the binary64 coordinates as written.
H5MD defines no mask element, and the POSCAR structure file defines no atom identifier.
Extended XYZ keeps both in column names that one library chose, and it names no time unit.
CON is the geometry format of eOn, ASE reads it too, and it had no specification, so each reader decided which files are legal and what each column means.

Solution method: CON 3.1 is a versioned grammar for that text, and readcon-core is its reference reader and writer.
The second line of a frame is a JSON object with the major and minor version, the units, the periodic flags and the per-atom sections after the coordinates.
The fourth column of a coordinate row is a three-bit mask of fixed components, and the fifth is an atom identifier.
From minor 1, a velocity section declares its time unit.
A reader of a known major accepts any later minor, keeps unknown keys and skips a later section by its declared width.
The writer prints the shortest decimal that reads back as the same binary64 value.
Two corpora of legal and illegal files fix the verdict of a conforming reader on the clauses of the specification.
The grammar is implemented once in Rust, behind a C interface for C++, Julia and Fortran, and a PyO3 binding for Python.
On more than one thread the reader walks the frame boundaries serially by the counts in each header, parses the frames in parallel and returns them in file order.
A path writer replaces the destination only after the last frame is accepted.

Additional comments including restrictions and unusual features: Every build reads gzip input.
Zstandard input and parallel parsing are build features, and the Python wheel enables both.
A frame stores Cartesian coordinates, the cell, the periodic flags, the mask, one identifier per atom, and optional per-atom velocities, forces, energies, charges, spins, magnetic moments, displacements and spreads.
It stores no optimiser history, so an optimiser with memory takes a different path after a restart.
Files that earlier eOn releases wrote are grammar version 1 and read as such.
In Python, readcon.read_con(path) returns the frames of a file.

References: [1] S. T. Chill, M. Welborn, R. Terrell, L. Zhang, J.-C.
Berthet, A. Pedersen, H. Jónsson, G. Henkelman, Modell. Simul. Mater. Sci. Eng. 22 (2014) 055002.
https://doi.org/10.1088/0965-0393/22/5/055002.
[2] R. Goswami et al., TheochemUI/eOn v3.6.0 [software], Zenodo, 2026. https://doi.org/10.5281/zenodo.23166734.
[3] R. Goswami, readcon-core [software], Zenodo, 2026.
TODO(Rohit): version DOI.
Concept record https://doi.org/10.5281/zenodo.18753567.
