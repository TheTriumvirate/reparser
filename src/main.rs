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

    /// How many seeding points to generate
    #[structopt(short = "s", long = "number-of-seeding-points", default_value = "10")]
    n_seeding_points: usize,
    
    /// Step size during seeding point generation
    #[structopt(short = "S", long = "seeding-point-calculation-stepsize", default_value = "2")]
    seeding_point_calculation_step_size: usize,
    
    /// Threshold for starting point directionality (product of 3x3x3 volume)
    #[structopt(short = "T", long = "fa-volume-product-threshold", default_value = "0.01")]
    fa_volume_product_threshold: f32,
}

pub type Field = Vec<Vec<Vec<(f32, f32,f32,f32)>>>;

#[derive(Serialize, Deserialize)]
pub struct VectorField {
    width: usize,
    height: usize,
    depth: usize,
    field: Field,
    directional: Vec<(f32,f32,f32)>,
}

fn distance(p1:(f32,f32,f32), p2:(f32,f32,f32)) -> f32 {
    ((p1.0-p2.0).powf(2.0) + (p1.1-p2.1).powf(2.0) + (p1.2-p2.2).powf(2.0)).sqrt()
}

impl VectorField {
    pub fn from_eigenanalysis(width: usize, height: usize, depth: usize, values: &Vec<f32>) -> Self {
        // difference between largest vector and the others, used as a measure
        // of whether the data is useful or not
        const FA_EPSILON : f32 = 0.0;
        const NDIM : usize = 7;
        
        // process the vector field
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
                            z = most_significant_vector[(0,   0  )];
                            y = most_significant_vector[(1,   0  )];
                            x = most_significant_vector[(2,   0  )];
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
        VectorField { width, height, depth, field: data, directional: Vec::new() }
    }
    
    /// data extension: automatically find good seeding locations
    pub fn calculate_seeding_points(&mut self, n_points: usize, step_size: usize, fa_area_threshold: f32) {
        let mut pts: Vec<(f32, f32, f32)> = vec![];
        
        // automatically discover interesting areas to start seeding from by inspecting the length
        // of the streamlines from that point.
        let mut streamlines: Vec<Vec<usize>> = Vec::new();
        for i in 0..n_points {
            let mut best_streamline = Vec::new();
            let mut best: (f32, f32, f32, f32, f32) = (-1.0, -1.0, -1.0, 0.0, 0.0,);
            for z in (step_size..self.depth-step_size).step_by(step_size) {
                for y in (step_size..self.height-step_size).step_by(step_size) {
                    for x in (step_size..self.width-step_size).step_by(step_size) {
                        let fx: f32 = x as f32;
                        let fy: f32 = y as f32;
                        let fz: f32 = z as f32;
                        
                        if pts.contains(&(fx,fy,fz)) {
                            continue; // we already found this point
                        }
                        
                        // reduce search space by only considering the areas which have several
                        // strongly directional vectors in them
                        let mut fa_combined: f32 = 1.0;
                        for x1 in 0..2 {
                            for y1 in 0..2 {
                                for z1 in 0..2 {
                                    fa_combined = fa_combined*self.field[(z+z1-1).min(self.depth-1)]
                                                                  [(y+y1-1).min(self.height-1)]
                                                                  [(x+x1-1).min(self.width-1)].3;
                                }
                            }
                        }
                        if fa_combined < fa_area_threshold { 
                            //println!("Ignoring paths starting at point below threshold");
                            continue;
                        }
                        
                        // attempt at discovering the length of the streamline
                        let mut this_streamline: Vec<usize> = Vec::new();
                        let mut xpos = fx;
                        let mut ypos = fy;
                        let mut zpos = fz;
                        const N_STEPS: usize = 1000;
                        
                        let mut skip: bool = false;
                        'outer: for _step in 0..N_STEPS {
                            // nearest interpolation
                            let ux = (xpos as usize).min(self.width-1);
                            let uy = (ypos as usize).min(self.height-1);
                            let uz = (zpos as usize).min(self.depth-1);
                            
                            let flat = ux*self.height*self.depth + uy*self.depth + uz; // unique identifier for this point
                            this_streamline.push(flat);
                            
                            // we want to discover starting points that will send particles down
                            // different paths, so ignore starting points that don't
                            for v in streamlines.iter() {
                                if v.contains(&flat) {
                                    //println!("Been down this path before");
                                    skip = true;
                                    break 'outer;
                                }
                            }
                            
                            // move forward one step
                            let delta = self.field[uz][uy][ux];
                            let fa = delta.3;

                            // If particles hit a point where fa=0, they will be killed and
                            // respawn. In order to ensure that we don't get a lot of stupid values
                            // at the wrong places, skip such starting points.
                            if fa == 0.0 {
                                //println!("Particles on this path will die");
                                skip = true;
                                break 'outer;
                            }
                            xpos += fa*delta.0;
                            ypos += fa*delta.1;
                            zpos += fa*delta.2;
                        }
                        
                        if !skip {
                            let dist = distance((xpos,ypos,zpos), (fx,fy,fz));
                            
                            // we want to also take into account and maximize distance to points
                            // we've found previously (so as to (hopefully) ensure that the paths
                            // do not collide with each other)
                            let mut sum : f32 = 0.0;
                            for p in pts.iter() {
                                sum += distance((fx,fy,fz), *p);
                            }
                            
                            // magic formula
                            if dist + dist*sum.powf((i as f32).sqrt()) > best.3 + best.3*best.4 {
                                best = (fx, fy, fz, dist, sum.powf((i as f32).sqrt()));
                                
                                // store so we can check for direct path collisions later
                                best_streamline = this_streamline;
                            }
                        }
                    }
                }
            }
            if best.0 > 0.0 {
                streamlines.push(best_streamline.to_vec());
                println!("Found point with best = {:?} and fa = {}", best, self.field[best.2 as usize][best.1 as usize][best.0 as usize].3);
                pts.push((best.0,best.1,best.2));
            }
        }
        
        self.directional = pts;
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
            let mut r = VectorField::from_eigenanalysis(opt.width as usize, opt.height as usize, opt.depth as usize, &o);
            r.calculate_seeding_points(
                opt.n_seeding_points,
                opt.seeding_point_calculation_step_size,
                opt.fa_volume_product_threshold);
            let s = bincode::serialize(&r)?;
            std::fs::write(&opt.output, &s)?;
            println!("Output written to {:?}", opt.output)
        }
        Err(e) => println!("{:?}", e),
    }
    Ok(())
}
