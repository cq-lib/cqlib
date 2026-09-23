// This code is part of Cqlib.
//
// (C) Copyright China Telecom Quantum Group 2026
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(cbindgen::Language::C)
        // num_complex::Complex64 is referenced by QIS signatures but is not a
        // cbindgen-known type; define it as a plain struct of two doubles.
        .with_after_include(
            "/* Complex64 mirrors num_complex::Complex<f64> as interleaved\n\
             * (real, imag) double pairs. */\n\
             typedef struct Complex64 {\n  double re;\n  double im;\n} Complex64;\n",
        )
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("include/cqlib_c.h");
}
