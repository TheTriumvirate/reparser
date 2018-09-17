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

use std::fs::File;
use std::io::prelude::*;
use nom::{be_f32, le_f32};
use failure::Error;

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

pub type Field = Vec<Vec<Vec<(f32,f32,f32)>>>;

#[derive(Serialize, Deserialize)]
pub struct VectorField {
    width: usize,
    height: usize,
    depth: usize,
    field: Field,
}

impl VectorField {
    pub fn confidence_weighted_235(width: usize, height: usize, depth: usize, ndim: usize, values: &Vec<f32>) -> Self {
        const EPSILON : f32 = 0.0000001;
        
        let mut data : Field = Vec::new();
        for i in 0..depth {
            let mut plane = Vec::new();
            for j in 0..height {
                let mut row = Vec::new();
                for k in 0..width {
                    let confidence: f32 = values[i*height*width*ndim+j*width*ndim+k*ndim+0];
                    let _dxx : f32 = values[i*height*width*ndim+j*width*ndim+k*ndim+1];
                    let dxy : f32 = values[i*height*width*ndim+j*width*ndim+k*ndim+2];
                    let dxz : f32 = values[i*height*width*ndim+j*width*ndim+k*ndim+3];
                    let _dyy : f32 = values[i*height*width*ndim+j*width*ndim+k*ndim+4];
                    let dyz : f32 = values[i*height*width*ndim+j*width*ndim+k*ndim+5];
                    let _dzz : f32 = values[i*height*width*ndim+j*width*ndim+k*ndim+6];
                    
                    let mut x:f32 = confidence*(0.5*dxy + 0.5*dxz);
                    let mut y:f32 = confidence*(0.5*dxy + 0.5*dyz);
                    let mut z:f32 = confidence*(0.5*dxz + 0.5*dyz);
                    
                    // NOTE: zero out stuff that's close to zero, removes noise
                    if x.abs() < EPSILON { x = 0.0; }
                    if y.abs() < EPSILON { y = 0.0; }
                    if z.abs() < EPSILON { z = 0.0; }
                    
                    row.push((x,y,z));
                }
                plane.push(row);
            }
            data.push(plane);
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
            let r = VectorField::confidence_weighted_235(opt.width as usize, opt.height as usize, opt.depth as usize, 7, &o);
            let s = bincode::serialize(&r)?;
            std::fs::write(&opt.output, &s)?;
            println!("Output written to {:?}", opt.output)
        }
        Err(e) => println!("{:?}", e),
    }
    Ok(())
}
