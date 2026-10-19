pub mod atom_dft;
pub mod pica;
mod tacot;
mod wall_gas;
mod wall_gas_species;
pub mod ablator_slab;
pub mod propeller_apc_10x7e;
pub mod seawater_column;
pub mod argon_pair_gas;
pub mod culture_trichoderma_ambr250;
#[cfg(test)]
mod workshop_series_2;
pub mod fusion_pb11_lhd;
pub mod fusion_pb11_thermal;
pub mod journal_bearing_sheffield;
pub mod modulus_bam_s355;
pub mod pb11_reaction;
pub mod step_flow_armaly;
pub mod pipe_flow_superpipe;
pub mod fuel_rod_triga_jsi;
pub mod culture_trichoderma_atp;
pub mod normal;
pub mod blast_wave_tnt_air;
pub mod plane_stress_fe;
pub mod stress_plate_hole_s235;
pub mod stress_plate_hole_s235_elastic;
pub mod air11;
pub mod plasma_sheath_ram_c2;

/// This crate's version, for a caller that stamps results.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub mod bosch_hale;
pub mod tokamak_dt_beam_target;
#[cfg(test)]
mod tokamak_dt_beam_target_reference;
pub mod junction_nist_b7_s2;
pub mod line_transient;
pub mod water_hammer_oil_line;
pub mod orifice_air_chamber;
pub mod pneumatic_hose_resonance;
pub mod icf_hotspot_burn;
pub mod ln2_nitrogen;
pub mod ln2_line_dlr_fttf;
