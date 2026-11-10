//! Write the CON-bin image `h2h.py` times beside a trajectory.
//!
//! ```text
//! peer_time CON THREADS rcbin
//! ```
//!
//! `rcbin` writes `CON` with the extension replaced by `rcbin`, through
//! `conbin::write_from_con`. The thread count is accepted and unused: the
//! writer is one pass over the text.

use std::env;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 || args[3] != "rcbin" {
        eprintln!("usage: peer_time CON THREADS rcbin");
        return ExitCode::from(2);
    }
    let con = Path::new(&args[1]);
    let out = con.with_extension("rcbin");
    match readcon_core::conbin::write_from_con(con, &out, false) {
        Ok(header) => {
            println!(
                "frames {} atoms {}",
                header.n_frames, header.n_atoms
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}
