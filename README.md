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
