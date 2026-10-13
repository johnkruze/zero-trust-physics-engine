/// File frame envelope. Matches Python `'<d6f2fQ16s'`.
/// Slot names depend on `body_id` (see SOMA.md).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LastStateFrame64 {
    pub t: f64,
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub force_torque: f32,
    pub residual: f32,
    pub flags: u64,
    pub proof: [u8; 16],
}

const _: () = {
    assert!(core::mem::size_of::<LastStateFrame64>() == 64);
    assert!(core::mem::offset_of!(LastStateFrame64, flags) == 40);
    assert!(core::mem::offset_of!(LastStateFrame64, proof) == 48);
};

impl LastStateFrame64 {
    pub fn to_bytes(self) -> [u8; 64] {
        unsafe { core::mem::transmute(self) }
    }

    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        unsafe { core::mem::transmute(bytes) }
    }
}

/// One slot of the frame: its name, the byte it starts at, its width in
/// bytes, and its type. The numbers are the struct's own layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSlotLayout {
    pub slot: &'static str,
    pub offset: usize,
    pub width: usize,
    /// `"f64"`, `"f32"`, `"u64"` or `"bytes"`. Numbers are little-endian.
    pub kind: &'static str,
}

const fn slot(slot: &'static str, offset: usize, width: usize, kind: &'static str) -> FrameSlotLayout {
    FrameSlotLayout {
        slot,
        offset,
        width,
        kind,
    }
}

/// Every slot of the 64-byte frame, in byte order, from the struct's own
/// offsets and sizes.
pub const FRAME_LAYOUT: [FrameSlotLayout; 11] = {
    let pos = core::mem::offset_of!(LastStateFrame64, pos);
    let vel = core::mem::offset_of!(LastStateFrame64, vel);
    let f32_width = core::mem::size_of::<f32>();
    [
        slot("t", core::mem::offset_of!(LastStateFrame64, t), core::mem::size_of::<f64>(), "f64"),
        slot("pos[0]", pos, f32_width, "f32"),
        slot("pos[1]", pos + f32_width, f32_width, "f32"),
        slot("pos[2]", pos + 2 * f32_width, f32_width, "f32"),
        slot("vel[0]", vel, f32_width, "f32"),
        slot("vel[1]", vel + f32_width, f32_width, "f32"),
        slot("vel[2]", vel + 2 * f32_width, f32_width, "f32"),
        slot("force_torque", core::mem::offset_of!(LastStateFrame64, force_torque), f32_width, "f32"),
        slot("residual", core::mem::offset_of!(LastStateFrame64, residual), f32_width, "f32"),
        slot("flags", core::mem::offset_of!(LastStateFrame64, flags), core::mem::size_of::<u64>(), "u64"),
        slot("proof", core::mem::offset_of!(LastStateFrame64, proof), 16, "bytes"),
    ]
};

#[cfg(test)]
mod tests {
    use super::*;

    /// The table covers the 64 bytes once, in order, and reads back what a
    /// frame holds at each offset.
    #[test]
    fn the_layout_covers_the_frame_once_and_reads_back_a_frame() {
        let mut end = 0;
        for row in FRAME_LAYOUT {
            assert_eq!(row.offset, end, "{}", row.slot);
            end += row.width;
        }
        assert_eq!(end, 64);
        let frame = LastStateFrame64 {
            t: 1.25,
            pos: [1.0, 2.0, 3.0],
            vel: [4.0, 5.0, 6.0],
            force_torque: 7.0,
            residual: 8.0,
            flags: 9,
            proof: [10; 16],
        };
        let bytes = frame.to_bytes();
        let f32_at = |row: &FrameSlotLayout| {
            f32::from_le_bytes(bytes[row.offset..row.offset + row.width].try_into().unwrap())
        };
        let by_name = |name: &str| FRAME_LAYOUT.iter().find(|row| row.slot == name).unwrap();
        let t = by_name("t");
        assert_eq!(f64::from_le_bytes(bytes[t.offset..t.offset + 8].try_into().unwrap()), 1.25);
        for (name, value) in [
            ("pos[0]", 1.0),
            ("pos[1]", 2.0),
            ("pos[2]", 3.0),
            ("vel[0]", 4.0),
            ("vel[1]", 5.0),
            ("vel[2]", 6.0),
            ("force_torque", 7.0),
            ("residual", 8.0),
        ] {
            assert_eq!(f32_at(by_name(name)), value, "{name}");
        }
        let flags = by_name("flags");
        assert_eq!(u64::from_le_bytes(bytes[flags.offset..flags.offset + 8].try_into().unwrap()), 9);
        let proof = by_name("proof");
        assert_eq!(bytes[proof.offset..proof.offset + proof.width], [10u8; 16]);
    }
}
