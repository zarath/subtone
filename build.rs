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
use std::f32::consts::PI;
use std::fs::{read, write, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::RgbColor;
use embedded_graphics::Pixel;
use tinybmp::Bmp;

const PDM_BITS: usize = 14;

fn pdm_table() -> Vec<u32> {
    let pdm_tics = 1 << PDM_BITS;
    let mut bits: Vec<u32> = Vec::with_capacity(pdm_tics);
    let mut qe: f32 = 0.0;

    let vals = (0..pdm_tics)
        .map(|i| (((i + (pdm_tics * 3 / 4)) as f32) / ((pdm_tics >> 1) as f32) * PI).sin());
    for v in vals {
        qe += v;
        if qe > 0.0 {
            bits.push(1);
            qe -= 1.0;
        } else {
            bits.push(0);
            qe += 1.0;
        }
    }

    bits.chunks(32)
        // .rev()
        .map(|x| x.iter().fold(0u32, |res, b| (res << 1) ^ *b))
        .collect::<Vec<u32>>()
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

    let pdm = pdm_table();
    let clk_div_1hz = 125_000_000.0_f32 / 2.0_f32.powi(PDM_BITS as i32);
    write(
        out.join("pdm_table.rs"),
        format!(
            "\
const PDM_TABLE: [u32; {}] = {:?};\n\
const CLK_DIV_1HZ: f32 = {};\n\
",
            pdm.len(),
            pdm,
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
