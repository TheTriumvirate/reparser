#![allow(unused_comparisons)]
// NOTE: The above quells warnings arising due to unused comparisons in macro expansion, which is
// otherwise impossible to remove

extern crate bincode;
extern crate colored;
extern crate failure;
extern crate reparser;
extern crate serde;
extern crate structopt;

use reparser::*;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
pub struct Opt {
    /// Present if input file uses little-endian encoding (defaults to big-endian)
    #[structopt(long = "little-endian")]
    little_endian: bool,

    /// Width of the model
    #[structopt(short = "w", long = "width")]
    width: usize,

    /// Height of the model
    #[structopt(short = "h", long = "height")]
    height: usize,

    /// Depth of the model
    #[structopt(short = "d", long = "depth")]
    depth: usize,

    /// Output file
    #[structopt(
        short = "o",
        long = "output",
        parse(from_os_str),
        default_value = "./out.bincode"
    )]
    output: PathBuf,

    /// Input file
    #[structopt(name = "FILE", parse(from_os_str))]
    file: PathBuf,

    /// How many seeding points to generate
    #[structopt(short = "s", long = "number-of-seeding-points", default_value = "10")]
    n_seeding_points: usize,

    /// Step size during seeding point generation
    #[structopt(
        short = "S",
        long = "seeding-point-calculation-stepsize",
        default_value = "2"
    )]
    seeding_point_calculation_step_size: usize,

    /// Threshold for starting point directionality (product of 3x3x3 volume)
    #[structopt(
        short = "T",
        long = "fa-volume-product-threshold",
        default_value = "0.01"
    )]
    fa_volume_product_threshold: f32,

    /// Header file in order to automatically determine options
    #[structopt(short = "H", long = "header-file", parse(from_os_str))]
    header: Option<PathBuf>,
}

impl Into<Options> for Opt {
    fn into(self) -> Options {
        Options {
            little_endian: self.little_endian,
            file: Some(self.file),
            width: self.width,
            height: self.height,
            depth: self.depth,
            n_seeding_points: self.n_seeding_points,
            seeding_point_calculation_step_size: self.seeding_point_calculation_step_size,
            fa_volume_product_threshold: self.fa_volume_product_threshold,
        }
    }
}

/// Given a path to an NHDR header, return an Opt containing sizes, input file and endianness, and
/// defaults for all other values
fn load_opt_from_header_file(header: &PathBuf) -> Result<Options, std::io::Error> {
    let h = File::open(&header)?;
    let br = BufReader::new(h);

    let mut lines: Vec<String> = Vec::new();
    for ln in br.lines() {
        lines.push(ln?);
    }
    Ok(Options::from_header_file(lines))
}

// NOTE: Removed for the sake of fewer warnings
/*
/// Return the contents of the data file pointed to by the NHDR header in bincode
fn load_data_file_from_header_file(header: &PathBuf) -> Result<Vec<u8>, String> {
    let options_maybe = load_opt_from_header_file(header);
    match options_maybe {
        Ok(opt) => load_data_file_from_opt(&opt),
        Err(_) => Err("Error loading header file".to_string()),
    }
}
*/

/// Returns the contents of the data file pointed to by opt.file in bincode
fn load_data_file_from_opt(opt: &Options) -> Result<Vec<u8>, String> {
    let file = match &opt.file {
        Some(x) => x,
        // Shit, I should have realized before that I could just add "return"
        None => return Err("Error: No file given".to_string()),
    };

    let mut f = match File::open(&file) {
        Ok(f) => Ok(f),
        Err(_) => Err("Error finding data file".to_string()),
    }?;
    let mut contents: Vec<u8> = Vec::new();
    match f.read_to_end(&mut contents) {
        Ok(_) => Ok(()),
        Err(_) => Err("Error reading data file".to_string()),
    }?;

    load_data_bytes_from_opt(&opt, &contents)
}

fn main() -> Result<(), String> {
    let opt = Opt::from_args();
    let output = opt.output.clone();

    let s = match opt.header.clone() {
        Some(fpath) => {
            let opt2 = match load_opt_from_header_file(&fpath) {
                Ok(o) => Ok(o),
                Err(_) => Err("Error loading header file"),
            }?;

            // NOTE: No longer applicable
            //output = opt2.output.clone();

            load_data_file_from_opt(&opt2)
        }
        None => load_data_file_from_opt(&opt.into()),
    }?;
    match std::fs::write(&output, &s) {
        Ok(_) => {
            println!("Output written to {:?}", output);
            Ok(())
        }
        Err(_) => Err("Error writing to output file".to_string()),
    }
}
