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
use std::fs::{read, write, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::RgbColor;
use embedded_graphics::Pixel;
use tinybmp::Bmp;

use pdm::{generate, sine_idx};

const PDM_BITS: usize = 1 << 14;
const PDM_BYTES: usize = PDM_BITS >> 3;

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
        enums.push_str(&format!("    F{src},\n"));
        images.push_str(&format!(
            "    ImageRaw::<BinaryColor>::new(DATA[Font::F{src} as usize], 24),\n"
        ));
        chars.push_str(&format!(
            "    Image::new(&IMAGES[Font::F{src} as usize], Point::zero()),\n"
        ));
    }
    let sources_len = sources.len();

    write(
        p,
        format!(
            "#[allow(unused)]\n\
            enum Font {{\n{enums}}}\n\n\
            const DATA: &[&[u8]] = &[\n{bitmaps}];\n\n\
            const IMAGES: [ImageRaw<BinaryColor>; {sources_len}] =[\n{images}];\n\n\
            const CHARS: [Image<ImageRaw<BinaryColor>>; {sources_len}] =[\n{chars}];\n\
            "
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

    let pdm = generate::<PDM_BITS, PDM_BYTES>(sine_idx::<PDM_BITS>);
    let clk_div_1hz = 125_000_000.0 / (PDM_BITS as f32);
    write(
        out.join("pdm_table.rs"),
        format!(
            "\
#[repr (C, align ({PDM_BYTES}))]
struct PdmBuffer([u8; {PDM_BYTES}]);

static PDM_TABLE: PdmBuffer = PdmBuffer({pdm:?});\n\
const CLK_DIV_1HZ: f32 = {clk_div_1hz};\n\
"
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
