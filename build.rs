//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

#![feature(iter_array_chunks)]

use std::env;
use std::f64::consts::PI;
use std::fs::{read, write, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::RgbColor;
use embedded_graphics::Pixel;
use tinybmp::Bmp;

const PDM_BITS: usize = 14;

// Default waveform function. One sine but starting with the absolute
// minimum value to suppress crackle on startup.
#[inline]
fn i_sine(index: usize, points: usize) -> f64 {
    -((index as f64) / (points as f64) * 2.0 * PI).cos()
}

// PDM modulation based on pseudo code from
// https://en.wikipedia.org/wiki/Pulse-density_modulation
// Takes the number of modulation bits and a curve function as
// parameters. Curve function gets current position and the
// ammount of modulation bits and should return a f64 value between
// -1.0 an 1.0 - preferable starting and ending with -1.0 to
// reduce crackle. Number of point have to be a multiple of 32!
fn pdm_table(nr_points: usize, curve: fn(usize, usize) -> f64) -> Vec<u32> {
    assert_eq!(nr_points % 32, 0);
    let mut qe = 0.0;
    (0..nr_points)
        .map(|i| curve(i, nr_points))
        .map(|v| {
            qe += v;
            if qe > 0.0 {
                qe -= 1.0;
                1
            } else {
                qe += 1.0;
                0
            }
        })
        .array_chunks::<32>()
        .map(|x| x.iter().fold(0u32, |res, b| (res << 1) ^ *b))
        .collect()
}

fn bmp2bitstr(s: &str) -> String {
    let f = read(Path::new("font").join(format!("{}.bmp", s))).unwrap();
    let lines = Bmp::from_slice(&f)
        .unwrap()
        .pixels()
        .map(|p: Pixel<Rgb888>| p.1)
        .array_chunks::<8>()
        .map(|p| {
            p.iter().fold("0b".to_string(), |res, p| {
                res + if p.r() | p.g() | p.b() > 127 {
                    "1"
                } else {
                    "0"
                }
            })
        })
        .array_chunks::<3>()
        .map(|s| format!("    {}, {}, {},\n", s[0], s[1], s[2]))
        .collect::<Vec<_>>()
        .concat();
    format!("&[\n{}],\n", lines)
}

fn fontset(p: PathBuf) {
    let sources = [
        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "space", "dash", "dot", "off", "mem",
    ];
    let mut bitmaps = "".to_string();
    for filename in sources {
        bitmaps.push_str(&bmp2bitstr(filename));
    }

    let mut enums = "".to_string();
    let mut images = "".to_string();
    let mut chars = "".to_string();
    for src in sources {
        enums.push_str(&format!("    F{},\n", src));
        images.push_str(&format!(
            "    ImageRaw::<BinaryColor>::new(DATA[Font::F{} as usize], 24),\n",
            src
        ));
        chars.push_str(&format!(
            "    Image::new(&IMAGES[Font::F{} as usize], Point::zero()),\n",
            src
        ));
    }

    write(
        p,
        format!(
            "#[allow(unused)]\n\
            enum Font {{\n{}}}\n\n\
            const DATA: &[&[u8]] = &[\n{}];\n\n\
            const IMAGES: [ImageRaw<BinaryColor>; {}] =[\n{}];\n\n\
            const CHARS: [Image<ImageRaw<BinaryColor>>; {}] =[\n{}];\n\
            ",
            enums,
            bitmaps,
            sources.len(),
            images,
            sources.len(),
            chars,
        ),
    )
    .unwrap();
}

fn main() {
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=-Tlink-rp.x");
    println!("cargo:rustc-link-arg-bins=-Tdefmt.x");

    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    let pdm = pdm_table(1 << PDM_BITS, i_sine);
    // 8 cycles in one table for higher frequencies
    let pdm_8 = pdm_table(1 << PDM_BITS, |i, s| i_sine(i * 8, s));
    let clk_div_1hz = 125_000_000.0 / 2.0f32.powi(PDM_BITS as i32);
    write(
        out.join("pdm_table.rs"),
        format!(
            "\
const PDM_TABLE: [u32; {}] = {:?};\n\
#[allow(dead_code)]\n\
const PDM8_TABLE: [u32; {}] = {:?};\n\
const CLK_DIV_1HZ: f32 = {};\n\
",
            pdm.len(),
            pdm,
            pdm_8.len(),
            pdm_8,
            clk_div_1hz,
        ),
    )
    .unwrap();
    fontset(out.join("fontmap.rs"));
    // By default, Cargo will re-run a build script whenever any file in the project changes. By
    // specifying `memory.x` here, we ensure the build script is only re-run when build.rs or
    // memory.x change.
    println!("cargo:rerun-if-changed=font/*.bmp");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=memory.x");
}
