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

/// This crate's version, for a caller that stamps results.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub mod bosch_hale;
pub mod tokamak_dt_beam_target;
#[cfg(test)]
mod tokamak_dt_beam_target_reference;
