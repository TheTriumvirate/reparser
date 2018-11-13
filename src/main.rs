#![allow(unused_comparisons)]
// NOTE: The above quells warnings arising due to unused comparisons in macro expansion, which is
// otherwise impossible to remove

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate nom;
#[macro_use]
extern crate structopt;

extern crate failure;
extern crate serde;
extern crate bincode;
extern crate nalgebra as na;

extern crate colored; // warnings

use colored::*;
use std::fs::File;
use std::io::prelude::*;
use nom::{be_f32, le_f32};
use failure::Error;
use na::{Matrix3};

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    /// Present if input file uses little-endian encoding (defaults to big-endian)
    #[structopt(long = "--little-endian")]
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
    #[structopt(short = "o", long = "output", parse(from_os_str), default_value = "./out.bincode")]
    output: PathBuf,

    /// Input file
    #[structopt(name = "FILE", parse(from_os_str))]
    file: PathBuf,
}

pub type Field = Vec<Vec<Vec<(f32, f32,f32,f32)>>>;

#[derive(Serialize, Deserialize)]
pub struct VectorField {
    width: usize,
    height: usize,
    depth: usize,
    field: Field,
}

impl VectorField {
    pub fn from_eigenanalysis(width: usize, height: usize, depth: usize, values: &Vec<f32>) -> Self {
        // difference between largest vector and the others, used as a measure
        // of whether the data is useful or not
        const FA_EPSILON : f32 = 0.0;
        const NDIM : usize = 7;
        
        let mut index: usize = 0;
        let mut data : Field = Vec::new();
        for ez in 0..depth {
            let mut plane = Vec::new();
            for ey in 0..height {
                let mut row = Vec::new();
                for ex in 0..width {
                    index = ez*width*height*NDIM + ey*width*NDIM + ex*NDIM;
                    let confidence: f32 = values[index];
                    let dxx : f32 = values[index+1];
                    let dxy : f32 = values[index+2];
                    let dxz : f32 = values[index+3];
                    let dyy : f32 = values[index+4];
                    let dyz : f32 = values[index+5];
                    let dzz : f32 = values[index+6];
                    let mut x:f32 = 0.0;
                    let mut y:f32 = 0.0;
                    let mut z:f32 = 0.0;

                    let mut fa:f32 = 0.0;
                    
                    if confidence == 1.0 {
                        let ds : Matrix3<f32> = Matrix3::new(dxx,dxy,dxz,
                                                             dxy,dyy,dyz,
                                                             dxz,dyz,dzz);
                        let mut res = ds.symmetric_eigen();
                        let mut ev_sorted = res.eigenvalues.iter().map(|&f| f.abs()).collect::<Vec<f32>>();
                        ev_sorted.sort_by(|&a,&b| a.partial_cmp(&b).unwrap());
                        let l1 = ev_sorted.pop().unwrap();
                        let l2 = ev_sorted.pop().unwrap();
                        let l3 = ev_sorted.pop().unwrap();
                        // fa value
                        fa = ((l1-l2).powf(2.0) + (l1-l3).powf(2.0) + (l2-l3).powf(2.0)).sqrt()/
                                 (2.0*(l1.powf(2.0)+l2.powf(2.0)+l3.powf(2.0))).sqrt();
                        if fa > FA_EPSILON { // needle shaped tensor
                            let a = res.eigenvalues.iamax_full();
                            let most_significant_vector = res.eigenvectors.column(a.0);
                            //                         row  col
                            x = most_significant_vector[(0,   0  )];
                            y = most_significant_vector[(1,   0  )];
                            z = most_significant_vector[(2,   0  )];
                        }
                    }
                    
                    row.push((x,y,z,fa));
                }
                plane.push(row);
            }
            data.push(plane);
        }
        
        if index+6 < width*height*depth-1 {
            
            println!("{} Last index visited was {}, but total number of floats was {}.",
                     "WARNING:".yellow().bold(), index+6, height*width*depth);
            println!("{:8} You probably fucked up how you do the dimensions", " ");
        }
        VectorField { width, height, depth, field: data }
    }
}

named_args!(pub parse_be (sz:usize)<&[u8], Vec<f32> >, many_m_n!(0, sz, be_f32));
named_args!(pub parse_le (sz:usize)<&[u8], Vec<f32> >, many_m_n!(0, sz, le_f32));

fn main() -> Result<(), Error> {
    let opt = Opt::from_args();

    let mut f = File::open(&opt.file)?;
    let mut contents: Vec<u8> = Vec::new();

    f.read_to_end(&mut contents)?;
    
    match
        if opt.little_endian {
            parse_le(&contents, opt.width*opt.height*opt.depth*7)
        } else {
            parse_be(&contents, opt.width*opt.height*opt.depth*7)
        }
    {
        Ok((_, o)) =>  {
            let r = VectorField::from_eigenanalysis(opt.width as usize, opt.height as usize, opt.depth as usize, &o);
            let s = bincode::serialize(&r)?;
            std::fs::write(&opt.output, &s)?;
            println!("Output written to {:?}", opt.output)
        }
        Err(e) => println!("{:?}", e),
    }
    Ok(())
}
