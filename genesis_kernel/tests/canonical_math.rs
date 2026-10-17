//! The served code calls no method of the platform's math library.
//!
//! A card's step gives the same 64 bits on every machine that follows IEEE
//! 754. The platform's libraries round `exp`, `ln`, `powf`, `sin` and the
//! rest each its own way (Apple's libm, glibc on arm64, glibc on x86_64 with
//! FMA and without), and the compiler folds a call whose arguments are
//! constants through the library of the machine that builds it. So the cards
//! call `crate::math` for each of them, and this test reads every file of
//! `src/` and refuses a call of a platform method:
//! `x.exp()` or `f64::exp(x)` and the rest of `PLATFORM`, outside comments,
//! strings and items marked `#[cfg(test)]`, and outside the files a
//! `#[cfg(test)] mod` brings in. `sqrt` and `mul_add` are allowed: IEEE 754
//! fixes both to the last bit.
//!
//! The same check, by type and not by name, is Clippy's `disallowed-methods`
//! (`clippy.toml`), from the workspace:
//!
//! ```text
//! cargo clippy --release -p genesis_kernel -p genesis_solvers -p protocol_mcp_gateway --lib --bins -- -A clippy::all -D clippy::disallowed_methods
//! ```

use std::fs;
use std::path::{Path, PathBuf};

/// The methods of `f64` that call the platform's math library.
const PLATFORM: [&str; 26] = [
    "exp", "exp2", "exp_m1", "ln", "log", "log2", "log10", "ln_1p", "powf", "powi", "sin", "cos", "tan", "sin_cos",
    "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "asinh", "acosh", "atanh", "cbrt", "hypot",
];

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// The bytes of a UTF-8 character whose first byte is `b`.
fn utf8_len(b: u8) -> usize {
    match b {
        0xF0..=0xF7 => 4,
        0xE0..=0xEF => 3,
        0xC0..=0xDF => 2,
        _ => 1,
    }
}

/// Where a comment, string or character literal that starts at `i` ends,
/// if one starts there.
fn literal_end(t: &[u8], i: usize) -> Option<usize> {
    let at = |k: usize| t.get(k).copied().unwrap_or(0);
    if at(i) == b'/' && at(i + 1) == b'/' {
        return Some(t[i..].iter().position(|&b| b == b'\n').map_or(t.len(), |n| i + n));
    }
    if at(i) == b'/' && at(i + 1) == b'*' {
        let (mut depth, mut j) = (1, i + 2);
        while j < t.len() && depth > 0 {
            if at(j) == b'/' && at(j + 1) == b'*' {
                depth += 1;
                j += 2;
            } else if at(j) == b'*' && at(j + 1) == b'/' {
                depth -= 1;
                j += 2;
            } else {
                j += 1;
            }
        }
        return Some(j);
    }
    // A raw string: r"..", r#".."#, br"..".
    if (at(i) == b'r' || (at(i) == b'b' && at(i + 1) == b'r')) && (i == 0 || !is_ident(t[i - 1])) {
        let mut j = if at(i) == b'b' { i + 2 } else { i + 1 };
        let hashes = t[j..].iter().take_while(|&&b| b == b'#').count();
        j += hashes;
        if at(j) == b'"' {
            let mut k = j + 1;
            while k < t.len() {
                if t[k] == b'"' && t[k + 1..].iter().take(hashes).filter(|&&b| b == b'#').count() == hashes {
                    return Some(k + 1 + hashes);
                }
                k += 1;
            }
            return Some(t.len());
        }
    }
    if at(i) == b'"' {
        let mut j = i + 1;
        while j < t.len() {
            match t[j] {
                b'\\' => j += 2,
                b'"' => return Some(j + 1),
                _ => j += 1,
            }
        }
        return Some(t.len());
    }
    if at(i) == b'\'' {
        // A character literal, not a lifetime.
        let j = if at(i + 1) == b'\\' {
            match at(i + 2) {
                b'x' => i + 5,
                b'u' => t[i..].iter().position(|&b| b == b'}').map_or(t.len(), |n| i + n + 1),
                _ => i + 3,
            }
        } else {
            i + 1 + utf8_len(at(i + 1))
        };
        if at(j) == b'\'' {
            return Some(j + 1);
        }
    }
    None
}

/// The end of the item that starts at `i`: the `;` at its own depth, or the
/// brace that closes its first `{`.
fn item_end(t: &[u8], mut i: usize) -> usize {
    let mut depth = 0i32;
    while i < t.len() {
        if let Some(j) = literal_end(t, i) {
            i = j;
            continue;
        }
        match t[i] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth -= 1,
            b';' if depth == 0 => return i + 1,
            b'{' => {
                let mut level = 1;
                i += 1;
                while i < t.len() && level > 0 {
                    if let Some(j) = literal_end(t, i) {
                        i = j;
                        continue;
                    }
                    match t[i] {
                        b'{' => level += 1,
                        b'}' => level -= 1,
                        _ => {}
                    }
                    i += 1;
                }
                return i;
            }
            _ => {}
        }
        i += 1;
    }
    i
}

/// The source with every comment, string, character literal and item marked
/// `#[cfg(test)]` (or `#[cfg(all(test, ..))]`) written as spaces.
fn served(text: &str) -> Vec<u8> {
    let t = text.as_bytes();
    let mut out = t.to_vec();
    let blank = |out: &mut Vec<u8>, a: usize, b: usize| {
        for byte in &mut out[a..b.min(t.len())] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    };
    let mut i = 0;
    while i < t.len() {
        if let Some(j) = literal_end(t, i) {
            blank(&mut out, i, j);
            i = j;
            continue;
        }
        if t[i..].starts_with(b"#[cfg(test)") || t[i..].starts_with(b"#[cfg(all(test") {
            let close = i + t[i..].iter().position(|&b| b == b']').unwrap();
            let end = item_end(t, close + 1);
            blank(&mut out, i, end);
            i = end;
            continue;
        }
        i += 1;
    }
    out
}

/// The modules a file brings in under `#[cfg(test)]`: `#[cfg(test)] mod name;`.
fn test_modules(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("#[cfg(test)]") {
        rest = &rest[at + "#[cfg(test)]".len()..];
        let mut after = rest.trim_start();
        while after.starts_with("#[") {
            after = after[after.find(']').unwrap() + 1..].trim_start();
        }
        let after = after.strip_prefix("pub(crate) ").or_else(|| after.strip_prefix("pub ")).unwrap_or(after);
        if let Some(declared) = after.strip_prefix("mod ") {
            let name: String = declared.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
            if declared[name.len()..].trim_start().starts_with(';') {
                names.push(name);
            }
        }
    }
    names
}

/// Each call of a platform method in served text, as (line, name).
fn platform_calls(served: &[u8]) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for i in 0..served.len() {
        let dotted = served[i] == b'.' && (i == 0 || served[i - 1] != b'.');
        let pathed = served[i..].starts_with(b"f64::") && (i == 0 || !is_ident(served[i - 1]));
        if !dotted && !pathed {
            continue;
        }
        let start = if dotted { i + 1 } else { i + 5 };
        let len = served[start..].iter().take_while(|&&b| is_ident(b)).count();
        let name = std::str::from_utf8(&served[start..start + len]).unwrap();
        let mut next = start + len;
        if served.get(next) == Some(&b':') {
            // A turbofish: `.powi::<..>(`; none is expected, so it is a call.
            next = served[next..].iter().position(|&b| b == b'(').map_or(served.len(), |n| next + n);
        }
        if PLATFORM.contains(&name) && served.get(next) == Some(&b'(') {
            let line = served[..i].iter().filter(|&&b| b == b'\n').count() + 1;
            found.push((line, name.to_string()));
        }
    }
    found
}

fn rust_files(folder: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Every platform call in the served code of `src/`, as "file:line name".
fn platform_calls_in(src: &Path) -> (usize, Vec<String>) {
    let mut files = Vec::new();
    rust_files(src, &mut files);
    files.sort();
    let mut test_only = Vec::new();
    for path in &files {
        test_only.extend(test_modules(&fs::read_to_string(path).unwrap()));
    }
    let mut scanned = 0;
    let mut calls = Vec::new();
    for path in &files {
        let stem = path.file_stem().unwrap().to_str().unwrap();
        if test_only.iter().any(|name| name == stem) {
            continue;
        }
        scanned += 1;
        for (line, name) in platform_calls(&served(&fs::read_to_string(path).unwrap())) {
            calls.push(format!("{}:{line} {name}", path.strip_prefix(src).unwrap().display()));
        }
    }
    (scanned, calls)
}

#[test]
fn served_code_calls_no_method_of_the_platform_s_math_library() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let (scanned, calls) = platform_calls_in(&src);
    assert!(scanned >= 40, "{scanned} files scanned");
    assert!(
        calls.is_empty(),
        "platform math in served code; call crate::math instead:\n{}",
        calls.join("\n")
    );
}

#[test]
fn the_scan_finds_each_platform_call_and_nothing_else() {
    // The attack on the check itself: a call in served code is found in each
    // form; a call in a comment, a string, a test module, a character
    // literal's neighbourhood or a path of the crate's own is not; and a
    // file a test module brings in is passed over.
    let text = r##"
use genesis_kernel::math;
pub fn served(x: f64) -> f64 {
    let a = x.exp() + (x * 2.0).ln() + f64::powf(x, 0.5) + 2.0_f64.powi(3);
    let b = x
        .sin();
    let c = math::exp(x) + x.sqrt() + x.mul_add(2.0, 1.0) + Complex::exp(z).re;
    let d = '\''; let e = 'x'; let f: &'static str = "y.cos()";
    // x.tan() in a comment
    /* x.atan2(y) /* nested */ x.cbrt() */
    let g = r#"x.hypot(y)"#;
    a + b + c + x.atan2(y) + x.0.cosh() + v[0].tanh() + x.sin_cos().0
}
#[cfg(test)]
mod tests {
    fn t(x: f64) -> f64 { x.exp() + x.sinh() }
}
#[cfg(all(test, feature = "x"))]
fn u(x: f64) -> f64 { x.asinh() }
#[cfg(test)]
mod workshop;
"##;
    let calls: Vec<String> = platform_calls(&served(text)).into_iter().map(|(line, name)| format!("{line} {name}")).collect();
    assert_eq!(calls, ["4 exp", "4 ln", "4 powf", "4 powi", "6 sin", "12 atan2", "12 cosh", "12 tanh", "12 sin_cos"]);
    assert_eq!(test_modules(text), ["workshop"]);
}
