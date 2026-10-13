#![no_implicit_prelude]

//! Sine, cosine, arcsine, hypotenuse, exponential, exponential minus one, and
//! natural logarithm, carried in this crate.
//!
//! `core` only: no heap and no standard prelude. The cards call these seven
//! functions and no method of the platform's math library, so a card's result
//! does not depend on which library a machine has.
//!
//! Every function here is built from addition, subtraction, multiplication,
//! division, comparison, conversion between a 64-bit number and an integer,
//! integer operations on the 64-bit pattern, and `f64::sqrt`. IEEE 754 fixes
//! the result of each of those to the last bit, so the seven functions return
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
//! arithmetic on its own. The optimised arm64 code of this crate holds no
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
//!
//! `log.rs` was not downloaded. It was read on 2026-10-08 from the copy of
//! the crate at version 0.2.16 that Cargo keeps on this machine
//! (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libm-0.2.16/`).
//! In that copy `src/math/exp.rs` and `LICENSE.txt` have the SHA-256 this
//! table gives for them.
//!
//! Ten of the routines are the Sun Microsystems routines of FreeBSD's
//! `msun`, as the crate carries them: `sin`, `cos`, `k_sin`, `k_cos`,
//! `rem_pio2`, `rem_pio2_large`, `asin`, `exp`, `exp_m1` and `log`. Each keeps
//! its notice, word for word, above its code.
//!
//! `hypot`, `scalbn`, `floor` and `fabs` carry no copyright notice of their
//! own in the source. `hypot` and `floor` came to the crate from musl, and
//! musl's notice stands above `hypot`. `scalbn` and `fabs` were written for
//! the crate. The crate's licence, below, covers all four.
//!
//! Three more files were read and no code was taken from them:
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
//!
//! # Accuracy
//!
//! The tests compare each function with correctly rounded values at fixed
//! arguments and require agreement within one unit in the last place. The
//! arguments, the rule that chose them and the script that produced the
//! correctly rounded values are named in the test module.
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
//! No CORE-MATH routine is carried here.

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
    //! Each function must land within one unit in the last place of every
    //! value. Each test also pins how many values the function hits exactly
    //! and a digest of every result bit. Both are properties of the function
    //! and not of the machine, so on a target where a result differs in any
    //! bit the digest test fails.

    extern crate core;
    extern crate std;

    use core::prelude::v1::*;
    use std::assert;
    use std::assert_eq;

    use super::{asin, cos, exp, exp_m1, floor, hypot, log, rem_pio2, scalbn, sin};

    std::include!("math_vectors.rs");
    std::include!("math_vectors_log.rs");

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
}
