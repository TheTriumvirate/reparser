#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate nom;

#[macro_use]
extern crate structopt;

extern crate failure;

extern crate serde;
extern crate serde_json;
extern crate bincode;

use std::fs::File;
use std::io::prelude::*;
use nom::be_f32;
use std::env;
use failure::Error;

use std::path::PathBuf;
use structopt::StructOpt;

const N_DIMEN  : usize = 7;
const EPSILON  : f32   = 0.0000001;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    /// Width of the model
    #[structopt(short = "w", long = "width")]
    width: u32,

    /// Height of the model
    #[structopt(short = "h", long = "height")]
    height: u32,

    /// Depth of the model
    #[structopt(short = "d", long = "depth")]
    depth: u32,

    /// Output file
    #[structopt(short = "o", long = "output", parse(from_os_str), default_value = "./out.bincode")]
    output: PathBuf,

    /// Input file
    #[structopt(name = "FILE", parse(from_os_str))]
    file: PathBuf,
}

// big endian

type Field = Vec<Vec<Vec<(f32,f32,f32)>>>;

#[derive(Serialize, Deserialize)]
struct VectorField {
    field: Field,
}

named_args!(parse_be (sz:usize)<&[u8], Vec<f32> >, many_m_n!(0, sz, be_f32));

fn main() -> Result<(), Error> {
    let width    : usize = 38;  // 148;
    let height   : usize = 39;  // 190;
    let depth    : usize = 40;  // 160;
    
    let index = |i: usize, j: usize, k: usize, l: usize| -> usize {
        i*height*width*N_DIMEN + j*width*N_DIMEN + k*N_DIMEN + l
    };
    let elem = |v: &Vec<f32>, i: usize, j: usize, k: usize, l: usize| -> f32 {
        v[index(i,j,k,l)]
    };
    let construct_field = |v: &Vec<f32>| -> Field {
        let mut data : Field = Vec::new();
        let mut prev : usize = 0;
        for i in 0..depth {
            let mut plane = Vec::new();
            for j in 0..height {
                let mut row = Vec::new();
                for k in 0..width {
                    let c = elem(&v,i,j,k,0); // confidence
                    let dxx = elem(&v,i,j,k,1);
                    let dxy = elem(&v,i,j,k,2);
                    let dxz = elem(&v,i,j,k,3);
                    let dyy = elem(&v,i,j,k,4);
                    let dyz = elem(&v,i,j,k,5);
                    let dzz = elem(&v,i,j,k,6);
                    let mut x:f32 = c*(0.25*dxy + 0.25*dxz);
                    let mut y:f32 = c*(0.25*dxy + 0.25*dyz);
                    let mut z:f32 = c*(0.25*dxz + 0.25*dyz);
                    // NOTE: zero out stuff that's close to zero, removes noise
                    if x.abs() < EPSILON { x = 0.0; }
                    if y.abs() < EPSILON { y = 0.0; }
                    if z.abs() < EPSILON { z = 0.0; }
                    //println!("{}, {}, {}", x, y, z);
                    row.push((x,y,z));
                }
                plane.push(row);
            }
            plane.reverse();
            data.push(plane);
        }
        return data;
    };
    let mut opt = Opt::from_args();

    let mut f = File::open(&opt.file)?;
    let mut contents: Vec<u8> = Vec::new();

    f.read_to_end(&mut contents)?;
    match parse_be(&contents, width*height*depth*N_DIMEN) {
        Ok((_, o)) =>  {
            let s = bincode::serialize(&construct_field(&o))?;
            std::fs::write("./out.bincode", &s)?;
            println!("Output written to {:?}", opt.output)
        }
        Err(e) => println!("{:?}", e),
    }
    Ok(())
}
