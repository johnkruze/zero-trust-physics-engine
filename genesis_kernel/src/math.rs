#![no_implicit_prelude]

//! Sine, cosine, arcsine, hypotenuse, exponential, exponential minus one, and
//! natural logarithm, carried in this crate for the cards; and for the
//! solvers of `genesis_solvers`, the logarithm of one plus a number, the
//! common logarithm, a power, a whole power, tangent, arccosine, arctangent,
//! the angle of a point, the hyperbolic sine, cosine and tangent, the inverse
//! hyperbolic sine and the cube root.
//!
//! `core` only: no heap and no standard prelude. The cards and the solvers
//! call these functions and no method of the platform's math library, so a
//! card's or a solve's result does not depend on which library a machine has.
//!
//! Every function here is built from addition, subtraction, multiplication,
//! division, comparison, conversion between a 64-bit number and an integer,
//! integer operations on the 64-bit pattern, and `f64::sqrt`. IEEE 754 fixes
//! the result of each of those to the last bit, so every function here returns
//! the same 64 bits on every target that follows the standard. The one
//! exception is a result that is not a number: every target returns NaN, and
//! the standard does not fix which NaN.
//!
//! `sqrt` is not carried. `f64::sqrt` is the one math method this crate still
//! takes from `std`, here and in the cards. IEEE 754 requires the square root
//! to be correctly rounded, so there is one right answer and every
//! conforming target returns it. It compiles to one instruction on arm64 and
//! in WebAssembly. A comparison of a native build with a WebAssembly build
//! over a million arguments found no differing bit.
//!
//! No product and sum is fused into one rounding. Nothing here calls
//! `mul_add`, and the Rust compiler does not fuse or reorder floating-point
//! arithmetic on its own. Where the source of `cbrt` calls a fused
//! multiply-add, this file computes the same exact value in separate
//! operations (below, "What differs from the source", 12). The optimised arm64 code of this crate holds no
//! fused multiply-add instruction. One kind of target is outside the
//! statement above: 32-bit x86 without SSE2, where 64-bit arithmetic is
//! carried at 80 bits.
//!
//! No `unsafe`. Where the source reads an array without a bounds check, this
//! file indexes it.
//!
//! # Source
//!
//! Ported from the Rust `libm` crate, version 0.2.16, in the repository
//! `rust-lang/compiler-builtins` on GitHub, tag `libm-v0.2.16`, commit
//! `dfd2203a4d6110820ad7bb65cafe1bf331a03a3d`. Each file was downloaded on
//! 2026-10-06 (2026-10-07 00:22 UTC) from
//!
//! ```text
//! https://raw.githubusercontent.com/rust-lang/compiler-builtins/dfd2203a4d6110820ad7bb65cafe1bf331a03a3d/libm/<path>
//! ```
//!
//! | `<path>` | SHA-256 | What was taken |
//! |---|---|---|
//! | `src/math/sin.rs` | `b48cebd120fc2ac93a6563059cabda28a72c8680304eb8335b92b412235cc232` | `sin` |
//! | `src/math/cos.rs` | `6e81752899c471bd42d04bdf08e0b22bc10a29f92fa2a343281b995fd342f035` | `cos` |
//! | `src/math/k_sin.rs` | `dfe6524cb2d530ec51eb1a2ab233aeb65b3f12177c1fb2f50f044ea41170f5d8` | `k_sin` |
//! | `src/math/k_cos.rs` | `d4867527877c846aa8e2a0b43cf06f03e2d94c321a548acaf8d5171c09ff4057` | `k_cos` |
//! | `src/math/rem_pio2.rs` | `5a040ab09dc356bca6d57712327151ed43003e2bac156553e3eb87d1cdefd2d1` | `rem_pio2` |
//! | `src/math/rem_pio2_large.rs` | `986b5b9ad2c8fba828aee47bf198eab97fab243359ab03eb09783d98ca5847b5` | `rem_pio2_large` |
//! | `src/math/asin.rs` | `9693a7cb61674ec6fe22be9db8fe20276729c48b1562fd86d5421e372a0a12e4` | `asin` |
//! | `src/math/hypot.rs` | `411edd6d95d4d5efc4b0ebfcd97820e337f4c8108dfea53957258cd411c7cd58` | `hypot` |
//! | `src/math/exp.rs` | `1d486e847e1752fb9ce74a714096a11adcac935dacd4f17625ff63819103a80d` | `exp` |
//! | `src/math/expm1.rs` | `6cc612ca7cf1384dc15f541b301a1f176efbc79536aff1293408b3121cc30ad3` | `exp_m1` |
//! | `src/math/log.rs` | `df5f4d1325e89621bfcd9cf7788d0d7e20a6ef7f429110fe35b370b3c9362615` | `log` |
//! | `src/math/generic/scalbn.rs` | `986e37f19c6cdf76867a365e1bc936050c238accda3fb327abccc488551ebf28` | `scalbn` |
//! | `src/math/generic/floor.rs` | `7d4f15369abefa27645d6deda51d30febbe60ee9dca76e72a9364d574d0711c0` | `floor` |
//! | `src/math/generic/fabs.rs` | `1381a8807d2c27b76996248bfcce6a2dc8982bdceee26febb15218e571b87908` | `fabs` |
//! | `src/math/support/float_traits.rs` | `510b4eb2f3c226eb78e332fe25060fc1d535da4c9f58ca0e047eff51c41da29b` | the 64-bit values of the constants `scalbn` and `floor` name |
//! | `LICENSE.txt` | `3823dda7cf046602f4b4e77ec8e227863dc4736037cc85bb33d9f19febe16bb7` | the licence text below |
//! | `src/math/log1p.rs` | `bb8ce741e54d8f001d9c12450a14a663bd413014da2ade9ba1f5fbc9ff0c9a88` | `ln_1p` |
//! | `src/math/log10.rs` | `4a96dd2b6a2a505497f2bb7ea956a50491d068dab1b6055a573aec1724a52e40` | `log10` |
//! | `src/math/pow.rs` | `c5566d0f48682d107e8803b0ed204ff3521503348f18a4f26cbe3f47fef15192` | `powf` |
//! | `src/math/tan.rs` | `2ea87ca18526753db6d149635ed6c895c8fbafd1e0fa7a9c04f7dc44b4eae986` | `tan` |
//! | `src/math/k_tan.rs` | `dc3f8ece9187bbd65792a7d1068e676092e9c2a0886f28c80579f9bd94d744b7` | `k_tan` |
//! | `src/math/acos.rs` | `efd83a6138061209867f06421c6a756b8ca3aab5c6c0a85c4aba31bedb299da5` | `acos` |
//! | `src/math/atan.rs` | `54181dbddf4f77b90b32b9f6819ac989da570dc02b791f3685d20a23d51324be` | `atan` |
//! | `src/math/atan2.rs` | `7d028d4d397f7484b8feed7b228eda65b84acac94d27729d236d5df95e38d8f0` | `atan2` |
//! | `src/math/sinh.rs` | `95cb1019a76c82312a5a65b3dd5f8a6a8b9b77aed1934c7254eafb94604b94a0` | `sinh` |
//! | `src/math/cosh.rs` | `71aed40077a04e659c15c2d2cc6e4faba89a89957e61ecacb7f6f5516bed1753` | `cosh` |
//! | `src/math/tanh.rs` | `c81ff9f4aa902d8fc63922f17d7d9e395983a6582c4587cbbeb194b613dd90c6` | `tanh` |
//! | `src/math/asinh.rs` | `8fd9431978e06904ed810d873f3de5bc097130d4b681a727644aa82d4f0fc627` | `asinh` |
//! | `src/math/expo2.rs` | `abd80ec291bebe24d3e0401f5600469f6248052be97f06885c32451fae57d3a8` | `expo2` |
//! | `src/math/k_expo2.rs` | `0e08b00b774731da6f16fd3ac8282a727a493e47b7a7292172b6136c04bd5dd4` | nothing: the same function as `expo2` |
//! | `src/math/cbrt.rs` | `1ed98a0943c46ca48e32af54b6baeb80b6e8da5dda550005711fcb26bfcf12c2` | `cbrt`, rounding to nearest |
//! | `src/math/mod.rs` | `c4ae2e62996ac9d7d6fdc0f7cb57ac0a815e639374b24068516ae3f092b0c59e` | `with_set_high_word`, `combine_words` |
//! | `src/math/generic/copysign.rs` | `c3fac45e8d84876204e5b11363ec77d6fc30010e0c32ab7558f2da538747e9d3` | `copysign` |
//! | `src/math/support/env.rs` | `e8bd76e77f562926bf4570352bb7ab91038be7703984619a2472683feafb268e` | the index of rounding to nearest, 0, that `cbrt` reads |
//!
//! `log.rs` was not downloaded. It was read on 2026-10-08 from the copy of
//! the crate at version 0.2.16 that Cargo keeps on this machine
//! (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libm-0.2.16/`).
//! In that copy `src/math/exp.rs` and `LICENSE.txt` have the SHA-256 this
//! table gives for them.
//!
//! The rows from `log1p.rs` down, the functions the solvers call, were added
//! on 2026-10-16. Each of those files was downloaded on 2026-10-16 (16:15
//! UTC) from the same commit, and with them every file of the rows above,
//! `log.rs` among them: each of the sixteen had the SHA-256 this table gives
//! it, and each downloaded file was byte for byte the file of the same path
//! in the copy Cargo keeps. `float_traits.rs`, above, also gives the 64-bit
//! masks `cbrt` names.
//!
//! Eighteen of the routines are the Sun Microsystems routines of FreeBSD's
//! `msun`, as the crate carries them: `sin`, `cos`, `k_sin`, `k_cos`,
//! `rem_pio2`, `rem_pio2_large`, `asin`, `exp`, `exp_m1`, `log`, `ln_1p`,
//! `log10`, `powf`, `tan`, `k_tan`, `acos`, `atan` and `atan2`. Each keeps its
//! notice, word for word, above its code.
//!
//! `hypot`, `scalbn`, `floor`, `fabs`, `sinh`, `cosh`, `tanh`, `asinh`,
//! `expo2` and the word helpers carry no copyright notice of their own in the
//! source. `hypot`, `floor`, `sinh`, `cosh`, `tanh`, `asinh` and `expo2` came
//! to the crate from musl, and musl's notice stands above `hypot` and above
//! the hyperbolic functions. `scalbn` and `fabs` were written for the crate.
//! The crate's licence, below, covers them all.
//!
//! `cbrt` is CORE-MATH's routine, as the crate carries it: its file states
//! the MIT licence (`SPDX-License-Identifier: MIT`) and its copyright, word
//! for word above its code; the MIT permission notice is the one below.
//!
//! `powi` is not from the source. It is this crate's own (its section says
//! how it is written), so that a whole power is the same multiplications on
//! every machine and in every build.
//!
//! More files were read and no code was taken from them:
//!
//! - FreeBSD `lib/msun/src/s_expm1.c` at tag `release/14.1.0`, SHA-256
//!   `3de68f78ad89f1a49c65d6670cd3a4fd28d56b6fe2b9bbee2da6344c619781d5`, from
//!   `https://raw.githubusercontent.com/freebsd/freebsd-src/release/14.1.0/lib/msun/src/s_expm1.c`.
//!   It states the accuracy of `expm1`, which the Rust file leaves out.
//! - musl `src/math/hypot.c` at tag `v1.2.5`, SHA-256
//!   `a5532b3328310bb3ac9f7cd9074fd26c5fe7288c719c907c5c03a615442f9e7e`, from
//!   `https://git.musl-libc.org/cgit/musl/plain/src/math/hypot.c?h=v1.2.5`.
//!   The Rust `hypot` is this routine, line for line.
//! - musl `COPYRIGHT` at the same tag, SHA-256
//!   `f9bc4423732350eb0b3f7ed7e91d530298476f8fec0c6c427a1c04ade22655af`, from
//!   `https://git.musl-libc.org/cgit/musl/plain/COPYRIGHT?h=v1.2.5`: the
//!   notice above `hypot`.
//! - musl `src/math/sinh.c`, `cosh.c`, `tanh.c`, `asinh.c` and `__expo2.c` at
//!   tag `v1.2.5`, SHA-256 `ab082814c4b1bbad8ea7dcfde1e36167cd72be5ef0e8b3ef3def0e052dd74261`,
//!   `5e5ed87e1a80e68734f4c87a4907a55f50a54df58261698aa94d5cb3276a3d43`,
//!   `7e290e86729cba6ccc8eb1f7db2452736daf1a2a8ce01b210db5787fee5cb762`,
//!   `a0a31b87d1f7b11c43500602fc537097c7f59a483c27a82513944df0b1d6c4f3` and
//!   `29cc9dd6d656cd0fbf921a8a1aeee81b430558d880194832a8d44d82c8d37d94`,
//!   downloaded on 2026-10-16 from the same address as `hypot.c`. They are
//!   the Rust functions' C origin and carry no notice of their own. musl's
//!   `__expo2` takes the sign as a factor, `exp(x - k ln2) * (sign * scale) *
//!   scale`, where the Rust `sinh` multiplies `expo2`'s result by the sign,
//!   `2h`, which is plus or minus one: the same number either way.
//! - `compiler-builtins/src/float/pow.rs` of the same repository at the same
//!   commit, SHA-256
//!   `79c04db2cca5d02b2fa69955fc71d687b801130f079399ec9911f8da05b9c913`,
//!   downloaded on 2026-10-16: the `__powidf2` that `f64::powi` reaches at
//!   run time, whose order of multiplications `powi` here follows.
//!
//! # What differs from the source
//!
//! Each function keeps the source's algorithm, constants and order of
//! operations. These are the differences, and none changes a returned bit:
//!
//! 1. The source evaluates a few expressions only to raise a floating-point
//!    exception flag (`force_eval!`). Rust cannot read those flags, and the
//!    source needs `unsafe` to do it. Those statements are left out, each
//!    marked where it stood.
//! 2. The source's `i!` and `div!` macros read an array or divide without a
//!    check in an optimised build. Here they are ordinary indexing and `/`.
//! 3. `rem_pio2_large` in the source serves four number widths. Here it is
//!    the 64-bit case alone (`prec = 1`, `jk = 4`). The table of 2/π is the
//!    66-entry table the source uses on 32-bit targets, on every target. It
//!    is the first 66 entries of the 690-entry table the source uses on
//!    64-bit targets, and a 64-bit argument reads no entry past them.
//! 4. `scalbn`, `floor` and `fabs` are generic over number widths in the
//!    source. Here they are written for 64 bits, with the constants replaced
//!    by their 64-bit values.
//! 5. Where the source calls its own `sqrt`, this file calls `f64::sqrt`.
//! 6. The source carries a line for 32-bit x86 without SSE2 in `rem_pio2`
//!    and another in `exp`. Neither is carried.
//! 7. `exp.rs` and `expm1.rs` each name a constant `INVLN2`, with the same
//!    value. The second is `EXPM1_INVLN2` here. The source's `expm1` is
//!    `exp_m1` here, the name Rust gives the method it replaces.
//! 8. `log.rs` names `LN2_HI` and `LN2_LO` with the values `expm1.rs` gives
//!    them. One pair serves both here.
//! 9. The source's `log1p` and `pow` are `ln_1p` and `powf` here, the names
//!    Rust gives the methods they replace.
//! 10. Constants and helpers that two source files name with the same value
//!    are carried once: `log1p.rs` and `log10.rs` name `LN2_HI`, `LN2_LO` and
//!    `LG1` to `LG7` with `log`'s values; `pow.rs` names `P1` to `P5` with
//!    `exp`'s; `acos.rs` names `PIO2_HI`, `PIO2_LO`, `PS0` to `PS5` and `QS1`
//!    to `QS4` with `asin`'s values for `PIO2_HI`, `PIO2_LO`, `P_S0` to `P_S5`
//!    and `Q_S1` to `Q_S4`, and its `r` is `asin`'s `comp_r`, the same
//!    operations. `pow.rs`'s `LG2`, `LG2_H` and `LG2_L` are `POW_LG2`,
//!    `POW_LG2_H` and `POW_LG2_L` here, because `log`'s `LG2` is another
//!    number. `k_expo2.rs`'s `k_expo2` and `expo2.rs`'s `expo2` are one
//!    function, the same constants and operations, its scale built from the
//!    same words: `expo2` serves `sinh` and `cosh`. `k_tan.rs`'s `T` is a
//!    `static` in the source and a `const` here.
//! 11. `cbrt` in the source is `cbrt_round(x, Round::Nearest)` of a routine
//!    that serves four rounding modes. Here it is rounding to nearest alone:
//!    `off[round]` is its first entry, 2^-53, and the list of hard cases the
//!    source reads only for the directed roundings is not carried. Its
//!    `cold_path()` calls are hints to the compiler with no value and are left
//!    out. Its constants, written in the source as hexadecimal floating-point
//!    literals (`hf64!`), are written as bit patterns, each with its literal
//!    beside it, and its generic float trait's constants as 64-bit values
//!    (`CBRT_SIG_BITS`, `CBRT_EXP_SAT`, `CBRT_SIG_MASK`, `CBRT_SIGN_MASK`).
//!    `abs` and `copysign` are the source's generic `fabs` and `copysign` for
//!    64 bits.
//! 12. The source's `cbrt` calls a fused multiply-add four times, each as
//!    `a.fma(b, -p)` with `p` the rounded product `a * b`. That returns the
//!    product's rounding error, which is a 64-bit number, exactly. Here
//!    `product_error` computes the same error exactly in separate additions
//!    and multiplications (Dekker's product with Veltkamp's split), so no
//!    returned bit changes and nothing here calls `mul_add`. A test holds
//!    the two equal on 84,028 products.
//! 13. `atan` in the source adds `f64::from_bits(0x0380_0000)` to `pi/2`,
//!    with the comment `0x1p-120f`; those bits are a 64-bit number near
//!    2.9e-316, and either leaves `pi/2` as it is. The line is kept.
//!
//! # Accuracy
//!
//! The tests compare each function with correctly rounded values at fixed
//! arguments, the test vectors. At every argument of the vectors the seven
//! functions of the cards, and `ln_1p`, `log10`, `powf`, `tan`, `acos`,
//! `atan`, `atan2`, `sinh`, `cosh` and `asinh`, agree within one unit in the
//! last place; `tanh` within two; `cbrt` is the correctly rounded value.
//! Each test pins how many arguments land at each distance. The arguments,
//! the rule that chose them and the scripts that produced the correctly
//! rounded values are named in the test module. `powi` is held to the bits
//! `f64::powi` gives at run time.
//!
//! Away from the vectors the bound is what was measured, not one unit for
//! every function. The red team of engine 0.14.0 rc1 compiled this file
//! byte for byte and measured every function against correctly rounded
//! values (mpmath at 160 bits or more) at 358 million arguments, over each
//! function's whole range and dense where its error is largest
//! (`evidence/verification/redteam_0.14.0_rc1/rt2/math_errors.json`,
//! `RT2.md`, section 1). Within one unit at every argument measured: `sin`,
//! `cos`, `tan`, `asin`, `acos`, `atan`, `hypot`, `exp`, `exp_m1`, `log`,
//! `ln_1p`, `log10`, `powf` and `cosh` (1.36 ulps at worst). Two units:
//! `atan2` (1.51 ulps at y = 2.471310338300148, x = 9.776455931779306,
//! found by local search and in none of 16 million uniform pairs), `sinh`
//! (1.81 ulps, on at least 7e-4 of negative arguments with 2^-26 <= |x| <
//! 1, and fewer of positive ones),
//! `asinh` (1.81 ulps, from 0.061 to 0.52, where its source states 1.6
//! ulps in [0.125, 0.5]) and `tanh` (2.19 ulps: two units on 1.4 % of
//! arguments in [0.1, 0.2554], as its source states, and also on 539
//! arguments found from 0.0296 to 0.0999). `cbrt` is correctly rounded at
//! every argument measured. `powi`, repeated squaring as `f64::powi` does
//! it, is up to 24 units off for |n| <= 64 and 3 at the exponents the
//! solvers use. `tanh` is not monotone on (0.2554, 0.5493], where no
//! solver's argument lies.
//!
//! # Licence of the source
//!
//! From `LICENSE.txt` of the crate, word for word:
//!
//! ```text
//! rust-lang/libm as a whole is available for use under the MIT license:
//!
//! ------------------------------------------------------------------------------
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.
//! ------------------------------------------------------------------------------
//! ```
//!
//! The same file offers the Apache-2.0 licence as an alternative; this crate
//! takes the code under the MIT licence above. The file continues:
//!
//! ```text
//! This Rust library contains the following copyrights:
//!
//!     Copyright (c) 2018 Jorge Aparicio
//!
//! Portions of this software are derived from third-party works licensed under
//! terms compatible with the above MIT license:
//!
//! * musl libc https://www.musl-libc.org/. This library contains the following
//!   copyright:
//!
//!       Copyright © 2005-2020 Rich Felker, et al.
//!
//! * The CORE-MATH project https://core-math.gitlabpages.inria.fr/. CORE-MATH
//!   routines are available under the MIT license on a per-file basis.
//!
//! The musl libc COPYRIGHT file also includes the following notice relevant to
//! math portions of the library:
//!
//! ------------------------------------------------------------------------------
//! Much of the math library code (src/math/* and src/complex/*) is
//! Copyright © 1993,2004 Sun Microsystems or
//! Copyright © 2003-2011 David Schultz or
//! Copyright © 2003-2009 Steven G. Kargl or
//! Copyright © 2003-2009 Bruce D. Evans or
//! Copyright © 2008 Stephen L. Moshier or
//! Copyright © 2017-2018 Arm Limited
//! and labelled as such in comments in the individual source files. All
//! have been licensed under extremely permissive terms.
//! ------------------------------------------------------------------------------
//!
//! Copyright notices are retained in src/* files where relevant.
//! ```
//!
//! One CORE-MATH routine is carried here, `cbrt`, under the MIT licence its
//! file states.

extern crate core;

use core::prelude::v1::*;

// ---------------------------------------------------------------------------
// sin
// ---------------------------------------------------------------------------

// origin: FreeBSD /usr/src/lib/msun/src/s_sin.c */
//
// ====================================================
// Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//
// Developed at SunPro, a Sun Microsystems, Inc. business.
// Permission to use, copy, modify, and distribute this
// software is freely granted, provided that this notice
// is preserved.
// ====================================================

// sin(x)
// Return sine function of x.
//
// kernel function:
//      k_sin            ... sine function on [-pi/4,pi/4]
//      k_cos            ... cose function on [-pi/4,pi/4]
//      rem_pio2         ... argument reduction routine
//
// Method.
//      Let S,C and T denote the sin, cos and tan respectively on
//      [-PI/4, +PI/4]. Reduce the argument x to y1+y2 = x-k*pi/2
//      in [-pi/4 , +pi/4], and let n = k mod 4.
//      We have
//
//          n        sin(x)      cos(x)        tan(x)
//     ----------------------------------------------------------
//          0          S           C             T
//          1          C          -S            -1/T
//          2         -S          -C             T
//          3         -C           S            -1/T
//     ----------------------------------------------------------
//
// Special cases:
//      Let trig be any of sin, cos, or tan.
//      trig(+-INF)  is NaN, with signals;
//      trig(NaN)    is that NaN;
//
// Accuracy:
//      TRIG(x) returns trig(x) nearly rounded

/// Sine of an angle in radians. The source states "TRIG(x) returns trig(x) nearly rounded"; the tests here hold it within one unit in the last place.
pub fn sin(x: f64) -> f64 {
    /* High word of x. */
    let ix = (f64::to_bits(x) >> 32) as u32 & 0x7fffffff;

    /* |x| ~< pi/4 */
    if ix <= 0x3fe921fb {
        if ix < 0x3e500000 {
            /* |x| < 2**-26 */
            /* raise inexact if x != 0 and underflow if subnormal*/
            // The source raises the two flags here. Not carried.
            return x;
        }
        return k_sin(x, 0.0, 0);
    }

    /* sin(Inf or NaN) is NaN */
    if ix >= 0x7ff00000 {
        return x - x;
    }

    /* argument reduction needed */
    let (n, y0, y1) = rem_pio2(x);
    match n & 3 {
        0 => k_sin(y0, y1, 1),
        1 => k_cos(y0, y1),
        2 => -k_sin(y0, y1, 1),
        _ => -k_cos(y0, y1),
    }
}

// ---------------------------------------------------------------------------
// cos
// ---------------------------------------------------------------------------

// origin: FreeBSD /usr/src/lib/msun/src/s_cos.c */
//
// ====================================================
// Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//
// Developed at SunPro, a Sun Microsystems, Inc. business.
// Permission to use, copy, modify, and distribute this
// software is freely granted, provided that this notice
// is preserved.
// ====================================================

// cos(x)
// Return cosine function of x.
//
// kernel function:
//      k_sin           ... sine function on [-pi/4,pi/4]
//      k_cos           ... cosine function on [-pi/4,pi/4]
//      rem_pio2        ... argument reduction routine
//
// Method.
//      Let S,C and T denote the sin, cos and tan respectively on
//      [-PI/4, +PI/4]. Reduce the argument x to y1+y2 = x-k*pi/2
//      in [-pi/4 , +pi/4], and let n = k mod 4.
//      We have
//
//          n        sin(x)      cos(x)        tan(x)
//     ----------------------------------------------------------
//          0          S           C             T
//          1          C          -S            -1/T
//          2         -S          -C             T
//          3         -C           S            -1/T
//     ----------------------------------------------------------
//
// Special cases:
//      Let trig be any of sin, cos, or tan.
//      trig(+-INF)  is NaN, with signals;
//      trig(NaN)    is that NaN;
//
// Accuracy:
//      TRIG(x) returns trig(x) nearly rounded
//

/// Cosine of an angle in radians. The source states "TRIG(x) returns trig(x) nearly rounded"; the tests here hold it within one unit in the last place.
pub fn cos(x: f64) -> f64 {
    let ix = (f64::to_bits(x) >> 32) as u32 & 0x7fffffff;

    /* |x| ~< pi/4 */
    if ix <= 0x3fe921fb {
        if ix < 0x3e46a09e {
            /* if x < 2**-27 * sqrt(2) */
            /* raise inexact if x != 0 */
            if x as i32 == 0 {
                return 1.0;
            }
        }
        return k_cos(x, 0.0);
    }

    /* cos(Inf or NaN) is NaN */
    if ix >= 0x7ff00000 {
        return x - x;
    }

    /* argument reduction needed */
    let (n, y0, y1) = rem_pio2(x);
    match n & 3 {
        0 => k_cos(y0, y1),
        1 => -k_sin(y0, y1, 1),
        2 => -k_cos(y0, y1),
        _ => k_sin(y0, y1, 1),
    }
}

// ---------------------------------------------------------------------------
// k_sin
// ---------------------------------------------------------------------------

// origin: FreeBSD /usr/src/lib/msun/src/k_sin.c
//
// ====================================================
// Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//
// Developed at SunSoft, a Sun Microsystems, Inc. business.
// Permission to use, copy, modify, and distribute this
// software is freely granted, provided that this notice
// is preserved.
// ====================================================

const S1: f64 = -1.66666666666666324348e-01; /* 0xBFC55555, 0x55555549 */
const S2: f64 = 8.33333333332248946124e-03; /* 0x3F811111, 0x1110F8A6 */
const S3: f64 = -1.98412698298579493134e-04; /* 0xBF2A01A0, 0x19C161D5 */
const S4: f64 = 2.75573137070700676789e-06; /* 0x3EC71DE3, 0x57B1FE7D */
const S5: f64 = -2.50507602534068634195e-08; /* 0xBE5AE5E6, 0x8A2B9CEB */
const S6: f64 = 1.58969099521155010221e-10; /* 0x3DE5D93A, 0x5ACFD57C */

// kernel sin function on ~[-pi/4, pi/4] (except on -0), pi/4 ~ 0.7854
// Input x is assumed to be bounded by ~pi/4 in magnitude.
// Input y is the tail of x.
// Input iy indicates whether y is 0. (if iy=0, y assume to be 0).
//
// Algorithm
//      1. Since sin(-x) = -sin(x), we need only to consider positive x.
//      2. Callers must return sin(-0) = -0 without calling here since our
//         odd polynomial is not evaluated in a way that preserves -0.
//         Callers may do the optimization sin(x) ~ x for tiny x.
//      3. sin(x) is approximated by a polynomial of degree 13 on
//         [0,pi/4]
//                               3            13
//              sin(x) ~ x + S1*x + ... + S6*x
//         where
//
//      |sin(x)         2     4     6     8     10     12  |     -58
//      |----- - (1+S1*x +S2*x +S3*x +S4*x +S5*x  +S6*x   )| <= 2
//      |  x                                               |
//
//      4. sin(x+y) = sin(x) + sin'(x')*y
//                  ~ sin(x) + (1-x*x/2)*y
//         For better accuracy, let
//                   3      2      2      2      2
//              r = x *(S2+x *(S3+x *(S4+x *(S5+x *S6))))
//         then                   3    2
//              sin(x) = x + (S1*x + (x *(r-y/2)+y))
fn k_sin(x: f64, y: f64, iy: i32) -> f64 {
    let z = x * x;
    let w = z * z;
    let r = S2 + z * (S3 + z * S4) + z * w * (S5 + z * S6);
    let v = z * x;
    if iy == 0 {
        x + v * (S1 + z * r)
    } else {
        x - ((z * (0.5 * y - v * r) - y) - v * S1)
    }
}

// ---------------------------------------------------------------------------
// k_cos
// ---------------------------------------------------------------------------

// origin: FreeBSD /usr/src/lib/msun/src/k_cos.c
//
// ====================================================
// Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//
// Developed at SunSoft, a Sun Microsystems, Inc. business.
// Permission to use, copy, modify, and distribute this
// software is freely granted, provided that this notice
// is preserved.
// ====================================================

const C1: f64 = 4.16666666666666019037e-02; /* 0x3FA55555, 0x5555554C */
const C2: f64 = -1.38888888888741095749e-03; /* 0xBF56C16C, 0x16C15177 */
const C3: f64 = 2.48015872894767294178e-05; /* 0x3EFA01A0, 0x19CB1590 */
const C4: f64 = -2.75573143513906633035e-07; /* 0xBE927E4F, 0x809C52AD */
const C5: f64 = 2.08757232129817482790e-09; /* 0x3E21EE9E, 0xBDB4B1C4 */
const C6: f64 = -1.13596475577881948265e-11; /* 0xBDA8FAE9, 0xBE8838D4 */

// kernel cos function on [-pi/4, pi/4], pi/4 ~ 0.785398164
// Input x is assumed to be bounded by ~pi/4 in magnitude.
// Input y is the tail of x.
//
// Algorithm
//      1. Since cos(-x) = cos(x), we need only to consider positive x.
//      2. if x < 2^-27 (hx<0x3e400000 0), return 1 with inexact if x!=0.
//      3. cos(x) is approximated by a polynomial of degree 14 on
//         [0,pi/4]
//                                       4            14
//              cos(x) ~ 1 - x*x/2 + C1*x + ... + C6*x
//         where the remez error is
//
//      |              2     4     6     8     10    12     14 |     -58
//      |cos(x)-(1-.5*x +C1*x +C2*x +C3*x +C4*x +C5*x  +C6*x  )| <= 2
//      |                                                      |
//
//                     4     6     8     10    12     14
//      4. let r = C1*x +C2*x +C3*x +C4*x +C5*x  +C6*x  , then
//             cos(x) ~ 1 - x*x/2 + r
//         since cos(x+y) ~ cos(x) - sin(x)*y
//                        ~ cos(x) - x*y,
//         a correction term is necessary in cos(x) and hence
//              cos(x+y) = 1 - (x*x/2 - (r - x*y))
//         For better accuracy, rearrange to
//              cos(x+y) ~ w + (tmp + (r-x*y))
//         where w = 1 - x*x/2 and tmp is a tiny correction term
//         (1 - x*x/2 == w + tmp exactly in infinite precision).
//         The exactness of w + tmp in infinite precision depends on w
//         and tmp having the same precision as x.  If they have extra
//         precision due to compiler bugs, then the extra precision is
//         only good provided it is retained in all terms of the final
//         expression for cos().  Retention happens in all cases tested
//         under FreeBSD, so don't pessimize things by forcibly clipping
//         any extra precision in w.
fn k_cos(x: f64, y: f64) -> f64 {
    let z = x * x;
    let w = z * z;
    let r = z * (C1 + z * (C2 + z * C3)) + w * w * (C4 + z * (C5 + z * C6));
    let hz = 0.5 * z;
    let w = 1.0 - hz;
    w + (((1.0 - w) - hz) + (z * r - x * y))
}

// ---------------------------------------------------------------------------
// rem_pio2
// ---------------------------------------------------------------------------

// origin: FreeBSD /usr/src/lib/msun/src/e_rem_pio2.c
//
// ====================================================
// Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//
// Developed at SunPro, a Sun Microsystems, Inc. business.
// Permission to use, copy, modify, and distribute this
// software is freely granted, provided that this notice
// is preserved.
// ====================================================
//
// Optimized by Bruce D. Evans. */

// #if FLT_EVAL_METHOD==0 || FLT_EVAL_METHOD==1
// #define EPS DBL_EPSILON
const EPS: f64 = 2.2204460492503131e-16;
// #elif FLT_EVAL_METHOD==2
// #define EPS LDBL_EPSILON
// #endif

const TO_INT: f64 = 1.5 / EPS;
/// 53 bits of 2/pi
const INV_PIO2: f64 = 6.36619772367581382433e-01; /* 0x3FE45F30, 0x6DC9C883 */
/// first 33 bits of pi/2
const PIO2_1: f64 = 1.57079632673412561417e+00; /* 0x3FF921FB, 0x54400000 */
/// pi/2 - PIO2_1
const PIO2_1T: f64 = 6.07710050650619224932e-11; /* 0x3DD0B461, 0x1A626331 */
/// second 33 bits of pi/2
const PIO2_2: f64 = 6.07710050630396597660e-11; /* 0x3DD0B461, 0x1A600000 */
/// pi/2 - (PIO2_1+PIO2_2)
const PIO2_2T: f64 = 2.02226624879595063154e-21; /* 0x3BA3198A, 0x2E037073 */
/// third 33 bits of pi/2
const PIO2_3: f64 = 2.02226624871116645580e-21; /* 0x3BA3198A, 0x2E000000 */
/// pi/2 - (PIO2_1+PIO2_2+PIO2_3)
const PIO2_3T: f64 = 8.47842766036889956997e-32; /* 0x397B839A, 0x252049C1 */

// return the remainder of x rem pi/2 in y[0]+y[1]
// use rem_pio2_large() for large x
//
// caller must handle the case when reduction is not needed: |x| ~<= pi/4 */
fn rem_pio2(x: f64) -> (i32, f64, f64) {
    let x1p24 = f64::from_bits(0x4170000000000000);

    let sign = (f64::to_bits(x) >> 63) as i32;
    let ix = (f64::to_bits(x) >> 32) as u32 & 0x7fffffff;

    fn medium(x: f64, ix: u32) -> (i32, f64, f64) {
        /* rint(x/(pi/2)), Assume round-to-nearest. */
        let tmp = x * INV_PIO2 + TO_INT;
        // The source forces `tmp` to 64 bits here on 32-bit x86 without
        // SSE2. Not carried.
        let f_n = tmp - TO_INT;
        let n = f_n as i32;
        let mut r = x - f_n * PIO2_1;
        let mut w = f_n * PIO2_1T; /* 1st round, good to 85 bits */
        let mut y0 = r - w;
        let ui = f64::to_bits(y0);
        let ey = (ui >> 52) as i32 & 0x7ff;
        let ex = (ix >> 20) as i32;
        if ex - ey > 16 {
            /* 2nd round, good to 118 bits */
            let t = r;
            w = f_n * PIO2_2;
            r = t - w;
            w = f_n * PIO2_2T - ((t - r) - w);
            y0 = r - w;
            let ey = (f64::to_bits(y0) >> 52) as i32 & 0x7ff;
            if ex - ey > 49 {
                /* 3rd round, good to 151 bits, covers all cases */
                let t = r;
                w = f_n * PIO2_3;
                r = t - w;
                w = f_n * PIO2_3T - ((t - r) - w);
                y0 = r - w;
            }
        }
        let y1 = (r - y0) - w;
        (n, y0, y1)
    }

    if ix <= 0x400f6a7a {
        /* |x| ~<= 5pi/4 */
        if (ix & 0xfffff) == 0x921fb {
            /* |x| ~= pi/2 or 2pi/2 */
            return medium(x, ix); /* cancellation -- use medium case */
        }
        if ix <= 0x4002d97c {
            /* |x| ~<= 3pi/4 */
            if sign == 0 {
                let z = x - PIO2_1; /* one round good to 85 bits */
                let y0 = z - PIO2_1T;
                let y1 = (z - y0) - PIO2_1T;
                return (1, y0, y1);
            } else {
                let z = x + PIO2_1;
                let y0 = z + PIO2_1T;
                let y1 = (z - y0) + PIO2_1T;
                return (-1, y0, y1);
            }
        } else if sign == 0 {
            let z = x - 2.0 * PIO2_1;
            let y0 = z - 2.0 * PIO2_1T;
            let y1 = (z - y0) - 2.0 * PIO2_1T;
            return (2, y0, y1);
        } else {
            let z = x + 2.0 * PIO2_1;
            let y0 = z + 2.0 * PIO2_1T;
            let y1 = (z - y0) + 2.0 * PIO2_1T;
            return (-2, y0, y1);
        }
    }
    if ix <= 0x401c463b {
        /* |x| ~<= 9pi/4 */
        if ix <= 0x4015fdbc {
            /* |x| ~<= 7pi/4 */
            if ix == 0x4012d97c {
                /* |x| ~= 3pi/2 */
                return medium(x, ix);
            }
            if sign == 0 {
                let z = x - 3.0 * PIO2_1;
                let y0 = z - 3.0 * PIO2_1T;
                let y1 = (z - y0) - 3.0 * PIO2_1T;
                return (3, y0, y1);
            } else {
                let z = x + 3.0 * PIO2_1;
                let y0 = z + 3.0 * PIO2_1T;
                let y1 = (z - y0) + 3.0 * PIO2_1T;
                return (-3, y0, y1);
            }
        } else {
            if ix == 0x401921fb {
                /* |x| ~= 4pi/2 */
                return medium(x, ix);
            }
            if sign == 0 {
                let z = x - 4.0 * PIO2_1;
                let y0 = z - 4.0 * PIO2_1T;
                let y1 = (z - y0) - 4.0 * PIO2_1T;
                return (4, y0, y1);
            } else {
                let z = x + 4.0 * PIO2_1;
                let y0 = z + 4.0 * PIO2_1T;
                let y1 = (z - y0) + 4.0 * PIO2_1T;
                return (-4, y0, y1);
            }
        }
    }
    if ix < 0x413921fb {
        /* |x| ~< 2^20*(pi/2), medium size */
        return medium(x, ix);
    }
    /*
     * all other (large) arguments
     */
    if ix >= 0x7ff00000 {
        /* x is inf or NaN */
        let y0 = x - x;
        let y1 = y0;
        return (0, y0, y1);
    }
    /* set z = scalbn(|x|,-ilogb(x)+23) */
    let mut ui = f64::to_bits(x);
    ui &= (!1) >> 12;
    ui |= (0x3ff + 23) << 52;
    let mut z = f64::from_bits(ui);
    let mut tx = [0.0; 3];
    for i in 0..2 {
        tx[i] = z as i32 as f64;
        z = (z - tx[i]) * x1p24;
    }
    tx[2] = z;
    /* skip zero terms, first term is non-zero */
    let mut i = 2;
    while i != 0 && tx[i] == 0.0 {
        i -= 1;
    }
    let mut ty = [0.0; 3];
    let n = rem_pio2_large(&tx[..=i], &mut ty, ((ix as i32) >> 20) - (0x3ff + 23));
    if sign != 0 {
        return (-n, -ty[0], -ty[1]);
    }
    (n, ty[0], ty[1])
}

// ---------------------------------------------------------------------------
// rem_pio2_large
// ---------------------------------------------------------------------------

/* origin: FreeBSD /usr/src/lib/msun/src/k_rem_pio2.c */
/*
 * ====================================================
 * Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
 *
 * Developed at SunSoft, a Sun Microsystems, Inc. business.
 * Permission to use, copy, modify, and distribute this
 * software is freely granted, provided that this notice
 * is preserved.
 * ====================================================
 */

// initial value for jk
//
// The source holds [3, 4, 4, 6] for the four number widths and reads entry
// `prec`. This file carries the 64-bit entry, `prec = 1`.
const JK: usize = 4;

// Table of constants for 2/pi, 396 Hex digits (476 decimal) of 2/pi
//
//              integer array, contains the (24*i)-th to (24*i+23)-th
//              bit of 2/pi after binary point. The corresponding
//              floating value is
//
//                      ipio2[i] * 2^(-24(i+1)).
//
// NB: This table must have at least (e0-3)/24 + jk terms.
//     For quad precision (e0 <= 16360, jk = 6), this is 686.
const IPIO2: [i32; 66] = [
    0xA2F983, 0x6E4E44, 0x1529FC, 0x2757D1, 0xF534DD, 0xC0DB62, 0x95993C, 0x439041, 0xFE5163,
    0xABDEBB, 0xC561B7, 0x246E3A, 0x424DD2, 0xE00649, 0x2EEA09, 0xD1921C, 0xFE1DEB, 0x1CB129,
    0xA73EE8, 0x8235F5, 0x2EBB44, 0x84E99C, 0x7026B4, 0x5F7E41, 0x3991D6, 0x398353, 0x39F49C,
    0x845F8B, 0xBDF928, 0x3B1FF8, 0x97FFDE, 0x05980F, 0xEF2F11, 0x8B5A0A, 0x6D1F6D, 0x367ECF,
    0x27CB09, 0xB74F46, 0x3F669E, 0x5FEA2D, 0x7527BA, 0xC7EBE5, 0xF17B3D, 0x0739F7, 0x8A5292,
    0xEA6BFB, 0x5FB11F, 0x8D5D08, 0x560330, 0x46FC7B, 0x6BABF0, 0xCFBC20, 0x9AF436, 0x1DA9E3,
    0x91615E, 0xE61B08, 0x659985, 0x5F14A0, 0x68408D, 0xFFD880, 0x4D7327, 0x310606, 0x1556CA,
    0x73A8C9, 0x60E27B, 0xC08C6B,
];

const PIO2: [f64; 8] = [
    1.57079625129699707031e+00, /* 0x3FF921FB, 0x40000000 */
    7.54978941586159635335e-08, /* 0x3E74442D, 0x00000000 */
    5.39030252995776476554e-15, /* 0x3CF84698, 0x80000000 */
    3.28200341580791294123e-22, /* 0x3B78CC51, 0x60000000 */
    1.27065575308067607349e-29, /* 0x39F01B83, 0x80000000 */
    1.22933308981111328932e-36, /* 0x387A2520, 0x40000000 */
    2.73370053816464559624e-44, /* 0x36E38222, 0x80000000 */
    2.16741683877804819444e-51, /* 0x3569F31D, 0x00000000 */
];

// fn rem_pio2_large(x : &[f64], y : &mut [f64], e0 : i32, prec : usize) -> i32
//
// Input parameters:
//      x[]     The input value (must be positive) is broken into nx
//              pieces of 24-bit integers in double precision format.
//              x[i] will be the i-th 24 bit of x. The scaled exponent
//              of x[0] is given in input parameter e0 (i.e., x[0]*2^e0
//              match x's up to 24 bits.
//
//              Example of breaking a double positive z into x[0]+x[1]+x[2]:
//                      e0 = ilogb(z)-23
//                      z  = scalbn(z,-e0)
//              for i = 0,1,2
//                      x[i] = floor(z)
//                      z    = (z-x[i])*2**24
//
//      y[]     output result in an array of double precision numbers.
//              The dimension of y[] is:
//                      24-bit  precision       1
//                      53-bit  precision       2
//                      64-bit  precision       2
//                      113-bit precision       3
//              The actual value is the sum of them. Thus for 113-bit
//              precison, one may have to do something like:
//
//              long double t,w,r_head, r_tail;
//              t = (long double)y[2] + (long double)y[1];
//              w = (long double)y[0];
//              r_head = t+w;
//              r_tail = w - (r_head - t);
//
//      e0      The exponent of x[0]. Must be <= 16360 or you need to
//              expand the ipio2 table.
//
//      prec    an integer indicating the precision:
//                      0       24  bits (single)
//                      1       53  bits (double)
//                      2       64  bits (extended)
//                      3       113 bits (quad)
//
// Here is the description of some local variables:
//
//      jk      jk+1 is the initial number of terms of ipio2[] needed
//              in the computation. The minimum and recommended value
//              for jk is 3,4,4,6 for single, double, extended, and quad.
//              jk+1 must be 2 larger than you might expect so that our
//              recomputation test works. (Up to 24 bits in the integer
//              part (the 24 bits of it that we compute) and 23 bits in
//              the fraction part may be lost to cancelation before we
//              recompute.)
//
//      jz      local integer variable indicating the number of
//              terms of ipio2[] used.
//
//      jx      nx - 1
//
//      jv      index for pointing to the suitable ipio2[] for the
//              computation. In general, we want
//                      ( 2^e0*x[0] * ipio2[jv-1]*2^(-24jv) )/8
//              is an integer. Thus
//                      e0-3-24*jv >= 0 or (e0-3)/24 >= jv
//              Hence jv = max(0,(e0-3)/24).
//
//      jp      jp+1 is the number of terms in PIo2[] needed, jp = jk.
//
//      q[]     double array with integral value, representing the
//              24-bits chunk of the product of x and 2/pi.
//
//      q0      the corresponding exponent of q[0]. Note that the
//              exponent for q[i] would be q0-24*i.
//
//      PIo2[]  double precision array, obtained by cutting pi/2
//              into 24 bits chunks.
//
//      f[]     ipio2[] in floating point
//
//      iq[]    integer array by breaking up q[] in 24-bits chunk.
//
//      fq[]    final product of x*(2/pi) in fq[0],..,fq[jk]
//
//      ih      integer. If >0 it indicates q[] is >= 0.5, hence
//              it also indicates the *sign* of the result.

/// Return the last three digits of N with y = x - N*pi/2
/// so that |y| < pi/2.
///
/// The method is to compute the integer (mod 8) and fraction parts of
/// (2/pi)*x without doing the full multiplication. In general we
/// skip the part of the product that are known to be a huge integer (
/// more accurately, = 0 mod 8 ). Thus the number of operations are
/// independent of the exponent of the input.
///
/// The 64-bit case of the source (`prec = 1`): `y[0]` and `y[1]` are written.
fn rem_pio2_large(x: &[f64], y: &mut [f64], e0: i32) -> i32 {
    let x1p24 = f64::from_bits(0x4170000000000000); // 0x1p24 === 2 ^ 24
    let x1p_24 = f64::from_bits(0x3e70000000000000); // 0x1p_24 === 2 ^ (-24)

    let nx = x.len();

    let mut fw: f64;
    let mut n: i32;
    let mut ih: i32;
    let mut z: f64;
    let mut f: [f64; 20] = [0.; 20];
    let mut fq: [f64; 20] = [0.; 20];
    let mut q: [f64; 20] = [0.; 20];
    let mut iq: [i32; 20] = [0; 20];

    /* initialize jk*/
    let jk = JK;
    let jp = jk;

    /* determine jx,jv,q0, note that 3>q0 */
    let jx = nx - 1;
    let mut jv = (e0 - 3) / 24;
    if jv < 0 {
        jv = 0;
    }
    let mut q0 = e0 - 24 * (jv + 1);
    let jv = jv as usize;

    /* set up f[0] to f[jx+jk] where f[jx+jk] = ipio2[jv+jk] */
    let mut j = (jv as i32) - (jx as i32);
    let m = jx + jk;
    for i in 0..=m {
        f[i] = if j < 0 { 0. } else { IPIO2[j as usize] as f64 };
        j += 1;
    }

    /* compute q[0],q[1],...q[jk] */
    for i in 0..=jk {
        fw = 0f64;
        for j in 0..=jx {
            fw += x[j] * f[jx + i - j];
        }
        q[i] = fw;
    }

    let mut jz = jk;

    'recompute: loop {
        /* distill q[] into iq[] reversingly */
        let mut i = 0i32;
        z = q[jz];
        for j in (1..=jz).rev() {
            fw = (x1p_24 * z) as i32 as f64;
            iq[i as usize] = (z - x1p24 * fw) as i32;
            z = q[j - 1] + fw;
            i += 1;
        }

        /* compute n */
        z = scalbn(z, q0); /* actual value of z */
        z -= 8.0 * floor(z * 0.125); /* trim off integer >= 8 */
        n = z as i32;
        z -= n as f64;
        ih = 0;
        if q0 > 0 {
            /* need iq[jz-1] to determine n */
            i = iq[jz - 1] >> (24 - q0);
            n += i;
            iq[jz - 1] -= i << (24 - q0);
            ih = iq[jz - 1] >> (23 - q0);
        } else if q0 == 0 {
            ih = iq[jz - 1] >> 23;
        } else if z >= 0.5 {
            ih = 2;
        }

        if ih > 0 {
            /* q > 0.5 */
            n += 1;
            let mut carry = 0i32;
            for i in 0..jz {
                /* compute 1-q */
                let j = iq[i];
                if carry == 0 {
                    if j != 0 {
                        carry = 1;
                        iq[i] = 0x1000000 - j;
                    }
                } else {
                    iq[i] = 0xffffff - j;
                }
            }
            if q0 > 0 {
                /* rare case: chance is 1 in 12 */
                match q0 {
                    1 => {
                        iq[jz - 1] &= 0x7fffff;
                    }
                    2 => {
                        iq[jz - 1] &= 0x3fffff;
                    }
                    _ => {}
                }
            }
            if ih == 2 {
                z = 1. - z;
                if carry != 0 {
                    z -= scalbn(1., q0);
                }
            }
        }

        /* check if recomputation is needed */
        if z == 0. {
            let mut j = 0;
            for i in (jk..=jz - 1).rev() {
                j |= iq[i];
            }
            if j == 0 {
                /* need recomputation */
                let mut k = 1;
                while iq[jk - k] == 0 {
                    k += 1; /* k = no. of terms needed */
                }

                for i in (jz + 1)..=(jz + k) {
                    /* add q[jz+1] to q[jz+k] */
                    f[jx + i] = IPIO2[jv + i] as f64;
                    fw = 0f64;
                    for j in 0..=jx {
                        fw += x[j] * f[jx + i - j];
                    }
                    q[i] = fw;
                }
                jz += k;
                continue 'recompute;
            }
        }

        break;
    }

    /* chop off zero terms */
    if z == 0. {
        jz -= 1;
        q0 -= 24;
        while iq[jz] == 0 {
            jz -= 1;
            q0 -= 24;
        }
    } else {
        /* break z into 24-bit if necessary */
        z = scalbn(z, -q0);
        if z >= x1p24 {
            fw = (x1p_24 * z) as i32 as f64;
            iq[jz] = (z - x1p24 * fw) as i32;
            jz += 1;
            q0 += 24;
            iq[jz] = fw as i32;
        } else {
            iq[jz] = z as i32;
        }
    }

    /* convert integer "bit" chunk to floating-point value */
    fw = scalbn(1., q0);
    for i in (0..=jz).rev() {
        q[i] = fw * (iq[i] as f64);
        fw *= x1p_24;
    }

    /* compute PIo2[0,...,jp]*q[jz,...,0] */
    for i in (0..=jz).rev() {
        fw = 0f64;
        let mut k = 0;
        while (k <= jp) && (k <= jz - i) {
            fw += PIO2[k] * q[i + k];
            k += 1;
        }
        fq[jz - i] = fw;
    }

    /* compress fq[] into y[] */
    // The source's arm for `prec` 1 and 2.
    fw = 0f64;
    for i in (0..=jz).rev() {
        fw += fq[i];
    }
    y[0] = if ih == 0 { fw } else { -fw };
    fw = fq[0] - fw;
    for i in 1..=jz {
        fw += fq[i];
    }
    y[1] = if ih == 0 { fw } else { -fw };
    n & 7
}

// ---------------------------------------------------------------------------
// asin
// ---------------------------------------------------------------------------

/* origin: FreeBSD /usr/src/lib/msun/src/e_asin.c */
/*
 * ====================================================
 * Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
 *
 * Developed at SunSoft, a Sun Microsystems, Inc. business.
 * Permission to use, copy, modify, and distribute this
 * software is freely granted, provided that this notice
 * is preserved.
 * ====================================================
 */
/* asin(x)
 * Method :
 *      Since  asin(x) = x + x^3/6 + x^5*3/40 + x^7*15/336 + ...
 *      we approximate asin(x) on [0,0.5] by
 *              asin(x) = x + x*x^2*R(x^2)
 *      where
 *              R(x^2) is a rational approximation of (asin(x)-x)/x^3
 *      and its remez error is bounded by
 *              |(asin(x)-x)/x^3 - R(x^2)| < 2^(-58.75)
 *
 *      For x in [0.5,1]
 *              asin(x) = pi/2-2*asin(sqrt((1-x)/2))
 *      Let y = (1-x), z = y/2, s := sqrt(z), and pio2_hi+pio2_lo=pi/2;
 *      then for x>0.98
 *              asin(x) = pi/2 - 2*(s+s*z*R(z))
 *                      = pio2_hi - (2*(s+s*z*R(z)) - pio2_lo)
 *      For x<=0.98, let pio4_hi = pio2_hi/2, then
 *              f = hi part of s;
 *              c = sqrt(z) - f = (z-f*f)/(s+f)         ...f+c=sqrt(z)
 *      and
 *              asin(x) = pi/2 - 2*(s+s*z*R(z))
 *                      = pio4_hi+(pio4-2s)-(2s*z*R(z)-pio2_lo)
 *                      = pio4_hi+(pio4-2f)-(2s*z*R(z)-(pio2_lo+2c))
 *
 * Special cases:
 *      if x is NaN, return x itself;
 *      if |x|>1, return NaN with invalid signal.
 *
 */

const PIO2_HI: f64 = 1.57079632679489655800e+00; /* 0x3FF921FB, 0x54442D18 */
const PIO2_LO: f64 = 6.12323399573676603587e-17; /* 0x3C91A626, 0x33145C07 */
/* coefficients for R(x^2) */
const P_S0: f64 = 1.66666666666666657415e-01; /* 0x3FC55555, 0x55555555 */
const P_S1: f64 = -3.25565818622400915405e-01; /* 0xBFD4D612, 0x03EB6F7D */
const P_S2: f64 = 2.01212532134862925881e-01; /* 0x3FC9C155, 0x0E884455 */
const P_S3: f64 = -4.00555345006794114027e-02; /* 0xBFA48228, 0xB5688F3B */
const P_S4: f64 = 7.91534994289814532176e-04; /* 0x3F49EFE0, 0x7501B288 */
const P_S5: f64 = 3.47933107596021167570e-05; /* 0x3F023DE1, 0x0DFDF709 */
const Q_S1: f64 = -2.40339491173441421878e+00; /* 0xC0033A27, 0x1C8A2D4B */
const Q_S2: f64 = 2.02094576023350569471e+00; /* 0x40002AE5, 0x9C598AC8 */
const Q_S3: f64 = -6.88283971605453293030e-01; /* 0xBFE6066C, 0x1B8D0159 */
const Q_S4: f64 = 7.70381505559019352791e-02; /* 0x3FB3B8C5, 0xB12E9282 */

fn comp_r(z: f64) -> f64 {
    let p = z * (P_S0 + z * (P_S1 + z * (P_S2 + z * (P_S3 + z * (P_S4 + z * P_S5)))));
    let q = 1.0 + z * (Q_S1 + z * (Q_S2 + z * (Q_S3 + z * Q_S4)));
    p / q
}

/// Arcsine in radians, for an argument from −1 to 1; NaN outside. The source states no bound in units in the last place; the tests here hold it within one.
pub fn asin(mut x: f64) -> f64 {
    let z: f64;
    let r: f64;
    let s: f64;
    let hx: u32;
    let ix: u32;

    hx = get_high_word(x);
    ix = hx & 0x7fffffff;
    /* |x| >= 1 or nan */
    if ix >= 0x3ff00000 {
        let lx: u32;
        lx = get_low_word(x);
        if ((ix - 0x3ff00000) | lx) == 0 {
            /* asin(1) = +-pi/2 with inexact */
            return x * PIO2_HI + f64::from_bits(0x3870000000000000);
        } else {
            return 0.0 / (x - x);
        }
    }
    /* |x| < 0.5 */
    if ix < 0x3fe00000 {
        /* if 0x1p-1022 <= |x| < 0x1p-26, avoid raising underflow */
        if (0x00100000..0x3e500000).contains(&ix) {
            return x;
        } else {
            return x + x * comp_r(x * x);
        }
    }
    /* 1 > |x| >= 0.5 */
    z = (1.0 - fabs(x)) * 0.5;
    s = f64::sqrt(z);
    r = comp_r(z);
    if ix >= 0x3fef3333 {
        /* if |x| > 0.975 */
        x = PIO2_HI - (2. * (s + s * r) - PIO2_LO);
    } else {
        let f: f64;
        let c: f64;
        /* f+c = sqrt(z) */
        f = with_set_low_word(s, 0);
        c = (z - f * f) / (s + f);
        x = 0.5 * PIO2_HI - (2.0 * s * r - (PIO2_LO - 2.0 * c) - (0.5 * PIO2_HI - 2.0 * f));
    }
    if hx >> 31 != 0 { -x } else { x }
}

// ---------------------------------------------------------------------------
// hypot
// ---------------------------------------------------------------------------

// origin: musl src/math/hypot.c. Neither that file nor the Rust file carries
// a notice of its own. musl's COPYRIGHT file covers it:
//
// ----------------------------------------------------------------------
// Copyright © 2005-2020 Rich Felker, et al.
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the
// "Software"), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to
// permit persons to whom the Software is furnished to do so, subject to
// the following conditions:
//
// The above copyright notice and this permission notice shall be
// included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
// SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
// ----------------------------------------------------------------------

const SPLIT: f64 = 134217728. + 1.; // 0x1p27 + 1 === (2 ^ 27) + 1

fn sq(x: f64) -> (f64, f64) {
    let xh: f64;
    let xl: f64;
    let xc: f64;

    xc = x * SPLIT;
    xh = x - xc + xc;
    xl = x - xh;
    let hi = x * x;
    let lo = xh * xh - hi + 2. * xh * xl + xl * xl;
    (hi, lo)
}

/// Square root of `x² + y²`, with no overflow or underflow on the way. The source states no bound; the tests here hold it within one unit in the last place.
pub fn hypot(mut x: f64, mut y: f64) -> f64 {
    let x1p700 = f64::from_bits(0x6bb0000000000000); // 0x1p700 === 2 ^ 700
    let x1p_700 = f64::from_bits(0x1430000000000000); // 0x1p-700 === 2 ^ -700

    let mut uxi = x.to_bits();
    let mut uyi = y.to_bits();
    let uti;
    let ex: i64;
    let ey: i64;
    let mut z: f64;

    /* arrange |x| >= |y| */
    uxi &= -1i64 as u64 >> 1;
    uyi &= -1i64 as u64 >> 1;
    if uxi < uyi {
        uti = uxi;
        uxi = uyi;
        uyi = uti;
    }

    /* special cases */
    ex = (uxi >> 52) as i64;
    ey = (uyi >> 52) as i64;
    x = f64::from_bits(uxi);
    y = f64::from_bits(uyi);
    /* note: hypot(inf,nan) == inf */
    if ey == 0x7ff {
        return y;
    }
    if ex == 0x7ff || uyi == 0 {
        return x;
    }
    /* note: hypot(x,y) ~= x + y*y/x/2 with inexact for small y/x */
    /* 64 difference is enough for ld80 double_t */
    if ex - ey > 64 {
        return x + y;
    }

    /* precise sqrt argument in nearest rounding mode without overflow */
    /* xh*xh must not overflow and xl*xl must not underflow in sq */
    z = 1.;
    if ex > 0x3ff + 510 {
        z = x1p700;
        x *= x1p_700;
        y *= x1p_700;
    } else if ey < 0x3ff - 450 {
        z = x1p_700;
        x *= x1p700;
        y *= x1p700;
    }
    let (hx, lx) = sq(x);
    let (hy, ly) = sq(y);
    z * f64::sqrt(ly + lx + hy + hx)
}

// ---------------------------------------------------------------------------
// exp
// ---------------------------------------------------------------------------

/* origin: FreeBSD /usr/src/lib/msun/src/e_exp.c */
/*
 * ====================================================
 * Copyright (C) 2004 by Sun Microsystems, Inc. All rights reserved.
 *
 * Permission to use, copy, modify, and distribute this
 * software is freely granted, provided that this notice
 * is preserved.
 * ====================================================
 */
/* exp(x)
 * Returns the exponential of x.
 *
 * Method
 *   1. Argument reduction:
 *      Reduce x to an r so that |r| <= 0.5*ln2 ~ 0.34658.
 *      Given x, find r and integer k such that
 *
 *               x = k*ln2 + r,  |r| <= 0.5*ln2.
 *
 *      Here r will be represented as r = hi-lo for better
 *      accuracy.
 *
 *   2. Approximation of exp(r) by a special rational function on
 *      the interval [0,0.34658]:
 *      Write
 *          R(r**2) = r*(exp(r)+1)/(exp(r)-1) = 2 + r*r/6 - r**4/360 + ...
 *      We use a special Remez algorithm on [0,0.34658] to generate
 *      a polynomial of degree 5 to approximate R. The maximum error
 *      of this polynomial approximation is bounded by 2**-59. In
 *      other words,
 *          R(z) ~ 2.0 + P1*z + P2*z**2 + P3*z**3 + P4*z**4 + P5*z**5
 *      (where z=r*r, and the values of P1 to P5 are listed below)
 *      and
 *          |                  5          |     -59
 *          | 2.0+P1*z+...+P5*z   -  R(z) | <= 2
 *          |                             |
 *      The computation of exp(r) thus becomes
 *                              2*r
 *              exp(r) = 1 + ----------
 *                            R(r) - r
 *                                 r*c(r)
 *                     = 1 + r + ----------- (for better accuracy)
 *                                2 - c(r)
 *      where
 *                              2       4             10
 *              c(r) = r - (P1*r  + P2*r  + ... + P5*r   ).
 *
 *   3. Scale back to obtain exp(x):
 *      From step 1, we have
 *         exp(x) = 2^k * exp(r)
 *
 * Special cases:
 *      exp(INF) is INF, exp(NaN) is NaN;
 *      exp(-INF) is 0, and
 *      for finite argument, only exp(0)=1 is exact.
 *
 * Accuracy:
 *      according to an error analysis, the error is always less than
 *      1 ulp (unit in the last place).
 *
 * Misc. info.
 *      For IEEE double
 *          if x >  709.782712893383973096 then exp(x) overflows
 *          if x < -745.133219101941108420 then exp(x) underflows
 */

const HALF: [f64; 2] = [0.5, -0.5];
const LN2HI: f64 = 6.93147180369123816490e-01; /* 0x3fe62e42, 0xfee00000 */
const LN2LO: f64 = 1.90821492927058770002e-10; /* 0x3dea39ef, 0x35793c76 */
const INVLN2: f64 = 1.44269504088896338700e+00; /* 0x3ff71547, 0x652b82fe */
const P1: f64 = 1.66666666666666019037e-01; /* 0x3FC55555, 0x5555553E */
const P2: f64 = -2.77777777770155933842e-03; /* 0xBF66C16C, 0x16BEBD93 */
const P3: f64 = 6.61375632143793436117e-05; /* 0x3F11566A, 0xAF25DE2C */
const P4: f64 = -1.65339022054652515390e-06; /* 0xBEBBBD41, 0xC5D26BF1 */
const P5: f64 = 4.13813679705723846039e-08; /* 0x3E663769, 0x72BEA4D0 */

/// The number e raised to the power `x`. The source states "the error is always less than 1 ulp (unit in the last place)".
pub fn exp(mut x: f64) -> f64 {
    let x1p1023 = f64::from_bits(0x7fe0000000000000); // 0x1p1023 === 2 ^ 1023

    let hi: f64;
    let lo: f64;
    let c: f64;
    let xx: f64;
    let y: f64;
    let k: i32;
    let sign: i32;
    let mut hx: u32;

    hx = (x.to_bits() >> 32) as u32;
    sign = (hx >> 31) as i32;
    hx &= 0x7fffffff; /* high word of |x| */

    /* special cases */
    if hx >= 0x4086232b {
        /* if |x| >= 708.39... */
        if x.is_nan() {
            return x;
        }
        if x > 709.782712893383973096 {
            /* overflow if x!=inf */
            x *= x1p1023;
            return x;
        }
        if x < -708.39641853226410622 {
            /* underflow if x!=-inf */
            // The source raises the flag here. Not carried.
            if x < -745.13321910194110842 {
                return 0.;
            }
        }
    }

    /* argument reduction */
    if hx > 0x3fd62e42 {
        /* if |x| > 0.5 ln2 */
        if hx >= 0x3ff0a2b2 {
            /* if |x| >= 1.5 ln2 */
            k = (INVLN2 * x + HALF[sign as usize]) as i32;
        } else {
            k = 1 - sign - sign;
        }
        hi = x - k as f64 * LN2HI; /* k*ln2hi is exact here */
        lo = k as f64 * LN2LO;
        x = hi - lo;
    } else if hx > 0x3e300000 {
        /* if |x| > 2**-28 */
        k = 0;
        hi = x;
        lo = 0.;
    } else {
        /* inexact if x!=0 */
        // The source raises the flag here. Not carried.
        return 1. + x;
    }

    /* x is now in primary range */
    xx = x * x;
    c = x - xx * (P1 + xx * (P2 + xx * (P3 + xx * (P4 + xx * P5))));
    y = 1. + (x * c / (2. - c) - lo + hi);
    if k == 0 { y } else { scalbn(y, k) }
}

// ---------------------------------------------------------------------------
// exp_m1
// ---------------------------------------------------------------------------

/* origin: FreeBSD /usr/src/lib/msun/src/s_expm1.c */
/*
 * ====================================================
 * Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
 *
 * Developed at SunPro, a Sun Microsystems, Inc. business.
 * Permission to use, copy, modify, and distribute this
 * software is freely granted, provided that this notice
 * is preserved.
 * ====================================================
 */

const O_THRESHOLD: f64 = 7.09782712893383973096e+02; /* 0x40862E42, 0xFEFA39EF */
const LN2_HI: f64 = 6.93147180369123816490e-01; /* 0x3fe62e42, 0xfee00000 */
const LN2_LO: f64 = 1.90821492927058770002e-10; /* 0x3dea39ef, 0x35793c76 */
const EXPM1_INVLN2: f64 = 1.44269504088896338700e+00; /* 0x3ff71547, 0x652b82fe */
/* Scaled Q's: Qn_here = 2**n * Qn_above, for R(2*z) where z = hxs = x*x/2: */
const Q1: f64 = -3.33333333333331316428e-02; /* BFA11111 111110F4 */
const Q2: f64 = 1.58730158725481460165e-03; /* 3F5A01A0 19FE5585 */
const Q3: f64 = -7.93650757867487942473e-05; /* BF14CE19 9EAADBB7 */
const Q4: f64 = 4.00821782732936239552e-06; /* 3ED0CFCA 86E65239 */
const Q5: f64 = -2.01099218183624371326e-07; /* BE8AFDB7 6E09C32D */

/// The number e raised to the power `x`, minus one. The Rust source states no bound; FreeBSD `s_expm1.c`, its origin, states "the error is always less than 1 ulp (unit in the last place)".
pub fn exp_m1(mut x: f64) -> f64 {
    let hi: f64;
    let lo: f64;
    let k: i32;
    let c: f64;
    let mut t: f64;
    let mut y: f64;

    let mut ui = x.to_bits();
    let hx = ((ui >> 32) & 0x7fffffff) as u32;
    let sign = (ui >> 63) as i32;

    /* filter out huge and non-finite argument */
    if hx >= 0x4043687A {
        /* if |x|>=56*ln2 */
        if x.is_nan() {
            return x;
        }
        if sign != 0 {
            return -1.0;
        }
        if x > O_THRESHOLD {
            x *= f64::from_bits(0x7fe0000000000000);
            return x;
        }
    }

    /* argument reduction */
    if hx > 0x3fd62e42 {
        /* if  |x| > 0.5 ln2 */
        if hx < 0x3FF0A2B2 {
            /* and |x| < 1.5 ln2 */
            if sign == 0 {
                hi = x - LN2_HI;
                lo = LN2_LO;
                k = 1;
            } else {
                hi = x + LN2_HI;
                lo = -LN2_LO;
                k = -1;
            }
        } else {
            k = (EXPM1_INVLN2 * x + if sign != 0 { -0.5 } else { 0.5 }) as i32;
            t = k as f64;
            hi = x - t * LN2_HI; /* t*ln2_hi is exact here */
            lo = t * LN2_LO;
        }
        x = hi - lo;
        c = (hi - x) - lo;
    } else if hx < 0x3c900000 {
        /* |x| < 2**-54, return x */
        // The source raises a flag here for a subnormal `x`. Not carried.
        return x;
    } else {
        c = 0.0;
        k = 0;
    }

    /* x is now in primary range */
    let hfx = 0.5 * x;
    let hxs = x * hfx;
    let r1 = 1.0 + hxs * (Q1 + hxs * (Q2 + hxs * (Q3 + hxs * (Q4 + hxs * Q5))));
    t = 3.0 - r1 * hfx;
    let mut e = hxs * ((r1 - t) / (6.0 - x * t));
    if k == 0 {
        /* c is 0 */
        return x - (x * e - hxs);
    }
    e = x * (e - c) - c;
    e -= hxs;
    /* exp(x) ~ 2^k (x_reduced - e + 1) */
    if k == -1 {
        return 0.5 * (x - e) - 0.5;
    }
    if k == 1 {
        if x < -0.25 {
            return -2.0 * (e - (x + 0.5));
        }
        return 1.0 + 2.0 * (x - e);
    }
    ui = ((0x3ff + k) as u64) << 52; /* 2^k */
    let twopk = f64::from_bits(ui);
    if !(0..=56).contains(&k) {
        /* suffice to return exp(x)-1 */
        y = x - e + 1.0;
        if k == 1024 {
            y = y * 2.0 * f64::from_bits(0x7fe0000000000000);
        } else {
            y = y * twopk;
        }
        return y - 1.0;
    }
    ui = ((0x3ff - k) as u64) << 52; /* 2^-k */
    let uf = f64::from_bits(ui);
    if k < 20 {
        y = (x - e + (1.0 - uf)) * twopk;
    } else {
        y = (x - (e + uf) + 1.0) * twopk;
    }
    y
}

// ---------------------------------------------------------------------------
// log
// ---------------------------------------------------------------------------

/* origin: FreeBSD /usr/src/lib/msun/src/e_log.c */
/*
 * ====================================================
 * Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
 *
 * Developed at SunSoft, a Sun Microsystems, Inc. business.
 * Permission to use, copy, modify, and distribute this
 * software is freely granted, provided that this notice
 * is preserved.
 * ====================================================
 */
/* log(x)
 * Return the logarithm of x
 *
 * Method :
 *   1. Argument Reduction: find k and f such that
 *                      x = 2^k * (1+f),
 *         where  sqrt(2)/2 < 1+f < sqrt(2) .
 *
 *   2. Approximation of log(1+f).
 *      Let s = f/(2+f) ; based on log(1+f) = log(1+s) - log(1-s)
 *               = 2s + 2/3 s**3 + 2/5 s**5 + .....,
 *               = 2s + s*R
 *      We use a special Remez algorithm on [0,0.1716] to generate
 *      a polynomial of degree 14 to approximate R The maximum error
 *      of this polynomial approximation is bounded by 2**-58.45. In
 *      other words,
 *                      2      4      6      8      10      12      14
 *          R(z) ~ Lg1*s +Lg2*s +Lg3*s +Lg4*s +Lg5*s  +Lg6*s  +Lg7*s
 *      (the values of Lg1 to Lg7 are listed in the program)
 *      and
 *          |      2          14          |     -58.45
 *          | Lg1*s +...+Lg7*s    -  R(z) | <= 2
 *          |                             |
 *      Note that 2s = f - s*f = f - hfsq + s*hfsq, where hfsq = f*f/2.
 *      In order to guarantee error in log below 1ulp, we compute log
 *      by
 *              log(1+f) = f - s*(f - R)        (if f is not too large)
 *              log(1+f) = f - (hfsq - s*(hfsq+R)).     (better accuracy)
 *
 *      3. Finally,  log(x) = k*ln2 + log(1+f).
 *                          = k*ln2_hi+(f-(hfsq-(s*(hfsq+R)+k*ln2_lo)))
 *         Here ln2 is split into two floating point number:
 *                      ln2_hi + ln2_lo,
 *         where n*ln2_hi is always exact for |n| < 2000.
 *
 * Special cases:
 *      log(x) is NaN with signal if x < 0 (including -INF) ;
 *      log(+INF) is +INF; log(0) is -INF with signal;
 *      log(NaN) is that NaN with no signal.
 *
 * Accuracy:
 *      according to an error analysis, the error is always less than
 *      1 ulp (unit in the last place).
 *
 * Constants:
 * The hexadecimal values are the intended ones for the following
 * constants. The decimal values may be used, provided that the
 * compiler will convert from decimal to binary accurately enough
 * to produce the hexadecimal values shown.
 */

const LG1: f64 = 6.666666666666735130e-01; /* 3FE55555 55555593 */
const LG2: f64 = 3.999999999940941908e-01; /* 3FD99999 9997FA04 */
const LG3: f64 = 2.857142874366239149e-01; /* 3FD24924 94229359 */
const LG4: f64 = 2.222219843214978396e-01; /* 3FCC71C5 1D8E78AF */
const LG5: f64 = 1.818357216161805012e-01; /* 3FC74664 96CB03DE */
const LG6: f64 = 1.531383769920937332e-01; /* 3FC39A09 D078C69F */
const LG7: f64 = 1.479819860511658591e-01; /* 3FC2F112 DF3E5244 */

/// The natural logarithm of `x`. The source states "the error is always less than 1 ulp (unit in the last place)".
pub fn log(mut x: f64) -> f64 {
    let x1p54 = f64::from_bits(0x4350000000000000); // 0x1p54 === 2 ^ 54

    let mut ui = x.to_bits();
    let mut hx: u32 = (ui >> 32) as u32;
    let mut k: i32 = 0;

    if (hx < 0x00100000) || ((hx >> 31) != 0) {
        /* x < 2**-126  */
        if ui << 1 == 0 {
            return -1. / (x * x); /* log(+-0)=-inf */
        }
        if hx >> 31 != 0 {
            return (x - x) / 0.0; /* log(-#) = NaN */
        }
        /* subnormal number, scale x up */
        k -= 54;
        x *= x1p54;
        ui = x.to_bits();
        hx = (ui >> 32) as u32;
    } else if hx >= 0x7ff00000 {
        return x;
    } else if hx == 0x3ff00000 && ui << 32 == 0 {
        return 0.;
    }

    /* reduce x into [sqrt(2)/2, sqrt(2)] */
    hx += 0x3ff00000 - 0x3fe6a09e;
    k += ((hx >> 20) as i32) - 0x3ff;
    hx = (hx & 0x000fffff) + 0x3fe6a09e;
    ui = ((hx as u64) << 32) | (ui & 0xffffffff);
    x = f64::from_bits(ui);

    let f: f64 = x - 1.0;
    let hfsq: f64 = 0.5 * f * f;
    let s: f64 = f / (2.0 + f);
    let z: f64 = s * s;
    let w: f64 = z * z;
    let t1: f64 = w * (LG2 + w * (LG4 + w * LG6));
    let t2: f64 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
    let r: f64 = t2 + t1;
    let dk: f64 = k as f64;
    s * (hfsq + r) + dk * LN2_LO - hfsq + f + dk * LN2_HI
}

// ---------------------------------------------------------------------------
// ln_1p
// ---------------------------------------------------------------------------

/* origin: FreeBSD /usr/src/lib/msun/src/s_log1p.c */
/*
 * ====================================================
 * Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
 *
 * Developed at SunPro, a Sun Microsystems, Inc. business.
 * Permission to use, copy, modify, and distribute this
 * software is freely granted, provided that this notice
 * is preserved.
 * ====================================================
 */
/* double log1p(double x)
 * Return the natural logarithm of 1+x.
 *
 * Method :
 *   1. Argument Reduction: find k and f such that
 *                      1+x = 2^k * (1+f),
 *         where  sqrt(2)/2 < 1+f < sqrt(2) .
 *
 *      Note. If k=0, then f=x is exact. However, if k!=0, then f
 *      may not be representable exactly. In that case, a correction
 *      term is need. Let u=1+x rounded. Let c = (1+x)-u, then
 *      log(1+x) - log(u) ~ c/u. Thus, we proceed to compute log(u),
 *      and add back the correction term c/u.
 *      (Note: when x > 2**53, one can simply return log(x))
 *
 *   2. Approximation of log(1+f): See log.c
 *
 *   3. Finally, log1p(x) = k*ln2 + log(1+f) + c/u. See log.c
 *
 * Special cases:
 *      log1p(x) is NaN with signal if x < -1 (including -INF) ;
 *      log1p(+INF) is +INF; log1p(-1) is -INF with signal;
 *      log1p(NaN) is that NaN with no signal.
 *
 * Accuracy:
 *      according to an error analysis, the error is always less than
 *      1 ulp (unit in the last place).
 *
 * Constants:
 * The hexadecimal values are the intended ones for the following
 * constants. The decimal values may be used, provided that the
 * compiler will convert from decimal to binary accurately enough
 * to produce the hexadecimal values shown.
 *
 * Note: Assuming log() return accurate answer, the following
 *       algorithm can be used to compute log1p(x) to within a few ULP:
 *
 *              u = 1+x;
 *              if(u==1.0) return x ; else
 *                         return log(u)*(x/(u-1.0));
 *
 *       See HP-15C Advanced Functions Handbook, p.193.
 */

// The source names LN2_HI, LN2_LO and LG1 to LG7 with the values `log`
// above gives them. Those serve here.

/// The natural logarithm of `1 + x`. The source states "the error is always less than 1 ulp (unit in the last place)".
pub fn ln_1p(x: f64) -> f64 {
    let mut ui: u64 = x.to_bits();
    let hfsq: f64;
    let mut f: f64 = 0.;
    let mut c: f64 = 0.;
    let s: f64;
    let z: f64;
    let r: f64;
    let w: f64;
    let t1: f64;
    let t2: f64;
    let dk: f64;
    let hx: u32;
    let mut hu: u32;
    let mut k: i32;

    hx = (ui >> 32) as u32;
    k = 1;
    if hx < 0x3fda827a || (hx >> 31) > 0 {
        /* 1+x < sqrt(2)+ */
        if hx >= 0xbff00000 {
            /* x <= -1.0 */
            if x == -1. {
                return x / 0.0; /* log1p(-1) = -inf */
            }
            return (x - x) / 0.0; /* log1p(x<-1) = NaN */
        }
        if hx << 1 < 0x3ca00000 << 1 {
            /* |x| < 2**-53 */
            /* underflow if subnormal */
            // The source raises the underflow flag here. Not carried.
            return x;
        }
        if hx <= 0xbfd2bec4 {
            /* sqrt(2)/2- <= 1+x < sqrt(2)+ */
            k = 0;
            c = 0.;
            f = x;
        }
    } else if hx >= 0x7ff00000 {
        return x;
    }
    if k > 0 {
        ui = (1. + x).to_bits();
        hu = (ui >> 32) as u32;
        hu += 0x3ff00000 - 0x3fe6a09e;
        k = (hu >> 20) as i32 - 0x3ff;
        /* correction term ~ log(1+x)-log(u), avoid underflow in c/u */
        if k < 54 {
            c = if k >= 2 {
                1. - (f64::from_bits(ui) - x)
            } else {
                x - (f64::from_bits(ui) - 1.)
            };
            c /= f64::from_bits(ui);
        } else {
            c = 0.;
        }
        /* reduce u into [sqrt(2)/2, sqrt(2)] */
        hu = (hu & 0x000fffff) + 0x3fe6a09e;
        ui = ((hu as u64) << 32) | (ui & 0xffffffff);
        f = f64::from_bits(ui) - 1.;
    }
    hfsq = 0.5 * f * f;
    s = f / (2.0 + f);
    z = s * s;
    w = z * z;
    t1 = w * (LG2 + w * (LG4 + w * LG6));
    t2 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
    r = t2 + t1;
    dk = k as f64;
    s * (hfsq + r) + (dk * LN2_LO + c) - hfsq + f + dk * LN2_HI
}

// ---------------------------------------------------------------------------
// log10
// ---------------------------------------------------------------------------

/* origin: FreeBSD /usr/src/lib/msun/src/e_log10.c */
/*
 * ====================================================
 * Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
 *
 * Developed at SunSoft, a Sun Microsystems, Inc. business.
 * Permission to use, copy, modify, and distribute this
 * software is freely granted, provided that this notice
 * is preserved.
 * ====================================================
 */
/*
 * Return the base 10 logarithm of x.  See log.c for most comments.
 *
 * Reduce x to 2^k (1+f) and calculate r = log(1+f) - f + f*f/2
 * as in log.c, then combine and scale in extra precision:
 *    log10(x) = (f - f*f/2 + r)/log(10) + k*log10(2)
 */

const IVLN10HI: f64 = 4.34294481878168880939e-01; /* 0x3fdbcb7b, 0x15200000 */
const IVLN10LO: f64 = 2.50829467116452752298e-11; /* 0x3dbb9438, 0xca9aadd5 */
const LOG10_2HI: f64 = 3.01029995663611771306e-01; /* 0x3FD34413, 0x509F6000 */
const LOG10_2LO: f64 = 3.69423907715893078616e-13; /* 0x3D59FEF3, 0x11F12B36 */
// The source names LG1 to LG7 with the values `log` above gives them. Those
// serve here.

/// The base 10 logarithm of `x`. The source states no bound in units in the last place; the tests here measure it.
pub fn log10(mut x: f64) -> f64 {
    let x1p54 = f64::from_bits(0x4350000000000000); // 0x1p54 === 2 ^ 54

    let mut ui: u64 = x.to_bits();
    let hfsq: f64;
    let f: f64;
    let s: f64;
    let z: f64;
    let r: f64;
    let mut w: f64;
    let t1: f64;
    let t2: f64;
    let dk: f64;
    let y: f64;
    let mut hi: f64;
    let lo: f64;
    let mut val_hi: f64;
    let mut val_lo: f64;
    let mut hx: u32;
    let mut k: i32;

    hx = (ui >> 32) as u32;
    k = 0;
    if hx < 0x00100000 || (hx >> 31) > 0 {
        if ui << 1 == 0 {
            return -1. / (x * x); /* log(+-0)=-inf */
        }
        if (hx >> 31) > 0 {
            return (x - x) / 0.0; /* log(-#) = NaN */
        }
        /* subnormal number, scale x up */
        k -= 54;
        x *= x1p54;
        ui = x.to_bits();
        hx = (ui >> 32) as u32;
    } else if hx >= 0x7ff00000 {
        return x;
    } else if hx == 0x3ff00000 && ui << 32 == 0 {
        return 0.;
    }

    /* reduce x into [sqrt(2)/2, sqrt(2)] */
    hx += 0x3ff00000 - 0x3fe6a09e;
    k += (hx >> 20) as i32 - 0x3ff;
    hx = (hx & 0x000fffff) + 0x3fe6a09e;
    ui = ((hx as u64) << 32) | (ui & 0xffffffff);
    x = f64::from_bits(ui);

    f = x - 1.0;
    hfsq = 0.5 * f * f;
    s = f / (2.0 + f);
    z = s * s;
    w = z * z;
    t1 = w * (LG2 + w * (LG4 + w * LG6));
    t2 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
    r = t2 + t1;

    /* See log2.c for details. */
    /* hi+lo = f - hfsq + s*(hfsq+R) ~ log(1+f) */
    hi = f - hfsq;
    ui = hi.to_bits();
    ui &= (-1i64 as u64) << 32;
    hi = f64::from_bits(ui);
    lo = f - hi - hfsq + s * (hfsq + r);

    /* val_hi+val_lo ~ log10(1+f) + k*log10(2) */
    val_hi = hi * IVLN10HI;
    dk = k as f64;
    y = dk * LOG10_2HI;
    val_lo = dk * LOG10_2LO + (lo + hi) * IVLN10LO + lo * IVLN10HI;

    /*
     * Extra precision in for adding y is not strictly needed
     * since there is no very large cancellation near x = sqrt(2) or
     * x = 1/sqrt(2), but we do it anyway since it costs little on CPUs
     * with some parallelism and it reduces the error for many args.
     */
    w = y + val_hi;
    val_lo += (y - w) + val_hi;
    val_hi = w;

    val_lo + val_hi
}

// ---------------------------------------------------------------------------
// powf
// ---------------------------------------------------------------------------

/* origin: FreeBSD /usr/src/lib/msun/src/e_pow.c */
/*
 * ====================================================
 * Copyright (C) 2004 by Sun Microsystems, Inc. All rights reserved.
 *
 * Permission to use, copy, modify, and distribute this
 * software is freely granted, provided that this notice
 * is preserved.
 * ====================================================
 */

// pow(x,y) return x**y
//
//                    n
// Method:  Let x =  2   * (1+f)
//      1. Compute and return log2(x) in two pieces:
//              log2(x) = w1 + w2,
//         where w1 has 53-24 = 29 bit trailing zeros.
//      2. Perform y*log2(x) = n+y' by simulating multi-precision
//         arithmetic, where |y'|<=0.5.
//      3. Return x**y = 2**n*exp(y'*log2)
//
// Special cases:
//      1.  (anything) ** 0  is 1
//      2.  1 ** (anything)  is 1
//      3.  (anything except 1) ** NAN is NAN
//      4.  NAN ** (anything except 0) is NAN
//      5.  +-(|x| > 1) **  +INF is +INF
//      6.  +-(|x| > 1) **  -INF is +0
//      7.  +-(|x| < 1) **  +INF is +0
//      8.  +-(|x| < 1) **  -INF is +INF
//      9.  -1          ** +-INF is 1
//      10. +0 ** (+anything except 0, NAN)               is +0
//      11. -0 ** (+anything except 0, NAN, odd integer)  is +0
//      12. +0 ** (-anything except 0, NAN)               is +INF, raise divbyzero
//      13. -0 ** (-anything except 0, NAN, odd integer)  is +INF, raise divbyzero
//      14. -0 ** (+odd integer) is -0
//      15. -0 ** (-odd integer) is -INF, raise divbyzero
//      16. +INF ** (+anything except 0,NAN) is +INF
//      17. +INF ** (-anything except 0,NAN) is +0
//      18. -INF ** (+odd integer) is -INF
//      19. -INF ** (anything) = -0 ** (-anything), (anything except odd integer)
//      20. (anything) ** 1 is (anything)
//      21. (anything) ** -1 is 1/(anything)
//      22. (-anything) ** (integer) is (-1)**(integer)*(+anything**integer)
//      23. (-anything except 0 and inf) ** (non-integer) is NAN
//
// Accuracy:
//      pow(x,y) returns x**y nearly rounded. In particular
//                      pow(integer,integer)
//      always returns the correct integer provided it is
//      representable.
//
// Constants :
// The hexadecimal values are the intended ones for the following
// constants. The decimal values may be used, provided that the
// compiler will convert from decimal to binary accurately enough
// to produce the hexadecimal values shown.
//

const BP: [f64; 2] = [1.0, 1.5];
const DP_H: [f64; 2] = [0.0, 5.84962487220764160156e-01]; /* 0x3fe2b803_40000000 */
const DP_L: [f64; 2] = [0.0, 1.35003920212974897128e-08]; /* 0x3E4CFDEB, 0x43CFD006 */
const TWO53: f64 = 9007199254740992.0; /* 0x43400000_00000000 */
const HUGE: f64 = 1.0e300;
const TINY: f64 = 1.0e-300;

// poly coefs for (3/2)*(log(x)-2s-2/3*s**3:
const L1: f64 = 5.99999999999994648725e-01; /* 0x3fe33333_33333303 */
const L2: f64 = 4.28571428578550184252e-01; /* 0x3fdb6db6_db6fabff */
const L3: f64 = 3.33333329818377432918e-01; /* 0x3fd55555_518f264d */
const L4: f64 = 2.72728123808534006489e-01; /* 0x3fd17460_a91d4101 */
const L5: f64 = 2.30660745775561754067e-01; /* 0x3fcd864a_93c9db65 */
const L6: f64 = 2.06975017800338417784e-01; /* 0x3fca7e28_4a454eef */
// The source names P1 to P5 with the values `exp` above gives them. Those
// serve here.
const POW_LG2: f64 = 6.93147180559945286227e-01; /* 0x3fe62e42_fefa39ef */
const POW_LG2_H: f64 = 6.93147182464599609375e-01; /* 0x3fe62e43_00000000 */
const POW_LG2_L: f64 = -1.90465429995776804525e-09; /* 0xbe205c61_0ca86c39 */
const OVT: f64 = 8.0085662595372944372e-017; /* -(1024-log2(ovfl+.5ulp)) */
const CP: f64 = 9.61796693925975554329e-01; /* 0x3feec709_dc3a03fd =2/(3ln2) */
const CP_H: f64 = 9.61796700954437255859e-01; /* 0x3feec709_e0000000 =(float)cp */
const CP_L: f64 = -7.02846165095275826516e-09; /* 0xbe3e2fe0_145b01f5 =tail of cp_h*/
const IVLN2: f64 = 1.44269504088896338700e+00; /* 0x3ff71547_652b82fe =1/ln2 */
const IVLN2_H: f64 = 1.44269502162933349609e+00; /* 0x3ff71547_60000000 =24b 1/ln2*/
const IVLN2_L: f64 = 1.92596299112661746887e-08; /* 0x3e54ae0b_f85ddf44 =1/ln2 tail*/

/// `x` raised to the power `y`. The source states "pow(x,y) returns x**y nearly rounded" and that a whole number to a whole power is exact when representable; the tests here measure the units in the last place.
pub fn powf(x: f64, y: f64) -> f64 {
    let t1: f64;
    let t2: f64;

    let (hx, lx): (i32, u32) = ((x.to_bits() >> 32) as i32, x.to_bits() as u32);
    let (hy, ly): (i32, u32) = ((y.to_bits() >> 32) as i32, y.to_bits() as u32);

    let mut ix: i32 = hx & 0x7fffffff_i32;
    let iy: i32 = hy & 0x7fffffff_i32;

    /* x**0 = 1, even if x is NaN */
    if ((iy as u32) | ly) == 0 {
        return 1.0;
    }

    /* 1**y = 1, even if y is NaN */
    if hx == 0x3ff00000 && lx == 0 {
        return 1.0;
    }

    /* NaN if either arg is NaN */
    if ix > 0x7ff00000
        || (ix == 0x7ff00000 && lx != 0)
        || iy > 0x7ff00000
        || (iy == 0x7ff00000 && ly != 0)
    {
        return x + y;
    }

    /* determine if y is an odd int when x < 0
     * yisint = 0       ... y is not an integer
     * yisint = 1       ... y is an odd int
     * yisint = 2       ... y is an even int
     */
    let mut yisint: i32 = 0;
    let mut k: i32;
    let mut j: i32;
    if hx < 0 {
        if iy >= 0x43400000 {
            yisint = 2; /* even integer y */
        } else if iy >= 0x3ff00000 {
            k = (iy >> 20) - 0x3ff; /* exponent */

            if k > 20 {
                j = (ly >> (52 - k)) as i32;

                if (j << (52 - k)) == (ly as i32) {
                    yisint = 2 - (j & 1);
                }
            } else if ly == 0 {
                j = iy >> (20 - k);

                if (j << (20 - k)) == iy {
                    yisint = 2 - (j & 1);
                }
            }
        }
    }

    if ly == 0 {
        /* special value of y */
        if iy == 0x7ff00000 {
            /* y is +-inf */

            return if ((ix - 0x3ff00000) | (lx as i32)) == 0 {
                /* (-1)**+-inf is 1 */
                1.0
            } else if ix >= 0x3ff00000 {
                /* (|x|>1)**+-inf = inf,0 */
                if hy >= 0 { y } else { 0.0 }
            } else {
                /* (|x|<1)**+-inf = 0,inf */
                if hy >= 0 { 0.0 } else { -y }
            };
        }

        if iy == 0x3ff00000 {
            /* y is +-1 */
            return if hy >= 0 { x } else { 1.0 / x };
        }

        if hy == 0x40000000 {
            /* y is 2 */
            return x * x;
        }

        if hy == 0x3fe00000 {
            /* y is 0.5 */
            if hx >= 0 {
                /* x >= +0 */
                return f64::sqrt(x);
            }
        }
    }

    let mut ax: f64 = fabs(x);
    if lx == 0 {
        /* special value of x */
        if ix == 0x7ff00000 || ix == 0 || ix == 0x3ff00000 {
            /* x is +-0,+-inf,+-1 */
            let mut z: f64 = ax;

            if hy < 0 {
                /* z = (1/|x|) */
                z = 1.0 / z;
            }

            if hx < 0 {
                if ((ix - 0x3ff00000) | yisint) == 0 {
                    z = (z - z) / (z - z); /* (-1)**non-int is NaN */
                } else if yisint == 1 {
                    z = -z; /* (x<0)**odd = -(|x|**odd) */
                }
            }

            return z;
        }
    }

    let mut s: f64 = 1.0; /* sign of result */
    if hx < 0 {
        if yisint == 0 {
            /* (x<0)**(non-int) is NaN */
            return (x - x) / (x - x);
        }

        if yisint == 1 {
            /* (x<0)**(odd int) */
            s = -1.0;
        }
    }

    /* |y| is HUGE */
    if iy > 0x41e00000 {
        /* if |y| > 2**31 */
        if iy > 0x43f00000 {
            /* if |y| > 2**64, must o/uflow */
            if ix <= 0x3fefffff {
                return if hy < 0 { HUGE * HUGE } else { TINY * TINY };
            }

            if ix >= 0x3ff00000 {
                return if hy > 0 { HUGE * HUGE } else { TINY * TINY };
            }
        }

        /* over/underflow if x is not close to one */
        if ix < 0x3fefffff {
            return if hy < 0 {
                s * HUGE * HUGE
            } else {
                s * TINY * TINY
            };
        }
        if ix > 0x3ff00000 {
            return if hy > 0 {
                s * HUGE * HUGE
            } else {
                s * TINY * TINY
            };
        }

        /* now |1-x| is TINY <= 2**-20, suffice to compute
        log(x) by x-x^2/2+x^3/3-x^4/4 */
        let t: f64 = ax - 1.0; /* t has 20 trailing zeros */
        let w: f64 = (t * t) * (0.5 - t * (0.3333333333333333333333 - t * 0.25));
        let u: f64 = IVLN2_H * t; /* ivln2_h has 21 sig. bits */
        let v: f64 = t * IVLN2_L - w * IVLN2;
        t1 = with_set_low_word(u + v, 0);
        t2 = v - (t1 - u);
    } else {
        // double ss,s2,s_h,s_l,t_h,t_l;
        let mut n: i32 = 0;

        if ix < 0x00100000 {
            /* take care subnormal number */
            ax *= TWO53;
            n -= 53;
            ix = get_high_word(ax) as i32;
        }

        n += (ix >> 20) - 0x3ff;
        j = ix & 0x000fffff;

        /* determine interval */
        let k: i32;
        ix = j | 0x3ff00000; /* normalize ix */
        if j <= 0x3988E {
            /* |x|<sqrt(3/2) */
            k = 0;
        } else if j < 0xBB67A {
            /* |x|<sqrt(3)   */
            k = 1;
        } else {
            k = 0;
            n += 1;
            ix -= 0x00100000;
        }
        ax = with_set_high_word(ax, ix as u32);

        /* compute ss = s_h+s_l = (x-1)/(x+1) or (x-1.5)/(x+1.5) */
        let u: f64 = ax - BP[k as usize]; /* bp[0]=1.0, bp[1]=1.5 */
        let v: f64 = 1.0 / (ax + BP[k as usize]);
        let ss: f64 = u * v;
        let s_h = with_set_low_word(ss, 0);

        /* t_h=ax+bp[k] High */
        let t_h: f64 = with_set_high_word(
            0.0,
            ((ix as u32 >> 1) | 0x20000000) + 0x00080000 + ((k as u32) << 18),
        );
        let t_l: f64 = ax - (t_h - BP[k as usize]);
        let s_l: f64 = v * ((u - s_h * t_h) - s_h * t_l);

        /* compute log(ax) */
        let s2: f64 = ss * ss;
        let mut r: f64 = s2 * s2 * (L1 + s2 * (L2 + s2 * (L3 + s2 * (L4 + s2 * (L5 + s2 * L6)))));
        r += s_l * (s_h + ss);
        let s2: f64 = s_h * s_h;
        let t_h: f64 = with_set_low_word(3.0 + s2 + r, 0);
        let t_l: f64 = r - ((t_h - 3.0) - s2);

        /* u+v = ss*(1+...) */
        let u: f64 = s_h * t_h;
        let v: f64 = s_l * t_h + t_l * ss;

        /* 2/(3log2)*(ss+...) */
        let p_h: f64 = with_set_low_word(u + v, 0);
        let p_l = v - (p_h - u);
        let z_h: f64 = CP_H * p_h; /* cp_h+cp_l = 2/(3*log2) */
        let z_l: f64 = CP_L * p_h + p_l * CP + DP_L[k as usize];

        /* log2(ax) = (ss+..)*2/(3*log2) = n + dp_h + z_h + z_l */
        let t: f64 = n as f64;
        t1 = with_set_low_word(((z_h + z_l) + DP_H[k as usize]) + t, 0);
        t2 = z_l - (((t1 - t) - DP_H[k as usize]) - z_h);
    }

    /* split up y into y1+y2 and compute (y1+y2)*(t1+t2) */
    let y1: f64 = with_set_low_word(y, 0);
    let p_l: f64 = (y - y1) * t1 + y * t2;
    let mut p_h: f64 = y1 * t1;
    let z: f64 = p_l + p_h;
    let mut j: i32 = (z.to_bits() >> 32) as i32;
    let i: i32 = z.to_bits() as i32;
    // let (j, i): (i32, i32) = ((z.to_bits() >> 32) as i32, z.to_bits() as i32);

    if j >= 0x40900000 {
        /* z >= 1024 */
        if (j - 0x40900000) | i != 0 {
            /* if z > 1024 */
            return s * HUGE * HUGE; /* overflow */
        }

        if p_l + OVT > z - p_h {
            return s * HUGE * HUGE; /* overflow */
        }
    } else if (j & 0x7fffffff) >= 0x4090cc00 {
        /* z <= -1075 */
        // FIXME: instead of abs(j) use unsigned j

        if (((j as u32) - 0xc090cc00) | (i as u32)) != 0 {
            /* z < -1075 */
            return s * TINY * TINY; /* underflow */
        }

        if p_l <= z - p_h {
            return s * TINY * TINY; /* underflow */
        }
    }

    /* compute 2**(p_h+p_l) */
    let i: i32 = j & 0x7fffffff_i32;
    k = (i >> 20) - 0x3ff;
    let mut n: i32 = 0;

    if i > 0x3fe00000 {
        /* if |z| > 0.5, set n = [z+0.5] */
        n = j + (0x00100000 >> (k + 1));
        k = ((n & 0x7fffffff) >> 20) - 0x3ff; /* new k for n */
        let t: f64 = with_set_high_word(0.0, (n & !(0x000fffff >> k)) as u32);
        n = ((n & 0x000fffff) | 0x00100000) >> (20 - k);
        if j < 0 {
            n = -n;
        }
        p_h -= t;
    }

    let t: f64 = with_set_low_word(p_l + p_h, 0);
    let u: f64 = t * POW_LG2_H;
    let v: f64 = (p_l - (t - p_h)) * POW_LG2 + t * POW_LG2_L;
    let mut z: f64 = u + v;
    let w: f64 = v - (z - u);
    let t: f64 = z * z;
    let t1: f64 = z - t * (P1 + t * (P2 + t * (P3 + t * (P4 + t * P5))));
    let r: f64 = (z * t1) / (t1 - 2.0) - (w + z * w);
    z = 1.0 - (r - z);
    j = get_high_word(z) as i32;
    j += n << 20;

    if (j >> 20) <= 0 {
        /* subnormal output */
        z = scalbn(z, n);
    } else {
        z = with_set_high_word(z, j as u32);
    }

    s * z
}

// ---------------------------------------------------------------------------
// powi
// ---------------------------------------------------------------------------

// Not from the source: this crate's own. The power of a 64-bit number to a
// whole exponent by the binary method, right to left (Knuth, The Art of
// Computer Programming, volume 2, section 4.6.3, algorithm A): the product
// takes the running square at each set bit of the exponent's size, from the
// lowest bit up, and a negative exponent divides one by the product. These
// are the multiplications, in the same order, that `__powidf2` of the Rust
// `compiler-builtins` performs, which a call of `f64::powi` with an exponent
// known only at run time reaches, and the order the compiler's own expansion
// of a small constant exponent follows. Written out here, a power whose base
// and exponent are both constants is folded by the compiler with the same
// IEEE 754 multiplications, and never through the `pow` of the machine that
// builds it.

/// `x` raised to the whole power `n`, by repeated squaring and multiplication: the bits `f64::powi` gives at run time.
#[inline]
pub fn powi(x: f64, n: i32) -> f64 {
    let mut square = x;
    let mut bits_left = n.unsigned_abs();
    let mut product = 1.0;
    loop {
        if bits_left & 1 != 0 {
            product *= square;
        }
        bits_left >>= 1;
        if bits_left == 0 {
            break;
        }
        square *= square;
    }
    if n < 0 { 1.0 / product } else { product }
}

// ---------------------------------------------------------------------------
// tan
// ---------------------------------------------------------------------------

// origin: FreeBSD /usr/src/lib/msun/src/s_tan.c */
//
// ====================================================
// Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//
// Developed at SunPro, a Sun Microsystems, Inc. business.
// Permission to use, copy, modify, and distribute this
// software is freely granted, provided that this notice
// is preserved.
// ====================================================

// tan(x)
// Return tangent function of x.
//
// kernel function:
//      k_tan           ... tangent function on [-pi/4,pi/4]
//      rem_pio2        ... argument reduction routine
//
// Method.
//      Let S,C and T denote the sin, cos and tan respectively on
//      [-PI/4, +PI/4]. Reduce the argument x to y1+y2 = x-k*pi/2
//      in [-pi/4 , +pi/4], and let n = k mod 4.
//      We have
//
//          n        sin(x)      cos(x)        tan(x)
//     ----------------------------------------------------------
//          0          S           C             T
//          1          C          -S            -1/T
//          2         -S          -C             T
//          3         -C           S            -1/T
//     ----------------------------------------------------------
//
// Special cases:
//      Let trig be any of sin, cos, or tan.
//      trig(+-INF)  is NaN, with signals;
//      trig(NaN)    is that NaN;
//
// Accuracy:
//      TRIG(x) returns trig(x) nearly rounded

/// Tangent of an angle in radians. The source states "TRIG(x) returns trig(x) nearly rounded"; the tests here measure the units in the last place.
pub fn tan(x: f64) -> f64 {
    let ix = (f64::to_bits(x) >> 32) as u32 & 0x7fffffff;
    /* |x| ~< pi/4 */
    if ix <= 0x3fe921fb {
        if ix < 0x3e400000 {
            /* |x| < 2**-27 */
            /* raise inexact if x!=0 and underflow if subnormal */
            // The source raises the two flags here. Not carried.
            return x;
        }
        return k_tan(x, 0.0, 0);
    }

    /* tan(Inf or NaN) is NaN */
    if ix >= 0x7ff00000 {
        return x - x;
    }

    /* argument reduction */
    let (n, y0, y1) = rem_pio2(x);
    k_tan(y0, y1, n & 1)
}

// ---------------------------------------------------------------------------
// k_tan
// ---------------------------------------------------------------------------

// origin: FreeBSD /usr/src/lib/msun/src/k_tan.c */
//
// ====================================================
// Copyright 2004 Sun Microsystems, Inc.  All Rights Reserved.
//
// Permission to use, copy, modify, and distribute this
// software is freely granted, provided that this notice
// is preserved.
// ====================================================

// kernel tan function on ~[-pi/4, pi/4] (except on -0), pi/4 ~ 0.7854
// Input x is assumed to be bounded by ~pi/4 in magnitude.
// Input y is the tail of x.
// Input odd indicates whether tan (if odd = 0) or -1/tan (if odd = 1) is returned.
//
// Algorithm
//      1. Since tan(-x) = -tan(x), we need only to consider positive x.
//      2. Callers must return tan(-0) = -0 without calling here since our
//         odd polynomial is not evaluated in a way that preserves -0.
//         Callers may do the optimization tan(x) ~ x for tiny x.
//      3. tan(x) is approximated by a odd polynomial of degree 27 on
//         [0,0.67434]
//                               3             27
//              tan(x) ~ x + T1*x + ... + T13*x
//         where
//
//              |tan(x)         2     4            26   |     -59.2
//              |----- - (1+T1*x +T2*x +.... +T13*x    )| <= 2
//              |  x                                    |
//
//         Note: tan(x+y) = tan(x) + tan'(x)*y
//                        ~ tan(x) + (1+x*x)*y
//         Therefore, for better accuracy in computing tan(x+y), let
//                   3      2      2       2       2
//              r = x *(T2+x *(T3+x *(...+x *(T12+x *T13))))
//         then
//                                  3    2
//              tan(x+y) = x + (T1*x + (x *(r+y)+y))
//
//      4. For x in [0.67434,pi/4],  let y = pi/4 - x, then
//              tan(x) = tan(pi/4-y) = (1-tan(y))/(1+tan(y))
//                     = 1 - 2*(tan(y) - (tan(y)^2)/(1+tan(y)))
const T: [f64; 13] = [
    3.33333333333334091986e-01,  /* 3FD55555, 55555563 */
    1.33333333333201242699e-01,  /* 3FC11111, 1110FE7A */
    5.39682539762260521377e-02,  /* 3FABA1BA, 1BB341FE */
    2.18694882948595424599e-02,  /* 3F9664F4, 8406D637 */
    8.86323982359930005737e-03,  /* 3F8226E3, E96E8493 */
    3.59207910759131235356e-03,  /* 3F6D6D22, C9560328 */
    1.45620945432529025516e-03,  /* 3F57DBC8, FEE08315 */
    5.88041240820264096874e-04,  /* 3F4344D8, F2F26501 */
    2.46463134818469906812e-04,  /* 3F3026F7, 1A8D1068 */
    7.81794442939557092300e-05,  /* 3F147E88, A03792A6 */
    7.14072491382608190305e-05,  /* 3F12B80F, 32F0A7E9 */
    -1.85586374855275456654e-05, /* BEF375CB, DB605373 */
    2.59073051863633712884e-05,  /* 3EFB2A70, 74BF7AD4 */
];
const PIO4: f64 = 7.85398163397448278999e-01; /* 3FE921FB, 54442D18 */
const PIO4_LO: f64 = 3.06161699786838301793e-17; /* 3C81A626, 33145C07 */

fn k_tan(mut x: f64, mut y: f64, odd: i32) -> f64 {
    let hx = (f64::to_bits(x) >> 32) as u32;
    let big = (hx & 0x7fffffff) >= 0x3FE59428; /* |x| >= 0.6744 */
    if big {
        let sign = hx >> 31;
        if sign != 0 {
            x = -x;
            y = -y;
        }
        x = (PIO4 - x) + (PIO4_LO - y);
        y = 0.0;
    }
    let z = x * x;
    let w = z * z;
    /*
     * Break x^5*(T[1]+x^2*T[2]+...) into
     * x^5(T[1]+x^4*T[3]+...+x^20*T[11]) +
     * x^5(x^2*(T[2]+x^4*T[4]+...+x^22*[T12]))
     */
    let r = T[1] + w * (T[3] + w * (T[5] + w * (T[7] + w * (T[9] + w * T[11]))));
    let v = z * (T[2] + w * (T[4] + w * (T[6] + w * (T[8] + w * (T[10] + w * T[12])))));
    let s = z * x;
    let r = y + z * (s * (r + v) + y) + s * T[0];
    let w = x + r;
    if big {
        let sign = hx >> 31;
        let s = 1.0 - 2.0 * odd as f64;
        let v = s - 2.0 * (x + (r - w * w / (w + s)));
        return if sign != 0 { -v } else { v };
    }
    if odd == 0 {
        return w;
    }
    /* -1.0/(x+r) has up to 2ulp error, so compute it accurately */
    let w0 = zero_low_word(w);
    let v = r - (w0 - x); /* w0+v = r+x */
    let a = -1.0 / w;
    let a0 = zero_low_word(a);
    a0 + a * (1.0 + a0 * w0 + a0 * v)
}

fn zero_low_word(x: f64) -> f64 {
    f64::from_bits(f64::to_bits(x) & 0xFFFF_FFFF_0000_0000)
}

// ---------------------------------------------------------------------------
// acos
// ---------------------------------------------------------------------------

/* origin: FreeBSD /usr/src/lib/msun/src/e_acos.c */
/*
 * ====================================================
 * Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
 *
 * Developed at SunSoft, a Sun Microsystems, Inc. business.
 * Permission to use, copy, modify, and distribute this
 * software is freely granted, provided that this notice
 * is preserved.
 * ====================================================
 */
/* acos(x)
 * Method :
 *      acos(x)  = pi/2 - asin(x)
 *      acos(-x) = pi/2 + asin(x)
 * For |x|<=0.5
 *      acos(x) = pi/2 - (x + x*x^2*R(x^2))     (see asin.c)
 * For x>0.5
 *      acos(x) = pi/2 - (pi/2 - 2asin(sqrt((1-x)/2)))
 *              = 2asin(sqrt((1-x)/2))
 *              = 2s + 2s*z*R(z)        ...z=(1-x)/2, s=sqrt(z)
 *              = 2f + (2c + 2s*z*R(z))
 *     where f=hi part of s, and c = (z-f*f)/(s+f) is the correction term
 *     for f so that f+c ~ sqrt(z).
 * For x<-0.5
 *      acos(x) = pi - 2asin(sqrt((1-|x|)/2))
 *              = pi - 0.5*(s+s*z*R(z)), where z=(1-|x|)/2,s=sqrt(z)
 *
 * Special cases:
 *      if x is NaN, return x itself;
 *      if |x|>1, return NaN with invalid signal.
 *
 * Function needed: sqrt
 */

// The source names PIO2_HI, PIO2_LO, PS0 to PS5 and QS1 to QS4 with the
// values `asin` above gives PIO2_HI, PIO2_LO, P_S0 to P_S5 and Q_S1 to Q_S4,
// and its `r` is `asin`'s `comp_r`, the same operations on the same
// constants. Those serve here.

/// Arccosine in radians, from 0 to π, for an argument from −1 to 1; NaN outside. The source states no bound in units in the last place; the tests here measure it.
pub fn acos(x: f64) -> f64 {
    let x1p_120f = f64::from_bits(0x3870000000000000); // 0x1p-120 === 2 ^ -120
    let z: f64;
    let w: f64;
    let s: f64;
    let c: f64;
    let df: f64;
    let hx: u32;
    let ix: u32;

    hx = (x.to_bits() >> 32) as u32;
    ix = hx & 0x7fffffff;
    /* |x| >= 1 or nan */
    if ix >= 0x3ff00000 {
        let lx: u32 = x.to_bits() as u32;

        if ((ix - 0x3ff00000) | lx) == 0 {
            /* acos(1)=0, acos(-1)=pi */
            if (hx >> 31) != 0 {
                return 2. * PIO2_HI + x1p_120f;
            }
            return 0.;
        }
        return 0. / (x - x);
    }
    /* |x| < 0.5 */
    if ix < 0x3fe00000 {
        if ix <= 0x3c600000 {
            /* |x| < 2**-57 */
            return PIO2_HI + x1p_120f;
        }
        return PIO2_HI - (x - (PIO2_LO - x * comp_r(x * x)));
    }
    /* x < -0.5 */
    if (hx >> 31) != 0 {
        z = (1.0 + x) * 0.5;
        s = f64::sqrt(z);
        w = comp_r(z) * s - PIO2_LO;
        return 2. * (PIO2_HI - (s + w));
    }
    /* x > 0.5 */
    z = (1.0 - x) * 0.5;
    s = f64::sqrt(z);
    // Set the low 4 bytes to zero
    df = f64::from_bits(s.to_bits() & 0xff_ff_ff_ff_00_00_00_00);

    c = (z - df * df) / (s + df);
    w = comp_r(z) * s + c;
    2. * (df + w)
}

// ---------------------------------------------------------------------------
// atan
// ---------------------------------------------------------------------------

/* origin: FreeBSD /usr/src/lib/msun/src/s_atan.c */
/*
 * ====================================================
 * Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
 *
 * Developed at SunPro, a Sun Microsystems, Inc. business.
 * Permission to use, copy, modify, and distribute this
 * software is freely granted, provided that this notice
 * is preserved.
 * ====================================================
 */
/* atan(x)
 * Method
 *   1. Reduce x to positive by atan(x) = -atan(-x).
 *   2. According to the integer k=4t+0.25 chopped, t=x, the argument
 *      is further reduced to one of the following intervals and the
 *      arctangent of t is evaluated by the corresponding formula:
 *
 *      [0,7/16]      atan(x) = t-t^3*(a1+t^2*(a2+...(a10+t^2*a11)...)
 *      [7/16,11/16]  atan(x) = atan(1/2) + atan( (t-0.5)/(1+t/2) )
 *      [11/16.19/16] atan(x) = atan( 1 ) + atan( (t-1)/(1+t) )
 *      [19/16,39/16] atan(x) = atan(3/2) + atan( (t-1.5)/(1+1.5t) )
 *      [39/16,INF]   atan(x) = atan(INF) + atan( -1/t )
 *
 * Constants:
 * The hexadecimal values are the intended ones for the following
 * constants. The decimal values may be used, provided that the
 * compiler will convert from decimal to binary accurately enough
 * to produce the hexadecimal values shown.
 */

const ATANHI: [f64; 4] = [
    4.63647609000806093515e-01, /* atan(0.5)hi 0x3FDDAC67, 0x0561BB4F */
    7.85398163397448278999e-01, /* atan(1.0)hi 0x3FE921FB, 0x54442D18 */
    9.82793723247329054082e-01, /* atan(1.5)hi 0x3FEF730B, 0xD281F69B */
    1.57079632679489655800e+00, /* atan(inf)hi 0x3FF921FB, 0x54442D18 */
];

const ATANLO: [f64; 4] = [
    2.26987774529616870924e-17, /* atan(0.5)lo 0x3C7A2B7F, 0x222F65E2 */
    3.06161699786838301793e-17, /* atan(1.0)lo 0x3C81A626, 0x33145C07 */
    1.39033110312309984516e-17, /* atan(1.5)lo 0x3C700788, 0x7AF0CBBD */
    6.12323399573676603587e-17, /* atan(inf)lo 0x3C91A626, 0x33145C07 */
];

const AT: [f64; 11] = [
    3.33333333333329318027e-01,  /* 0x3FD55555, 0x5555550D */
    -1.99999999998764832476e-01, /* 0xBFC99999, 0x9998EBC4 */
    1.42857142725034663711e-01,  /* 0x3FC24924, 0x920083FF */
    -1.11111104054623557880e-01, /* 0xBFBC71C6, 0xFE231671 */
    9.09088713343650656196e-02,  /* 0x3FB745CD, 0xC54C206E */
    -7.69187620504482999495e-02, /* 0xBFB3B0F2, 0xAF749A6D */
    6.66107313738753120669e-02,  /* 0x3FB10D66, 0xA0D03D51 */
    -5.83357013379057348645e-02, /* 0xBFADDE2D, 0x52DEFD9A */
    4.97687799461593236017e-02,  /* 0x3FA97B4B, 0x24760DEB */
    -3.65315727442169155270e-02, /* 0xBFA2B444, 0x2C6A6C2F */
    1.62858201153657823623e-02,  /* 0x3F90AD3A, 0xE322DA11 */
];

/// Arctangent in radians, from −π/2 to π/2. The source states no bound in units in the last place; the tests here measure it.
pub fn atan(x: f64) -> f64 {
    let mut x = x;
    let mut ix = (x.to_bits() >> 32) as u32;
    let sign = ix >> 31;
    ix &= 0x7fff_ffff;
    if ix >= 0x4410_0000 {
        if x.is_nan() {
            return x;
        }

        // The source's comment says 0x1p-120f; the bits it adds are those of
        // a 64-bit number near 2.9e-316. Either leaves ATANHI[3] unchanged.
        let z = ATANHI[3] + f64::from_bits(0x0380_0000); // 0x1p-120f
        return if sign != 0 { -z } else { z };
    }

    let id = if ix < 0x3fdc_0000 {
        /* |x| < 0.4375 */
        if ix < 0x3e40_0000 {
            /* |x| < 2^-27 */
            // The source raises the underflow flag here for a subnormal x. Not carried.

            return x;
        }

        -1
    } else {
        x = fabs(x);
        if ix < 0x3ff30000 {
            /* |x| < 1.1875 */
            if ix < 0x3fe60000 {
                /* 7/16 <= |x| < 11/16 */
                x = (2. * x - 1.) / (2. + x);
                0
            } else {
                /* 11/16 <= |x| < 19/16 */
                x = (x - 1.) / (x + 1.);
                1
            }
        } else if ix < 0x40038000 {
            /* |x| < 2.4375 */
            x = (x - 1.5) / (1. + 1.5 * x);
            2
        } else {
            /* 2.4375 <= |x| < 2^66 */
            x = -1. / x;
            3
        }
    };

    let z = x * x;
    let w = z * z;
    /* break sum from i=0 to 10 AT[i]z**(i+1) into odd and even poly */
    let s1 = z * (AT[0] + w * (AT[2] + w * (AT[4] + w * (AT[6] + w * (AT[8] + w * AT[10])))));
    let s2 = w * (AT[1] + w * (AT[3] + w * (AT[5] + w * (AT[7] + w * AT[9]))));

    if id < 0 {
        return x - x * (s1 + s2);
    }

    let z = ATANHI[id as usize] - (x * (s1 + s2) - ATANLO[id as usize] - x);

    if sign != 0 { -z } else { z }
}

// ---------------------------------------------------------------------------
// atan2
// ---------------------------------------------------------------------------

/* origin: FreeBSD /usr/src/lib/msun/src/e_atan2.c */
/*
 * ====================================================
 * Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
 *
 * Developed at SunSoft, a Sun Microsystems, Inc. business.
 * Permission to use, copy, modify, and distribute this
 * software is freely granted, provided that this notice
 * is preserved.
 * ====================================================
 *
 */
/* atan2(y,x)
 * Method :
 *      1. Reduce y to positive by atan2(y,x)=-atan2(-y,x).
 *      2. Reduce x to positive by (if x and y are unexceptional):
 *              ARG (x+iy) = arctan(y/x)           ... if x > 0,
 *              ARG (x+iy) = pi - arctan[y/(-x)]   ... if x < 0,
 *
 * Special cases:
 *
 *      ATAN2((anything), NaN ) is NaN;
 *      ATAN2(NAN , (anything) ) is NaN;
 *      ATAN2(+-0, +(anything but NaN)) is +-0  ;
 *      ATAN2(+-0, -(anything but NaN)) is +-pi ;
 *      ATAN2(+-(anything but 0 and NaN), 0) is +-pi/2;
 *      ATAN2(+-(anything but INF and NaN), +INF) is +-0 ;
 *      ATAN2(+-(anything but INF and NaN), -INF) is +-pi;
 *      ATAN2(+-INF,+INF ) is +-pi/4 ;
 *      ATAN2(+-INF,-INF ) is +-3pi/4;
 *      ATAN2(+-INF, (anything but,0,NaN, and INF)) is +-pi/2;
 *
 * Constants:
 * The hexadecimal values are the intended ones for the following
 * constants. The decimal values may be used, provided that the
 * compiler will convert from decimal to binary accurately enough
 * to produce the hexadecimal values shown.
 */

const PI: f64 = 3.1415926535897931160E+00; /* 0x400921FB, 0x54442D18 */
const PI_LO: f64 = 1.2246467991473531772E-16; /* 0x3CA1A626, 0x33145C07 */

/// The angle of the point (`x`, `y`) from the positive `x` axis, in radians from −π to π: the arctangent of `y / x` in its quadrant. The source states no bound in units in the last place; the tests here measure it, within one unit at the vectors, and two units at most were measured away from them (the module's "Accuracy").
pub fn atan2(y: f64, x: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return x + y;
    }
    let mut ix = (x.to_bits() >> 32) as u32;
    let lx = x.to_bits() as u32;
    let mut iy = (y.to_bits() >> 32) as u32;
    let ly = y.to_bits() as u32;
    if ((ix.wrapping_sub(0x3ff00000)) | lx) == 0 {
        /* x = 1.0 */
        return atan(y);
    }
    let m = ((iy >> 31) & 1) | ((ix >> 30) & 2); /* 2*sign(x)+sign(y) */
    ix &= 0x7fffffff;
    iy &= 0x7fffffff;

    /* when y = 0 */
    if (iy | ly) == 0 {
        return match m {
            0 | 1 => y, /* atan(+-0,+anything)=+-0 */
            2 => PI,    /* atan(+0,-anything) = PI */
            _ => -PI,   /* atan(-0,-anything) =-PI */
        };
    }
    /* when x = 0 */
    if (ix | lx) == 0 {
        return if m & 1 != 0 { -PI / 2.0 } else { PI / 2.0 };
    }
    /* when x is INF */
    if ix == 0x7ff00000 {
        if iy == 0x7ff00000 {
            return match m {
                0 => PI / 4.0,        /* atan(+INF,+INF) */
                1 => -PI / 4.0,       /* atan(-INF,+INF) */
                2 => 3.0 * PI / 4.0,  /* atan(+INF,-INF) */
                _ => -3.0 * PI / 4.0, /* atan(-INF,-INF) */
            };
        } else {
            return match m {
                0 => 0.0,  /* atan(+...,+INF) */
                1 => -0.0, /* atan(-...,+INF) */
                2 => PI,   /* atan(+...,-INF) */
                _ => -PI,  /* atan(-...,-INF) */
            };
        }
    }
    /* |y/x| > 0x1p64 */
    if ix.wrapping_add(64 << 20) < iy || iy == 0x7ff00000 {
        return if m & 1 != 0 { -PI / 2.0 } else { PI / 2.0 };
    }

    /* z = atan(|y/x|) without spurious underflow */
    let z = if (m & 2 != 0) && iy.wrapping_add(64 << 20) < ix {
        /* |y/x| < 0x1p-64, x<0 */
        0.0
    } else {
        atan(fabs(y / x))
    };
    match m {
        0 => z,                /* atan(+,+) */
        1 => -z,               /* atan(-,+) */
        2 => PI - (z - PI_LO), /* atan(+,-) */
        _ => (z - PI_LO) - PI, /* atan(-,-) */
    }
}

// ---------------------------------------------------------------------------
// sinh, cosh, tanh, asinh and expo2
// ---------------------------------------------------------------------------

// origin: musl src/math/sinh.c, cosh.c, tanh.c, asinh.c and __expo2.c.
// Neither those files nor the Rust files carry a notice of their own. musl's
// COPYRIGHT file covers them, the notice above `hypot`.

// sinh(x) = (exp(x) - 1/exp(x))/2
//         = (exp(x)-1 + (exp(x)-1)/exp(x))/2
//         = x + x^3/6 + o(x^5)
//

/// Hyperbolic sine. The source states no bound in units in the last place; the tests here measure it, within one unit at the vectors, and two units at most were measured away from them (the module's "Accuracy").
pub fn sinh(x: f64) -> f64 {
    // union {double f; uint64_t i;} u = {.f = x};
    // uint32_t w;
    // double t, h, absx;

    let mut uf: f64 = x;
    let mut ui: u64 = f64::to_bits(uf);
    let w: u32;
    let t: f64;
    let mut h: f64;
    let absx: f64;

    h = 0.5;
    if ui >> 63 != 0 {
        h = -h;
    }
    /* |x| */
    ui &= !1 / 2;
    uf = f64::from_bits(ui);
    absx = uf;
    w = (ui >> 32) as u32;

    /* |x| < log(DBL_MAX) */
    if w < 0x40862e42 {
        t = exp_m1(absx);
        if w < 0x3ff00000 {
            if w < 0x3ff00000 - (26 << 20) {
                /* note: inexact and underflow are raised by expm1 */
                /* note: this branch avoids spurious underflow */
                return x;
            }
            return h * (2.0 * t - t * t / (t + 1.0));
        }
        /* note: |x|>log(0x1p26)+eps could be just h*exp(x) */
        return h * (t + t / (t + 1.0));
    }

    /* |x| > log(DBL_MAX) or nan */
    /* note: the result is stored to handle overflow */
    t = 2.0 * h * expo2(absx);
    t
}

/// Hyperbolic cosine. The source states no bound in units in the last place; the tests here measure it, and one unit at most was measured away from the vectors too (the module's "Accuracy").
pub fn cosh(mut x: f64) -> f64 {
    /* |x| */
    let mut ix = x.to_bits();
    ix &= 0x7fffffffffffffff;
    x = f64::from_bits(ix);
    let w = ix >> 32;

    /* |x| < log(2) */
    if w < 0x3fe62e42 {
        if w < 0x3ff00000 - (26 << 20) {
            // The source raises the inexact flag here. Not carried.
            return 1.;
        }
        let t = exp_m1(x); // exponential minus 1
        return 1. + t * t / (2. * (1. + t));
    }

    /* |x| < log(DBL_MAX) */
    if w < 0x40862e42 {
        let t = exp(x);
        /* note: if x>log(0x1p26) then the 1/t is not needed */
        return 0.5 * (t + 1. / t);
    }

    /* |x| > log(DBL_MAX) or nan */
    expo2(x)
}

/* tanh(x) = (exp(x) - exp(-x))/(exp(x) + exp(-x))
 *         = (exp(2*x) - 1)/(exp(2*x) - 1 + 2)
 *         = (1 - exp(-2*x))/(exp(-2*x) - 1 + 2)
 */

/// Hyperbolic tangent. The source states "up to 2ulp error in [0.1,0.2554]"; the tests here measure it, and two units were also measured from 0.0296 to 0.0999 (the module's "Accuracy").
pub fn tanh(mut x: f64) -> f64 {
    let mut uf: f64 = x;
    let mut ui: u64 = f64::to_bits(uf);

    let w: u32;
    let sign: bool;
    let mut t: f64;

    /* x = |x| */
    sign = ui >> 63 != 0;
    ui &= !1 / 2;
    uf = f64::from_bits(ui);
    x = uf;
    w = (ui >> 32) as u32;

    if w > 0x3fe193ea {
        /* |x| > log(3)/2 ~= 0.5493 or nan */
        if w > 0x40340000 {
            /* |x| > 20 or nan */
            /* note: this branch avoids raising overflow */
            t = 1.0 - 0.0 / x;
        } else {
            t = exp_m1(2.0 * x);
            t = 1.0 - 2.0 / (t + 2.0);
        }
    } else if w > 0x3fd058ae {
        /* |x| > log(5/3)/2 ~= 0.2554 */
        t = exp_m1(2.0 * x);
        t = t / (t + 2.0);
    } else if w >= 0x00100000 {
        /* |x| >= 0x1p-1022, up to 2ulp error in [0.1,0.2554] */
        t = exp_m1(-2.0 * x);
        t = -t / (t + 2.0);
    } else {
        /* |x| is subnormal */
        /* note: the branch above would not raise underflow in [0x1p-1023,0x1p-1022) */
        // The source raises the underflow flag here. Not carried.
        t = x;
    }

    if sign { -t } else { t }
}

const LN2: f64 = 0.693147180559945309417232121458176568; /* 0x3fe62e42,  0xfefa39ef*/

/* asinh(x) = sign(x)*log(|x|+sqrt(x*x+1)) ~= x - x^3/6 + o(x^5) */
/// Inverse hyperbolic sine. The source states "up to 1.6ulp error in [0.125,0.5]"; the tests here measure it, within one unit at the vectors, and two units at most were measured from 0.061 to 0.52 (the module's "Accuracy").
pub fn asinh(mut x: f64) -> f64 {
    let mut u = x.to_bits();
    let e = ((u >> 52) as usize) & 0x7ff;
    let sign = (u >> 63) != 0;

    /* |x| */
    u &= (!0) >> 1;
    x = f64::from_bits(u);

    if e >= 0x3ff + 26 {
        /* |x| >= 0x1p26 or inf or nan */
        x = log(x) + LN2;
    } else if e >= 0x3ff + 1 {
        /* |x| >= 2 */
        x = log(2.0 * x + 1.0 / (f64::sqrt(x * x + 1.0) + x));
    } else if e >= 0x3ff - 26 {
        /* |x| >= 0x1p-26, up to 1.6ulp error in [0.125,0.5] */
        x = ln_1p(x + x * x / (f64::sqrt(x * x + 1.0) + 1.0));
    } else {
        /* |x| < 0x1p-26, raise inexact if x != 0 */
        // The source raises the inexact flag here. Not carried.
    }

    if sign { -x } else { x }
}

/* exp(x)/2 for x >= log(DBL_MAX), slightly better than 0.5*exp(x/2)*exp(x/2) */
fn expo2(x: f64) -> f64 {
    /* k is such that k*ln2 has minimal relative error and x - kln2 > log(DBL_MIN) */
    const K: i32 = 2043;
    let kln2 = f64::from_bits(0x40962066151add8b);

    /* note that k is odd and scale*scale overflows */
    let scale = combine_words(((0x3ff + K / 2) as u32) << 20, 0);
    /* exp(x - k ln2) * 2**(k-1) */
    exp(x - kln2) * scale * scale
}

// ---------------------------------------------------------------------------
// cbrt
// ---------------------------------------------------------------------------

/* SPDX-License-Identifier: MIT */
/* origin: core-math/src/binary64/cbrt/cbrt.c
 * Copyright (c) 2021-2022 Alexei Sibidanov.
 * Ported to Rust in 2025 by Trevor Gross.
 */

/// The cube root, correctly rounded: CORE-MATH's routine, made to round correctly; the tests here hold it to the correctly rounded value at every argument.
pub fn cbrt(x: f64) -> f64 {
    // The source's `cbrt_round(x, Round::Nearest)`: rounding to nearest, the
    // one mode carried, so `round as usize` is 0 throughout.
    const ESCALE: [f64; 3] = [
        1.0,
        f64::from_bits(0x3FF428A2F98D728B), /* 0x1.428a2f98d728bp+0, 2^(1/3) */
        f64::from_bits(0x3FF965FEA53D6E3D), /* 0x1.965fea53d6e3dp+0, 2^(2/3) */
    ];

    /* the polynomial c0+c1*x+c2*x^2+c3*x^3 approximates x^(1/3) on [1,2]
    with maximal error < 9.2e-5 (attained at x=2) */
    const C: [f64; 4] = [
        f64::from_bits(0x3FE1B0BABCCFEF9C), /* 0x1.1b0babccfef9cp-1 */
        f64::from_bits(0x3FE2C9A3E94D1DA5), /* 0x1.2c9a3e94d1da5p-1 */
        f64::from_bits(0xBFC4DC30B1A1DDBA), /* -0x1.4dc30b1a1ddbap-3 */
        f64::from_bits(0x3F97A8D3E4EC9B07), /* 0x1.7a8d3e4ec9b07p-6 */
    ];

    let u0: f64 = f64::from_bits(0x3FD5555555555555); /* 0x1.5555555555555p-2 */
    let u1: f64 = f64::from_bits(0x3FCC71C71C71C71C); /* 0x1.c71c71c71c71cp-3 */

    let rsc = [1.0, -1.0, 0.5, -0.5, 0.25, -0.25];

    // off[round as usize] for rounding to nearest: 0x1p-53.
    let off = f64::from_bits(0x3CA0000000000000);

    /* rm=0 for rounding to nearest, and other values for directed roundings */
    let hx: u64 = x.to_bits();
    let mut mant: u64 = hx & CBRT_SIG_MASK;
    let sign: u64 = hx >> 63;

    let mut e: u32 = (hx >> CBRT_SIG_BITS) as u32 & CBRT_EXP_SAT;

    if ((e + 1) & CBRT_EXP_SAT) < 2 {
        let ix: u64 = hx & !CBRT_SIGN_MASK;

        /* 0, inf, nan: we return x + x instead of simply x,
        to that for x a signaling NaN, it correctly triggers
        the invalid exception. */
        if e == CBRT_EXP_SAT || ix == 0 {
            return x + x;
        }

        let nz = ix.leading_zeros() - 11; /* subnormal */
        mant <<= nz;
        mant &= CBRT_SIG_MASK;
        e = e.wrapping_sub(nz - 1);
    }

    e = e.wrapping_add(3072);
    let cvt1: u64 = mant | (0x3ffu64 << 52);
    let mut cvt5: u64 = cvt1;

    let et: u32 = e / 3;
    let it: u32 = e % 3;

    /* 2^(3k+it) <= x < 2^(3k+it+1), with 0 <= it <= 3 */
    cvt5 += (it as u64) << CBRT_SIG_BITS;
    cvt5 |= sign << 63;
    let zz: f64 = f64::from_bits(cvt5);

    /* cbrt(x) = cbrt(zz)*2^(et-1365) where 1 <= zz < 8 */
    let mut isc: u64 = ESCALE[it as usize].to_bits(); // todo: index
    isc |= sign << 63;
    let cvt2: u64 = isc;
    let z: f64 = f64::from_bits(cvt1);

    /* cbrt(zz) = cbrt(z)*isc, where isc encodes 1, 2^(1/3) or 2^(2/3),
    and 1 <= z < 2 */
    let r: f64 = 1.0 / z;
    let rr: f64 = r * rsc[((it as usize) << 1) | sign as usize];
    let z2: f64 = z * z;
    let c0: f64 = C[0] + z * C[1];
    let c2: f64 = C[2] + z * C[3];
    let mut y: f64 = c0 + z2 * c2;
    let mut y2: f64 = y * y;

    /* y is an approximation of z^(1/3) */
    let mut h: f64 = y2 * (y * r) - 1.0;

    /* h determines the error between y and z^(1/3) */
    y -= (h * y) * (u0 - u1 * h);

    /* The correction y -= (h*y)*(u0 - u1*h) corresponds to a cubic variant
    of Newton's method, with the function f(y) = 1-z/y^3. */
    y *= f64::from_bits(cvt2);

    /* Now y is an approximation of zz^(1/3),
     * and rr an approximation of 1/zz. We now perform another iteration of
     * Newton-Raphson, this time with a linear approximation only. */
    y2 = y * y;
    let mut y2l: f64 = product_error(y, y, y2); // the source: y.fma(y, -y2)

    /* y2 + y2l = y^2 exactly */
    let mut y3: f64 = y2 * y;
    let mut y3l: f64 = product_error(y, y2, y3) + y * y2l; // the source: y.fma(y2, -y3) + y * y2l

    /* y3 + y3l approximates y^3 with about 106 bits of accuracy */
    h = ((y3 - zz) + y3l) * rr;
    let mut dy: f64 = h * (y * u0);

    /* the approximation of zz^(1/3) is y - dy */
    let mut y1: f64 = y - dy;
    dy = (y - y1) - dy;

    /* the approximation of zz^(1/3) is now y1 + dy, where |dy| < 1/2 ulp(y)
     * (for rounding to nearest) */
    let mut ady: f64 = fabs(dy);

    /* For directed roundings, ady0 is tiny when dy is tiny, or ady0 is near
     * from ulp(1);
     * for rounding to nearest, ady0 is tiny when dy is near from 1/2 ulp(1),
     * or from 3/2 ulp(1). */
    let mut ady0: f64 = fabs(ady - off);
    let mut ady1: f64 = fabs(ady - (f64::from_bits(0x3CB0000000000000) + off)); // 0x1p-52

    if ady0 < f64::from_bits(0x3B40000000000000) || ady1 < f64::from_bits(0x3B40000000000000) {
        // 0x1p-75
        y2 = y1 * y1;
        y2l = product_error(y1, y1, y2); // the source: y1.fma(y1, -y2)
        y3 = y2 * y1;
        y3l = product_error(y1, y2, y3) + y1 * y2l; // the source: y1.fma(y2, -y3) + y1 * y2l
        h = ((y3 - zz) + y3l) * rr;
        dy = h * (y1 * u0);
        y = y1 - dy;
        dy = (y1 - y) - dy;
        y1 = y;
        ady = fabs(dy);
        ady0 = fabs(ady - off);
        ady1 = fabs(ady - (f64::from_bits(0x3CB0000000000000) + off)); // 0x1p-52

        if ady0 < f64::from_bits(0x39D0000000000000) || ady1 < f64::from_bits(0x39D0000000000000) {
            // 0x1p-98
            let azz: f64 = fabs(zz);

            // ~ 0x1.79d15d0e8d59b80000000000000ffc3dp+0
            if azz == f64::from_bits(0x4009B78223AA307C) {
                // 0x1.9b78223aa307cp+1
                y1 = copysign(f64::from_bits(0x3FF79D15D0E8D59C), zz); // 0x1.79d15d0e8d59cp+0
            }

            // ~ 0x1.de87aa837820e80000000000001c0f08p+0
            if azz == f64::from_bits(0x401A202BFC89DDFF) {
                // 0x1.a202bfc89ddffp+2
                y1 = copysign(f64::from_bits(0x3FFDE87AA837820F), zz); // 0x1.de87aa837820fp+0
            }

            // The source's list of hard cases for directed rounding is
            // reached only when `round` is not to nearest. Not carried.
        }
    }

    let mut cvt3: u64 = y1.to_bits();
    cvt3 = cvt3.wrapping_add(((et.wrapping_sub(342).wrapping_sub(1023)) as u64) << 52);
    let m0: u64 = cvt3 << 30;
    let m1 = m0 >> 63;

    if (m0 ^ m1) <= (1u64 << 30) {
        let mut cvt4: u64 = y1.to_bits();
        cvt4 = (cvt4 + (164 << 15)) & 0xffffffffffff0000u64;

        if fabs((f64::from_bits(cvt4) - y1) - dy) < f64::from_bits(0x3C30000000000000) || fabs(zz) == 1.0 {
            // 0x1p-60
            cvt3 = (cvt3 + (1u64 << 15)) & 0xffffffffffff0000u64;
        }
    }

    f64::from_bits(cvt3)
}

// The 64-bit values of the constants `cbrt` names in the source's generic
// float trait: 52 significand bits, the exponent field's largest value, and
// the masks of the significand and the sign.
const CBRT_SIG_BITS: u32 = 52;
const CBRT_EXP_SAT: u32 = 0x7ff;
const CBRT_SIG_MASK: u64 = 0x000f_ffff_ffff_ffff;
const CBRT_SIGN_MASK: u64 = 0x8000_0000_0000_0000;

/// The rounding error of a product, exactly: `a × b − p`, where `p` is the
/// product `a * b` rounded. Dekker's product with Veltkamp's split (T. J.
/// Dekker, "A floating-point technique for extending the available
/// precision", Numerische Mathematik 18, 1971), in the order of J. R.
/// Shewchuk's `Two_Product` (Discrete & Computational Geometry 18, 1997).
/// Where neither the product nor its parts overflow or fall below the normal
/// range, as for every argument `cbrt` gives it, the error is a 64-bit number
/// and this is it exactly, which is what the source's fused multiply-add
/// `a.fma(b, -p)` returns.
fn product_error(a: f64, b: f64, p: f64) -> f64 {
    let ca = SPLIT * a;
    let a_hi = ca - (ca - a);
    let a_lo = a - a_hi;
    let cb = SPLIT * b;
    let b_hi = cb - (cb - b);
    let b_lo = b - b_hi;
    let err1 = p - a_hi * b_hi;
    let err2 = err1 - a_lo * b_hi;
    let err3 = err2 - a_hi * b_lo;
    a_lo * b_lo - err3
}

// ---------------------------------------------------------------------------
// scalbn, floor, fabs and the word helpers
// ---------------------------------------------------------------------------

/// `x` times two to the power `n`, rounded once.
///
/// The source's generic `scalbn`, with the 64-bit constants written out:
/// largest exponent 1023, smallest normal exponent −1022, 53 significand
/// bits, exponent bias 1023.
fn scalbn(mut x: f64, mut n: i32) -> f64 {
    // Maximum and minimum values when biased
    let exp_max: i32 = 1023;
    let exp_min: i32 = -1022;

    // 2 ^ Emax, maximum positive with null significand (0x1p1023 for f64)
    let f_exp_max = f64::from_bits(0x7fe0000000000000);

    // 2 ^ Emin, minimum positive normal with null significand (0x1p-1022 for f64)
    let f_exp_min = f64::from_bits(0x0010000000000000);

    // 2 ^ sig_total_bits, moltiplier to normalize subnormals (0x1p53 for f64)
    let f_pow_subnorm = f64::from_bits(0x4340000000000000);

    /*
     * The goal is to multiply `x` by a scale factor that applies `n`. However, there are cases
     * where `2^n` is not representable by `F` but the result should be, e.g. `x = 2^Emin` with
     * `n = -EMin + 2` (one out of range of 2^Emax). To get around this, reduce the magnitude of
     * the final scale operation by prescaling by the max/min power representable by `F`.
     */

    if n > exp_max {
        // Worse case positive `n`: `x`  is the minimum subnormal value, the result is `F::MAX`.
        // This can be reached by three scaling multiplications (two here and one final).
        x *= f_exp_max;
        n -= exp_max;
        if n > exp_max {
            x *= f_exp_max;
            n -= exp_max;
            if n > exp_max {
                n = exp_max;
            }
        }
    } else if n < exp_min {
        // When scaling toward 0, the prescaling is limited to a value that does not allow `x` to
        // go subnormal. This avoids double rounding.

        // `mul` s.t. `!(x * mul).is_subnormal() ∀ x`
        let mul = f_exp_min * f_pow_subnorm;
        let add = -exp_min - 53;

        // Worse case negative `n`: `x`  is the maximum positive value, the result is `F::MIN`.
        // This must be reachable by three scaling multiplications (two here and one final).
        x *= mul;
        n += add;

        if n < exp_min {
            x *= mul;
            n += add;

            if n < exp_min {
                n = exp_min;
            }
        }
    }

    let scale = f64::from_bits((((1023 + n) as u32 & 0x7ff) as u64) << 52);
    x * scale
}

/* SPDX-License-Identifier: MIT
 * origin: musl src/math/floor.c */

/// The largest whole number not above `x`. Exact.
///
/// The source's generic `floor`, with the 64-bit constants written out.
fn floor(x: f64) -> f64 {
    let mut ix = x.to_bits();
    let e = ((ix >> 52) as u32 & 0x7ff) as i32 - 1023;

    // If the represented value has no fractional part, no truncation is needed.
    if e >= 52 {
        return x;
    }

    if e >= 0 {
        // |x| >= 1.0
        let m = 0x000f_ffff_ffff_ffff_u64 >> (e as u32);
        if ix & m == 0 {
            // Portion to be masked is already zero; no adjustment needed.
            return x;
        }

        if (ix >> 63) != 0 {
            ix += m;
        }

        ix &= !m;
        f64::from_bits(ix)
    } else {
        // |x| < 1.0
        if (ix >> 63) == 0 {
            // 0.0 <= x < 1.0; rounding down goes toward +0.0.
            0.0
        } else if ix << 1 != 0 {
            // -1.0 < x < 0.0; rounding down goes toward -1.0.
            -1.0
        } else {
            // -0.0 remains unchanged
            x
        }
    }
}

/// `x` with its sign bit cleared.
fn fabs(x: f64) -> f64 {
    f64::from_bits(x.to_bits() & 0x7fff_ffff_ffff_ffff)
}

fn get_high_word(x: f64) -> u32 {
    (x.to_bits() >> 32) as u32
}

fn get_low_word(x: f64) -> u32 {
    x.to_bits() as u32
}

fn with_set_low_word(f: f64, lo: u32) -> f64 {
    let mut tmp = f.to_bits();
    tmp &= 0xffffffff_00000000;
    tmp |= lo as u64;
    f64::from_bits(tmp)
}

fn with_set_high_word(f: f64, hi: u32) -> f64 {
    let mut tmp = f.to_bits();
    tmp &= 0x00000000_ffffffff;
    tmp |= (hi as u64) << 32;
    f64::from_bits(tmp)
}

fn combine_words(hi: u32, lo: u32) -> f64 {
    f64::from_bits(((hi as u64) << 32) | lo as u64)
}

/// `x` with the sign of `y`. Exact.
///
/// The source's generic `copysign`, written for 64 bits.
fn copysign(x: f64, y: f64) -> f64 {
    let mut ux = x.to_bits();
    let uy = y.to_bits();
    ux &= !0x8000_0000_0000_0000;
    ux |= uy & 0x8000_0000_0000_0000;
    f64::from_bits(ux)
}

#[cfg(test)]
mod tests {
    //! The seven functions against correctly rounded values, the values the
    //! source states for zeros, infinities and NaN, and the source's own
    //! test points.
    //!
    //! `math_vectors.rs` holds the correctly rounded values. It is written
    //! by `make_vectors.py` in the checks folder `kernel_math`: about three
    //! thousand arguments a function, chosen by a fixed rule in integer
    //! arithmetic on the 64-bit pattern, with no random number. That script
    //! states the rule. The values are computed with mpmath at 384 bits or
    //! more and rounded to the nearest 64-bit number.
    //!
    //! `math_vectors_log.rs` holds the correctly rounded values of the
    //! logarithm. It is written by `make_log_vectors.py`, by the same rule,
    //! which that script states.
    //!
    //! `math_vectors_solvers.rs` holds the correctly rounded values of the
    //! twelve functions carried for the solvers, 39,811 rows. It is written by
    //! `make_vectors_solvers.py` in the checks folder `kernel_math_014`, by
    //! the same rule, which that script states, with mpmath at 200 bits or
    //! more and a second precision 320 bits higher.
    //!
    //! Each function must land within one unit in the last place of every
    //! value, `tanh` within two and `cbrt` on the value itself. Each test also
    //! pins how many values the function hits exactly, or at each distance,
    //! and a digest of every result bit. Both are properties of the function
    //! and not of the machine, so on a target where a result differs in any
    //! bit the digest test fails.

    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{
        acos, asin, asinh, atan, atan2, cbrt, cos, cosh, exp, exp_m1, floor, hypot, ln_1p, log, log10, powf, powi,
        product_error, rem_pio2, scalbn, sin, sinh, tan, tanh,
    };

    std::include!("math_vectors.rs");
    std::include!("math_vectors_log.rs");
    std::include!("math_vectors_solvers.rs");

    const NAN: f64 = f64::NAN;
    const INFINITY: f64 = f64::INFINITY;

    /// Position of a 64-bit pattern on the number line, counted in
    /// representable numbers. Infinity is one past the largest finite number.
    fn ordinal(bits: u64) -> i128 {
        let magnitude = (bits & 0x7fff_ffff_ffff_ffff) as i128;
        if bits >> 63 != 0 { -magnitude } else { magnitude }
    }

    /// Units in the last place between two results.
    fn units_apart(got: f64, want_bits: u64) -> i128 {
        assert!(!got.is_nan());
        let d = ordinal(got.to_bits()) - ordinal(want_bits);
        if d < 0 { -d } else { d }
    }

    fn mix(h: u64, w: u64) -> u64 {
        let mut x = (h ^ w).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        x ^= x >> 29;
        x
    }

    /// Rows exactly right, rows one unit off, and a digest of every result.
    /// Panics on a row more than one unit off.
    fn score(name: &str, rows: &[[u64; 2]], function: fn(f64) -> f64) -> (usize, usize, u64) {
        let mut exact = 0;
        let mut one_off = 0;
        let mut digest = 0;
        for row in rows {
            let got = function(f64::from_bits(row[0]));
            let apart = units_apart(got, row[1]);
            assert!(
                apart <= 1,
                "{name}({:#018x}) = {:#018x}, correctly rounded {:#018x}",
                row[0],
                got.to_bits(),
                row[1]
            );
            if apart == 0 {
                exact += 1;
            } else {
                one_off += 1;
            }
            digest = mix(digest, got.to_bits());
        }
        (exact, one_off, digest)
    }

    #[test]
    fn constants_have_the_bit_patterns_the_source_states() {
        let pairs: [(f64, u64); 48] = [
            (super::S1, 0xBFC5555555555549),
            (super::S2, 0x3F8111111110F8A6),
            (super::S3, 0xBF2A01A019C161D5),
            (super::S4, 0x3EC71DE357B1FE7D),
            (super::S5, 0xBE5AE5E68A2B9CEB),
            (super::S6, 0x3DE5D93A5ACFD57C),
            (super::C1, 0x3FA555555555554C),
            (super::C2, 0xBF56C16C16C15177),
            (super::C3, 0x3EFA01A019CB1590),
            (super::C4, 0xBE927E4F809C52AD),
            (super::C5, 0x3E21EE9EBDB4B1C4),
            (super::C6, 0xBDA8FAE9BE8838D4),
            (super::INV_PIO2, 0x3FE45F306DC9C883),
            (super::PIO2_1, 0x3FF921FB54400000),
            (super::PIO2_1T, 0x3DD0B4611A626331),
            (super::PIO2_2, 0x3DD0B4611A600000),
            (super::PIO2_2T, 0x3BA3198A2E037073),
            (super::PIO2_3, 0x3BA3198A2E000000),
            (super::PIO2_3T, 0x397B839A252049C1),
            (super::PIO2_HI, 0x3FF921FB54442D18),
            (super::PIO2_LO, 0x3C91A62633145C07),
            (super::P_S0, 0x3FC5555555555555),
            (super::P_S1, 0xBFD4D61203EB6F7D),
            (super::P_S2, 0x3FC9C1550E884455),
            (super::P_S3, 0xBFA48228B5688F3B),
            (super::P_S4, 0x3F49EFE07501B288),
            (super::P_S5, 0x3F023DE10DFDF709),
            (super::Q_S1, 0xC0033A271C8A2D4B),
            (super::Q_S2, 0x40002AE59C598AC8),
            (super::Q_S3, 0xBFE6066C1B8D0159),
            (super::Q_S4, 0x3FB3B8C5B12E9282),
            (super::LN2HI, 0x3FE62E42FEE00000),
            (super::LN2LO, 0x3DEA39EF35793C76),
            (super::INVLN2, 0x3FF71547652B82FE),
            (super::P1, 0x3FC555555555553E),
            (super::P2, 0xBF66C16C16BEBD93),
            (super::P3, 0x3F11566AAF25DE2C),
            (super::P4, 0xBEBBBD41C5D26BF1),
            (super::P5, 0x3E66376972BEA4D0),
            (super::O_THRESHOLD, 0x40862E42FEFA39EF),
            (super::LN2_HI, 0x3FE62E42FEE00000),
            (super::LN2_LO, 0x3DEA39EF35793C76),
            (super::EXPM1_INVLN2, 0x3FF71547652B82FE),
            (super::Q1, 0xBFA11111111110F4),
            (super::Q2, 0x3F5A01A019FE5585),
            (super::Q3, 0xBF14CE199EAADBB7),
            (super::Q4, 0x3ED0CFCA86E65239),
            (super::Q5, 0xBE8AFDB76E09C32D),
        ];
        for (index, (constant, bits)) in pairs.iter().enumerate() {
            assert_eq!(constant.to_bits(), *bits, "constant {index}");
        }
        let pio2: [u64; 8] = [
            0x3FF921FB40000000,
            0x3E74442D00000000,
            0x3CF8469880000000,
            0x3B78CC5160000000,
            0x39F01B8380000000,
            0x387A252040000000,
            0x36E3822280000000,
            0x3569F31D00000000,
        ];
        for index in 0..8 {
            assert_eq!(super::PIO2[index].to_bits(), pio2[index], "PIO2[{index}]");
        }
        // 2^-52, 1.5 × 2^52, 2^27 + 1.
        assert_eq!(super::EPS.to_bits(), 0x3CB0000000000000);
        assert_eq!(super::TO_INT.to_bits(), 0x4338000000000000);
        assert_eq!(super::SPLIT.to_bits(), 0x41A0000002000000);
        // The table of 2/π: 66 entries of 24 bits, starting 0.A2F983 6E4E44 …
        assert_eq!(super::IPIO2.len(), 66);
        assert_eq!(super::IPIO2[0], 0xA2F983);
        assert_eq!(super::IPIO2[65], 0xC08C6B);
        let mut sum: u64 = 0;
        for entry in super::IPIO2 {
            assert!(entry >= 0 && entry < 0x1000000);
            sum += entry as u64;
        }
        assert_eq!(sum, 532_010_188);
    }

    #[test]
    fn sin_is_within_one_unit_of_the_correctly_rounded_value() {
        let (exact, one_off, digest) = score("sin", &SIN, sin);
        assert_eq!(SIN.len(), 3117);
        assert_eq!((exact, one_off), (3059, 58));
        assert_eq!(digest, 0x411c7778da050c17);
    }

    #[test]
    fn cos_is_within_one_unit_of_the_correctly_rounded_value() {
        let (exact, one_off, digest) = score("cos", &COS, cos);
        assert_eq!(COS.len(), 3117);
        assert_eq!((exact, one_off), (3057, 60));
        assert_eq!(digest, 0xa969488dafa6b1ad);
    }

    #[test]
    fn asin_is_within_one_unit_of_the_correctly_rounded_value() {
        let (exact, one_off, digest) = score("asin", &ASIN, asin);
        assert_eq!(ASIN.len(), 2899);
        assert_eq!((exact, one_off), (2770, 129));
        assert_eq!(digest, 0xd63e737c721dd8cf);
    }

    #[test]
    fn exp_is_within_one_unit_of_the_correctly_rounded_value() {
        let (exact, one_off, digest) = score("exp", &EXP, exp);
        assert_eq!(EXP.len(), 3178);
        assert_eq!((exact, one_off), (2956, 222));
        assert_eq!(digest, 0x72b6bf83ad9744ec);
    }

    #[test]
    fn exp_m1_is_within_one_unit_of_the_correctly_rounded_value() {
        let (exact, one_off, digest) = score("exp_m1", &EXP_M1, exp_m1);
        assert_eq!(EXP_M1.len(), 3402);
        assert_eq!((exact, one_off), (3188, 214));
        assert_eq!(digest, 0xc869d9b030e9f6f9);
    }

    #[test]
    fn log_is_within_one_unit_of_the_correctly_rounded_value() {
        let (exact, one_off, digest) = score("log", &LOG, log);
        assert_eq!(LOG.len(), 2990);
        assert_eq!((exact, one_off), (2887, 103));
        assert_eq!(digest, 0xbdd78e3becb684bd);
    }

    #[test]
    fn the_constants_of_log_have_the_bit_patterns_the_source_states() {
        let pairs: [(f64, u64); 9] = [
            (super::LN2_HI, 0x3FE62E42FEE00000),
            (super::LN2_LO, 0x3DEA39EF35793C76),
            (super::LG1, 0x3FE5555555555593),
            (super::LG2, 0x3FD999999997FA04),
            (super::LG3, 0x3FD2492494229359),
            (super::LG4, 0x3FCC71C51D8E78AF),
            (super::LG5, 0x3FC7466496CB03DE),
            (super::LG6, 0x3FC39A09D078C69F),
            (super::LG7, 0x3FC2F112DF3E5244),
        ];
        for (index, (constant, bits)) in pairs.iter().enumerate() {
            assert_eq!(constant.to_bits(), *bits, "constant {index}");
        }
    }

    #[test]
    fn log_at_zero_one_infinity_nan_and_below_zero_follows_the_source() {
        // "log(0) is -INF", for either zero.
        assert_eq!(log(0.0).to_bits(), (-INFINITY).to_bits());
        assert_eq!(log(-0.0).to_bits(), (-INFINITY).to_bits());
        // "log(x) is NaN ... if x < 0 (including -INF)".
        assert!(log(-1.0).is_nan());
        assert!(log(-f64::from_bits(1)).is_nan());
        assert!(log(-INFINITY).is_nan());
        // "log(+INF) is +INF"; "log(NaN) is that NaN".
        assert_eq!(log(INFINITY).to_bits(), INFINITY.to_bits());
        assert!(log(NAN).is_nan());
        // The logarithm of one is exactly zero.
        assert_eq!(log(1.0).to_bits(), 0.0_f64.to_bits());
        // A power of two is k times ln 2, and e gives one.
        assert_eq!(log(2.0).to_bits(), core::f64::consts::LN_2.to_bits());
        assert_eq!(log(0.5).to_bits(), (-core::f64::consts::LN_2).to_bits());
        assert_eq!(log(core::f64::consts::E).to_bits(), 1.0_f64.to_bits());
        // The smallest subnormal and the largest finite number are finite.
        assert_eq!(log(f64::from_bits(1)).to_bits(), (-744.4400719213812_f64).to_bits());
        assert_eq!(log(f64::MAX).to_bits(), 709.782712893384_f64.to_bits());
        // exp undoes log to a few parts in 10^16 over the resistances and
        // time constants the cell card reads: one unit of the logarithm is
        // up to |ln x| units of the value.
        for x in [0.005, 0.0152768, 0.1, 0.40219909702068646, 2.0, 60.68, 101.83, 300.0] {
            let back = exp(log(x));
            assert!(((back - x) / x).abs() <= 2.0e-15, "{x} came back {back}");
        }
    }

    #[test]
    fn hypot_is_within_one_unit_of_the_correctly_rounded_value() {
        let mut exact = 0;
        let mut one_off = 0;
        let mut digest = 0;
        for row in &HYPOT {
            let x = f64::from_bits(row[0]);
            let y = f64::from_bits(row[1]);
            let got = hypot(x, y);
            let apart = units_apart(got, row[2]);
            assert!(
                apart <= 1,
                "hypot({:#018x}, {:#018x}) = {:#018x}, correctly rounded {:#018x}",
                row[0],
                row[1],
                got.to_bits(),
                row[2]
            );
            if apart == 0 {
                exact += 1;
            } else {
                one_off += 1;
            }
            digest = mix(digest, got.to_bits());
            // The order and the signs of the two legs do not matter.
            assert_eq!(hypot(y, x).to_bits(), got.to_bits());
            assert_eq!(hypot(-x, y).to_bits(), got.to_bits());
            assert_eq!(hypot(x, -y).to_bits(), got.to_bits());
        }
        assert_eq!(HYPOT.len(), 3173);
        assert_eq!((exact, one_off), (2907, 266));
        assert_eq!(digest, 0x6440d6407d007b5b);
    }

    #[test]
    fn sin_and_cos_at_zero_infinity_and_nan_follow_the_source() {
        // sin keeps the sign of zero. cos of either zero is exactly 1.
        assert_eq!(sin(0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(sin(-0.0).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(cos(0.0).to_bits(), 1.0_f64.to_bits());
        assert_eq!(cos(-0.0).to_bits(), 1.0_f64.to_bits());
        // "trig(+-INF) is NaN", "trig(NaN) is that NaN".
        assert!(sin(INFINITY).is_nan());
        assert!(sin(-INFINITY).is_nan());
        assert!(cos(INFINITY).is_nan());
        assert!(cos(-INFINITY).is_nan());
        assert!(sin(NAN).is_nan());
        assert!(cos(NAN).is_nan());
        // Below 2^-26 the sine is the argument, subnormals included.
        for bits in [1_u64, 0x000f_ffff_ffff_ffff, 0x0010_0000_0000_0000, 0x3e4f_ffff_ffff_ffff] {
            let x = f64::from_bits(bits);
            assert_eq!(sin(x).to_bits(), bits);
            assert_eq!(sin(-x).to_bits(), (-x).to_bits());
        }
        // Below 2^-27 × √2 the cosine is 1.
        for bits in [1_u64, 0x000f_ffff_ffff_ffff, 0x0010_0000_0000_0000, 0x3e46_a09d_ffff_ffff] {
            let x = f64::from_bits(bits);
            assert_eq!(cos(x).to_bits(), 1.0_f64.to_bits());
            assert_eq!(cos(-x).to_bits(), 1.0_f64.to_bits());
        }
        // sin is odd and cos is even, to the bit, across every branch.
        for row in &SIN {
            let x = f64::from_bits(row[0]);
            assert_eq!(sin(-x).to_bits(), (-sin(x)).to_bits());
            assert_eq!(cos(-x).to_bits(), cos(x).to_bits());
        }
    }

    #[test]
    fn asin_at_its_ends_and_outside_them_follows_the_source() {
        assert_eq!(asin(0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(asin(-0.0).to_bits(), (-0.0_f64).to_bits());
        // asin(±1) is ±π/2 rounded: 0x3FF921FB54442D18.
        assert_eq!(asin(1.0).to_bits(), 0x3FF921FB54442D18);
        assert_eq!(asin(-1.0).to_bits(), 0xBFF921FB54442D18);
        assert_eq!(asin(1.0).to_bits(), core::f64::consts::FRAC_PI_2.to_bits());
        // "if |x|>1, return NaN", "if x is NaN, return x itself".
        assert!(asin(f64::from_bits(0x3FF0000000000001)).is_nan());
        assert!(asin(-1.5).is_nan());
        assert!(asin(INFINITY).is_nan());
        assert!(asin(-INFINITY).is_nan());
        assert!(asin(NAN).is_nan());
        // Below 2^-26 the arcsine is the argument.
        for bits in [1_u64, 0x0010_0000_0000_0000, 0x3e4f_ffff_ffff_ffff] {
            assert_eq!(asin(f64::from_bits(bits)).to_bits(), bits);
        }
        // Odd, to the bit.
        for row in &ASIN {
            let x = f64::from_bits(row[0]);
            assert_eq!(asin(-x).to_bits(), (-asin(x)).to_bits());
        }
    }

    #[test]
    fn hypot_exact_cases_zeros_infinities_and_nan_follow_the_source() {
        assert_eq!(hypot(3.0, 4.0).to_bits(), 5.0_f64.to_bits());
        assert_eq!(hypot(-5.0, 12.0).to_bits(), 13.0_f64.to_bits());
        assert_eq!(hypot(8.0e200, 15.0e200).to_bits(), 17.0e200_f64.to_bits());
        // Either zero with either zero is +0. A zero leg returns the size of the other.
        for x in [0.0_f64, -0.0] {
            for y in [0.0_f64, -0.0] {
                assert_eq!(hypot(x, y).to_bits(), 0.0_f64.to_bits());
            }
            assert_eq!(hypot(x, -2.5).to_bits(), 2.5_f64.to_bits());
            assert_eq!(hypot(7.0e-320, x).to_bits(), 7.0e-320_f64.to_bits());
        }
        // "hypot(inf,nan) == inf": an infinite leg wins over NaN.
        for inf in [INFINITY, -INFINITY] {
            assert_eq!(hypot(inf, NAN).to_bits(), INFINITY.to_bits());
            assert_eq!(hypot(NAN, inf).to_bits(), INFINITY.to_bits());
            assert_eq!(hypot(inf, 1.0).to_bits(), INFINITY.to_bits());
            assert_eq!(hypot(-0.0, inf).to_bits(), INFINITY.to_bits());
        }
        assert!(hypot(NAN, 1.0).is_nan());
        assert!(hypot(0.0, NAN).is_nan());
        assert!(hypot(NAN, NAN).is_nan());
        // No overflow on the way to a result that fits, and none lost at the bottom.
        assert_eq!(hypot(3.0e307, 4.0e307).to_bits(), 5.0e307_f64.to_bits());
        let least = f64::from_bits(1);
        assert_eq!(hypot(3.0 * least, 4.0 * least).to_bits(), (5.0 * least).to_bits());
        // Past the largest finite number the result is infinity.
        assert_eq!(hypot(f64::MAX, f64::MAX).to_bits(), INFINITY.to_bits());
    }

    #[test]
    fn exp_at_zero_infinity_nan_and_past_its_range_follows_the_source() {
        // "for finite argument, only exp(0)=1 is exact".
        assert_eq!(exp(0.0).to_bits(), 1.0_f64.to_bits());
        assert_eq!(exp(-0.0).to_bits(), 1.0_f64.to_bits());
        // "exp(INF) is INF, exp(NaN) is NaN; exp(-INF) is 0".
        assert_eq!(exp(INFINITY).to_bits(), INFINITY.to_bits());
        assert_eq!(exp(-INFINITY).to_bits(), 0.0_f64.to_bits());
        assert!(exp(NAN).is_nan());
        // "if x > 709.782712893383973096 then exp(x) overflows".
        assert!(exp(709.782712893384).is_finite());
        assert_eq!(exp(709.7827128933841).to_bits(), INFINITY.to_bits());
        assert_eq!(exp(1.0e300).to_bits(), INFINITY.to_bits());
        // "if x < -745.133219101941108420 then exp(x) underflows".
        assert_eq!(exp(-745.1332191019411).to_bits(), 1);
        assert_eq!(exp(-745.2).to_bits(), 0.0_f64.to_bits());
        assert_eq!(exp(-1.0e300).to_bits(), 0.0_f64.to_bits());
        // exp(1) is one unit above e correctly rounded, inside the source's stated bound.
        assert_eq!(exp(1.0).to_bits(), core::f64::consts::E.to_bits() + 1);
        // Under 2^-28 the result is 1 + x.
        assert_eq!(exp(1.0e-300).to_bits(), 1.0_f64.to_bits());
        assert_eq!(exp(f64::from_bits(1)).to_bits(), 1.0_f64.to_bits());
    }

    #[test]
    fn exp_m1_at_zero_infinity_nan_and_past_its_range_follows_the_source() {
        // "for finite argument, only expm1(0)=0 is exact". The sign of zero is kept.
        assert_eq!(exp_m1(0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(exp_m1(-0.0).to_bits(), (-0.0_f64).to_bits());
        // "expm1(INF) is INF, expm1(NaN) is NaN; expm1(-INF) is -1".
        assert_eq!(exp_m1(INFINITY).to_bits(), INFINITY.to_bits());
        assert_eq!(exp_m1(-INFINITY).to_bits(), (-1.0_f64).to_bits());
        assert!(exp_m1(NAN).is_nan());
        // Overflow past 709.782712893384; −1 from 56 ln 2 down.
        assert!(exp_m1(709.782712893384).is_finite());
        assert_eq!(exp_m1(709.7827128933841).to_bits(), INFINITY.to_bits());
        assert_eq!(exp_m1(-38.9).to_bits(), (-1.0_f64).to_bits());
        assert_eq!(exp_m1(-1.0e300).to_bits(), (-1.0_f64).to_bits());
        // Under 2^-54 the result is the argument, subnormals included.
        for bits in [1_u64, 0x0010_0000_0000_0000, 0x3c8f_ffff_ffff_ffff] {
            let x = f64::from_bits(bits);
            assert_eq!(exp_m1(x).to_bits(), bits);
            assert_eq!(exp_m1(-x).to_bits(), (-x).to_bits());
        }
        // 1 − exp(−x) from exp_m1 keeps the digits a subtraction from 1 loses.
        let x = 1.0e-10;
        let direct = 1.0 - exp(-x);
        let kept = -exp_m1(-x);
        assert!((kept - x).abs() < 1.0e-20);
        assert!((direct - x).abs() > 1.0e-18);
    }

    #[test]
    fn the_test_points_of_the_source_hold() {
        // sin.rs, `test_near_pi`.
        assert_eq!(sin(f64::from_bits(0x400921fb000FD5DD)).to_bits(), 0x3ea50d15ced1a4a2);
        // expm1.rs, `sanity_check`.
        assert_eq!(exp_m1(1.1).to_bits(), 2.0041660239464334_f64.to_bits());
        // rem_pio2.rs, `test_near_pi`.
        for (x, y0) in [
            (3.141592025756836, -6.278329573009626e-7),
            (3.141592033207416, -6.20382377148128e-7),
            (3.141592144966125, -5.086236681942706e-7),
            (3.141592979431152, 3.2584135866119817e-7),
        ] {
            let (n, got_0, got_1) = rem_pio2(x);
            assert_eq!(n, 2);
            assert_eq!(got_0.to_bits(), f64::to_bits(y0));
            assert_eq!(got_1.to_bits(), (-2.1125998133974653e-23_f64).to_bits());
        }
        // rem_pio2.rs, `test_overflow_b9b847` and `test_overflow_4747b9`: these must not panic.
        let _ = rem_pio2(-3054214.5490637687);
        let _ = rem_pio2(917340800458.2274);
    }

    #[test]
    fn large_arguments_reduce_without_reading_past_the_table() {
        // Every power of two from 2^20 to 2^1023, and the number just under
        // each: the large reduction at every exponent it serves. The result
        // is a sine and a cosine, so the two squares sum to 1.
        let mut exponent = 20;
        while exponent <= 1023 {
            let power = f64::from_bits(((1023 + exponent) as u64) << 52);
            for x in [power, f64::from_bits(power.to_bits() - 1), -power] {
                let s = sin(x);
                let c = cos(x);
                assert!(s.abs() <= 1.0 && c.abs() <= 1.0);
                assert!((s * s + c * c - 1.0).abs() < 4.0e-16, "x = {x:e}");
            }
            exponent += 1;
        }
    }

    #[test]
    fn scalbn_and_floor_are_exact() {
        // scalbn rounds once, at the end.
        let least = f64::from_bits(1);
        assert_eq!(scalbn(1.0, 0).to_bits(), 1.0_f64.to_bits());
        assert_eq!(scalbn(1.0, 1023).to_bits(), 0x7fe0000000000000);
        assert_eq!(scalbn(1.0, 1024).to_bits(), INFINITY.to_bits());
        assert_eq!(scalbn(1.0, -1022).to_bits(), 0x0010000000000000);
        assert_eq!(scalbn(1.0, -1074).to_bits(), 1);
        assert_eq!(scalbn(1.0, -1075).to_bits(), 0);
        assert_eq!(scalbn(1.5, -1074).to_bits(), 2);
        assert_eq!(scalbn(1.25, -1074).to_bits(), 1);
        assert_eq!(scalbn(least, 1074).to_bits(), 1.0_f64.to_bits());
        assert_eq!(scalbn(least, 2097).to_bits(), 0x7fe0000000000000);
        assert_eq!(scalbn(f64::MAX, -2097).to_bits(), 2);
        assert_eq!(scalbn(-3.0, 4).to_bits(), (-48.0_f64).to_bits());
        assert_eq!(scalbn(-0.0, 7).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(scalbn(INFINITY, -5000).to_bits(), INFINITY.to_bits());
        assert!(scalbn(NAN, 3).is_nan());
        // A double rounding would give 0x0008000000000001 here. One rounding
        // of 0x1.0000000000001p-1023 at half spacing ties to even.
        assert_eq!(
            scalbn(f64::from_bits(0x3ff0000000000001), -1023).to_bits(),
            0x0008000000000000
        );
        // floor.
        for (x, want) in [
            (0.0_f64, 0.0_f64),
            (0.5, 0.0),
            (0.9999999999999999, 0.0),
            (1.0, 1.0),
            (1.5, 1.0),
            (-0.5, -1.0),
            (-1.0, -1.0),
            (-1.5, -2.0),
            (7.999999999999999, 7.0),
            (4503599627370495.5, 4503599627370495.0),
            (-4503599627370495.5, -4503599627370496.0),
            (9007199254740992.0, 9007199254740992.0),
            (1.0e300, 1.0e300),
        ] {
            assert_eq!(floor(x).to_bits(), want.to_bits(), "floor({x})");
        }
        assert_eq!(floor(-0.0).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(floor(-f64::from_bits(1)).to_bits(), (-1.0_f64).to_bits());
        assert_eq!(floor(INFINITY).to_bits(), INFINITY.to_bits());
        assert!(floor(NAN).is_nan());
    }

    // ------------------------------------------------------------------
    // The twelve functions carried for the solvers, and powi
    // ------------------------------------------------------------------

    /// Rows at 0, 1, 2 and 3 units in the last place from the correctly
    /// rounded value, and a digest of every result. Panics on a row more
    /// than `most` units off.
    fn spread(name: &str, rows: &[[u64; 2]], function: fn(f64) -> f64, most: i128) -> ([usize; 4], u64) {
        let mut counts = [0; 4];
        let mut digest = 0;
        for row in rows {
            let got = function(f64::from_bits(row[0]));
            let apart = units_apart(got, row[1]);
            assert!(
                apart <= most,
                "{name}({:#018x}) = {:#018x}, correctly rounded {:#018x}, {apart} units apart",
                row[0],
                got.to_bits(),
                row[1]
            );
            counts[apart as usize] += 1;
            digest = mix(digest, got.to_bits());
        }
        (counts, digest)
    }

    /// The same for a function of two arguments: rows of (a, b, value).
    fn spread_2(name: &str, rows: &[[u64; 3]], function: fn(f64, f64) -> f64, most: i128) -> ([usize; 4], u64) {
        let mut counts = [0; 4];
        let mut digest = 0;
        for row in rows {
            let got = function(f64::from_bits(row[0]), f64::from_bits(row[1]));
            let apart = units_apart(got, row[2]);
            assert!(
                apart <= most,
                "{name}({:#018x}, {:#018x}) = {:#018x}, correctly rounded {:#018x}, {apart} units apart",
                row[0],
                row[1],
                got.to_bits(),
                row[2]
            );
            counts[apart as usize] += 1;
            digest = mix(digest, got.to_bits());
        }
        (counts, digest)
    }

    #[test]
    fn ln_1p_is_within_one_unit_of_the_correctly_rounded_value() {
        let (counts, digest) = spread("ln_1p", &LN_1P, ln_1p, 1);
        assert_eq!(LN_1P.len(), 2946);
        assert_eq!((counts, digest), ([2894, 52, 0, 0], 3399679559959605104));
    }

    #[test]
    fn log10_is_within_one_unit_of_the_correctly_rounded_value() {
        let (counts, digest) = spread("log10", &LOG10, log10, 1);
        assert_eq!(LOG10.len(), 4927);
        assert_eq!((counts, digest), ([4910, 17, 0, 0], 1731637385017840254));
    }

    #[test]
    fn powf_is_within_one_unit_of_the_correctly_rounded_value() {
        let (counts, digest) = spread_2("powf", &POWF, powf, 1);
        assert_eq!(POWF.len(), 6174);
        assert_eq!((counts, digest), ([5723, 451, 0, 0], 14208054534584837222));
    }

    #[test]
    fn tan_is_within_one_unit_of_the_correctly_rounded_value() {
        let (counts, digest) = spread("tan", &TAN, tan, 1);
        assert_eq!(TAN.len(), 3243);
        assert_eq!((counts, digest), ([3166, 77, 0, 0], 12543872719073775628));
    }

    #[test]
    fn acos_is_within_one_unit_of_the_correctly_rounded_value() {
        let (counts, digest) = spread("acos", &ACOS, acos, 1);
        assert_eq!(ACOS.len(), 2558);
        assert_eq!((counts, digest), ([2480, 78, 0, 0], 15347337500759488670));
    }

    #[test]
    fn atan_is_within_one_unit_of_the_correctly_rounded_value() {
        let (counts, digest) = spread("atan", &ATAN, atan, 1);
        assert_eq!(ATAN.len(), 2320);
        assert_eq!((counts, digest), ([2281, 39, 0, 0], 11657626960692843446));
    }

    #[test]
    fn atan2_is_within_one_unit_of_the_correctly_rounded_value() {
        let (counts, digest) = spread_2("atan2", &ATAN2, atan2, 1);
        assert_eq!(ATAN2.len(), 4288);
        assert_eq!((counts, digest), ([3614, 674, 0, 0], 10675228869357620538));
    }

    #[test]
    fn sinh_is_within_one_unit_of_the_correctly_rounded_value() {
        let (counts, digest) = spread("sinh", &SINH, sinh, 1);
        assert_eq!(SINH.len(), 1940);
        assert_eq!((counts, digest), ([1583, 357, 0, 0], 16685771979442723828));
    }

    #[test]
    fn cosh_is_within_one_unit_of_the_correctly_rounded_value() {
        let (counts, digest) = spread("cosh", &COSH, cosh, 1);
        assert_eq!(COSH.len(), 1878);
        assert_eq!((counts, digest), ([1727, 151, 0, 0], 12988181801389683065));
    }

    #[test]
    fn tanh_is_within_two_units_of_the_correctly_rounded_value() {
        let (counts, digest) = spread("tanh", &TANH, tanh, 2);
        assert_eq!(TANH.len(), 1932);
        assert_eq!((counts, digest), ([1606, 322, 4, 0], 9605832495615318557));
    }

    #[test]
    fn asinh_is_within_one_unit_of_the_correctly_rounded_value() {
        let (counts, digest) = spread("asinh", &ASINH, asinh, 1);
        assert_eq!(ASINH.len(), 2142);
        assert_eq!((counts, digest), ([1763, 379, 0, 0], 18331786373033139161));
    }

    #[test]
    fn cbrt_is_the_correctly_rounded_value() {
        let (counts, digest) = spread("cbrt", &CBRT, cbrt, 0);
        assert_eq!(CBRT.len(), 5463);
        assert_eq!((counts, digest), ([5463, 0, 0, 0], 3678021832609668043));
    }

    #[test]
    fn the_constants_of_the_twelve_have_the_bit_patterns_the_source_states() {
        let pairs: [(f64, u64); 46] = [
            (super::IVLN10HI, 0x3FDBCB7B15200000),
            (super::IVLN10LO, 0x3DBB9438CA9AADD5),
            (super::LOG10_2HI, 0x3FD34413509F6000),
            (super::LOG10_2LO, 0x3D59FEF311F12B36),
            (super::DP_H[1], 0x3FE2B80340000000),
            (super::DP_L[1], 0x3E4CFDEB43CFD006),
            (super::TWO53, 0x4340000000000000),
            (super::L1, 0x3FE3333333333303),
            (super::L2, 0x3FDB6DB6DB6FABFF),
            (super::L3, 0x3FD55555518F264D),
            (super::L4, 0x3FD17460A91D4101),
            (super::L5, 0x3FCD864A93C9DB65),
            (super::L6, 0x3FCA7E284A454EEF),
            (super::POW_LG2, 0x3FE62E42FEFA39EF),
            (super::POW_LG2_H, 0x3FE62E4300000000),
            (super::POW_LG2_L, 0xBE205C610CA86C39),
            (super::CP, 0x3FEEC709DC3A03FD),
            (super::CP_H, 0x3FEEC709E0000000),
            (super::CP_L, 0xBE3E2FE0145B01F5),
            (super::IVLN2, 0x3FF71547652B82FE),
            (super::IVLN2_H, 0x3FF7154760000000),
            (super::IVLN2_L, 0x3E54AE0BF85DDF44),
            (super::PIO4, 0x3FE921FB54442D18),
            (super::PIO4_LO, 0x3C81A62633145C07),
            (super::ATANHI[0], 0x3FDDAC670561BB4F),
            (super::ATANHI[1], 0x3FE921FB54442D18),
            (super::ATANHI[2], 0x3FEF730BD281F69B),
            (super::ATANHI[3], 0x3FF921FB54442D18),
            (super::ATANLO[0], 0x3C7A2B7F222F65E2),
            (super::ATANLO[1], 0x3C81A62633145C07),
            (super::ATANLO[2], 0x3C7007887AF0CBBD),
            (super::ATANLO[3], 0x3C91A62633145C07),
            (super::AT[0], 0x3FD555555555550D),
            (super::AT[1], 0xBFC999999998EBC4),
            (super::AT[2], 0x3FC24924920083FF),
            (super::AT[3], 0xBFBC71C6FE231671),
            (super::AT[4], 0x3FB745CDC54C206E),
            (super::AT[5], 0xBFB3B0F2AF749A6D),
            (super::AT[6], 0x3FB10D66A0D03D51),
            (super::AT[7], 0xBFADDE2D52DEFD9A),
            (super::AT[8], 0x3FA97B4B24760DEB),
            (super::AT[9], 0xBFA2B4442C6A6C2F),
            (super::AT[10], 0x3F90AD3AE322DA11),
            (super::PI, 0x400921FB54442D18),
            (super::PI_LO, 0x3CA1A62633145C07),
            (super::LN2, 0x3FE62E42FEFA39EF),
        ];
        for (index, (constant, bits)) in pairs.iter().enumerate() {
            assert_eq!(constant.to_bits(), *bits, "constant {index}");
        }
        let tan: [u64; 13] = [
            0x3FD5555555555563,
            0x3FC111111110FE7A,
            0x3FABA1BA1BB341FE,
            0x3F9664F48406D637,
            0x3F8226E3E96E8493,
            0x3F6D6D22C9560328,
            0x3F57DBC8FEE08315,
            0x3F4344D8F2F26501,
            0x3F3026F71A8D1068,
            0x3F147E88A03792A6,
            0x3F12B80F32F0A7E9,
            0xBEF375CBDB605373,
            0x3EFB2A7074BF7AD4,
        ];
        for index in 0..13 {
            assert_eq!(super::T[index].to_bits(), tan[index], "T[{index}]");
        }
        // The constants the source names twice, with one value: ln_1p's and
        // log10's are log's, powf's P1 to P5 are exp's, acos's are asin's.
        assert_eq!(super::LN2_HI.to_bits(), 0x3FE62E42FEE00000);
        assert_eq!(super::LN2_LO.to_bits(), 0x3DEA39EF35793C76);
        assert_eq!(super::P1.to_bits(), 0x3FC555555555553E);
        assert_eq!(super::P5.to_bits(), 0x3E66376972BEA4D0);
        assert_eq!(super::P_S0.to_bits(), 0x3FC5555555555555);
        assert_eq!(super::Q_S4.to_bits(), 0x3FB3B8C5B12E9282);
        // The large values HUGE and TINY.
        assert_eq!(super::HUGE, 1.0e300);
        assert_eq!(super::TINY, 1.0e-300);
    }

    #[test]
    fn ln_1p_and_log10_at_their_ends_follow_the_source() {
        // "log1p(-1) is -INF", "log1p(x) is NaN ... if x < -1 (including -INF)".
        assert_eq!(ln_1p(-1.0).to_bits(), (-INFINITY).to_bits());
        assert!(ln_1p(-1.0000000000000002).is_nan());
        assert!(ln_1p(-INFINITY).is_nan());
        assert_eq!(ln_1p(INFINITY).to_bits(), INFINITY.to_bits());
        assert!(ln_1p(NAN).is_nan());
        // The sign of zero is kept, and below 2^-53 the result is the argument.
        assert_eq!(ln_1p(0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(ln_1p(-0.0).to_bits(), (-0.0_f64).to_bits());
        for bits in [1_u64, 0x0010_0000_0000_0000, 0x3c9f_ffff_ffff_ffff] {
            assert_eq!(ln_1p(f64::from_bits(bits)).to_bits(), bits);
            assert_eq!(ln_1p(-f64::from_bits(bits)).to_bits(), bits | 0x8000_0000_0000_0000);
        }
        // log10: "log(+-0)=-inf", "log(-#) = NaN", log10(1) = 0, and the
        // whole powers of ten that are 64-bit numbers give their exponent.
        assert_eq!(log10(0.0).to_bits(), (-INFINITY).to_bits());
        assert_eq!(log10(-0.0).to_bits(), (-INFINITY).to_bits());
        assert!(log10(-1.0).is_nan());
        assert!(log10(-INFINITY).is_nan());
        assert!(log10(NAN).is_nan());
        assert_eq!(log10(INFINITY).to_bits(), INFINITY.to_bits());
        assert_eq!(log10(1.0).to_bits(), 0.0_f64.to_bits());
        let mut power = 1.0_f64;
        for k in 1..=22 {
            power *= 10.0;
            assert_eq!(log10(power).to_bits(), (k as f64).to_bits(), "log10(1e{k})");
        }
    }

    #[test]
    fn tan_at_zero_infinity_and_nan_follows_the_source_and_is_odd() {
        assert_eq!(tan(0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(tan(-0.0).to_bits(), (-0.0_f64).to_bits());
        assert!(tan(INFINITY).is_nan());
        assert!(tan(-INFINITY).is_nan());
        assert!(tan(NAN).is_nan());
        // Below 2^-27 the tangent is the argument, subnormals included.
        for bits in [1_u64, 0x0010_0000_0000_0000, 0x3e3f_ffff_ffff_ffff] {
            assert_eq!(tan(f64::from_bits(bits)).to_bits(), bits);
        }
        // Odd to the bit, across every branch; and sin / cos within two
        // units of the tangent, two units of rounding being the quotient's.
        for row in &TAN {
            let x = f64::from_bits(row[0]);
            assert_eq!(tan(-x).to_bits(), (-tan(x)).to_bits());
        }
        for row in &SIN {
            let x = f64::from_bits(row[0]);
            let t = tan(x);
            let q = sin(x) / cos(x);
            if t.is_finite() && q.is_finite() && t.abs() < 1e300 && t != 0.0 {
                assert!(((t - q) / t).abs() < 1.0e-15, "tan({x:e}) = {t:e}, sin/cos {q:e}");
            }
        }
    }

    #[test]
    fn acos_atan_and_atan2_at_their_ends_follow_the_source() {
        // acos: "acos(1)=0, acos(-1)=pi"; "if |x|>1, return NaN"; "if x is NaN, return x itself".
        assert_eq!(acos(1.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(acos(-1.0).to_bits(), core::f64::consts::PI.to_bits());
        assert_eq!(acos(0.0).to_bits(), core::f64::consts::FRAC_PI_2.to_bits());
        assert_eq!(acos(-0.0).to_bits(), core::f64::consts::FRAC_PI_2.to_bits());
        assert!(acos(f64::from_bits(0x3FF0000000000001)).is_nan());
        assert!(acos(-1.5).is_nan());
        assert!(acos(INFINITY).is_nan());
        assert!(acos(NAN).is_nan());
        // acos(x) + acos(-x) is pi, and acos is pi/2 less asin, each within two units.
        for row in &ACOS {
            let x = f64::from_bits(row[0]);
            assert!((acos(x) + acos(-x) - core::f64::consts::PI).abs() <= 2.0 * 4.440892098500626e-16);
            assert!((acos(x) - (core::f64::consts::FRAC_PI_2 - asin(x))).abs() <= 2.0e-15);
        }
        // atan.rs, `zero`, `infinity`, `minus_infinity`, `nan` and `sanity_check`.
        assert_eq!(atan(0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(atan(-0.0).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(atan(INFINITY).to_bits(), core::f64::consts::FRAC_PI_2.to_bits());
        assert_eq!(atan(-INFINITY).to_bits(), (-core::f64::consts::FRAC_PI_2).to_bits());
        assert!(atan(NAN).is_nan());
        let root3 = f64::sqrt(3.0);
        for (input, answer) in [
            (root3 / 3.0, core::f64::consts::FRAC_PI_6),
            (1.0, core::f64::consts::FRAC_PI_4),
            (root3, core::f64::consts::FRAC_PI_3),
            (-root3 / 3.0, -core::f64::consts::FRAC_PI_6),
            (-1.0, -core::f64::consts::FRAC_PI_4),
            (-root3, -core::f64::consts::FRAC_PI_3),
        ] {
            assert!((atan(input) - answer) / answer < 1e-5);
        }
        // Odd to the bit.
        for row in &ATAN {
            let x = f64::from_bits(row[0]);
            assert_eq!(atan(-x).to_bits(), (-atan(x)).to_bits());
        }
        // atan2.rs, `sanity_check`, and the source's special cases.
        let pi = core::f64::consts::PI;
        assert_eq!(atan2(0.0, 1.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(atan2(0.0, -1.0).to_bits(), pi.to_bits());
        assert_eq!(atan2(-0.0, -1.0).to_bits(), (-pi).to_bits());
        assert_eq!(atan2(3.0, 2.0).to_bits(), atan(3.0 / 2.0).to_bits());
        assert_eq!(atan2(2.0, -1.0).to_bits(), (atan(2.0 / -1.0) + pi).to_bits());
        assert_eq!(atan2(-2.0, -1.0).to_bits(), (atan(-2.0 / -1.0) - pi).to_bits());
        assert!(atan2(NAN, 1.0).is_nan());
        assert!(atan2(1.0, NAN).is_nan());
        assert_eq!(atan2(-0.0, 5.0).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(atan2(0.0, -0.0).to_bits(), pi.to_bits());
        assert_eq!(atan2(2.0, 0.0).to_bits(), (pi / 2.0).to_bits());
        assert_eq!(atan2(-2.0, -0.0).to_bits(), (-pi / 2.0).to_bits());
        assert_eq!(atan2(7.0, INFINITY).to_bits(), 0.0_f64.to_bits());
        assert_eq!(atan2(-7.0, INFINITY).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(atan2(7.0, -INFINITY).to_bits(), pi.to_bits());
        assert_eq!(atan2(INFINITY, INFINITY).to_bits(), (pi / 4.0).to_bits());
        assert_eq!(atan2(-INFINITY, -INFINITY).to_bits(), (-3.0 * pi / 4.0).to_bits());
        assert_eq!(atan2(INFINITY, 3.0).to_bits(), (pi / 2.0).to_bits());
        // atan2(y, 1) is atan(y), the source's shortcut, and the angle of
        // (x, y) and of (x, -y) differ in sign only.
        for row in &ATAN {
            let y = f64::from_bits(row[0]);
            assert_eq!(atan2(y, 1.0).to_bits(), atan(y).to_bits());
        }
        for row in &ATAN2 {
            let (y, x) = (f64::from_bits(row[0]), f64::from_bits(row[1]));
            assert_eq!(atan2(-y, x).to_bits(), (-atan2(y, x)).to_bits());
        }
    }

    #[test]
    fn sinh_cosh_tanh_and_asinh_at_their_ends_follow_the_source() {
        for (f, odd) in [(sinh as fn(f64) -> f64, true), (tanh, true), (asinh, true), (cosh, false)] {
            assert!(f(NAN).is_nan());
            for row in SINH.iter().chain(COSH.iter()).chain(TANH.iter()).chain(ASINH.iter()) {
                let x = f64::from_bits(row[0]);
                if odd {
                    assert_eq!(f(-x).to_bits(), (-f(x)).to_bits());
                } else {
                    assert_eq!(f(-x).to_bits(), f(x).to_bits());
                }
            }
        }
        assert_eq!(sinh(0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(sinh(-0.0).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(sinh(INFINITY).to_bits(), INFINITY.to_bits());
        assert_eq!(sinh(-INFINITY).to_bits(), (-INFINITY).to_bits());
        assert_eq!(cosh(0.0).to_bits(), 1.0_f64.to_bits());
        assert_eq!(cosh(-INFINITY).to_bits(), INFINITY.to_bits());
        assert_eq!(tanh(-0.0).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(tanh(INFINITY).to_bits(), 1.0_f64.to_bits());
        assert_eq!(tanh(-INFINITY).to_bits(), (-1.0_f64).to_bits());
        assert_eq!(asinh(-0.0).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(asinh(INFINITY).to_bits(), INFINITY.to_bits());
        // Past 710.4758600739439 the hyperbolic sine and cosine overflow;
        // below it they are finite.
        assert!(sinh(710.4758600739439).is_finite() && cosh(710.4758600739439).is_finite());
        assert_eq!(sinh(710.475860073944).to_bits(), INFINITY.to_bits());
        assert_eq!(cosh(710.475860073944).to_bits(), INFINITY.to_bits());
        // Below 2^-26 sinh and asinh are the argument and cosh is 1; tanh of a
        // subnormal is the argument.
        for bits in [1_u64, 0x0010_0000_0000_0000, 0x3e4f_ffff_ffff_ffff] {
            assert_eq!(sinh(f64::from_bits(bits)).to_bits(), bits);
            assert_eq!(asinh(f64::from_bits(bits)).to_bits(), bits);
            assert_eq!(cosh(f64::from_bits(bits)).to_bits(), 1.0_f64.to_bits());
        }
        assert_eq!(tanh(f64::from_bits(7)).to_bits(), 7);
        // cosh² - sinh² = 1 to a few units where neither is large.
        for row in &SINH {
            let x = f64::from_bits(row[0]);
            if x.abs() < 5.0 {
                let (c, s) = (cosh(x), sinh(x));
                assert!((c * c - s * s - 1.0).abs() < 8.0 * c * c * 2.220446049250313e-16);
            }
        }
    }

    #[test]
    fn cbrt_at_its_ends_follows_the_source_and_cubes_are_exact() {
        // "0, inf, nan: we return x + x".
        assert_eq!(cbrt(0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(cbrt(-0.0).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(cbrt(INFINITY).to_bits(), INFINITY.to_bits());
        assert_eq!(cbrt(-INFINITY).to_bits(), (-INFINITY).to_bits());
        assert!(cbrt(NAN).is_nan());
        // cbrt.rs, `spot_checks`.
        assert_eq!(cbrt(f64::from_bits(0xf7f792b28f600000)).to_bits(), 0xd29ce68655d962f3);
        // A whole cube's root is exact, and the root is odd to the bit.
        for k in 1..=2000_u32 {
            let root = k as f64;
            assert_eq!(cbrt(root * root * root).to_bits(), root.to_bits(), "cbrt({k}^3)");
            assert_eq!(cbrt(-(root * root * root)).to_bits(), (-root).to_bits());
        }
        for row in &CBRT {
            let x = f64::from_bits(row[0]);
            assert_eq!(cbrt(-x).to_bits(), (-cbrt(x)).to_bits());
        }
        // The smallest subnormal and the largest number.
        assert_eq!(cbrt(f64::from_bits(1)).to_bits(), 0x2990000000000000);
        assert_eq!(cbrt(f64::MAX).to_bits(), 0x554428a2f98d728b);
    }

    #[test]
    fn the_product_error_is_the_fused_multiply_add() {
        // `cbrt` takes y² and y³'s rounding errors from `product_error`
        // where the source calls a fused multiply-add. Over the factors
        // cbrt gives it, about 0.79 to 2.6 in size, and far past them, the
        // two agree to the bit: the error of a rounded product is a 64-bit
        // number, and each computes it exactly.
        let mut checked = 0;
        for a_bits in ladder(0.25, 16.0, 3001) {
            for k in 0..7_u64 {
                let a = f64::from_bits(a_bits);
                let b = f64::from_bits(a_bits.wrapping_mul(0x9E37_79B9).wrapping_add(k) % (1u64 << 52) | 0x3FF0_0000_0000_0000);
                for (x, y) in [(a, a), (a, b), (-a, b), (a, a * a)] {
                    let p = x * y;
                    assert_eq!(product_error(x, y, p).to_bits(), x.mul_add(y, -p).to_bits(), "{x:e} × {y:e}");
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 84_028);
    }

    /// n bit patterns equally spaced from the pattern of a to that of b.
    fn ladder(a: f64, b: f64, n: u64) -> impl Iterator<Item = u64> {
        let (lo, hi) = (a.to_bits(), b.to_bits());
        (0..n).map(move |i| lo + (i * (hi - lo)) / (n - 1))
    }

    #[test]
    fn powf_at_its_special_values_follows_the_source() {
        // pow.rs's own tests: `zero_as_exponent`, `one_as_base`, `nan_inputs`,
        // `infinity_as_base`, `infinity_as_exponent`, `zero_as_base`,
        // `special_cases` and `normal_cases`, over the source's sets.
        let e = core::f64::consts::E;
        let pi = core::f64::consts::PI;
        let pos_floats = [99.0 / 70.0, e, pi];
        let neg_floats = [-99.0 / 70.0, -e, -pi];
        let pos_small = [0.5, f64::MIN_POSITIVE, f64::EPSILON];
        let neg_small = [-0.5, -f64::MIN_POSITIVE, -f64::EPSILON];
        let pos_evens = [2.0, 6.0, 8.0, 10.0, 22.0, 100.0, f64::MAX];
        let neg_evens = [f64::MIN, -100.0, -22.0, -10.0, -8.0, -6.0, -2.0];
        let pos_odds = [3.0, 7.0];
        let neg_odds = [-7.0, -3.0];
        let same = |got: f64, want: f64| if want.is_nan() { got.is_nan() } else { got.to_bits() == want.to_bits() || got == want };
        let all: [&[f64]; 15] = [
            &[0.0],
            &[-0.0],
            &[NAN],
            &neg_small,
            &pos_small,
            &neg_floats,
            &pos_floats,
            &neg_evens,
            &pos_evens,
            &neg_odds,
            &pos_odds,
            &[-INFINITY],
            &[INFINITY],
            &[-1.0],
            &[1.0],
        ];
        for set in all {
            for &v in set {
                assert_eq!(powf(v, 0.0), 1.0);
                assert_eq!(powf(v, -0.0), 1.0);
                assert_eq!(powf(1.0, v), 1.0);
                // "anything ** 1 is anything", "anything ** -1 is 1/anything".
                assert!(same(powf(v, 1.0), v));
                assert!(same(powf(v, -1.0), 1.0 / v));
            }
        }
        for set in &all[2..] {
            for &v in *set {
                assert!(powf(NAN, v).is_nan());
            }
        }
        for set in &all[..13] {
            for &v in *set {
                assert!(powf(v, NAN).is_nan());
            }
        }
        // +inf to a positive power is +inf, to a negative one +0; -inf to an
        // odd positive whole power is -inf; -inf ** v is -0 ** (-v).
        for v in [3.0, 7.0, 1.0, 99.0 / 70.0, e, pi, 2.0, 100.0, f64::MAX, INFINITY] {
            assert_eq!(powf(INFINITY, v), INFINITY);
            assert_eq!(powf(INFINITY, -v).to_bits(), 0.0_f64.to_bits());
        }
        for v in pos_odds {
            assert_eq!(powf(-INFINITY, v), -INFINITY);
        }
        for set in all {
            for &v in set {
                assert!(same(powf(-INFINITY, v), powf(-0.0, -v)));
            }
        }
        // |x| > 1 to +inf is +inf and to -inf is 0; |x| < 1 the other way; -1 to either is 1.
        for v in [-99.0 / 70.0, -e, -pi, 99.0 / 70.0, e, pi, -2.0, 100.0, -7.0, 3.0] {
            assert_eq!(powf(v, INFINITY), INFINITY);
            assert_eq!(powf(v, -INFINITY).to_bits(), 0.0_f64.to_bits());
        }
        for v in [0.0, -0.0, -0.5, -f64::MIN_POSITIVE, -f64::EPSILON, 0.5, f64::MIN_POSITIVE, f64::EPSILON] {
            assert_eq!(powf(v, INFINITY).to_bits(), 0.0_f64.to_bits());
            assert_eq!(powf(v, -INFINITY), INFINITY);
        }
        assert_eq!(powf(-1.0, INFINITY), 1.0);
        assert_eq!(powf(-1.0, -INFINITY), 1.0);
        // Zero as the base.
        for v in [3.0, 7.0, 1.0, 99.0 / 70.0, e, pi, 2.0, 100.0, f64::MAX, INFINITY] {
            assert_eq!(powf(0.0, v).to_bits(), 0.0_f64.to_bits());
            assert_eq!(powf(0.0, -v), INFINITY);
        }
        for v in [99.0 / 70.0, e, pi, 2.0, 100.0, f64::MAX, INFINITY] {
            assert_eq!(powf(-0.0, v).to_bits(), 0.0_f64.to_bits());
            assert_eq!(powf(-0.0, -v), INFINITY);
        }
        for v in pos_odds {
            assert_eq!(powf(-0.0, v).to_bits(), (-0.0_f64).to_bits());
            assert_eq!(powf(-0.0, -v), -INFINITY);
        }
        // A negative base to a whole power is (-1)^n |x|^n; to a power that
        // is not whole, NaN.
        for n in [0.0, -0.0, 1.0, -1.0, 2.0, 6.0, 8.0, 10.0, 22.0, 100.0, f64::MAX, f64::MIN, -100.0, -22.0, -2.0] {
            for set in all {
                for &v in set {
                    assert!(same(powf(-v, n), powf(-1.0, n) * powf(v, n)), "-({v}) ** {n}");
                }
            }
        }
        for base in [-99.0 / 70.0, -e, -pi, -2.0, -100.0, -7.0, -1.0] {
            for exponent in [-0.5, -f64::MIN_POSITIVE, -f64::EPSILON, 0.5, f64::EPSILON, 99.0 / 70.0, e, -pi] {
                assert!(powf(base, exponent).is_nan(), "{base} ** {exponent}");
            }
        }
        assert_eq!(powf(2.0, 20.0), (1 << 20) as f64);
        assert_eq!(powf(-1.0, 9.0), -1.0);
        assert!(powf(-1.0, 2.2).is_nan());
        assert!(powf(-1.0, -1.14).is_nan());
        // "pow(integer,integer) always returns the correct integer provided it is representable."
        for (base, top) in [(3_u64, 33), (5, 22), (7, 18), (10, 15), (11, 15)] {
            let mut want = 1_u64;
            for k in 1..=top {
                want *= base;
                assert_eq!(powf(base as f64, k as f64), want as f64, "{base}^{k}");
            }
        }
        // y = 0.5 is the square root, and y = 2 the square.
        for row in &LN_1P {
            let x = f64::from_bits(row[0]).abs();
            assert_eq!(powf(x, 0.5).to_bits(), f64::sqrt(x).to_bits());
            assert_eq!(powf(x, 2.0).to_bits(), (x * x).to_bits());
        }
    }

    #[test]
    fn powi_is_the_run_time_product_of_f64_powi() {
        // At run time `f64::powi` multiplies by repeated squaring in the
        // order `powi` here follows: the same bits for every base and
        // exponent. black_box keeps the compiler from folding the platform's
        // call with constants.
        let exponents: [i32; 24] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15, 16, 17, 31, 64, 100, 1023, -1, -2, -3, -7, -64, -1074];
        let mut checked = 0;
        for row in SIN.iter().chain(EXP.iter()).chain(CBRT.iter()) {
            let x = f64::from_bits(row[0]);
            for n in exponents {
                let platform = core::hint::black_box(x).powi(core::hint::black_box(n));
                let ours = powi(core::hint::black_box(x), core::hint::black_box(n));
                assert!(
                    ours.to_bits() == platform.to_bits() || (ours.is_nan() && platform.is_nan()),
                    "powi({x:e}, {n})"
                );
                checked += 1;
            }
        }
        for n in [i32::MIN, i32::MAX, i32::MIN + 1, 1 << 30] {
            for x in [1.0_f64, -1.0, 1.0000000000000002, 0.9999999999999999, 2.0, 0.5, -0.0] {
                let platform = core::hint::black_box(x).powi(core::hint::black_box(n));
                assert_eq!(powi(x, n).to_bits(), platform.to_bits(), "powi({x}, {n})");
            }
        }
        assert_eq!(checked, 24 * (3117 + 3178 + 5463));
        assert_eq!(powi(NAN, 0).to_bits(), 1.0_f64.to_bits());
        assert!(powi(NAN, 3).is_nan());
        assert_eq!(powi(-2.0, 3), -8.0);
        // The method squares up to 2^1074, which overflows, and divides one by
        // infinity: zero, where 2^-1074 is a 64-bit number. f64::powi gives
        // the same, by the same method.
        assert_eq!(powi(2.0, -1074).to_bits(), 0);
        assert_eq!(powi(2.0, 1024).to_bits(), INFINITY.to_bits());
    }

    #[test]
    fn powi_of_constants_is_the_product_and_never_the_build_machine_s_pow() {
        // blast_wave_tnt_air raised 0.05 to the fifth with `f64::powi`, both
        // constants. The compiler folded that call through the pow of the
        // machine that built it, which rounds the exact 3.125e-7 correctly,
        // 0x3e94f8b588e368f2; at run time the product of squares is
        // 0x3e94f8b588e368f4. Here the constants are folded, if at all, by
        // the same multiplications, so the build gives the run-time bits.
        assert_eq!(powi(0.05, 5).to_bits(), 0x3e94f8b588e368f4);
        assert_eq!(powi(core::hint::black_box(0.05), core::hint::black_box(5)).to_bits(), 0x3e94f8b588e368f4);
        assert_eq!(powi(1.5, 2), 2.25);
        assert_eq!(powi(10.0, 22), 1e22);
        assert_eq!(powi(10.0, -1), 0.1);
    }
}
