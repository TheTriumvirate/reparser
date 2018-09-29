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

pub type Field = Vec<Vec<Vec<(f32,f32,f32)>>>;

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
        const EPSILON : f32 = 0.000001;
        const NDIM : usize = 7;
        
        let mut data : Field = Vec::new();
        for i in 0..depth {
            let mut plane = Vec::new();
            for j in 0..height {
                let mut row = Vec::new();
                for k in 0..width {
                    let confidence: f32 = values[i*height*width*NDIM+j*width*NDIM+k*NDIM+0];
                    let dxx : f32 = values[i*height*width*NDIM+j*width*NDIM+k*NDIM+1];
                    let dxy : f32 = values[i*height*width*NDIM+j*width*NDIM+k*NDIM+2];
                    let dxz : f32 = values[i*height*width*NDIM+j*width*NDIM+k*NDIM+3];
                    let dyy : f32 = values[i*height*width*NDIM+j*width*NDIM+k*NDIM+4];
                    let dyz : f32 = values[i*height*width*NDIM+j*width*NDIM+k*NDIM+5];
                    let dzz : f32 = values[i*height*width*NDIM+j*width*NDIM+k*NDIM+6];
                    let mut x:f32 = 0.0;
                    let mut y:f32 = 0.0;
                    let mut z:f32 = 0.0;
                    
                    if confidence == 1.0 {
                        let ds : Matrix3<f32> = Matrix3::new(dxx,dxy,dxz,
                                                             dxy,dyy,dyz,
                                                             dxz,dyz,dzz);
                        let mut res = ds.symmetric_eigen();
                        let mut ev_sorted = res.eigenvalues.iter().map(|&f| f.abs()).collect::<Vec<f32>>();
                        ev_sorted.sort_by(|&a,&b| a.partial_cmp(&b).unwrap());
                        let max = ev_sorted.pop().unwrap();
                        let not_max = ev_sorted.pop().unwrap();
                        //println!("MAX: {}", max);
                        //println!("NOT_MAX: {}", not_max);
                        //println!("EIG: {:?}", res.eigenvalues);
                        if max-not_max > EPSILON { // needle shaped tensor
                            let a = res.eigenvalues.iamax_full();
                            //println!("IAMAX: {:?}", a.0);
                            //println!("RES: {}", res.eigenvectors);
                            let most_significant_vector = res.eigenvectors.column(a.0);
                            //                         row  col
                            x = max*most_significant_vector[(0,   0  )];
                            y = max*most_significant_vector[(1,   0  )];
                            z = max*most_significant_vector[(2,   0  )];
                            //println!("x,y,z = {}, {}, {}", x, y, z);
                            if (x*x+y*y+z*z).sqrt() < EPSILON { x = 0.0; y = 0.0; z = 0.0; }
                        }
                    }
                    
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
            let r = VectorField::from_eigenanalysis(opt.width as usize, opt.height as usize, opt.depth as usize, &o);
            let s = bincode::serialize(&r)?;
            std::fs::write(&opt.output, &s)?;
            println!("Output written to {:?}", opt.output)
        }
        Err(e) => println!("{:?}", e),
    }
    Ok(())
}
