# reparser - A Rust tool for translating binary data into binary data that Rust can deserialize


## Usage:

The following command converts the binary file `dt-helix.raw` (consisting of big-endian 32-bit floating points) to a binary representation in [bincode](https://docs.rs/bincode/1.0.1/bincode/) format and saves it as the file out.bincode.

```
cargo run -- dt-helix.raw -w 38 -h 39 -d 40
```

This can be directly serialized by Rust using `bincode::deserialize(byte_array)`.

```
$ cargo run -- --help

basic 0.1.0
Vegard Ekblad Itland <Vegard.Itland@student.uib.no>

USAGE:
    adhoc-parser [FLAGS] [OPTIONS] <FILE> --depth <depth> --height <height> --width <width>

FLAGS:
        --help             Prints help information
        --little-endian    Present if input file uses little-endian encoding (defaults to big-endian)
    -V, --version          Prints version information

OPTIONS:
    -d, --depth <depth>      Depth of the model
    -h, --height <height>    Height of the model
    -o, --output <output>    Output file [default: ./out.bincode]
    -w, --width <width>      Width of the model

ARGS:
    <FILE>    Input file
```

## Data representation

### Data structure

```
pub type Field = Vec<Vec<Vec<(f32,f32,f32)>>>;

#[derive(Serialize, Deserialize)]
pub struct VectorField {
    field: Field,
}
```

### Layout in memory

The default implementation makes a few assumptions about the format of the data in the input file:
- The data consists of a 3D array of 7-dimensional 32-bits floating point vectors.
- Each vector `v = [confidence, Dxx, Dxy, Dxz, Dyy, Dyz, Dzz]` results in a `v' = confidence*[0.5*Dxy + 0.5*Dxz, 0.5*Dxy + 0.5*Dyz, 0.5*Dxz + 0.5*Dyz]`
  - The `confidence` component is a number between 0 and 1, a measure of how sure we are that the data point contains information and not noise, and is typically either 0 or 1, although other values are possible
  - This peculiar way of interpreting the data is the one which has proved to yield the best results with [the data available to us](http://www.sci.utah.edu/~gk/DTI-data/). Attempting to weigh the Dxx, Dyy and Dzz values into it results in all the particles drifting off the screen
- The data is read as vertical slices, and a slice is read row by row.

It should, however, be possible to customize this to account for any 3-dimensional uniformly spaced 32-bit floating point data organization in a file, by writing custom function implementations for constructing the VectorField data type by interpreting the data in a certain way.

The current representation does not account for non-uniform spacing of the data, and does not perform any form of processing of the resulting vector field, such as for instance vector length scaling. The default interpretation does, however, zero the components of vectors below a certain threshold in order to remove potential noise from the data.
