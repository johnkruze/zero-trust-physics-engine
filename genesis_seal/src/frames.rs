//! The frame file of a dataset, `frames.bin`.
//!
//! A 64-byte header, then the 64-byte frames of every row in row order. Each
//! row's frames are chained from that row's own `link_0`. The file does not
//! say where one row ends: the count of frames of each row is in the rows
//! file.
//!
//! The header is stated byte by byte in this crate's `AGENTS.md`, under
//! "The frame file". Whole numbers are little-endian.

use std::fmt;
use std::io::{self, Read, Write};

use crate::run::FRAME_LEN;

/// The first four bytes of a frame file.
pub const MAGIC: [u8; 4] = *b"GKF2";
/// The version of the layout this crate reads and writes.
pub const VERSION: u32 = 1;
/// The length of the header, in bytes.
pub const HEADER_LEN: usize = 64;

/// The header of a frame file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramesHeader {
    pub rows: u64,
    pub frames: u64,
    pub dataset_seal: [u8; 32],
}

/// Why a frame file was not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FramesRefusal {
    /// The file is shorter than its header.
    NoHeader,
    /// The file does not start with `GKF2`.
    NotAFrameFile,
    /// The file states a version this crate does not read.
    UnknownVersion(u32),
    /// The unused bytes of the header are not zero.
    UnusedBytesSet,
    /// The bytes after the header are not the stated count of 64-byte frames.
    WrongLength { stated_frames: u64, bytes_after_header: u64 },
    /// The file could not be read.
    NotRead(String),
}

impl fmt::Display for FramesRefusal {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FramesRefusal::NoHeader => {
                write!(out, "The frame file is shorter than its 64-byte header.")
            }
            FramesRefusal::NotAFrameFile => {
                write!(out, "The frame file does not start with GKF2.")
            }
            FramesRefusal::UnknownVersion(version) => write!(
                out,
                "The frame file states version {version}. This reader reads version {VERSION}."
            ),
            FramesRefusal::UnusedBytesSet => write!(
                out,
                "Bytes 56 to 63 of the frame file's header are not zero."
            ),
            FramesRefusal::WrongLength {
                stated_frames,
                bytes_after_header,
            } => write!(
                out,
                "The frame file states {stated_frames} frames and holds {bytes_after_header} bytes after its header."
            ),
            FramesRefusal::NotRead(reason) => {
                write!(out, "The frame file could not be read: {reason}")
            }
        }
    }
}

impl FramesHeader {
    pub fn to_bytes(&self) -> [u8; HEADER_LEN] {
        let mut bytes = [0_u8; HEADER_LEN];
        bytes[0..4].copy_from_slice(&MAGIC);
        bytes[4..8].copy_from_slice(&VERSION.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.rows.to_le_bytes());
        bytes[16..24].copy_from_slice(&self.frames.to_le_bytes());
        bytes[24..56].copy_from_slice(&self.dataset_seal);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<FramesHeader, FramesRefusal> {
        if bytes.len() < HEADER_LEN {
            return Err(FramesRefusal::NoHeader);
        }
        if bytes[0..4] != MAGIC {
            return Err(FramesRefusal::NotAFrameFile);
        }
        let version = u32::from_le_bytes(four(&bytes[4..8]));
        if version != VERSION {
            return Err(FramesRefusal::UnknownVersion(version));
        }
        if bytes[56..64].iter().any(|byte| *byte != 0) {
            return Err(FramesRefusal::UnusedBytesSet);
        }
        let mut dataset_seal = [0_u8; 32];
        dataset_seal.copy_from_slice(&bytes[24..56]);
        Ok(FramesHeader {
            rows: u64::from_le_bytes(eight(&bytes[8..16])),
            frames: u64::from_le_bytes(eight(&bytes[16..24])),
            dataset_seal,
        })
    }
}

fn four(bytes: &[u8]) -> [u8; 4] {
    let mut out = [0_u8; 4];
    out.copy_from_slice(bytes);
    out
}

fn eight(bytes: &[u8]) -> [u8; 8] {
    let mut out = [0_u8; 8];
    out.copy_from_slice(bytes);
    out
}

/// A frame file held as one slice of bytes.
#[derive(Debug, Clone, Copy)]
pub struct FramesFile<'a> {
    pub header: FramesHeader,
    frames: &'a [u8],
}

impl<'a> FramesFile<'a> {
    /// Read the header and require the stated count of frames after it.
    pub fn read(bytes: &'a [u8]) -> Result<FramesFile<'a>, FramesRefusal> {
        let header = FramesHeader::from_bytes(bytes)?;
        let frames = &bytes[HEADER_LEN..];
        let stated = header.frames.checked_mul(FRAME_LEN as u64);
        if stated != Some(frames.len() as u64) {
            return Err(FramesRefusal::WrongLength {
                stated_frames: header.frames,
                bytes_after_header: frames.len() as u64,
            });
        }
        Ok(FramesFile { header, frames })
    }

    /// Frame `index`, counted from 0 over the whole file.
    pub fn frame(&self, index: u64) -> Option<&'a [u8]> {
        if index >= self.header.frames {
            return None;
        }
        let start = (index as usize).checked_mul(FRAME_LEN)?;
        self.frames.get(start..start + FRAME_LEN)
    }
}

/// Write the header, then every frame in the order given.
pub fn write_frames<W: Write>(
    out: &mut W,
    header: &FramesHeader,
    frames: &[[u8; FRAME_LEN]],
) -> io::Result<()> {
    out.write_all(&header.to_bytes())?;
    for frame in frames {
        out.write_all(frame)?;
    }
    Ok(())
}

/// Read a whole frame file from `input`.
pub fn read_frames<R: Read>(
    input: &mut R,
) -> Result<(FramesHeader, Vec<[u8; FRAME_LEN]>), FramesRefusal> {
    let mut bytes = Vec::new();
    input
        .read_to_end(&mut bytes)
        .map_err(|error| FramesRefusal::NotRead(error.to_string()))?;
    let file = FramesFile::read(&bytes)?;
    let mut frames = Vec::new();
    for chunk in file.frames.chunks_exact(FRAME_LEN) {
        let mut frame = [0_u8; FRAME_LEN];
        frame.copy_from_slice(chunk);
        frames.push(frame);
    }
    Ok((file.header, frames))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> FramesHeader {
        FramesHeader {
            rows: 3,
            frames: 2,
            dataset_seal: [0xab; 32],
        }
    }

    #[test]
    fn the_header_is_64_bytes_laid_out_as_stated() {
        let bytes = header().to_bytes();
        assert_eq!(&bytes[0..4], b"GKF2");
        assert_eq!(bytes[4..8], [1, 0, 0, 0]);
        assert_eq!(bytes[8..16], [3, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(bytes[16..24], [2, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(bytes[24..56], [0xab; 32]);
        assert_eq!(bytes[56..64], [0; 8]);
        assert_eq!(FramesHeader::from_bytes(&bytes), Ok(header()));
        let large = FramesHeader {
            rows: 0x0102_0304_0506_0708,
            frames: 16_777_216,
            dataset_seal: [0; 32],
        };
        assert_eq!(large.to_bytes()[8..16], [8, 7, 6, 5, 4, 3, 2, 1]);
        assert_eq!(large.to_bytes()[16..24], [0, 0, 0, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn a_frame_file_is_written_and_read_back() {
        let frames = [[1_u8; FRAME_LEN], [2_u8; FRAME_LEN]];
        let mut bytes = Vec::new();
        write_frames(&mut bytes, &header(), &frames).unwrap();
        assert_eq!(bytes.len(), 64 + 128);
        let file = FramesFile::read(&bytes).unwrap();
        assert_eq!(file.header, header());
        assert_eq!(file.frame(0), Some(&frames[0][..]));
        assert_eq!(file.frame(1), Some(&frames[1][..]));
        assert_eq!(file.frame(2), None);
        let (read_header, read) = read_frames(&mut bytes.as_slice()).unwrap();
        assert_eq!(read_header, header());
        assert_eq!(read, frames);
    }

    #[test]
    fn a_malformed_frame_file_is_refused_by_name() {
        let frames = [[1_u8; FRAME_LEN], [2_u8; FRAME_LEN]];
        let mut good = Vec::new();
        write_frames(&mut good, &header(), &frames).unwrap();

        assert_eq!(FramesFile::read(&good[..63]).unwrap_err(), FramesRefusal::NoHeader);
        let mut bytes = good.clone();
        bytes[3] = b'1';
        assert_eq!(FramesFile::read(&bytes).unwrap_err(), FramesRefusal::NotAFrameFile);
        let mut bytes = good.clone();
        bytes[4] = 2;
        assert_eq!(FramesFile::read(&bytes).unwrap_err(), FramesRefusal::UnknownVersion(2));
        let mut bytes = good.clone();
        bytes[60] = 1;
        assert_eq!(FramesFile::read(&bytes).unwrap_err(), FramesRefusal::UnusedBytesSet);
        let mut bytes = good.clone();
        bytes.pop();
        assert_eq!(
            FramesFile::read(&bytes).unwrap_err(),
            FramesRefusal::WrongLength { stated_frames: 2, bytes_after_header: 127 }
        );
        let mut bytes = good.clone();
        bytes.extend_from_slice(&[0; FRAME_LEN]);
        assert!(matches!(
            FramesFile::read(&bytes).unwrap_err(),
            FramesRefusal::WrongLength { .. }
        ));
        let mut bytes = good;
        bytes[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(matches!(
            FramesFile::read(&bytes).unwrap_err(),
            FramesRefusal::WrongLength { .. }
        ));
    }
}
