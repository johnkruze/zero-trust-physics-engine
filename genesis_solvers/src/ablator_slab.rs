//! A slab of charring heat-shield material heated on one face: heat
//! conduction, resin that decomposes one way, the decomposition gas flowing
//! out through the char, and, under a gas stream, a face that finds its own
//! temperature, radiates and wears away.
//!
//! # The entry
//!
//! One function for a caller: [`answer`]. It takes a material by name, a
//! [`SlabQuestion`] (thickness, start temperature, what is done to the face,
//! thermocouple depths) and a [`Wanted`] (what to return beside the summary).
//! It returns an [`Answer`] or a refusal by name, [`SlabReject`]. The
//! [`Summary`] always comes back: wear, peak face temperature and when, depth
//! of the heat-affected material at two thresholds, the heat and mass
//! accounts with how they close, which envelope compared with measurement
//! the question lies in ([`Summary::inside_arc_jet_comparison`],
//! [`Summary::inside_stardust_comparison`],
//! [`Summary::inside_thermocouple_block_comparison`]), and the hardest step's
//! Newton passes. Histories and the final profile come back when asked for
//! by name, thinned to [`HISTORY_POINTS_MAX`] and [`PROFILE_POINTS_MAX`]
//! points. A thermocouple's record ends where the face passes it.
//!
//! Two more for a caller that wants to know before asking:
//! [`check_question`] decides, before anything is computed, whether a
//! question for a material is inside the declared range, and gives the same
//! refusal [`answer`] would; [`stream_pressure_range_pa`] gives a material's
//! wall-pressure range by name. [`MATERIAL_NAMES`] lists the names.
//!
//! What is done to the face is one of two things:
//!
//! - **A held temperature.** The face is taken to a given temperature in a
//!   straight line over 0.1 s and held there. After the hold it is returned
//!   to the start temperature over 0.1 s and kept there for as long as the
//!   caller asks. The face does not wear.
//! - **A gas stream.** The caller gives a heating history: rows of time,
//!   wall pressure, heat-transfer coefficient, flow enthalpy and radiative
//!   heating, with straight lines between rows, each row a millisecond or
//!   more after the one before ([`HISTORY_ROW_SPAN_MIN_S`]); and what the
//!   stream is made of ([`StreamGas`]: oxygen and argon by mass, the rest
//!   nitrogen; for air, [`StreamGas::AIR`]). The face is heated by the
//!   stream, radiates to its surroundings, and loses char at the rate the
//!   material's surface table gives for that gas, so it moves inward.
//!
//! # The stream's gas
//!
//! How fast a carbon face burns depends on how much oxygen the stream
//! carries. An arc jet's stream is not air: NASA's are air with 8 to 28
//! percent argon, or nitrogen with a stated share of oxygen.
//!
//! **Before this change the entry could only represent an air stream.** It
//! held one surface table, for air, and had no input for the gas. The 36
//! arc-jet tests the PICA model was developed on were run by a study
//! program that handed the solver a table for each test's own gas, so a
//! caller of the entry could not run those tests as they were developed:
//! asked as air, their wear came out 9.6 percent high on average where the
//! record is 4.9 percent low. The model is version five, unchanged; it has
//! been given the input it always needed.
//!
//! For air the locked table is read, and every air answer has the bits it
//! had. For any other gas the same table is computed when the question
//! arrives, by chemical equilibrium of the wall gas with solid carbon from
//! NASA's open thermodynamic records (`wall_gas.rs`, `wall_gas_species.rs`):
//! the calculation that made the locked table, moved into the solver. It is
//! computed at the lowest and the highest pressure of the history and at
//! every pressure of the locked table between them, so a test at one
//! pressure is computed at that pressure. A table costs 0.08 s for each
//! pressure and is kept for the next question about the same stream.
//!
//! Held against the surface work's own program: its tables for the 18
//! arc-jet streams are reproduced to 5 × 10⁻¹⁵ in the char's share of the
//! wall gas and 10⁻⁵ J/kg in the wall-gas enthalpies, and the 36
//! development tests asked through this entry return the recorded wear to
//! 3 parts in 10¹⁵ and the recorded peak face temperature to 4 × 10⁻¹² K.
//! The locked air table's twenty main pressures are reproduced the same
//! way. The one place the two differ is a handful of cold
//! states (below 475 K with about a tenth as much decomposition gas as
//! stream) where the method does not converge in either and the value is
//! carried from the nearest temperature that did; they differ there by 25
//! to 90 kJ/kg, which no test result shows.
//!
//! An arc-jet test becomes a question by the rule the model was developed
//! with: heat-transfer coefficient = cold-wall heating / (flow enthalpy −
//! [`stream_enthalpy_at_cold_wall_j_per_kg`]). The recipe is written out in
//! `evidence/research/stardust_heatshield_model_run/arcjet_test_to_question/`.
//!
//! # The materials, and where every number comes from
//!
//! A caller supplies no material number. Each material is a file of locked
//! data that names the origin of every number.
//!
//! - **[`MATERIAL_TACOT`]**, `tacot.rs`: TACOT version 3.0 (`TACOT_3.0.xls`,
//!   Lachaud, van Eekelen, Bianchi and Martin, [`MATERIAL_URL`]) with its
//!   published surface table in air. TACOT is a made-up material of the same
//!   family as PICA, for comparing codes in the open. It is not PICA: its
//!   conductivity is more than twice PICA's. Nothing in it is fitted.
//! - **[`MATERIAL_PICA_SERVED`]**, which is [`MATERIAL_PICA_VERSION_5`],
//!   `pica.rs`: **the PICA model a caller is pointed to.** A stand-in for
//!   PICA, whose own property model is export controlled. What it is made
//!   of:
//!   - PICA's measured density (274 kg/m³) and room-temperature virgin
//!     conductivity, with TACOT's shape against temperature;
//!   - PICA's measured heat capacity, one curve for both states, and the
//!     measured shape of its char conductivity against temperature (NASA
//!     TM-110440, 1997), carried from 240 to 274 kg/m³;
//!   - conductivity that falls with pressure by what Monk and others 2022
//!     measured;
//!   - TACOT's two decomposition reactions and decomposition gas, with
//!     neither reaction below 556 K, as in NASA's own 1997 PICA input;
//!   - char at 78.6 percent of the virgin density (215.3 kg/m³), which is
//!     also PICA's measured mass remaining (78.5 percent);
//!   - a surface table at 23 pressures computed from NASA's open
//!     thermodynamic file, locked for air and computed on demand for any
//!     other stream of oxygen, nitrogen and argon (see "The stream's gas"),
//!     with the low-pressure rule (Milos, Chen and
//!     Gokcen 2012), oxidation limited by the wall reaction (Park's
//!     probability for molecules, 0.9 for atoms after Driver and others
//!     2010), and 5 percent of the char lost as solid. Carbon boiling
//!     limited by evaporation is in the code and off: the arc-jet tests are
//!     slightly worse with it;
//!   - at or below 7 kPa the decomposition gas does not burn in oxygen the
//!     wall reaction leaves unused (the low-pressure rule applied to the
//!     whole wall gas; `GasBesideUnusedOxygen` in this file).
//!
//!   **One number is fitted: the measured char conductivity times 0.71**,
//!   fitted to the thermocouple records of four arc-jet blocks.
//! - **[`MATERIAL_PICA_VERSION_4`]**: version 5 without its last item. In a
//!   cold thin flow late in an entry it burns the decomposition gas at the
//!   wall and heats the face by about twenty times what the flow gives.
//! - **[`MATERIAL_PICA_VERSION_3`]**: TACOT's heat capacity and the shape of
//!   TACOT's char conductivity times 0.745 (fitted), decomposition from
//!   333 K, and the same surface as version 4.
//! - **[`MATERIAL_PICA`]**: the first PICA version, version 3 without the
//!   pressure dependence of conductivity.
//!
//! Versions 4, 3 and 1 are kept so that the recorded Stardust runs can be
//! made again, bit for bit.
//!
//! The equations inside the slab are Moyer and Rindal, NASA CR-1061 (1968),
//! equations (1) to (39); at the face, the balance of the Ablation Workshop's
//! second series; the surface pieces are specified in
//! `evidence/research/heatshield_surface_model/SURFACE_MODEL.md`.
//!
//! # What is in the model
//!
//! - **Conduction.** `∂/∂x (k ∂T/∂x)`. Specific heat and conductivity are
//!   mixed from the virgin and char tables by the mass fraction of virgin
//!   material, `τ = ρ_v (ρ − ρ_c) / (ρ (ρ_v − ρ_c))`.
//! - **Decomposition.** Two parts of the resin, each obeying
//!   `∂ρ_i/∂t = −A_i ρ_vi ((ρ_i − ρ_ci)/ρ_vi)³ exp(−(E_i/R)/T)` above its
//!   threshold temperature. What is left of each part is stored in every
//!   cell and only ever goes down, whether the cell is heating or cooling.
//!   The density is computed from what is left, never from the temperature.
//! - **The gas.** Gas appears where the resin decomposes, at the rate the
//!   solid loses mass, and passes at once toward the heated face (the back is
//!   closed). At each depth it is at the solid's temperature.
//! - **Energy.** `∂(ρh)/∂t = ∂/∂x (k ∂T/∂x) + ∂(ṁ_g h_g)/∂x`, which is the
//!   same equation as
//!   `ρ c_p ∂T/∂t = ∂/∂x (k ∂T/∂x) + (h_g − h̄) ∂ρ/∂t + ṁ_g ∂h_g/∂x`:
//!   heat stored, conduction, the heat of decomposition, and the heat the gas
//!   takes up on its way out. Both gas terms are in.
//! - **The face in a gas stream.** Heat conducted into the slab equals
//!   `ρ_e u_e C_H Ω (h_e − h_w)`, the stream's heating against the wall gas's
//!   enthalpy, reduced by the blowing correction `Ω = ln(1 + 2λB')/(2λB')`
//!   with λ = 0.5 and `B' = B'g + B'c`; plus the radiative heating the face
//!   takes up; less `ε σ (T_w⁴ − T_∞⁴)` radiated away, with the emissivity
//!   mixed between virgin (0.8) and char (0.9); plus `ṁ_g (h_g − h_w)` and
//!   `ṁ_c (h_char − h_w)`, the decomposition gas and the char becoming wall
//!   gas. `B'g = ṁ_g / (ρ_e u_e C_H Ω)`. The surface table gives `B'c` and
//!   `h_w` from the pressure, `B'g` and the face temperature;
//!   `ṁ_c = B'c ρ_e u_e C_H Ω`, and the face moves inward at `ṁ_c` over the
//!   density of the solid at the face.
//!
//! With TACOT's numbers `h_g − h̄` is negative below about 939 K, so
//! decomposition gives off heat there, and the cooling comes from the gas
//! term. Two things follow. Leaving the gas term out puts the 8 mm
//! temperature of Test Case 1 about 150 K too high. And a thin slab held
//! below 939 K soaks through and then warms itself: a 10 mm slab held at
//! 600 K reaches 631 K at its back wall. That is what the equations and
//! TACOT's numbers say; the solver reports it.
//!
//! The surface table is chemical equilibrium, and it says carbon is consumed
//! by air even when cold. So a face in a stream wears from the first instant.
//!
//! # What is left out
//!
//! Gas pressure inside the slab, gas storage and the time the gas takes to
//! leave. The gas's own chemistry inside the slab (it is in equilibrium at
//! 1 atm there, at every wall pressure). Any solution of the flow: the stream
//! is a heat-transfer coefficient, an enthalpy and a pressure, with one
//! blowing correction. Two and three dimensions. For TACOT, any limit on how
//! fast the surface reactions can go and any loss of solid char; the PICA
//! model has both. Wall pressures outside the material's surface table
//! (TACOT 0.001 to 1 atm; PICA 0.00001 to 1 atm). Temperatures above the
//! material's tables (TACOT 3333 K), and for PICA above
//! [`COMPARED_TEMPERATURE_MAX_K`]. Streams of anything but
//! oxygen, nitrogen and argon (carbon dioxide, for one); for TACOT, any
//! stream but air.
//!
//! # How it is solved
//!
//! Finite volumes on cells that are finest at the original face
//! ([`FIRST_CELL_M`], each [`CELL_GROWTH`] times the one above, up to
//! [`LARGEST_CELL_M`], or a quarter of that under a gas stream; 195 and 528
//! cells for 5 cm). Conduction and the gas's enthalpy are differenced
//! centrally (second order in the cell width). Time steps are backward Euler
//! (first order), short after each corner in the face condition and growing
//! after it. Within a step the resin is advanced by the exact integral of its
//! rate law at the step's new temperature, so it cannot overshoot or reverse
//! for any step length.
//!
//! **The moving face.** Cells stay where they are in the material. The face
//! is the upper edge of the front cell, and the char that leaves is taken
//! from that cell at the face temperature. A front cell worn thinner than
//! half its neighbour is joined to it, keeping the resin and the enthalpy of
//! the two exactly. So nothing is carried through the grid, the face never
//! sits on a cell boundary, and the cell at the face is between half and one
//! and a half times a cell no wider than 0.1 mm. A time step is shortened so
//! that the face crosses no more than a quarter of its cell.
//!
//! The face temperature, the char flux and all cell temperatures and gas
//! fluxes of a step are solved together by Newton's method with exact slopes;
//! each pass is one banded solve. Measured over sweeps of the declared
//! ranges, the hardest step of any run took 4 passes on 888 held-temperature
//! questions, 6 on 3,456 heating histories for TACOT (2,145 answered) and 11
//! on 294 heating histories for the PICA model (269 answered, the rest
//! refused because the slab wore away; versions 4 and 3 the same with 268),
//! and 9 on 350 histories in seven streams that are not air (all answered). The limit is [`NEWTON_PASS_LIMIT`].
//!
//! Measured on Test Case 1: halving every cell moves no reported temperature
//! by more than 0.14 K (0.04 K at 60 s); halving every time step, by more
//! than 0.22 K (0.06 K at 60 s). With decomposition off and constant
//! properties the solver is within 0.41 K of the exact conduction solution.
//! Measured on cases 2.2 and 2.3: halving every cell moves the face
//! temperature by at most 0.12 and 0.48 K, a covered thermocouple by at most
//! 0.18 and 0.85 K, and the recession by under 0.01 percent; halving every
//! time step, 0.19 and 0.50 K, 0.21 and 0.95 K, and under 0.01 percent.
//!
//! Every step is checked before it is accepted: every value finite, every
//! temperature in its band, no resin grown back, the face not moved outward,
//! solid lost equal to gas out, the energy account closed, and the heat
//! balance at the face closed. A step that fails is refused by name. Nothing
//! is pulled back to a limit.
//!
//! # Readings chosen where the sources allow more than one
//!
//! - The face is held at 1644 K in Test Case 1. One sentence of the test-case
//!   document says 1664 K; its list of conditions, its figures and the FIAT
//!   file say 1644 K.
//! - Test Case 1 ends at 60 s. What a held face does after the hold is this
//!   file's choice: it returns to the start temperature over 0.1 s, as in
//!   PATO's tutorial for the same case.
//! - A resin part is switched on for a step by the temperature the step
//!   starts from, not the one it ends at. At the thresholds (333 K, 556 K)
//!   the rates are below 10⁻⁶ per second, so the choice moves nothing.
//! - Enthalpy of virgin and of char is the exact integral of the straight-
//!   line specific heat, a parabola between table temperatures. It equals
//!   the tables' enthalpy columns at the table temperatures. Reading the
//!   enthalpy columns along straight lines instead would not agree with the
//!   specific heat between them.
//! - The gas crosses a cell face at the temperature of that face (the mean of
//!   the two cells, weighted by distance), and leaves the slab at the
//!   temperature of the heated face.
//! - The conductivity at a face is the mean of the two cells', weighted by
//!   distance; at the heated face, the mean of the face's and the first
//!   cell's.
//! - The reported gas flux and char flux are the means over the time step
//!   that ends at the report time.
//! - The fronts are read from straight lines between cell centres, starting
//!   from the density at the face: for a held face its own, which follows the
//!   same rate law at the face temperature; in a stream the front cell's.
//! - **The surface table is read through the char's share of the wall gas**,
//!   `χ = B'c / (1 + B'g + B'c)`, and the wall-gas enthalpy, each along
//!   straight lines in the logarithm of pressure, in B'g and in temperature;
//!   then `B'c = χ (1 + B'g)/(1 − χ)`. B'c runs away where carbon boils and χ
//!   does not. Where cases 2.2 and 2.3 sit, reading B'c itself along straight
//!   lines instead moves the recession by 0.02 and 0.001 percent.
//! - Above the temperature at which the table's carbon boils at a pressure
//!   the sheet repeats its last B'c. That is a cap, not a value. A face that
//!   hot is refused by name.
//! - Beyond the table's largest B'g (10), which a fading stream reaches, the
//!   wall gas is the table's gas at B'g = 10 with the further decomposition
//!   gas added unchanged, and B'c stays at its value at 10. The balance then
//!   runs smoothly to that of no stream at all.
//! - With no stream (a zero coefficient) there is no convective heating, no
//!   char is consumed and the gas leaves at its own enthalpy; radiation
//!   remains. This is what PATO's input does for the cooling phase. The codes
//!   differ by about 5 K there.
//! - The char leaves with the enthalpy of the solid at the face, which is
//!   char's when the face is fully charred; the references write char's.
//! - The face recedes at `ṁ_c` over the density of the solid at the face, not
//!   over the char density: early on the face is not yet char.
//! - Emissivity is mixed by the same virgin fraction as the other
//!   properties, and radiative heating is taken up with that emissivity.
//!   Neither reading was found stated for the reference codes.
//! - A face that does not wear (case 2.1) has B'c left out of the blowing
//!   correction as well as out of the mass, and still reads its wall gas from
//!   the table.
//! - The Stefan-Boltzmann constant is the CODATA value; PATO's input has
//!   5.6697 × 10⁻⁸, lower by 0.012 percent.
//!
//! # What it has been compared with, and how it came out
//!
//! All of it is in the tests of this file or in
//! `evidence/research/stardust_heatshield_model_run/`.
//!
//! **TACOT against established codes** (Ablation Workshop cases, reference
//! files shipped in PATO 3.1, [`REFERENCE_URL`]).
//!
//! - Test Case 1 (face held at 1644 K for 60 s) against FIAT: root mean
//!   square 0.63 percent of temperature at five depths over the minute,
//!   largest 1.77 percent; at 60 s within 5.5 K. Against PATO's stored
//!   result: 0.31 percent, largest 0.53 percent. The differences from FIAT
//!   are the size and sign of a first-order error on FIAT's own 0.42 mm
//!   grid: run that way this solver lands on FIAT (0.27 percent), and at
//!   60 s it matches Sandia's Chaleur to 0.03 of a percentage point.
//! - Cases 2.1 and 2.2 (a gas stream for 60 s, without and with wear)
//!   against Amaryllis: face within 1.1 and 1.4 K from 10 to 60 s, deep
//!   thermocouples within 0.9 percent, recession 11.906 against 11.918 mm.
//!   Against PATO on 2.2: face within 0.4 K, recession +0.27 percent.
//! - Case 2.3 (the same at 3155 K): the 2012 reference was made with an
//!   older surface table. Read from the older 1-atm table that ships in
//!   PATO, face within 1.5 K and recession 14.433 against 14.442 mm; read
//!   from TACOT 3.0's own table, 51 K hotter and 1.3 percent more recession.
//! - In the cooling phase of these cases the face is within 6 K from 62 s.
//!
//! **The PICA model against measurement** (version 5; versions 4 and 3 in
//! brackets where they differ).
//!
//! - Four arc-jet blocks with thermocouples in depth (Tran and others 1997;
//!   400 and 580 W/cm², 0.2 and 0.45 atm): root mean square 31.3 K over 259
//!   samples below 1500 K (31.3, 30.3). This is the data the one fitted
//!   number was fitted to.
//! - 36 arc-jet tests of PICA (Milos and Chen 2010; 107 to 1,102 W/cm², 2.3
//!   to 84 kPa), each asked through [`answer`] as a transient in its own
//!   stream gas (air with 7.6 to 27.6 percent argon; nitrogen with 10 or 30
//!   percent oxygen): wear within
//!   20 percent on 35, mean −4.9 percent (−4.9, −5.0); face temperature
//!   within 7 percent on 30 of 34, mean +3.8 percent. The surface options
//!   were chosen on these tests. They do not tell version 5 from version 4:
//!   the largest difference is 4 K at one face.
//! - 31 further arc-jet tests that no model had seen, run once on
//!   2026-10-09 with the predictions fixed before the measurements were
//!   read, against lines fixed on 2026-10-07
//!   (`evidence/research/stardust_heatshield_model_run/final_check_31_arcjet/`).
//!   Wear: inside the line on 28 of 31 where 25 were needed, mean −2.6
//!   percent against 8 allowed: passes. Hottest face temperature: within 7
//!   percent on 24 of 28 where 23 were needed, mean +4.1 percent against
//!   3.5 allowed: misses on the mean, as on the 36 (+3.8). By the rule fixed
//!   beforehand the model as a whole does not pass. One test, in pure
//!   nitrogen, was refused (`StreamOxygenOutOfRange`) and counts as outside.
//!   The face misses are above about 45 kPa, at 190 and 970 W/cm²; two long
//!   exposures in one facility miss on wear. The 31 are spent, and 26 of
//!   them repeat a condition and exposure of the 36.
//! - The Stardust entry at the two places where cores were cut from the
//!   recovered shield, each version frozen before it was flown. Measured
//!   wear 5.7 mm near the nose (6.1 by X-ray scan) and 3.2 mm on the flank.
//!   Measured depth below the final surface at which the density is half way
//!   from char to virgin: 5.6 and 9.0 mm; back within 2 percent of virgin:
//!   8.65 and 10.74 mm (29 slices a core, 4 percent error or more a slice).
//!   NASA's own code on the same cores: 7.1 and 8.5 mm; 9.2 and 10.7 mm.
//!   - With NASA's bare-wall heating and this solver's own hot-wall and
//!     blowing corrections: wear 7.58 and 3.32 mm, a third high at the nose,
//!     3 percent high on the flank; peak face 3,412 and 3,103 K. Half way at
//!     7.4 and 9.1 mm (version 4: 7.7 and 9.3; version 3: 8.1 and 9.7).
//!     Within 2 percent at 12.0 and 14.4 mm (13.2 and 15.5; 14.9 and 17.3),
//!     read at 600 s. Slices between 3 and 16 mm outside the drawn error
//!     bar: 3 of 11 and 0 of 11 (5 and 1; 6 and 2).
//!   - With the stream heating taken from NASA's coupled flow solution
//!     (Johnston 2014): wear 6.45 and 3.19 mm, 13 percent high at the nose,
//!     on the measurement at the flank; peak face 3,289 and 2,877 K. Half
//!     way at 7.9 and 8.6 mm; within 2 percent at 12.7 and 13.7 mm (12.7 and
//!     13.8; 13.9 and 15.1). Slices outside: 4 of 11 and 0 of 11 (5 and 0;
//!     5 and 0). NASA's own code: 1 of 11 and 0 of 11.
//!   - The face at the end of the heating table (133 s): 600 and 631 K with
//!     this solver's own heating, 598 and 599 K with NASA's. Version 4 had
//!     813 and 828 K against 608 and 616 K.
//!   - NASA's standard tools were 61 and 25 percent high on wear; its best
//!     coupled codes 22 percent high and 14 percent low at the nose, 20
//!     percent low on the flank.
//!
//! # Known misses
//!
//! - **The faint end of the charring is too deep.** The depth at which the
//!   density is back within 2 percent of virgin is 3 to 4 mm deeper than the
//!   cores show and than NASA's code computes, at both cores, with either
//!   heating. The half-way depth is within 0.4 mm on the flank and 2 mm deep
//!   at the nose, where NASA's code is 1.5 mm deep. From half way to the
//!   2 percent line the model takes about 5 mm where the cores and NASA's
//!   code take 2 to 3. Not explained. The pressure dependence of
//!   conductivity, PICA's own heat capacity and conductivity, a later start
//!   of decomposition and the cold flow late in the entry have each been
//!   put in; together they brought it from 7 mm to this.
//! - **Wear at the Stardust nose is a third high** with this solver's own
//!   heating and 13 percent high with NASA's coupled heating: 22 to 35
//!   percent too much stream heating at the peak against NASA's coupled
//!   solutions. One blowing number cannot stand in for a flow solution that
//!   carries the ablation products. The arc-jet wear does not allow a
//!   stronger one.
//! - **The face runs hot at high heating**: 6.8 percent in the eight arc-jet
//!   tests above 700 W/cm². NASA's own model shows the same against the
//!   same pyrometers and could not explain it.
//! - **Wear is low at low pressure**: 12 percent on the ten arc-jet tests at
//!   8.5 kPa and below.
//! - **One number is fitted**, the char conductivity scale. Used as
//!   measured, PICA's 1997 conductivity leaves the thermocouple blocks 121 K
//!   out.
//! - **The decomposition law is the stand-in material's**, which is NASA's
//!   1997 law in substance. Measured PICA starts later than it when heated
//!   slowly and earlier when heated fast; no law of this kind does both, and
//!   the three laws published for PICA char deeper still. The model's char
//!   is 215 kg/m³ where the recovered cores measured 209 to 221, and 185 to
//!   194 in the outermost slice.
//! - **Above 13 kPa a cool wall still burns the decomposition gas** in
//!   oxygen the wall reaction leaves unused, as the equilibrium table says.
//!   No opened source gives a rule there. Leaving it unburnt at every
//!   pressure moves the Stardust depths by 0.1 mm or less and no arc-jet
//!   score.
//! - Park's probability for oxygen molecules has a constant part, so a face
//!   at 800 K still burns a little char and is warmed by it.
//! - The decomposition gas inside the slab is read at 1 atm at every wall
//!   pressure.
//!
//! - **The locked air table is not all for the same air.** Its three lowest
//!   pressures (1 to 30 Pa) and its table of atomic oxygen were made for
//!   23.14 percent oxygen by mass; the other twenty pressures for 23.29
//!   (21 percent by mole, which [`StreamGas::AIR`] names). The char rate
//!   differs by 0.65 percent between the two. It is kept as it is so that
//!   every recorded air answer repeats; a test holds the fact. An air
//!   question is touched only below 101 Pa: with the air table computed
//!   afresh for one air throughout, the Stardust wear moves by 0.006
//!   percent.
//! - **An air question at one pressure is read between the locked table's
//!   pressures; any other stream at one pressure is computed at it.** For
//!   air at 20 kPa the two ways differ by under one part in 100,000 in
//!   wear.
//!
//! Not compared: PICA in any cooling phase with a measured temperature; any
//!   case with measured radiative heating; TACOT versions 1.5 and 2.2, which
//!   the workshop codes used.
//!
//! # The range of a PICA question is what was compared
//!
//! Since engine 0.7.0 a PICA question outside what was compared with
//! measurement is refused by name, where before it was answered with an
//! account of the seconds it spent outside. Three envelopes were compared,
//! and a gas-stream question is answered when it lies wholly inside one of
//! them, its slab, start and surroundings included:
//!
//! - **The arc-jet tests** (the 36 and the 31): thickness 0.06 m, start
//!   300 K, surroundings 300 K; wall pressure 2.3 to
//!   84.4 kPa; cold-wall heating 107 to 1,102 W/cm² from 0.5 s on (below it
//!   only in the swing-in before 0.5 s); flow enthalpy 7.3 to 29.2 MJ/kg;
//!   up to 400 s; oxygen 10 to 30 percent and argon up to 27.6 percent by
//!   mass; no radiative heating and no cooling phase.
//! - **The Stardust flight** (the two cores, NASA's bare-wall heating):
//!   thickness 0.0582 m, start 260 K, surroundings 266.48 K; air; wall
//!   pressure 1.42 Pa to 34.0 kPa; flow enthalpy 85 kJ/kg to 78.3 MJ/kg;
//!   coefficient up to 0.1971 kg/m²/s and cold-wall heating up to
//!   1,001.6 W/cm²; radiative heating up to 0.74 MW/m²; one stretch of
//!   stream ending by 134 s, then cooling. Below 107 W/cm² in its tails it
//!   is inside this envelope.
//! - **The thermocouple blocks** (Tran and others 1997, blocks 9B, 7B, 3B
//!   and 6A, the data the one fitted number was fitted to): thickness 0.0224
//!   to 0.0325 m, start 303.15 K, surroundings 300 K; air at one wall
//!   pressure from 0.2 to 0.45 atm; flow enthalpy 29.5 MJ/kg; cold-wall
//!   heating 400 to 580 W/cm² held from 0.5 s to a time from 15 to 40 s,
//!   then falling to zero over 0.5 s; cooling to 152 s; no radiative
//!   heating.
//!
//! Common to all three: a face that wears, and no temperature above
//! [`COMPARED_TEMPERATURE_MAX_K`]. A value of thickness, start or
//! surroundings in no envelope is refused by its own name; values that each
//! lie in some envelope but not all in one are refused as
//! [`SlabReject::QuestionNotCompared`]. A held face on PICA was never compared and
//! is refused. From 3333 K, where TACOT's tables stop, to that limit the PICA
//! tables are extended: the specific heat held at its 3333 K value and the
//! conductivity continued along its last straight line. TACOT, compared with
//! other codes only, keeps its whole declared range.

use std::fmt;

/// The material.
pub const MATERIAL_NAME: &str = "TACOT 3.0 (Theoretical Ablative Composite for Open Testing), Lachaud, van Eekelen, Bianchi, Martin";
pub const MATERIAL_URL: &str = "https://jeanlachaud.com/research/TACOT_3.0.xls";
/// The published calculation this solver is checked against.
pub const REFERENCE_NAME: &str = "FIAT on Test Case 1 of the Ablation Workshop (version 1.1, 2 February 2011), files tutorials/1D/AblationTestCase_1.0/data/ref/FIAT/T and pyrolysisFront of PATO 3.1";
pub const REFERENCE_URL: &str = "https://github.com/nasa/pato/archive/refs/tags/3.1.tar.gz";
/// Date the material file and the reference files were read.
pub const REFERENCE_RETRIEVED: &str = "2026-10-09";

// ---------------------------------------------------------------------------
// The question a caller may ask, and its declared range.
// ---------------------------------------------------------------------------

/// Thinnest slab, metres.
pub const THICKNESS_MIN_M: f64 = 0.01;
/// Thickest slab, metres.
pub const THICKNESS_MAX_M: f64 = 0.10;
/// Coldest uniform start temperature, kelvin.
pub const START_TEMPERATURE_MIN_K: f64 = 260.0;
/// Warmest uniform start temperature, kelvin.
pub const START_TEMPERATURE_MAX_K: f64 = 400.0;
/// Lowest temperature the heated face may be held at, kelvin.
pub const HELD_TEMPERATURE_MIN_K: f64 = 400.0;
/// Highest temperature the heated face may be held at, kelvin.
pub const HELD_TEMPERATURE_MAX_K: f64 = 3300.0;
/// Shortest hold, seconds, counted from the start of the ramp.
pub const HOLD_DURATION_MIN_S: f64 = 1.0;
/// Longest hold, seconds.
pub const HOLD_DURATION_MAX_S: f64 = 300.0;
/// Longest time computed after the hold, seconds. Zero is allowed.
pub const CONTINUE_AFTER_MAX_S: f64 = 300.0;
/// Most thermocouple depths in one question.
pub const THERMOCOUPLES_MAX: usize = 8;
/// The face goes from the start temperature to the held temperature in a
/// straight line over this time, and back again after the hold. Seconds.
pub const SURFACE_RAMP_S: f64 = 0.1;
/// Histories are reported at whole multiples of this, at the end of the hold
/// and at the end of the run. Seconds.
pub const REPORT_INTERVAL_S: f64 = 1.0;

/// Fewest and most rows in a heating history.
pub const HISTORY_ROWS_MIN: usize = 2;
pub const HISTORY_ROWS_MAX: usize = 256;
/// Shortest and longest heating history, seconds, from its first row (at
/// zero) to its last.
pub const HISTORY_DURATION_MIN_S: f64 = 1.0;
pub const HISTORY_DURATION_MAX_S: f64 = 600.0;
/// Least time between two rows of a heating history, seconds: a
/// millisecond. Between two rows every column runs along a straight line,
/// so a row a vanishing time after the one before it is a jump, and the
/// stepping goes through a jump in steps as small as its span: with one row
/// 1e-300 s after the first a ten-second history took 139,626 time steps,
/// where it takes 1,844 with that row half a second after the first and
/// 2,511 with it a millisecond after. The compared histories are far above
/// the floor: their rows are 0.1 s apart at the least in the workshop
/// cases, 0.5 s in the arc-jet tests and the thermocouple blocks, and 1 s
/// in the Stardust flight.
///
/// The time between two rows is the difference of their two times as the
/// solver holds them, the span the stepping divides by. Two times written a
/// millisecond apart in decimal can be a hair under one apart as binary
/// numbers (1 and 1.001 s are 0.0009999999999998899 s apart), so the floor
/// is held with the rounding allowance of the compared ranges,
/// [`COMPARED_ROUNDING`] of itself: rows written a millisecond apart are
/// taken at every time a history may hold, and a span of 0.000999999 s is
/// refused.
pub const HISTORY_ROW_SPAN_MIN_S: f64 = 1.0e-3;
/// Lowest and highest wall pressure in a heating history for TACOT, Pa: the
/// ends of TACOT's surface table, 0.001 atm and 1 atm.
pub const STREAM_PRESSURE_MIN_PA: f64 = 101.325;
pub const STREAM_PRESSURE_MAX_PA: f64 = 101325.0;
/// Lowest and highest wall pressure in a heating history for every PICA
/// version, Pa: the ends of the PICA surface table, 0.00001 atm and 1 atm.
/// [`stream_pressure_range_pa`] gives the pair for a material by name.
pub const PICA_STREAM_PRESSURE_MIN_PA: f64 = 1.01325;
pub const PICA_STREAM_PRESSURE_MAX_PA: f64 = 101325.0;
/// Largest heat-transfer coefficient `ρ_e u_e C_H` in a heating history,
/// kg/m²/s. The smallest is zero: no stream.
pub const HEAT_TRANSFER_COEFFICIENT_MAX_KG_PER_M2_S: f64 = 1.0;
/// Largest flow enthalpy in a heating history, J/kg. The smallest is zero.
pub const FLOW_ENTHALPY_MAX_J_PER_KG: f64 = 1.0e8;
/// Largest radiative heating in a heating history, W/m². The smallest is zero.
pub const RADIATIVE_HEATING_MAX_W_PER_M2: f64 = 1.0e7;
/// Coldest and warmest surroundings the face radiates to, kelvin.
pub const SURROUNDINGS_TEMPERATURE_MIN_K: f64 = 250.0;
pub const SURROUNDINGS_TEMPERATURE_MAX_K: f64 = 400.0;
/// λ in the blowing correction `Ω = ln(1 + 2λB') / (2λB')`.
pub const BLOWING_CORRECTION_LAMBDA: f64 = 0.5;
/// Stefan-Boltzmann constant, W/m²/K⁴ (CODATA 2018).
pub const STEFAN_BOLTZMANN_W_PER_M2_K4: f64 = 5.670374419e-8;

// ---------------------------------------------------------------------------
// What a material is. The numbers of the one material served, TACOT 3.0, are
// in `tacot.rs`.
// ---------------------------------------------------------------------------

use crate::tacot::*;
pub use crate::tacot::{CHAR_DENSITY_KG_PER_M3, VIRGIN_DENSITY_KG_PER_M3};

/// One decomposing part of the resin. Densities are per volume of resin.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ResinPart {
    pub(crate) virgin_density_kg_per_m3: f64,
    pub(crate) char_density_kg_per_m3: f64,
    pub(crate) rate_constant_per_s: f64,
    /// `E/R`, kelvin.
    pub(crate) activation_temperature_k: f64,
    /// The part does not decompose at or below this temperature, kelvin.
    pub(crate) threshold_temperature_k: f64,
}

/// A surface table: for each wall pressure, each B'g and each wall
/// temperature, B'c and the enthalpy of the gas mixture at the wall. Any
/// number of pressures, B'g values and temperatures, each list rising.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SurfaceTable {
    pub(crate) pressure_pa: &'static [f64],
    /// B'g: decomposition-gas flux over the blown transfer coefficient.
    pub(crate) blowing: &'static [f64],
    pub(crate) temperature_k: &'static [f64],
    /// B'c at each node: pressure outermost, then B'g, then temperature.
    pub(crate) char_rate: &'static [f64],
    /// Enthalpy of the wall gas at each node, J/kg, same order.
    pub(crate) wall_enthalpy_j_per_kg: &'static [f64],
}

/// Everything the solver knows about a material. All of it is locked data;
/// a caller supplies none of it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Material {
    /// Temperatures of the virgin and char tables, kelvin.
    pub(crate) solid_table_k: &'static [f64],
    /// Index in `solid_table_k` at which the two enthalpies are given.
    pub(crate) solid_anchor_node: usize,
    pub(crate) virgin_specific_heat_j_per_kg_k: &'static [f64],
    pub(crate) char_specific_heat_j_per_kg_k: &'static [f64],
    pub(crate) virgin_conductivity_w_per_m_k: &'static [f64],
    pub(crate) char_conductivity_w_per_m_k: &'static [f64],
    pub(crate) virgin_enthalpy_at_anchor_j_per_kg: f64,
    pub(crate) char_enthalpy_at_anchor_j_per_kg: f64,
    pub(crate) virgin_density_kg_per_m3: f64,
    pub(crate) char_density_kg_per_m3: f64,
    /// Solid volume per volume of material.
    pub(crate) solid_volume_fraction: f64,
    /// Resin volume per volume of solid.
    pub(crate) resin_volume_fraction: f64,
    pub(crate) fibre_density_kg_per_m3: f64,
    /// The two decomposing parts of the resin.
    pub(crate) resin_parts: [ResinPart; 2],
    pub(crate) gas_table_first_k: f64,
    pub(crate) gas_table_step_k: f64,
    /// Enthalpy of the decomposition gas, J/kg, at evenly spaced temperatures.
    pub(crate) gas_enthalpy_j_per_kg: &'static [f64],
    pub(crate) virgin_emissivity: f64,
    pub(crate) char_emissivity: f64,
    pub(crate) surface: SurfaceTable,
    /// Heat carried by the gas in the pores, when the material's
    /// conductivity is to depend on pressure.
    pub(crate) pore_gas: Option<PoreGas>,
}

/// The part of a porous material's conductivity that the gas in its pores
/// carries. It follows the gas's own conductivity in temperature, about
/// `T^0.8`, and it fades as pressure falls and the gas molecules' free path
/// outgrows the pores: by the factor `p / (p + p_half)`, with `p_half`
/// proportional to temperature (the free path at a pressure grows with it).
/// The conductivity curves of the material hold at the reference pressures;
/// at another pressure the difference in the gas's part is taken off or
/// added.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PoreGas {
    /// Gas-borne conductivity of virgin material and of char at 300 K with
    /// the pores at full pressure, W/m/K.
    pub(crate) virgin_w_per_m_k_at_300_k: f64,
    pub(crate) char_w_per_m_k_at_300_k: f64,
    /// Pressure at which the gas carries half of that, at 300 K, Pa.
    pub(crate) half_pressure_pa_at_300_k: f64,
    /// Pressures at which the virgin and char conductivity curves hold, Pa.
    pub(crate) virgin_reference_pa: f64,
    pub(crate) char_reference_pa: f64,
}

impl PoreGas {
    /// What to add to the virgin and to the char conductivity at this
    /// temperature and pore pressure, W/m/K, and the slopes in temperature.
    fn departure(&self, temperature_k: f64, pressure_pa: f64) -> [(f64, f64); 2] {
        let ratio = temperature_k / 300.0;
        let growth = ratio.powf(0.8);
        let half = self.half_pressure_pa_at_300_k * ratio;
        let share = |p: f64| {
            (
                p / (p + half),
                -p * (half / temperature_k) / ((p + half) * (p + half)),
            )
        };
        let (here, here_slope) = share(pressure_pa);
        let one = |at_300: f64, reference_pa: f64| {
            let (reference, reference_slope) = share(reference_pa);
            let difference = here - reference;
            (
                at_300 * growth * difference,
                at_300 * growth * (0.8 / temperature_k * difference + here_slope - reference_slope),
            )
        };
        [
            one(self.virgin_w_per_m_k_at_300_k, self.virgin_reference_pa),
            one(self.char_w_per_m_k_at_300_k, self.char_reference_pa),
        ]
    }
}

/// Share of the decomposable resin that counts as "virgin" at the virgin
/// front and "char" at the char front: 98 percent and 2 percent.
pub const FRONT_FRACTION: f64 = 0.02;

// ---------------------------------------------------------------------------
// Numerics, fixed.
// ---------------------------------------------------------------------------

/// Width of the cell at the heated face, metres, before the widths are
/// scaled to fill the slab exactly.
pub const FIRST_CELL_M: f64 = 2.0e-5;
/// Each cell is this much wider than the one above it, until
/// [`LARGEST_CELL_M`].
pub const CELL_GROWTH: f64 = 1.03;
/// Widest cell, metres.
pub const LARGEST_CELL_M: f64 = 4.0e-4;
/// First time step after a corner in the face temperature, seconds.
pub const FIRST_STEP_S: f64 = 5.0e-5;
/// A step is this fraction of the time since the last corner in the face
/// temperature, between [`FIRST_STEP_S`] and [`LARGEST_STEP_S`].
pub const STEP_FRACTION: f64 = 0.005;
/// Longest time step, seconds.
pub const LARGEST_STEP_S: f64 = 0.05;
/// A step is accepted when no temperature moved by more than this in the
/// last Newton pass. Kelvin.
pub const NEWTON_TEMPERATURE_TOLERANCE_K: f64 = 1.0e-9;
/// And no gas flux moved by more than this. kg/m²/s.
pub const NEWTON_GAS_FLUX_TOLERANCE_KG_PER_M2_S: f64 = 1.0e-12;
/// A step that has not settled after this many Newton passes is refused.
/// Measured: the hardest step of any run took 4 passes over 888
/// held-temperature questions, 6 over 3,456 heating histories on TACOT, and
/// 11 over 294 heating histories on the PICA model, whose surface slopes are
/// taken by differences. The limit is a little over twice the largest.
pub const NEWTON_PASS_LIMIT: u32 = 24;
/// No temperature may be this far outside the band described at
/// [`SlabReject::TemperatureOutOfBounds`]. Kelvin.
pub const TEMPERATURE_BOUND_TOLERANCE_K: f64 = 1.0e-6;
/// The energy account of a step must close to this fraction of the energy
/// that moved in the step, or to [`ACCOUNT_ROUNDING_FLOOR`] of the enthalpy
/// the slab holds, whichever is larger.
pub const ENERGY_ACCOUNT_TOLERANCE: f64 = 1.0e-9;
/// The mass account of a step must close to this fraction of the mass that
/// moved in the step, or to [`ACCOUNT_ROUNDING_FLOOR`] of the slab's mass,
/// whichever is larger.
pub const MASS_ACCOUNT_TOLERANCE: f64 = 1.0e-9;
/// A step's change is a difference of two totals. This fraction of a total
/// is allowed for the rounding of that difference.
pub const ACCOUNT_ROUNDING_FLOOR: f64 = 1.0e-12;

/// Under a gas stream no cell is wider than this fraction of
/// [`LARGEST_CELL_M`], so the cell at a face that has worn inward is still fine.
pub const LARGEST_CELL_IN_A_STREAM: f64 = 0.25;
/// A front cell worn thinner than this fraction of the cell below it is
/// joined to that cell.
pub const FRONT_CELL_MERGE_FRACTION: f64 = 0.5;
/// A time step is shortened so that the face crosses no more than this
/// fraction of its cell in it.
pub const WEAR_PER_STEP: f64 = 0.25;
/// A step that leaves less than this fraction of the front cell is refused.
pub const FRONT_CELL_LEFT_AFTER_A_STEP: f64 = 0.25;
/// Under a gas stream a time step is shortened so that no column of the
/// heating history changes by more than this fraction of its largest value.
pub const FORCING_CHANGE_PER_STEP: f64 = 0.005;
/// At a row of the heating history where a column turns a corner, the time
/// step restarts at this fraction of the time in which the corner alone would
/// change that column by its largest value.
pub const STEP_AFTER_A_CORNER: f64 = 5.0e-4;
/// A slab worn down to this many cells is refused.
pub const CELLS_MIN: usize = 8;
/// A char flux below minus this is a face moving backward. kg/m²/s.
pub const CHAR_FLUX_TOLERANCE_KG_PER_M2_S: f64 = 1.0e-12;

#[derive(Clone, Copy, Debug)]
struct Numerics {
    first_cell_m: f64,
    cell_growth: f64,
    largest_cell_m: f64,
    first_step_s: f64,
    step_fraction: f64,
    largest_step_s: f64,
    newton_pass_limit: u32,
    wear_per_step: f64,
}

const NUMERICS: Numerics = Numerics {
    first_cell_m: FIRST_CELL_M,
    cell_growth: CELL_GROWTH,
    largest_cell_m: LARGEST_CELL_M,
    first_step_s: FIRST_STEP_S,
    step_fraction: STEP_FRACTION,
    largest_step_s: LARGEST_STEP_S,
    newton_pass_limit: NEWTON_PASS_LIMIT,
    wear_per_step: WEAR_PER_STEP,
};

/// Which terms are in the model. The public solver uses [`Terms::ALL`]. The
/// other settings exist so that tests can show what each term does and that
/// a broken account is refused.
#[derive(Clone, Copy, Debug)]
struct Terms {
    /// The resin decomposes.
    decomposition: bool,
    /// The heat of decomposition, `(h_g − h̄) ∂ρ/∂t`.
    decomposition_heat: bool,
    /// The heat the gas picks up on its way out, `ṁ_g ∂h_g/∂x`.
    gas_carries_heat: bool,
    /// When set, specific heat and conductivity are read at this one
    /// temperature for every cell. Kelvin.
    properties_frozen_at_k: Option<f64>,
    /// Heat put into the back cell that no account knows about. W/m².
    unaccounted_heat_w_per_m2: f64,
    /// Gas put into the back cell that no account knows about. kg/m²/s.
    unaccounted_gas_kg_per_m2_s: f64,
    /// When set, the gas crossing a cell face is given the enthalpy at this
    /// mix of the two cells' temperatures: 1 is the cell nearer the heated
    /// face, 0 the cell below. Unset, the mix is by distance, which is the
    /// face's own temperature.
    gas_face_share_of_cell_above: Option<f64>,
    /// The stream's heating is reduced by the gases blowing out of the face.
    blowing_correction: bool,
    /// The face radiates to its surroundings.
    reradiation: bool,
    /// The decomposition gas and the char take heat at the face as they
    /// become wall gas.
    gases_take_heat_at_face: bool,
    /// When set, B'c itself is read along straight lines in the surface
    /// table, instead of the char's share of the wall gas.
    char_rate_along_straight_lines: bool,
    /// When set, the face stops at this depth below the original face, metres.
    face_stops_at_m: Option<f64>,
    /// When set, a gas-stream run hotter than this anywhere is refused as
    /// [`SlabReject::TemperatureAboveCompared`]. The served entry sets it
    /// for PICA to [`COMPARED_TEMPERATURE_MAX_K`]. Kelvin.
    compared_temperature_top_k: Option<f64>,
}

impl Terms {
    const ALL: Terms = Terms {
        decomposition: true,
        decomposition_heat: true,
        gas_carries_heat: true,
        properties_frozen_at_k: None,
        unaccounted_heat_w_per_m2: 0.0,
        unaccounted_gas_kg_per_m2_s: 0.0,
        gas_face_share_of_cell_above: None,
        blowing_correction: true,
        reradiation: true,
        gases_take_heat_at_face: true,
        char_rate_along_straight_lines: false,
        face_stops_at_m: None,
        compared_temperature_top_k: None,
    };
}

// ---------------------------------------------------------------------------
// The surface condition.
// ---------------------------------------------------------------------------

/// One row of a heating history: the gas stream at one time. Between rows
/// every column runs along a straight line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreamRow {
    /// Seconds from the start. The first row is at zero, and each row is
    /// [`HISTORY_ROW_SPAN_MIN_S`] or more after the one before.
    pub time_s: f64,
    /// Pressure at the wall, Pa.
    pub pressure_pa: f64,
    /// Heat-transfer coefficient of the stream without blowing,
    /// `ρ_e u_e C_H`, kg/m²/s. Zero means no stream.
    pub heat_transfer_coefficient_kg_per_m2_s: f64,
    /// Enthalpy of the flow (recovery enthalpy), J/kg.
    pub flow_enthalpy_j_per_kg: f64,
    /// Radiative heating arriving at the face, W/m². The face takes up the
    /// share its emissivity gives.
    pub radiative_heating_w_per_m2: f64,
}

/// What a gas stream is made of, by mass: oxygen and argon. The rest is
/// nitrogen. A caller states it for every gas-stream question; for air,
/// [`StreamGas::AIR`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreamGas {
    pub oxygen_mass_fraction: f64,
    pub argon_mass_fraction: f64,
}

/// Oxygen in the air of the locked surface table, by mass: 21 percent
/// oxygen and 79 percent nitrogen by mole, the air of the published TACOT
/// table.
pub const AIR_OXYGEN_MASS_FRACTION: f64 = 0.23291751145757963;
/// Oxygen in dry air by mass (US Standard Atmosphere), which the arc-jet
/// streams the PICA model was developed on were made up with: air with
/// argon mixed in has `0.2314 × (1 − argon)` of oxygen.
pub const ARCJET_AIR_OXYGEN_MASS_FRACTION: f64 = 0.2314;
/// Least and most oxygen a stream may hold, by mass.
pub const STREAM_OXYGEN_MASS_FRACTION_MIN: f64 = 0.05;
pub const STREAM_OXYGEN_MASS_FRACTION_MAX: f64 = 0.40;
/// Most argon a stream may hold, by mass. The least is none.
pub const STREAM_ARGON_MASS_FRACTION_MAX: f64 = 0.50;

impl StreamGas {
    /// Air: 21 percent oxygen and 79 percent nitrogen by mole (23.29
    /// percent oxygen by mass), no argon. The surface table for exactly this
    /// gas is locked in the crate; any other gas has its table computed when
    /// it is asked for.
    pub const AIR: StreamGas = StreamGas {
        oxygen_mass_fraction: AIR_OXYGEN_MASS_FRACTION,
        argon_mass_fraction: 0.0,
    };

    /// Dry air with argon mixed in to the mass fraction given, as in an arc
    /// jet: oxygen is `0.2314 × (1 − argon)`. This is the rule the 36 arc-jet
    /// tests were run with. With no argon it is not [`StreamGas::AIR`]: it
    /// has 23.14 percent oxygen where the locked table's air has 23.29.
    pub fn air_with_argon(argon_mass_fraction: f64) -> StreamGas {
        StreamGas {
            oxygen_mass_fraction: ARCJET_AIR_OXYGEN_MASS_FRACTION * (1.0 - argon_mass_fraction),
            argon_mass_fraction,
        }
    }

    /// Whether this is exactly [`StreamGas::AIR`].
    pub fn is_air(&self) -> bool {
        *self == StreamGas::AIR
    }

    /// Whether the arc-jet tests of the PICA model ran in a stream of this
    /// make-up: see [`ARC_JET_STREAM_OXYGEN_MASS_FRACTION_MIN`].
    pub fn inside_arc_jet_gases(&self) -> bool {
        (ARC_JET_STREAM_OXYGEN_MASS_FRACTION_MIN..=ARC_JET_STREAM_OXYGEN_MASS_FRACTION_MAX)
            .contains(&self.oxygen_mass_fraction)
            && (0.0..=ARC_JET_STREAM_ARGON_MASS_FRACTION_MAX).contains(&self.argon_mass_fraction)
    }
}

/// What is done to the heated face.
#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceCondition {
    /// The face is taken from the start temperature to `held_temperature_k`
    /// in a straight line over [`SURFACE_RAMP_S`], held there until
    /// `hold_duration_s` after the start, then returned to the start
    /// temperature over [`SURFACE_RAMP_S`] and kept there.
    HeldTemperature {
        held_temperature_k: f64,
        hold_duration_s: f64,
    },
    /// The face is heated by a gas stream that follows `history`, radiates
    /// to surroundings at `surroundings_temperature_k`, and, when
    /// `face_wears`, loses char at the rate the surface table gives. The run
    /// ends at the last row. With `face_wears` false no char leaves and the
    /// blowing correction counts the decomposition gas alone, while the wall
    /// gas is still read from the table: the device of workshop case 2.1.
    /// `stream_gas` is what the stream is made of; TACOT takes air only.
    GasStream {
        history: Vec<StreamRow>,
        surroundings_temperature_k: f64,
        face_wears: bool,
        stream_gas: StreamGas,
    },
}

impl SurfaceCondition {
    /// Temperature of a held face at `time_s`, kelvin.
    fn face_temperature_k(&self, start_temperature_k: f64, time_s: f64) -> f64 {
        match self {
            Self::HeldTemperature {
                held_temperature_k,
                hold_duration_s,
            } => {
                let rise = held_temperature_k - start_temperature_k;
                if time_s <= *hold_duration_s {
                    if time_s >= SURFACE_RAMP_S {
                        *held_temperature_k
                    } else {
                        start_temperature_k + rise * (time_s / SURFACE_RAMP_S)
                    }
                } else {
                    let since = time_s - hold_duration_s;
                    if since >= SURFACE_RAMP_S {
                        start_temperature_k
                    } else {
                        held_temperature_k - rise * (since / SURFACE_RAMP_S)
                    }
                }
            }
            // The face in a stream finds its own temperature.
            Self::GasStream { .. } => start_temperature_k,
        }
    }

    /// Times at which the face condition has a corner, seconds.
    fn corner_times_s(&self) -> Vec<f64> {
        match self {
            Self::HeldTemperature {
                hold_duration_s, ..
            } => vec![
                0.0,
                SURFACE_RAMP_S,
                *hold_duration_s,
                hold_duration_s + SURFACE_RAMP_S,
            ],
            Self::GasStream { history, .. } => history.iter().map(|row| row.time_s).collect(),
        }
    }

    /// When the heating ends, seconds.
    fn heating_ends_s(&self) -> f64 {
        match self {
            Self::HeldTemperature {
                hold_duration_s, ..
            } => *hold_duration_s,
            Self::GasStream { history, .. } => history[history.len() - 1].time_s,
        }
    }
}

/// The question.
#[derive(Clone, Debug, PartialEq)]
pub struct SlabQuestion {
    /// Slab thickness, metres.
    pub thickness_m: f64,
    /// Uniform temperature at the start, kelvin.
    pub start_temperature_k: f64,
    /// What is done to the heated face.
    pub surface: SurfaceCondition,
    /// Time computed after the hold, seconds.
    pub continue_after_s: f64,
    /// Depths below the heated face at which the temperature history is
    /// reported, metres. Up to [`THERMOCOUPLES_MAX`].
    pub thermocouple_depths_m: Vec<f64>,
}

/// Test Case 1 of the Ablation Workshop (version 1.1, 2 February 2011).
pub fn ablation_workshop_test_case_1() -> SlabQuestion {
    SlabQuestion {
        thickness_m: 0.05,
        start_temperature_k: 298.0,
        surface: SurfaceCondition::HeldTemperature {
            held_temperature_k: 1644.0,
            hold_duration_s: 60.0,
        },
        continue_after_s: 0.0,
        thermocouple_depths_m: vec![0.001, 0.002, 0.004, 0.008, 0.016, 0.05],
    }
}

/// Test cases 2.1, 2.2 and 2.3 of the Ablation Workshop (series 2, version
/// 2.8, 6 February 2012): 5 cm of TACOT from 300 K at 1 atm, a stream with
/// `ρ_e u_e C_H` = 0.3 kg/m²/s for a minute, then a minute of cooling by
/// radiation to 300 K.
fn ablation_workshop_series_2(flow_enthalpy_j_per_kg: f64, face_wears: bool) -> SlabQuestion {
    let row = |time_s: f64, on: f64| StreamRow {
        time_s,
        pressure_pa: 101325.0,
        heat_transfer_coefficient_kg_per_m2_s: 0.3 * on,
        flow_enthalpy_j_per_kg: flow_enthalpy_j_per_kg * on,
        radiative_heating_w_per_m2: 0.0,
    };
    SlabQuestion {
        thickness_m: 0.05,
        start_temperature_k: 300.0,
        surface: SurfaceCondition::GasStream {
            history: vec![
                row(0.0, 0.0),
                row(0.1, 1.0),
                row(60.0, 1.0),
                row(60.1, 0.0),
                row(120.0, 0.0),
            ],
            surroundings_temperature_k: 300.0,
            face_wears,
            stream_gas: StreamGas::AIR,
        },
        continue_after_s: 0.0,
        thermocouple_depths_m: vec![0.001, 0.002, 0.004, 0.008, 0.012, 0.016, 0.024, 0.05],
    }
}

/// Case 2.1: flow enthalpy 1.5 MJ/kg, the face does not wear.
pub fn ablation_workshop_test_case_2_1() -> SlabQuestion {
    ablation_workshop_series_2(1.5e6, false)
}

/// Case 2.2: flow enthalpy 1.5 MJ/kg, the face wears.
pub fn ablation_workshop_test_case_2_2() -> SlabQuestion {
    ablation_workshop_series_2(1.5e6, true)
}

/// Case 2.3: flow enthalpy 25 MJ/kg, the face wears.
pub fn ablation_workshop_test_case_2_3() -> SlabQuestion {
    ablation_workshop_series_2(2.5e7, true)
}

// ---------------------------------------------------------------------------
// Refusals.
// ---------------------------------------------------------------------------

/// Why a result was not returned.
#[derive(Clone, Debug, PartialEq)]
pub enum SlabReject {
    /// `thickness_m` is outside [`THICKNESS_MIN_M`]..=[`THICKNESS_MAX_M`].
    ThicknessOutOfRange { thickness_m: f64 },
    /// `start_temperature_k` is outside
    /// [`START_TEMPERATURE_MIN_K`]..=[`START_TEMPERATURE_MAX_K`].
    StartTemperatureOutOfRange { start_temperature_k: f64 },
    /// `held_temperature_k` is outside
    /// [`HELD_TEMPERATURE_MIN_K`]..=[`HELD_TEMPERATURE_MAX_K`].
    HeldTemperatureOutOfRange { held_temperature_k: f64 },
    /// `hold_duration_s` is outside
    /// [`HOLD_DURATION_MIN_S`]..=[`HOLD_DURATION_MAX_S`].
    HoldDurationOutOfRange { hold_duration_s: f64 },
    /// `continue_after_s` is outside 0..=[`CONTINUE_AFTER_MAX_S`].
    ContinueAfterOutOfRange { continue_after_s: f64 },
    /// More than [`THERMOCOUPLES_MAX`] thermocouple depths were given.
    TooManyThermocouples { count: usize },
    /// A thermocouple depth is not between the heated face and the back.
    ThermocoupleOutsideSlab { index: usize, depth_m: f64 },
    /// A time step did not settle within the Newton pass limit.
    StepNotConverged { time_s: f64, passes: u32 },
    /// A temperature, a density or a flux was NaN or infinite.
    NonFiniteValue { time_s: f64 },
    /// A temperature left its band by more than
    /// [`TEMPERATURE_BOUND_TOLERANCE_K`]. For a held face the bottom of the
    /// band is the start temperature, and the top is the held temperature,
    /// or the temperature below which decomposition gives off heat (about
    /// 939 K) when that is higher: a slab held below it can be warmer inside
    /// than at its face. In a gas stream the bottom is the lower of the start
    /// and the surroundings temperatures, and the top is the top of the
    /// material's tables (3333 K for TACOT); a PICA run above
    /// [`COMPARED_TEMPERATURE_MAX_K`] is refused as
    /// `TemperatureAboveCompared` first.
    TemperatureOutOfBounds {
        time_s: f64,
        depth_m: f64,
        temperature_k: f64,
    },
    /// The solid's density rose somewhere, or fell below char.
    DecompositionRanBackwards { time_s: f64, depth_m: f64 },
    /// The solid lost a different mass from the gas that left.
    MassAccountBroken {
        time_s: f64,
        solid_mass_lost_kg_per_m2: f64,
        gas_mass_out_kg_per_m2: f64,
    },
    /// The heat stored differs from the heat in, the heat of decomposition
    /// and the heat the gas took; or the heat balance at the face is open.
    EnergyAccountBroken {
        time_s: f64,
        residual_j_per_m2: f64,
        tolerance_j_per_m2: f64,
    },
    /// The heating history has fewer than [`HISTORY_ROWS_MIN`] or more than
    /// [`HISTORY_ROWS_MAX`] rows.
    HistoryRowsOutOfRange { rows: usize },
    /// The first row of the heating history is not at time zero, or a row is
    /// not later than the one before it.
    HistoryTimesOutOfOrder { row: usize, time_s: f64 },
    /// The heating history's last row is outside
    /// [`HISTORY_DURATION_MIN_S`]..=[`HISTORY_DURATION_MAX_S`].
    HistoryDurationOutOfRange { duration_s: f64 },
    /// A row of the heating history is less than [`HISTORY_ROW_SPAN_MIN_S`]
    /// after the row before it. A row at the same time as the one before
    /// it, or earlier, is `HistoryTimesOutOfOrder`.
    HistoryRowsTooClose { row: usize, span_s: f64 },
    /// A wall pressure is outside the surface table of the material that
    /// was asked for; the refusal carries that table's two ends
    /// ([`stream_pressure_range_pa`]).
    StreamPressureOutOfRange {
        row: usize,
        pressure_pa: f64,
        lowest_pressure_pa: f64,
        highest_pressure_pa: f64,
    },
    /// A heat-transfer coefficient is outside
    /// 0..=[`HEAT_TRANSFER_COEFFICIENT_MAX_KG_PER_M2_S`].
    HeatTransferCoefficientOutOfRange {
        row: usize,
        heat_transfer_coefficient_kg_per_m2_s: f64,
    },
    /// A flow enthalpy is outside 0..=[`FLOW_ENTHALPY_MAX_J_PER_KG`].
    FlowEnthalpyOutOfRange {
        row: usize,
        flow_enthalpy_j_per_kg: f64,
    },
    /// A radiative heating is outside 0..=[`RADIATIVE_HEATING_MAX_W_PER_M2`].
    RadiativeHeatingOutOfRange {
        row: usize,
        radiative_heating_w_per_m2: f64,
    },
    /// `surroundings_temperature_k` is outside
    /// [`SURROUNDINGS_TEMPERATURE_MIN_K`]..=[`SURROUNDINGS_TEMPERATURE_MAX_K`].
    SurroundingsTemperatureOutOfRange { surroundings_temperature_k: f64 },
    /// `continue_after_s` is not zero with a gas stream. The heating history
    /// is the whole run.
    ContinueAfterWithStream { continue_after_s: f64 },
    /// The face is hotter than the surface table goes at this pressure: at
    /// or above the temperature at which the table's carbon boils.
    FaceBeyondSurfaceTable {
        time_s: f64,
        face_temperature_k: f64,
    },
    /// The face has worn to within [`CELLS_MIN`] cells of the back wall.
    SlabWornAway { time_s: f64, recession_m: f64 },
    /// The char flux at the face came out negative.
    FaceMovedBackward { time_s: f64 },
    /// The material name is not one the solver holds.
    UnknownMaterial { name: String },
    /// The stream's oxygen is outside
    /// [`STREAM_OXYGEN_MASS_FRACTION_MIN`]..=[`STREAM_OXYGEN_MASS_FRACTION_MAX`].
    StreamOxygenOutOfRange { oxygen_mass_fraction: f64 },
    /// The stream's argon is outside 0..=[`STREAM_ARGON_MASS_FRACTION_MAX`].
    StreamArgonOutOfRange { argon_mass_fraction: f64 },
    /// The stream is not air and the material's surface table is for air
    /// alone (TACOT's published table).
    StreamNotAirForThisMaterial { stream_gas: StreamGas },
    /// The chemical equilibrium of the wall gas did not converge at a state
    /// of the surface table being computed for the stream. None has been
    /// seen inside the declared range.
    SurfaceChemistryNotConverged {
        pressure_pa: f64,
        blowing: f64,
        temperature_k: f64,
    },
    /// A surface bundle handed to a study could not be read.
    SurfaceBundleUnreadable { why: String },
    /// The face would have crossed more than the allowed share of its cell
    /// in one step.
    FaceOutranItsCell { time_s: f64 },
    /// A row of the heating history has a stream (a coefficient above zero)
    /// whose flow enthalpy is below the enthalpy of the stream's own gas at a
    /// wall at the lower of the start and surroundings temperatures. Such a
    /// stream draws heat out of a face that cold, below its band.
    FlowEnthalpyBelowWall {
        row: usize,
        flow_enthalpy_j_per_kg: f64,
        wall_enthalpy_j_per_kg: f64,
        wall_temperature_k: f64,
    },
    /// A held face on a PICA material. The PICA model was compared with
    /// measurement in gas streams only.
    HeldFaceNotCompared,
    /// `face_wears` is false on a PICA material. Every measured case wore its
    /// face.
    FaceNotWearingNotCompared,
    /// A PICA slab's thickness is in no envelope compared with measurement:
    /// not [`ARC_JET_THICKNESS_M`], not [`STARDUST_THICKNESS_M`], and outside
    /// [`THERMOCOUPLE_BLOCK_THICKNESS_MIN_M`]..=[`THERMOCOUPLE_BLOCK_THICKNESS_MAX_M`].
    ThicknessNotCompared { thickness_m: f64 },
    /// A PICA slab's start temperature is in no envelope compared with
    /// measurement: not [`ARC_JET_START_TEMPERATURE_K`],
    /// [`STARDUST_START_TEMPERATURE_K`] or [`THERMOCOUPLE_BLOCK_START_TEMPERATURE_K`].
    StartTemperatureNotCompared { start_temperature_k: f64 },
    /// A PICA question's surroundings temperature is in no envelope compared
    /// with measurement: not [`ARC_JET_SURROUNDINGS_TEMPERATURE_K`],
    /// [`STARDUST_SURROUNDINGS_TEMPERATURE_K`] or
    /// [`THERMOCOUPLE_BLOCK_SURROUNDINGS_TEMPERATURE_K`].
    SurroundingsNotCompared { surroundings_temperature_k: f64 },
    /// A PICA gas-stream question lies wholly inside no envelope compared
    /// with measurement, its slab, start and surroundings included: not the
    /// arc-jet tests, not the Stardust flight, not the thermocouple blocks.
    /// The refusal says why for each.
    QuestionNotCompared {
        arc_jet: String,
        stardust: String,
        thermocouple_blocks: String,
    },
    /// A PICA temperature rose above [`COMPARED_TEMPERATURE_MAX_K`], the
    /// hottest any compared run reached.
    TemperatureAboveCompared {
        time_s: f64,
        depth_m: f64,
        temperature_k: f64,
    },
}

impl fmt::Display for SlabReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ThicknessOutOfRange { thickness_m } => write!(
                f,
                "thickness {thickness_m} m is outside {THICKNESS_MIN_M}..={THICKNESS_MAX_M}"
            ),
            Self::StartTemperatureOutOfRange {
                start_temperature_k,
            } => write!(
                f,
                "start temperature {start_temperature_k} K is outside {START_TEMPERATURE_MIN_K}..={START_TEMPERATURE_MAX_K}"
            ),
            Self::HeldTemperatureOutOfRange { held_temperature_k } => write!(
                f,
                "held temperature {held_temperature_k} K is outside {HELD_TEMPERATURE_MIN_K}..={HELD_TEMPERATURE_MAX_K}"
            ),
            Self::HoldDurationOutOfRange { hold_duration_s } => write!(
                f,
                "hold duration {hold_duration_s} s is outside {HOLD_DURATION_MIN_S}..={HOLD_DURATION_MAX_S}"
            ),
            Self::ContinueAfterOutOfRange { continue_after_s } => write!(
                f,
                "time after the hold {continue_after_s} s is outside 0..={CONTINUE_AFTER_MAX_S}"
            ),
            Self::TooManyThermocouples { count } => write!(
                f,
                "{count} thermocouple depths were given; the most is {THERMOCOUPLES_MAX}"
            ),
            Self::ThermocoupleOutsideSlab { index, depth_m } => write!(
                f,
                "thermocouple {index} at depth {depth_m} m is not inside the slab"
            ),
            Self::StepNotConverged { time_s, passes } => write!(
                f,
                "the step ending at {time_s} s had not settled after {passes} Newton passes"
            ),
            Self::NonFiniteValue { time_s } => {
                write!(f, "a value was not finite in the step ending at {time_s} s")
            }
            Self::TemperatureOutOfBounds {
                time_s,
                depth_m,
                temperature_k,
            } => write!(
                f,
                "temperature {temperature_k} K at depth {depth_m} m and {time_s} s is outside its band"
            ),
            Self::DecompositionRanBackwards { time_s, depth_m } => write!(
                f,
                "the solid's density rose or fell below char at depth {depth_m} m in the step ending at {time_s} s"
            ),
            Self::MassAccountBroken {
                time_s,
                solid_mass_lost_kg_per_m2,
                gas_mass_out_kg_per_m2,
            } => write!(
                f,
                "at {time_s} s the solid had lost {solid_mass_lost_kg_per_m2} kg/m2 and {gas_mass_out_kg_per_m2} kg/m2 of gas had left"
            ),
            Self::EnergyAccountBroken {
                time_s,
                residual_j_per_m2,
                tolerance_j_per_m2,
            } => write!(
                f,
                "at {time_s} s the energy account was open by {residual_j_per_m2} J/m2, more than {tolerance_j_per_m2} J/m2"
            ),
            Self::HistoryRowsOutOfRange { rows } => write!(
                f,
                "the heating history has {rows} rows; it may have {HISTORY_ROWS_MIN}..={HISTORY_ROWS_MAX}"
            ),
            Self::HistoryTimesOutOfOrder { row, time_s } => write!(
                f,
                "row {row} of the heating history is at {time_s} s; the first row is at zero and each row is later than the one before"
            ),
            Self::HistoryDurationOutOfRange { duration_s } => write!(
                f,
                "the heating history lasts {duration_s} s, outside {HISTORY_DURATION_MIN_S}..={HISTORY_DURATION_MAX_S}"
            ),
            Self::HistoryRowsTooClose { row, span_s } => write!(
                f,
                "row {row} of the heating history is {span_s:e} s after the row before it; the least time between two rows is {HISTORY_ROW_SPAN_MIN_S} s"
            ),
            Self::StreamPressureOutOfRange {
                row,
                pressure_pa,
                lowest_pressure_pa,
                highest_pressure_pa,
            } => write!(
                f,
                "row {row} of the heating history has pressure {pressure_pa} Pa, outside this material's surface table, {lowest_pressure_pa}..={highest_pressure_pa} Pa"
            ),
            Self::HeatTransferCoefficientOutOfRange {
                row,
                heat_transfer_coefficient_kg_per_m2_s,
            } => write!(
                f,
                "row {row} of the heating history has heat-transfer coefficient {heat_transfer_coefficient_kg_per_m2_s} kg/m2/s, outside 0..={HEAT_TRANSFER_COEFFICIENT_MAX_KG_PER_M2_S}"
            ),
            Self::FlowEnthalpyOutOfRange {
                row,
                flow_enthalpy_j_per_kg,
            } => write!(
                f,
                "row {row} of the heating history has flow enthalpy {flow_enthalpy_j_per_kg} J/kg, outside 0..={FLOW_ENTHALPY_MAX_J_PER_KG}"
            ),
            Self::RadiativeHeatingOutOfRange {
                row,
                radiative_heating_w_per_m2,
            } => write!(
                f,
                "row {row} of the heating history has radiative heating {radiative_heating_w_per_m2} W/m2, outside 0..={RADIATIVE_HEATING_MAX_W_PER_M2}"
            ),
            Self::SurroundingsTemperatureOutOfRange {
                surroundings_temperature_k,
            } => write!(
                f,
                "surroundings temperature {surroundings_temperature_k} K is outside {SURROUNDINGS_TEMPERATURE_MIN_K}..={SURROUNDINGS_TEMPERATURE_MAX_K}"
            ),
            Self::ContinueAfterWithStream { continue_after_s } => write!(
                f,
                "time after the hold is {continue_after_s} s; with a gas stream it is zero and the heating history is the whole run"
            ),
            Self::FaceBeyondSurfaceTable {
                time_s,
                face_temperature_k,
            } => write!(
                f,
                "at {time_s} s the face is at {face_temperature_k} K, hotter than the surface table goes at this pressure"
            ),
            Self::SlabWornAway {
                time_s,
                recession_m,
            } => write!(
                f,
                "at {time_s} s the face had worn {recession_m} m, to within {CELLS_MIN} cells of the back wall"
            ),
            Self::StreamOxygenOutOfRange {
                oxygen_mass_fraction,
            } => write!(
                f,
                "the stream's oxygen mass fraction is {oxygen_mass_fraction}, outside {STREAM_OXYGEN_MASS_FRACTION_MIN}..={STREAM_OXYGEN_MASS_FRACTION_MAX}"
            ),
            Self::StreamArgonOutOfRange {
                argon_mass_fraction,
            } => write!(
                f,
                "the stream's argon mass fraction is {argon_mass_fraction}, outside 0..={STREAM_ARGON_MASS_FRACTION_MAX}"
            ),
            Self::StreamNotAirForThisMaterial { stream_gas } => write!(
                f,
                "the stream has oxygen mass fraction {} and argon mass fraction {}; this material's surface table is for air alone ({AIR_OXYGEN_MASS_FRACTION} and 0)",
                stream_gas.oxygen_mass_fraction, stream_gas.argon_mass_fraction
            ),
            Self::SurfaceChemistryNotConverged {
                pressure_pa,
                blowing,
                temperature_k,
            } => write!(
                f,
                "the wall gas's chemical equilibrium did not converge at {pressure_pa} Pa, B'g {blowing}, {temperature_k} K while the surface table for this stream was being computed"
            ),
            Self::UnknownMaterial { name } => write!(
                f,
                "no material is named {name:?}; the names are {MATERIAL_TACOT:?}, {MATERIAL_PICA_VERSION_5:?}, {MATERIAL_PICA_VERSION_4:?}, {MATERIAL_PICA_VERSION_3:?} and {MATERIAL_PICA:?}"
            ),
            Self::SurfaceBundleUnreadable { why } => {
                write!(f, "the surface bundle could not be read: {why}")
            }
            Self::FaceMovedBackward { time_s } => {
                write!(
                    f,
                    "the char flux at the face was negative in the step ending at {time_s} s"
                )
            }
            Self::FaceOutranItsCell { time_s } => write!(
                f,
                "the face would have crossed too much of its cell in the step ending at {time_s} s"
            ),
            Self::FlowEnthalpyBelowWall {
                row,
                flow_enthalpy_j_per_kg,
                wall_enthalpy_j_per_kg,
                wall_temperature_k,
            } => write!(
                f,
                "row {row} of the heating history has a stream of flow enthalpy {flow_enthalpy_j_per_kg} J/kg, below {wall_enthalpy_j_per_kg} J/kg, the enthalpy of the stream's gas at a wall at {wall_temperature_k} K"
            ),
            Self::HeldFaceNotCompared => write!(
                f,
                "a held face on PICA was never compared with measurement; the PICA model takes a gas stream"
            ),
            Self::FaceNotWearingNotCompared => write!(
                f,
                "a PICA face that does not wear was never compared with measurement"
            ),
            Self::ThicknessNotCompared { thickness_m } => write!(
                f,
                "thickness {thickness_m} m is in no PICA envelope compared with measurement: {ARC_JET_THICKNESS_M} m for the arc-jet tests, {STARDUST_THICKNESS_M} m for the Stardust flight, {THERMOCOUPLE_BLOCK_THICKNESS_MIN_M}..={THERMOCOUPLE_BLOCK_THICKNESS_MAX_M} m for the thermocouple blocks"
            ),
            Self::StartTemperatureNotCompared {
                start_temperature_k,
            } => write!(
                f,
                "start temperature {start_temperature_k} K is in no PICA envelope compared with measurement: {ARC_JET_START_TEMPERATURE_K} K for the arc-jet tests, {STARDUST_START_TEMPERATURE_K} K for the Stardust flight, {THERMOCOUPLE_BLOCK_START_TEMPERATURE_K} K for the thermocouple blocks"
            ),
            Self::SurroundingsNotCompared {
                surroundings_temperature_k,
            } => write!(
                f,
                "surroundings temperature {surroundings_temperature_k} K is in no PICA envelope compared with measurement: {ARC_JET_SURROUNDINGS_TEMPERATURE_K} K for the arc-jet tests, {STARDUST_SURROUNDINGS_TEMPERATURE_K} K for the Stardust flight, {THERMOCOUPLE_BLOCK_SURROUNDINGS_TEMPERATURE_K} K for the thermocouple blocks"
            ),
            Self::QuestionNotCompared {
                arc_jet,
                stardust,
                thermocouple_blocks,
            } => write!(
                f,
                "the question lies inside no envelope the PICA model was compared with measurement on: for the arc-jet tests, {arc_jet}; for the Stardust flight, {stardust}; for the thermocouple blocks, {thermocouple_blocks}"
            ),
            Self::TemperatureAboveCompared {
                time_s,
                depth_m,
                temperature_k,
            } => write!(
                f,
                "temperature {temperature_k} K at depth {depth_m} m and {time_s} s is above {COMPARED_TEMPERATURE_MAX_K} K, the hottest any PICA run compared with measurement reached"
            ),
        }
    }
}

impl std::error::Error for SlabReject {}

impl SlabReject {
    /// The refusal's name, as a caller's program reads it.
    pub fn name(&self) -> &'static str {
        match self {
            Self::ThicknessOutOfRange { .. } => "ThicknessOutOfRange",
            Self::StartTemperatureOutOfRange { .. } => "StartTemperatureOutOfRange",
            Self::HeldTemperatureOutOfRange { .. } => "HeldTemperatureOutOfRange",
            Self::HoldDurationOutOfRange { .. } => "HoldDurationOutOfRange",
            Self::ContinueAfterOutOfRange { .. } => "ContinueAfterOutOfRange",
            Self::TooManyThermocouples { .. } => "TooManyThermocouples",
            Self::ThermocoupleOutsideSlab { .. } => "ThermocoupleOutsideSlab",
            Self::StepNotConverged { .. } => "StepNotConverged",
            Self::NonFiniteValue { .. } => "NonFiniteValue",
            Self::TemperatureOutOfBounds { .. } => "TemperatureOutOfBounds",
            Self::DecompositionRanBackwards { .. } => "DecompositionRanBackwards",
            Self::MassAccountBroken { .. } => "MassAccountBroken",
            Self::EnergyAccountBroken { .. } => "EnergyAccountBroken",
            Self::HistoryRowsOutOfRange { .. } => "HistoryRowsOutOfRange",
            Self::HistoryTimesOutOfOrder { .. } => "HistoryTimesOutOfOrder",
            Self::HistoryDurationOutOfRange { .. } => "HistoryDurationOutOfRange",
            Self::HistoryRowsTooClose { .. } => "HistoryRowsTooClose",
            Self::StreamPressureOutOfRange { .. } => "StreamPressureOutOfRange",
            Self::HeatTransferCoefficientOutOfRange { .. } => "HeatTransferCoefficientOutOfRange",
            Self::FlowEnthalpyOutOfRange { .. } => "FlowEnthalpyOutOfRange",
            Self::RadiativeHeatingOutOfRange { .. } => "RadiativeHeatingOutOfRange",
            Self::SurroundingsTemperatureOutOfRange { .. } => "SurroundingsTemperatureOutOfRange",
            Self::ContinueAfterWithStream { .. } => "ContinueAfterWithStream",
            Self::FaceBeyondSurfaceTable { .. } => "FaceBeyondSurfaceTable",
            Self::SlabWornAway { .. } => "SlabWornAway",
            Self::FaceMovedBackward { .. } => "FaceMovedBackward",
            Self::FaceOutranItsCell { .. } => "FaceOutranItsCell",
            Self::UnknownMaterial { .. } => "UnknownMaterial",
            Self::StreamOxygenOutOfRange { .. } => "StreamOxygenOutOfRange",
            Self::StreamArgonOutOfRange { .. } => "StreamArgonOutOfRange",
            Self::StreamNotAirForThisMaterial { .. } => "StreamNotAirForThisMaterial",
            Self::SurfaceChemistryNotConverged { .. } => "SurfaceChemistryNotConverged",
            Self::SurfaceBundleUnreadable { .. } => "SurfaceBundleUnreadable",
            Self::FlowEnthalpyBelowWall { .. } => "FlowEnthalpyBelowWall",
            Self::HeldFaceNotCompared => "HeldFaceNotCompared",
            Self::FaceNotWearingNotCompared => "FaceNotWearingNotCompared",
            Self::ThicknessNotCompared { .. } => "ThicknessNotCompared",
            Self::StartTemperatureNotCompared { .. } => "StartTemperatureNotCompared",
            Self::SurroundingsNotCompared { .. } => "SurroundingsNotCompared",
            Self::QuestionNotCompared { .. } => "QuestionNotCompared",
            Self::TemperatureAboveCompared { .. } => "TemperatureAboveCompared",
        }
    }
}

// ---------------------------------------------------------------------------
// The result.
// ---------------------------------------------------------------------------

/// What the slab did. Every number is finite.
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub(crate) struct SlabResult {
    /// Times of the histories below, seconds.
    pub report_time_s: Vec<f64>,
    /// Temperature of the heated face at each report time, kelvin.
    pub surface_temperature_k: Vec<f64>,
    /// The thermocouple depths as asked, metres.
    pub thermocouple_depth_m: Vec<f64>,
    /// Temperature at each thermocouple at each report time, kelvin:
    /// `thermocouple_temperature_k[thermocouple][time]`.
    pub thermocouple_temperature_k: Vec<Vec<f64>>,
    /// Gas leaving through the heated face at each report time, averaged over
    /// the time step that ends there, kg/m²/s.
    pub gas_mass_flux_at_surface_kg_per_m2_s: Vec<f64>,
    /// Depth at which the density is 221.2 kg/m³ (2 percent of the way from
    /// char to virgin) at each report time, metres. Zero while the face itself
    /// is denser than that.
    pub char_front_depth_m: Vec<f64>,
    /// Depth at which the density is 278.8 kg/m³ (98 percent of the way from
    /// char to virgin) at each report time, metres.
    pub virgin_front_depth_m: Vec<f64>,
    /// Depth of each cell centre, metres.
    pub final_depth_m: Vec<f64>,
    /// Temperature of each cell at the end, kelvin.
    pub final_temperature_k: Vec<f64>,
    /// Density of the solid in each cell at the end, kg/m³.
    pub final_density_kg_per_m3: Vec<f64>,
    /// Density of the solid at the heated face at the end, kg/m³.
    pub final_surface_density_kg_per_m3: f64,
    /// Heat conducted into the slab through the heated face, J/m². Negative
    /// when more left than entered.
    pub heat_in_through_face_j_per_m2: f64,
    /// Heat stored in the solid as temperature rise, `∫ρ c_p dT`, J/m².
    pub heat_stored_in_solid_j_per_m2: f64,
    /// Heat the gas took from the solid on its way to the face, J/m².
    pub heat_taken_by_gas_j_per_m2: f64,
    /// Heat given off by decomposition, J/m². Negative when taken up.
    pub heat_given_off_by_decomposition_j_per_m2: f64,
    /// `stored − in − given off + taken by gas`, J/m².
    pub energy_account_residual_j_per_m2: f64,
    /// The residual above was required to be no larger than this, J/m².
    pub energy_account_tolerance_j_per_m2: f64,
    /// Enthalpy of the solid at the end minus at the start, formation
    /// included, J/m². Computed from the two states alone.
    pub solid_enthalpy_change_j_per_m2: f64,
    /// Enthalpy carried out through the heated face by the gas, J/m².
    pub gas_enthalpy_out_j_per_m2: f64,
    /// Mass the solid lost, from its density at the start and at the end, kg/m².
    pub solid_mass_lost_kg_per_m2: f64,
    /// Gas that left through the heated face, summed over the steps, kg/m².
    pub gas_mass_out_kg_per_m2: f64,
    /// `solid lost − gas out`, kg/m².
    pub mass_account_residual_kg_per_m2: f64,
    /// The residual above was required to be no larger than this, kg/m².
    pub mass_account_tolerance_kg_per_m2: f64,
    /// Cells through the thickness.
    pub cells: u32,
    /// Time steps taken.
    pub time_steps: u32,
    /// Newton passes in the step that needed the most.
    pub hardest_step_newton_passes: u32,
    /// End time of that step, seconds.
    pub hardest_step_time_s: f64,
    /// How far the face has worn inward at each report time, metres.
    pub recession_m: Vec<f64>,
    /// Char leaving the face at each report time, averaged over the time step
    /// that ends there, kg/m²/s.
    pub char_mass_flux_kg_per_m2_s: Vec<f64>,
    /// Char worn off the face, summed over the steps, kg/m².
    pub char_mass_removed_kg_per_m2: f64,
    /// Enthalpy of that char as it left, at the face temperature, J/m².
    pub char_enthalpy_removed_j_per_m2: f64,
    /// Heat from the gas stream, after the blowing correction, J/m².
    pub heat_from_stream_j_per_m2: f64,
    /// Radiative heating taken up by the face, J/m².
    pub heat_absorbed_from_radiation_j_per_m2: f64,
    /// Heat radiated by the face to its surroundings, J/m².
    pub heat_radiated_away_j_per_m2: f64,
    /// Heat taken at the face by the decomposition gas and the char as they
    /// become wall gas, J/m².
    pub heat_taken_at_face_by_gases_j_per_m2: f64,
    /// `conducted in − (from stream + absorbed − radiated − taken by gases)`, J/m².
    pub face_balance_residual_j_per_m2: f64,
    /// The residual above was required to be no larger than this, J/m².
    pub face_balance_tolerance_j_per_m2: f64,
    /// Cells left at the end. A wearing face uses cells up.
    pub cells_at_end: u32,
    /// Depth below the face as it ended at which the density is half way
    /// from char to virgin, and within 2 percent of virgin density, metres.
    pub depth_half_way_to_virgin_m: f64,
    pub depth_within_2_percent_of_virgin_m: f64,
}

// ---------------------------------------------------------------------------
// Material functions.
// ---------------------------------------------------------------------------

/// A material made ready: its enthalpies of virgin and char at the table
/// temperatures, J/kg, the trapezoid integral of the specific heat from the
/// anchor temperature.
struct SolidEnthalpyNodes<'m> {
    material: &'m Material,
    /// Pressure in the pores, Pa, for a material whose conductivity depends
    /// on it; set at each step from the wall pressure. Not a number until
    /// then, and the conductivity curves are then used as they stand.
    pore_pressure_pa: std::cell::Cell<f64>,
    virgin_j_per_kg: Vec<f64>,
    char_j_per_kg: Vec<f64>,
}

fn enthalpy_nodes(
    table_k: &[f64],
    specific_heat: &[f64],
    anchor: usize,
    at_anchor: f64,
) -> Vec<f64> {
    let mut h = vec![0.0; table_k.len()];
    h[anchor] = at_anchor;
    for j in anchor..table_k.len() - 1 {
        h[j + 1] =
            h[j] + 0.5 * (specific_heat[j] + specific_heat[j + 1]) * (table_k[j + 1] - table_k[j]);
    }
    for j in (0..anchor).rev() {
        h[j] = h[j + 1]
            - 0.5 * (specific_heat[j] + specific_heat[j + 1]) * (table_k[j + 1] - table_k[j]);
    }
    h
}

#[cfg(test)]
impl SolidEnthalpyNodes<'static> {
    /// TACOT 3.0, by the name the stage-one tests use.
    fn new() -> Self {
        Self::of(&TACOT)
    }
}

impl<'m> SolidEnthalpyNodes<'m> {
    fn of(material: &'m Material) -> Self {
        Self {
            material,
            pore_pressure_pa: std::cell::Cell::new(f64::NAN),
            virgin_j_per_kg: enthalpy_nodes(
                material.solid_table_k,
                material.virgin_specific_heat_j_per_kg_k,
                material.solid_anchor_node,
                material.virgin_enthalpy_at_anchor_j_per_kg,
            ),
            char_j_per_kg: enthalpy_nodes(
                material.solid_table_k,
                material.char_specific_heat_j_per_kg_k,
                material.solid_anchor_node,
                material.char_enthalpy_at_anchor_j_per_kg,
            ),
        }
    }
}

/// Virgin and char properties at one temperature.
#[derive(Clone, Copy, Debug)]
struct SolidPoint {
    virgin_specific_heat: f64,
    char_specific_heat: f64,
    virgin_enthalpy: f64,
    char_enthalpy: f64,
    virgin_conductivity: f64,
    char_conductivity: f64,
    /// Slopes of the two conductivities in temperature, W/m/K².
    virgin_conductivity_slope: f64,
    char_conductivity_slope: f64,
    /// Densities of virgin material and of char, kg/m³.
    virgin_density: f64,
    char_density: f64,
}

/// Segment of a rising table that holds `value`. Outside the table the end
/// segment is used, which continues its straight line. No state is accepted
/// outside the tables: see `ensure_temperature_in_band`.
fn segment(table: &[f64], value: f64) -> usize {
    let mut j = 0;
    while j + 2 < table.len() && value > table[j + 1] {
        j += 1;
    }
    j
}

/// Specific heat and conductivity are straight lines between table
/// temperatures. Enthalpy is the exact integral of that specific heat, so it
/// is a parabola between table temperatures and equals the node values.
fn solid_point(
    nodes: &SolidEnthalpyNodes,
    temperature_k: f64,
    frozen_at_k: Option<f64>,
) -> SolidPoint {
    let m = nodes.material;
    let read_at = frozen_at_k.unwrap_or(temperature_k);
    let j = segment(m.solid_table_k, read_at);
    let span = m.solid_table_k[j + 1] - m.solid_table_k[j];
    let d = read_at - m.solid_table_k[j];
    let line = |table: &[f64]| {
        let slope = (table[j + 1] - table[j]) / span;
        (table[j] + slope * d, slope)
    };
    let (cv, cv_slope) = line(m.virgin_specific_heat_j_per_kg_k);
    let (cc, cc_slope) = line(m.char_specific_heat_j_per_kg_k);
    let (mut kv, mut kv_slope) = line(m.virgin_conductivity_w_per_m_k);
    let (mut kc, mut kc_slope) = line(m.char_conductivity_w_per_m_k);
    if let (Some(pore_gas), true) = (m.pore_gas, nodes.pore_pressure_pa.get().is_finite()) {
        let [virgin, char] = pore_gas.departure(read_at, nodes.pore_pressure_pa.get());
        kv += virgin.0;
        kv_slope += virgin.1;
        kc += char.0;
        kc_slope += char.1;
    }
    let hv =
        nodes.virgin_j_per_kg[j] + (m.virgin_specific_heat_j_per_kg_k[j] + 0.5 * cv_slope * d) * d;
    let hc = nodes.char_j_per_kg[j] + (m.char_specific_heat_j_per_kg_k[j] + 0.5 * cc_slope * d) * d;
    match frozen_at_k {
        None => SolidPoint {
            virgin_specific_heat: cv,
            char_specific_heat: cc,
            virgin_enthalpy: hv,
            char_enthalpy: hc,
            virgin_conductivity: kv,
            char_conductivity: kc,
            virgin_conductivity_slope: kv_slope,
            char_conductivity_slope: kc_slope,
            virgin_density: m.virgin_density_kg_per_m3,
            char_density: m.char_density_kg_per_m3,
        },
        Some(frozen) => SolidPoint {
            virgin_specific_heat: cv,
            char_specific_heat: cc,
            virgin_enthalpy: hv + cv * (temperature_k - frozen),
            char_enthalpy: hc + cc * (temperature_k - frozen),
            virgin_conductivity: kv,
            char_conductivity: kc,
            virgin_conductivity_slope: 0.0,
            char_conductivity_slope: 0.0,
            virgin_density: m.virgin_density_kg_per_m3,
            char_density: m.char_density_kg_per_m3,
        },
    }
}

impl Material {
    /// Enthalpy of the decomposition gas and its slope in temperature, J/kg
    /// and J/kg/K: straight lines between the table's temperatures.
    fn gas_enthalpy(&self, temperature_k: f64) -> (f64, f64) {
        let table = self.gas_enthalpy_j_per_kg;
        let position = (temperature_k - self.gas_table_first_k) / self.gas_table_step_k;
        let mut j = 0usize;
        if position > 0.0 {
            j = position as usize;
            if j > table.len() - 2 {
                j = table.len() - 2;
            }
        }
        let slope = (table[j + 1] - table[j]) / self.gas_table_step_k;
        let from = self.gas_table_first_k + self.gas_table_step_k * j as f64;
        (table[j] + slope * (temperature_k - from), slope)
    }

    /// Density of the solid, kg/m³ of material, from how much of each resin
    /// part is left. `remaining[i]` is `(ρ_i − ρ_ci) / ρ_vi`: 1 and 1/3 in
    /// virgin TACOT, 0 and 0 in char.
    fn solid_density(&self, remaining: [f64; 2]) -> f64 {
        let a = &self.resin_parts[0];
        let b = &self.resin_parts[1];
        let resin = (a.char_density_kg_per_m3 + a.virgin_density_kg_per_m3 * remaining[0])
            + (b.char_density_kg_per_m3 + b.virgin_density_kg_per_m3 * remaining[1]);
        self.solid_volume_fraction
            * (self.resin_volume_fraction * resin
                + (1.0 - self.resin_volume_fraction) * self.fibre_density_kg_per_m3)
    }

    /// `remaining` of each part in virgin material.
    fn virgin_remaining(&self) -> [f64; 2] {
        let part = |p: &ResinPart| {
            (p.virgin_density_kg_per_m3 - p.char_density_kg_per_m3) / p.virgin_density_kg_per_m3
        };
        [part(&self.resin_parts[0]), part(&self.resin_parts[1])]
    }

    /// Density of the solid per unit of `remaining` of one part, kg/m³.
    fn part_weight(&self, part: usize) -> f64 {
        self.solid_volume_fraction
            * self.resin_volume_fraction
            * self.resin_parts[part].virgin_density_kg_per_m3
    }

    /// Emissivity mixed by the mass fraction of virgin material, and its
    /// slope in density.
    fn emissivity(&self, density: f64) -> (f64, f64) {
        let span = self.virgin_density_kg_per_m3 - self.char_density_kg_per_m3;
        let fraction = self.virgin_density_kg_per_m3 * (density - self.char_density_kg_per_m3)
            / (density * span);
        let fraction_slope = self.virgin_density_kg_per_m3 * self.char_density_kg_per_m3
            / (density * density * span);
        let step = self.virgin_emissivity - self.char_emissivity;
        (
            self.char_emissivity + step * fraction,
            step * fraction_slope,
        )
    }
}

/// TACOT's gas enthalpy, by the name the stage-one tests use.
#[cfg(test)]
fn gas_enthalpy(temperature_k: f64) -> (f64, f64) {
    TACOT.gas_enthalpy(temperature_k)
}

/// TACOT's solid density, by the name the stage-one tests use.
#[cfg(test)]
fn solid_density(remaining: [f64; 2]) -> f64 {
    TACOT.solid_density(remaining)
}

/// `remaining` in virgin TACOT, by the name the stage-one tests use.
#[cfg(test)]
fn virgin_remaining() -> [f64; 2] {
    TACOT.virgin_remaining()
}

/// One resin part over one time step with the temperature held: the exact
/// integral of `du/dt = −A u³ exp(−(E/R)/T)`. Returns what is left and its
/// slope in temperature. What is left is never more than `remaining`, at any
/// temperature and for any step.
fn decomposed(
    part: &ResinPart,
    remaining: f64,
    temperature_k: f64,
    active: bool,
    step_s: f64,
) -> (f64, f64) {
    if !active || remaining <= 0.0 {
        return (remaining, 0.0);
    }
    let rate = part.rate_constant_per_s * (-part.activation_temperature_k / temperature_k).exp();
    let used = rate * step_s * remaining * remaining;
    let growth = 1.0 + 2.0 * used;
    let root = growth.sqrt();
    let left = remaining / root;
    let slope = -remaining * used * part.activation_temperature_k
        / (temperature_k * temperature_k * growth * root);
    (left, slope)
}

/// Enthalpy of the solid per volume of material, J/m³, at density
/// `density` and the temperature of `point`:
/// `[ρ_v h_v (ρ − ρ_c) + ρ_c h_c (ρ_v − ρ)] / (ρ_v − ρ_c)`.
fn solid_enthalpy_per_volume(point: &SolidPoint, density: f64) -> f64 {
    (point.virgin_density * point.virgin_enthalpy * (density - point.char_density)
        + point.char_density * point.char_enthalpy * (point.virgin_density - density))
        / (point.virgin_density - point.char_density)
}

/// `ρ c_p`, J/m³/K, with the specific heat mixed by the mass fraction of
/// virgin material.
fn heat_capacity_per_volume(point: &SolidPoint, density: f64) -> f64 {
    (point.virgin_density * point.virgin_specific_heat * (density - point.char_density)
        + point.char_density * point.char_specific_heat * (point.virgin_density - density))
        / (point.virgin_density - point.char_density)
}

/// `h̄ = (ρ_v h_v − ρ_c h_c) / (ρ_v − ρ_c)`, J/kg: the enthalpy the solid
/// gives up per kilogram of gas made.
fn enthalpy_of_solid_lost(point: &SolidPoint) -> f64 {
    (point.virgin_density * point.virgin_enthalpy - point.char_density * point.char_enthalpy)
        / (point.virgin_density - point.char_density)
}

/// Slope of `h̄` in temperature, J/kg/K.
fn enthalpy_of_solid_lost_slope(point: &SolidPoint) -> f64 {
    (point.virgin_density * point.virgin_specific_heat
        - point.char_density * point.char_specific_heat)
        / (point.virgin_density - point.char_density)
}

/// Conductivity mixed by the mass fraction of virgin material
/// `τ = ρ_v (ρ − ρ_c) / (ρ (ρ_v − ρ_c))`, W/m/K, with its slopes in
/// temperature and in density.
fn conductivity(point: &SolidPoint, density: f64) -> (f64, f64, f64) {
    let span = point.virgin_density - point.char_density;
    let virgin_fraction = point.virgin_density * (density - point.char_density) / (density * span);
    let fraction_slope = point.virgin_density * point.char_density / (density * density * span);
    let k = virgin_fraction * point.virgin_conductivity
        + (1.0 - virgin_fraction) * point.char_conductivity;
    let k_temperature = virgin_fraction * point.virgin_conductivity_slope
        + (1.0 - virgin_fraction) * point.char_conductivity_slope;
    let k_density = (point.virgin_conductivity - point.char_conductivity) * fraction_slope;
    (k, k_temperature, k_density)
}

/// Temperature below which decomposition gives off heat and above which it
/// takes heat up: where `h_g = h̄`, kelvin. About 939 K for TACOT. Found by
/// halving the interval between two table temperatures that bracket it.
fn heat_release_ends_k(nodes: &SolidEnthalpyNodes) -> f64 {
    let heat = |t: f64| {
        nodes.material.gas_enthalpy(t).0 - enthalpy_of_solid_lost(&solid_point(nodes, t, None))
    };
    let table = nodes.material.solid_table_k;
    let mut j = 0;
    while j + 2 < table.len() && heat(table[j + 1]) < 0.0 {
        j += 1;
    }
    let mut below = table[j];
    let mut above = table[j + 1];
    for _ in 0..60 {
        let middle = 0.5 * (below + above);
        if heat(middle) < 0.0 {
            below = middle;
        } else {
            above = middle;
        }
    }
    above
}

// ---------------------------------------------------------------------------
// The surface table.
// ---------------------------------------------------------------------------

/// A surface table made ready to be read.
///
/// What is interpolated is not B'c but the share of the wall gas that came
/// from the char, `χ = B'c / (1 + B'g + B'c)`, and the wall-gas enthalpy.
/// Both are read along straight lines in the logarithm of pressure, in B'g
/// and in temperature; then `B'c = χ (1 + B'g) / (1 − χ)`. B'c runs away
/// toward infinity where carbon boils and χ runs smoothly toward 1, so
/// straight lines in χ follow the table where straight lines in B'c cut
/// across it. In pressure the logarithm is used because the temperature at
/// which carbon boils moves by about the same amount for each tenfold
/// change in pressure.
struct SurfaceLookup<'m> {
    table: &'m SurfaceTable,
    /// χ at each node, in the table's order.
    share: Vec<f64>,
    /// Natural logarithm of each table pressure.
    log_pressure: Vec<f64>,
    /// For each pressure and B'g, the last temperature node at which the
    /// table still gives a value of its own. Above it the sheet repeats that
    /// value (carbon has boiled).
    last_own_node: Vec<usize>,
}

/// The wall gas at one pressure, B'g and wall temperature.
#[derive(Clone, Copy, Debug, Default)]
struct WallGas {
    /// B'c and its slopes in B'g and in temperature.
    char_rate: f64,
    char_rate_blowing: f64,
    char_rate_temperature: f64,
    /// Enthalpy of the gas mixture at the wall, J/kg, and its slopes.
    enthalpy: f64,
    enthalpy_blowing: f64,
    enthalpy_temperature: f64,
    /// The temperature is above a node at which the sheet stops giving
    /// values of its own, or outside the table's temperatures.
    beyond_table: bool,
}

impl<'m> SurfaceLookup<'m> {
    fn of(table: &'m SurfaceTable) -> Self {
        let nb = table.blowing.len();
        let nt = table.temperature_k.len();
        let mut share = Vec::with_capacity(table.char_rate.len());
        let mut last_own_node = Vec::with_capacity(table.pressure_pa.len() * nb);
        for ip in 0..table.pressure_pa.len() {
            for ib in 0..nb {
                let column = &table.char_rate[(ip * nb + ib) * nt..(ip * nb + ib + 1) * nt];
                for &char_rate in column {
                    share.push(char_rate / (1.0 + table.blowing[ib] + char_rate));
                }
                let mut last = nt - 1;
                while last > 0 && column[last] > 0.0 && column[last] == column[last - 1] {
                    last -= 1;
                }
                last_own_node.push(last);
            }
        }
        Self {
            table,
            share,
            log_pressure: table.pressure_pa.iter().map(|p| p.ln()).collect(),
            last_own_node,
        }
    }

    /// Largest B'g in the table.
    fn largest_blowing(&self) -> f64 {
        self.table.blowing[self.table.blowing.len() - 1]
    }

    /// The wall gas at `pressure_pa`, `blowing` (B'g) and `temperature_k`.
    /// B'g above the table's largest is read at the largest. With
    /// `char_rate_along_straight_lines` B'c itself is interpolated instead of
    /// χ; the solver never does that, and a test uses it to show how little
    /// the choice matters where the workshop cases sit.
    fn read(
        &self,
        pressure_pa: f64,
        blowing: f64,
        temperature_k: f64,
        char_rate_along_straight_lines: bool,
    ) -> WallGas {
        let t = self.table;
        let nb = t.blowing.len();
        let nt = t.temperature_k.len();
        // Pressure.
        let (ip, wp) = if t.pressure_pa.len() == 1 {
            (0, 0.0)
        } else {
            let ip = segment(t.pressure_pa, pressure_pa);
            let w = (pressure_pa.ln() - self.log_pressure[ip])
                / (self.log_pressure[ip + 1] - self.log_pressure[ip]);
            (ip, w)
        };
        // B'g, held at the table's largest beyond it.
        let top = self.largest_blowing();
        let (b, inside) = if blowing >= top {
            (top, false)
        } else {
            (blowing, true)
        };
        let ib = segment(t.blowing, b);
        let b_span = t.blowing[ib + 1] - t.blowing[ib];
        let wb = (b - t.blowing[ib]) / b_span;
        // Temperature.
        let it = segment(t.temperature_k, temperature_k);
        let t_span = t.temperature_k[it + 1] - t.temperature_k[it];
        let wt = (temperature_k - t.temperature_k[it]) / t_span;

        let mut beyond_table =
            temperature_k < t.temperature_k[0] || temperature_k > t.temperature_k[nt - 1];
        let mut value = [0.0; 2];
        let mut value_blowing = [0.0; 2];
        let mut value_temperature = [0.0; 2];
        let pressures = if t.pressure_pa.len() == 1 { 1 } else { 2 };
        for dp in 0..pressures {
            let weight_p = if pressures == 1 {
                1.0
            } else if dp == 0 {
                1.0 - wp
            } else {
                wp
            };
            for db in 0..2 {
                let weight_b = if db == 0 { 1.0 - wb } else { wb };
                let column = (ip + dp) * nb + ib + db;
                if weight_p * weight_b != 0.0
                    && temperature_k > t.temperature_k[self.last_own_node[column]]
                {
                    beyond_table = true;
                }
                let at = column * nt + it;
                let first: [&[f64]; 2] = if char_rate_along_straight_lines {
                    [t.char_rate, t.wall_enthalpy_j_per_kg]
                } else {
                    [&self.share, t.wall_enthalpy_j_per_kg]
                };
                for q in 0..2 {
                    let low = first[q][at];
                    let high = first[q][at + 1];
                    let here = low + (high - low) * wt;
                    value[q] += weight_p * weight_b * here;
                    value_temperature[q] += weight_p * weight_b * (high - low) / t_span;
                    let sign = if db == 0 { -1.0 } else { 1.0 };
                    value_blowing[q] += weight_p * sign * here / b_span;
                }
            }
        }
        if !inside {
            value_blowing = [0.0; 2];
        }
        let (char_rate, char_rate_blowing, char_rate_temperature) =
            if char_rate_along_straight_lines {
                (value[0], value_blowing[0], value_temperature[0])
            } else {
                // B'c = χ (1 + B'g) / (1 − χ), with B'g as read (held at the top).
                let share = value[0];
                let room = 1.0 - share;
                let lift = (1.0 + b) / (room * room);
                let own = if inside { share / room } else { 0.0 };
                (
                    share * (1.0 + b) / room,
                    value_blowing[0] * lift + own,
                    value_temperature[0] * lift,
                )
            };
        WallGas {
            char_rate,
            char_rate_blowing,
            char_rate_temperature,
            enthalpy: value[1],
            enthalpy_blowing: value_blowing[1],
            enthalpy_temperature: value_temperature[1],
            beyond_table,
        }
    }
}

/// The blowing correction `Ω = ln(1 + 2λB') / (2λB')` and its slope in B'.
fn blowing_correction(blowing_total: f64) -> (f64, f64) {
    let x = 2.0 * BLOWING_CORRECTION_LAMBDA * blowing_total;
    if x.abs() < 1.0e-4 {
        (
            1.0 - x / 2.0 + x * x / 3.0 - x * x * x / 4.0,
            2.0 * BLOWING_CORRECTION_LAMBDA * (-0.5 + 2.0 * x / 3.0 - 0.75 * x * x),
        )
    } else {
        let log = x.ln_1p();
        (
            log / x,
            2.0 * BLOWING_CORRECTION_LAMBDA * (x / (1.0 + x) - log) / (x * x),
        )
    }
}

/// The stream at the face at one instant.
#[derive(Clone, Copy, Debug)]
struct StreamNow {
    pressure_pa: f64,
    /// Heat-transfer coefficient without blowing, `ρ_e u_e C_H`, kg/m²/s.
    coefficient: f64,
    flow_enthalpy_j_per_kg: f64,
    radiative_heating_w_per_m2: f64,
    surroundings_temperature_k: f64,
    /// Whether there is a stream: its coefficient is above zero. With no
    /// stream there is no convective heating and no char is consumed, and the
    /// decomposition gas leaves at its own enthalpy. Radiation remains.
    present: bool,
    face_wears: bool,
}

/// What the wall chemistry and the blowing give for a wall temperature and a
/// decomposition-gas flux arriving at the wall, with slopes in both.
#[derive(Clone, Copy, Debug, Default)]
struct Blowing {
    /// B'g. Read by the tests.
    #[allow(dead_code)]
    blowing: f64,
    /// Ω.
    correction: f64,
    correction_temperature: f64,
    correction_flux: f64,
    /// B'c as used: the table's, or zero for a face that does not wear.
    char_rate: f64,
    char_rate_temperature: f64,
    char_rate_flux: f64,
    wall_enthalpy: f64,
    wall_enthalpy_temperature: f64,
    wall_enthalpy_flux: f64,
    beyond_table: bool,
    /// B'g came out above the table's largest. Read by the tests.
    #[allow(dead_code)]
    past_largest_blowing: bool,
}

/// Solves `B'g · Ω(B'g + B'c(B'g, T_w)) = ṁ_g / (ρ_e u_e C_H)` for B'g:
/// the decomposition gas is measured against the blown coefficient, which
/// itself depends on the blowing. Newton's method kept inside a bracket.
fn blowing_state(
    lookup: &SurfaceLookup,
    stream: &StreamNow,
    gas_flux: f64,
    wall_temperature_k: f64,
    gas_enthalpy: (f64, f64),
    char_rate_along_straight_lines: bool,
) -> Blowing {
    // Past this the stream's share of the wall gas is below e⁻⁵⁰⁰.
    let target = (gas_flux / stream.coefficient).clamp(0.0, 500.0);
    let top = lookup.largest_blowing();
    // Beyond the table's largest B'g the wall gas is the table's gas at the
    // largest B'g with the further decomposition gas added to it unchanged,
    // so that it runs smoothly to the gas's own enthalpy as the stream
    // fades. B'c stays at its value at the largest B'g.
    let read = |b: f64| {
        let mut gas = lookup.read(
            stream.pressure_pa,
            b,
            wall_temperature_k,
            char_rate_along_straight_lines,
        );
        if b > top {
            let table_mass = 1.0 + top + gas.char_rate;
            let mass = 1.0 + b + gas.char_rate;
            let share = table_mass / mass;
            let above = gas.enthalpy - gas_enthalpy.0;
            let share_temperature = gas.char_rate_temperature * (b - top) / (mass * mass);
            gas.enthalpy_temperature = gas_enthalpy.1
                + (gas.enthalpy_temperature - gas_enthalpy.1) * share
                + above * share_temperature;
            gas.enthalpy = gas_enthalpy.0 + above * share;
            gas.enthalpy_blowing = -above * share / mass;
        }
        gas
    };
    // f(b) = b Ω(b + B'c(b)) − target, and its slope.
    let evaluate = |b: f64| {
        let gas = read(b);
        let (char_rate, char_rate_b) = if stream.face_wears {
            (gas.char_rate, gas.char_rate_blowing)
        } else {
            (0.0, 0.0)
        };
        let (omega, omega_slope) = blowing_correction(b + char_rate);
        (
            b * omega - target,
            omega + b * omega_slope * (1.0 + char_rate_b),
            gas,
        )
    };
    let mut low = 0.0;
    let mut high = top;
    let mut past = false;
    if target > 0.0 && evaluate(top).0 < 0.0 {
        past = true;
        low = top;
        high = 2.0 * top;
        while evaluate(high).0 < 0.0 && high < 1.0e300 {
            low = high;
            high *= 2.0;
        }
    }
    let mut b = if target <= 0.0 {
        0.0
    } else {
        (low + high) * 0.5
    };
    if target > 0.0 {
        // A first guess from no blowing correction at all.
        let guess = target;
        if guess > low && guess < high {
            b = guess;
        }
        for _ in 0..200 {
            let (f, slope, _) = evaluate(b);
            if f < 0.0 {
                low = b;
            } else {
                high = b;
            }
            let mut next = b - f / slope;
            if !(next > low && next < high) {
                next = 0.5 * (low + high);
            }
            let moved = (next - b).abs();
            b = next;
            if moved <= 1.0e-15 * b.max(1.0e-300) || high - low <= 1.0e-15 * high {
                break;
            }
        }
    }
    let (_, slope, gas) = evaluate(b);
    let (char_rate, char_rate_b, char_rate_t) = if stream.face_wears {
        (
            gas.char_rate,
            gas.char_rate_blowing,
            gas.char_rate_temperature,
        )
    } else {
        (0.0, 0.0, 0.0)
    };
    let (omega, omega_slope) = blowing_correction(b + char_rate);
    // Slopes of B'g in the wall temperature and in the gas flux.
    let b_temperature = -b * omega_slope * char_rate_t / slope;
    let b_flux = 1.0 / (stream.coefficient * slope);
    Blowing {
        blowing: b,
        correction: omega,
        correction_temperature: omega_slope * (b_temperature * (1.0 + char_rate_b) + char_rate_t),
        correction_flux: omega_slope * (1.0 + char_rate_b) * b_flux,
        char_rate,
        char_rate_temperature: char_rate_t + char_rate_b * b_temperature,
        char_rate_flux: char_rate_b * b_flux,
        wall_enthalpy: gas.enthalpy,
        wall_enthalpy_temperature: gas.enthalpy_temperature + gas.enthalpy_blowing * b_temperature,
        wall_enthalpy_flux: gas.enthalpy_blowing * b_flux,
        beyond_table: gas.beyond_table,
        past_largest_blowing: past,
    }
}

// ---------------------------------------------------------------------------
// The surface beyond plain equilibrium.
// ---------------------------------------------------------------------------

/// The pieces of the surface model beyond plain chemical equilibrium, each
/// switched on its own. Constants and sources are with the functions below.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SurfaceOptions {
    /// At low wall pressure the decomposition gas neither takes carbon from
    /// the wall nor leaves carbon on it. It still blows and still carries its
    /// enthalpy. Fully so at or below 7 kPa, not at all at or above 13 kPa.
    pub low_pressure_rule: bool,
    /// Oxygen reaching the wall reacts with a probability per collision, so
    /// a cool face burns more slowly than oxygen can reach it.
    pub oxidation_limited_by_wall_reaction: bool,
    /// Probability that an oxygen atom striking the wall reacts, when the
    /// wall-reaction limit is on. `None` is Park's form for carbon in
    /// general, `0.63 exp(−1160/T)`. `Some(0.9)` is the constant Driver and
    /// coauthors (2010) found for PICA's own fibre preform in arc jets.
    pub oxygen_atom_reaction_probability: Option<f64>,
    /// Carbon vapour cannot leave faster than free evaporation allows.
    pub carbon_vapour_limited_by_evaporation: bool,
    /// Char lost as solid, as a fraction of the char consumed by reaction.
    /// It takes its own enthalpy away, is not in the blowing correction and
    /// has no term in the heat balance.
    pub solid_char_loss_fraction: f64,
    /// A diagnostic, off in every model: the stream still brings oxygen and
    /// still carries the gases away, but its own heating term is left out,
    /// so that a heating computed elsewhere (a coupled flow solution's) can
    /// be supplied through the radiative column instead.
    pub stream_heating_supplied_separately: bool,
    /// What the decomposition gas does in the share of the wall gas whose
    /// oxygen the wall reaction leaves unused.
    pub gas_beside_unused_oxygen: GasBesideUnusedOxygen,
    /// A diagnostic, off in every model: an air stream's surface table is
    /// computed afresh, as any other stream's is, in place of the locked
    /// table.
    pub air_table_computed_afresh: bool,
}

/// When the wall reaction is slower than the supply of oxygen, a share of
/// the wall gas (`1 − Da/(1 + Da)`) keeps its oxygen. The equilibrium table
/// burns the decomposition gas in that oxygen completely at any wall
/// temperature, and the heat of that burning reaches the face as heating
/// from the stream: on a face of 800 K late in an entry, some twenty times
/// what the cold flow gives. This says what is done instead.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum GasBesideUnusedOxygen {
    /// It burns completely: the table as it stands. Versions one to four.
    Burns,
    /// It follows the low-pressure rule there as it already does in the
    /// reacting share: at or below 7 kPa the decomposition gas and the
    /// stream are two separate sets of gases and do not react with each
    /// other; at or above 13 kPa the table as it stands; a straight blend
    /// between (Milos, Chen and Gokcen 2012, pp. 899-900). No new number.
    /// Version five.
    FollowsTheLowPressureRule,
    /// It does not burn there at any pressure. No source gives this above
    /// 13 kPa; it is the other end of a bracket and is in no model: only
    /// the studies in this file's tests ask for it.
    #[cfg_attr(not(test), allow(dead_code))]
    DoesNotBurn,
}

/// Surface tables computed for streams that are not air are kept for reuse
/// in the same process, this many at most.
const STREAM_TABLES_KEPT: usize = 8;

/// Wall pressures of the low-pressure rule, Pa (Milos, Chen and Gokcen 2012, p. 900).
pub const LOW_PRESSURE_RULE_FULL_BELOW_PA: f64 = 7.0e3;
pub const LOW_PRESSURE_RULE_NONE_ABOVE_PA: f64 = 13.0e3;
/// Molar mass of the gas at the wall used for its density in the wall-reaction limit, kg/mol.
pub const WALL_GAS_MOLAR_MASS_KG_PER_MOL: f64 = 0.029;
/// Molar masses of atomic and molecular oxygen, kg/mol.
const OXYGEN_ATOM_KG_PER_MOL: f64 = 0.0159994;
const OXYGEN_MOLECULE_KG_PER_MOL: f64 = 0.0319988;
/// Gas constant, J/mol/K.
const GAS_CONSTANT_J_PER_MOL_K: f64 = 8.31446261815324;

/// Probability that an oxygen atom striking carbon reacts (Park, as printed in Johnston 2014).
fn oxygen_atom_reaction_probability(temperature_k: f64) -> f64 {
    0.63 * (-1160.0 / temperature_k).exp()
}

/// Probability that an oxygen molecule striking carbon reacts (Park, as printed in Johnston 2014).
fn oxygen_molecule_reaction_probability(temperature_k: f64) -> f64 {
    (1.43e-3 + 0.01 * (-1450.0 / temperature_k).exp())
        / (1.0 + 2.0e-4 * (13000.0 / temperature_k).exp())
}

/// A surface table with what the options need, for one stream gas and one
/// decomposition gas, computed by chemical equilibrium from open
/// thermodynamic data (`evidence/research/heatshield_surface_model`). Any
/// number of pressures, B'g values and temperatures.
pub(crate) struct SurfaceBundle {
    pressure_pa: Vec<f64>,
    log_pressure: Vec<f64>,
    blowing: Vec<f64>,
    temperature_k: Vec<f64>,
    /// The char's share of the wall gas, `B'c / (1 + B'g + B'c)`, 1 where
    /// carbon boils; pressure outermost, then B'g, then temperature.
    share: Vec<f64>,
    wall_enthalpy: Vec<f64>,
    /// The same with the five carbon vapours (C to C5) left out: oxidation alone.
    share_without_vapour: Vec<f64>,
    wall_enthalpy_without_vapour: Vec<f64>,
    /// Enthalpy of stream gas and decomposition gas with no carbon exchanged with the wall.
    wall_enthalpy_no_exchange: Vec<f64>,
    /// Enthalpy of the decomposition gas alone, by pressure and temperature.
    gas_alone_enthalpy: Vec<f64>,
    /// Fraction of the stream's oxygen that is atomic when the stream is
    /// brought to rest, by pressure and by the stream's enthalpy.
    atom_pressure_pa: Vec<f64>,
    atom_enthalpy_j_per_kg: Vec<f64>,
    atom_fraction: Vec<f64>,
    /// Free evaporation of each carbon vapour: temperatures, the logarithm of
    /// each species' largest mass flux (kg/m²/s), and each species' share of
    /// the vapour the wall gas holds at equilibrium.
    vapour_temperature_k: Vec<f64>,
    vapour_species: usize,
    vapour_log_flux: Vec<f64>,
    vapour_share: Vec<f64>,
}

/// Position in a rising grid: the lower index and the weight of the upper
/// node, held at the ends.
fn bracket(grid: &[f64], value: f64) -> (usize, usize, f64) {
    let n = grid.len();
    if n == 1 {
        return (0, 0, 0.0);
    }
    if value <= grid[0] {
        return (0, 1, 0.0);
    }
    if value >= grid[n - 1] {
        return (n - 2, n - 1, 1.0);
    }
    let i = segment(grid, value);
    (i, i + 1, (value - grid[i]) / (grid[i + 1] - grid[i]))
}

/// Straight lines through `(grid, values)`, held at the ends.
fn held_line(grid: &[f64], values: &[f64], stride: usize, offset: usize, at: f64) -> f64 {
    let (i, j, w) = bracket(grid, at);
    values[i * stride + offset] * (1.0 - w) + values[j * stride + offset] * w
}

/// What the wall chemistry gives at one state.
#[derive(Clone, Copy, Debug)]
struct WallChemistry {
    char_rate: f64,
    wall_enthalpy: f64,
    /// Carbon boils at this pressure and temperature: there is no finite B'c.
    boiling: bool,
}

impl SurfaceBundle {
    /// Reads a bundle from the bytes written by the table code's exporter:
    /// the tag `ZTPSURF1`, then counts and little-endian 64-bit numbers.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        struct Reader<'a> {
            bytes: &'a [u8],
            at: usize,
        }
        impl Reader<'_> {
            fn count(&mut self) -> Result<usize, String> {
                let end = self.at + 8;
                let raw = self
                    .bytes
                    .get(self.at..end)
                    .ok_or("surface bundle is cut short")?;
                self.at = end;
                Ok(u64::from_le_bytes(raw.try_into().unwrap()) as usize)
            }
            fn numbers(&mut self, n: usize) -> Result<Vec<f64>, String> {
                let end = self.at + 8 * n;
                let raw = self
                    .bytes
                    .get(self.at..end)
                    .ok_or("surface bundle is cut short")?;
                self.at = end;
                Ok(raw
                    .chunks_exact(8)
                    .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
                    .collect())
            }
        }
        if bytes.get(..8) != Some(b"ZTPSURF1".as_slice()) {
            return Err("not a surface bundle".to_string());
        }
        let mut r = Reader { bytes, at: 8 };
        let (np, nb, nt) = (r.count()?, r.count()?, r.count()?);
        if np == 0 || nb < 2 || nt < 2 || np.saturating_mul(nb).saturating_mul(nt) > 10_000_000 {
            return Err("surface bundle has impossible sizes".to_string());
        }
        let pressure_pa = r.numbers(np)?;
        let blowing = r.numbers(nb)?;
        let temperature_k = r.numbers(nt)?;
        let nodes = np * nb * nt;
        let to_share = |char_rate: Vec<f64>| -> Vec<f64> {
            char_rate
                .iter()
                .enumerate()
                .map(|(at, &bc)| {
                    let b = blowing[(at / nt) % nb];
                    if bc.is_finite() {
                        bc / (1.0 + b + bc)
                    } else {
                        1.0
                    }
                })
                .collect()
        };
        let share = to_share(r.numbers(nodes)?);
        let wall_enthalpy = r.numbers(nodes)?;
        let share_without_vapour = to_share(r.numbers(nodes)?);
        let wall_enthalpy_without_vapour = r.numbers(nodes)?;
        let wall_enthalpy_no_exchange = r.numbers(nodes)?;
        let gas_alone_enthalpy = r.numbers(np * nt)?;
        let (na, nh) = (r.count()?, r.count()?);
        let atom_pressure_pa = r.numbers(na)?;
        let atom_enthalpy_j_per_kg = r.numbers(nh)?;
        let atom_fraction = r.numbers(na * nh)?;
        let (nv, ns) = (r.count()?, r.count()?);
        let vapour_temperature_k = r.numbers(nv)?;
        let vapour_log_flux = r.numbers(nv * ns)?;
        let vapour_share = r.numbers(nv * ns)?;
        let rising = |list: &[f64]| list.windows(2).all(|pair| pair[1] > pair[0]);
        if !rising(&pressure_pa)
            || !rising(&blowing)
            || !rising(&temperature_k)
            || pressure_pa[0] <= 0.0
        {
            return Err("surface bundle grids do not rise".to_string());
        }
        for list in [
            &wall_enthalpy,
            &wall_enthalpy_without_vapour,
            &wall_enthalpy_no_exchange,
            &gas_alone_enthalpy,
            &share,
            &share_without_vapour,
        ] {
            if list.iter().any(|v| !v.is_finite()) {
                return Err("surface bundle holds a value that is not a number".to_string());
            }
        }
        Ok(Self {
            log_pressure: pressure_pa.iter().map(|p| p.ln()).collect(),
            pressure_pa,
            blowing,
            temperature_k,
            share,
            wall_enthalpy,
            share_without_vapour,
            wall_enthalpy_without_vapour,
            wall_enthalpy_no_exchange,
            gas_alone_enthalpy,
            atom_pressure_pa,
            atom_enthalpy_j_per_kg,
            atom_fraction,
            vapour_temperature_k,
            vapour_species: ns,
            vapour_log_flux,
            vapour_share,
        })
    }

    /// The same bundle for another stream gas, computed by chemical
    /// equilibrium (`wall_gas.rs`) on this bundle's own B'g and temperature
    /// grids. Its pressures are the lowest and the highest of the history
    /// and every pressure of this bundle between them, so a history at one
    /// pressure is computed at that pressure and nothing is read beyond the
    /// history's range. The free-evaporation data of the carbon vapours does
    /// not depend on the stream and is copied.
    fn for_stream(
        &self,
        stream_gas: &StreamGas,
        history: &[StreamRow],
    ) -> Result<std::sync::Arc<SurfaceBundle>, SlabReject> {
        let lowest = history
            .iter()
            .map(|row| row.pressure_pa)
            .fold(f64::INFINITY, f64::min);
        let highest = history
            .iter()
            .map(|row| row.pressure_pa)
            .fold(f64::NEG_INFINITY, f64::max);
        // A pressure of this bundle within a billionth of an end is that end.
        let mut pressure_pa = vec![lowest];
        pressure_pa.extend(
            self.pressure_pa
                .iter()
                .copied()
                .filter(|&p| p > lowest * (1.0 + 1.0e-9) && p < highest * (1.0 - 1.0e-9)),
        );
        if highest > lowest {
            pressure_pa.push(highest);
        }
        type Key = (u64, u64, Vec<u64>);
        static MADE: std::sync::Mutex<Vec<(Key, std::sync::Arc<SurfaceBundle>)>> =
            std::sync::Mutex::new(Vec::new());
        let key: Key = (
            stream_gas.oxygen_mass_fraction.to_bits(),
            stream_gas.argon_mass_fraction.to_bits(),
            pressure_pa.iter().map(|p| p.to_bits()).collect(),
        );
        // Tables already made in this process are kept (the last few): the
        // same stream and pressures give the same table, so keeping one
        // changes no answer.
        if let Ok(made) = MADE.lock() {
            if let Some((_, bundle)) = made.iter().find(|(k, _)| *k == key) {
                return Ok(bundle.clone());
            }
        }
        let t = crate::wall_gas::stream_tables(
            stream_gas.oxygen_mass_fraction,
            stream_gas.argon_mass_fraction,
            &pressure_pa,
            &self.blowing,
            &self.temperature_k,
        )
        .map_err(|at| SlabReject::SurfaceChemistryNotConverged {
            pressure_pa: at.pressure_pa,
            blowing: at.blowing,
            temperature_k: at.temperature_k,
        })?;
        let (nb, nt) = (self.blowing.len(), self.temperature_k.len());
        let to_share = |char_rate: &[f64]| -> Vec<f64> {
            char_rate
                .iter()
                .enumerate()
                .map(|(at, &bc)| {
                    let b = self.blowing[(at / nt) % nb];
                    if bc.is_finite() {
                        bc / (1.0 + b + bc)
                    } else {
                        1.0
                    }
                })
                .collect()
        };
        let bundle = std::sync::Arc::new(SurfaceBundle {
            log_pressure: pressure_pa.iter().map(|p| p.ln()).collect(),
            pressure_pa,
            blowing: self.blowing.clone(),
            temperature_k: self.temperature_k.clone(),
            share: to_share(&t.char_rate),
            wall_enthalpy: t.wall_enthalpy,
            share_without_vapour: to_share(&t.char_rate_without_vapour),
            wall_enthalpy_without_vapour: t.wall_enthalpy_without_vapour,
            wall_enthalpy_no_exchange: t.wall_enthalpy_no_exchange,
            gas_alone_enthalpy: t.gas_alone_enthalpy,
            atom_pressure_pa: t.atom_pressure_pa,
            atom_enthalpy_j_per_kg: t.atom_enthalpy_j_per_kg,
            atom_fraction: t.atom_fraction,
            vapour_temperature_k: self.vapour_temperature_k.clone(),
            vapour_species: self.vapour_species,
            vapour_log_flux: self.vapour_log_flux.clone(),
            vapour_share: self.vapour_share.clone(),
        });
        if let Ok(mut made) = MADE.lock() {
            if made.len() >= STREAM_TABLES_KEPT {
                made.remove(0);
            }
            made.push((key, bundle.clone()));
        }
        Ok(bundle)
    }

    /// Lowest and highest wall pressure the bundle holds, Pa.
    pub fn pressure_range_pa(&self) -> (f64, f64) {
        (
            self.pressure_pa[0],
            self.pressure_pa[self.pressure_pa.len() - 1],
        )
    }

    fn largest_blowing(&self) -> f64 {
        self.blowing[self.blowing.len() - 1]
    }

    /// One field along straight lines in the logarithm of pressure, in B'g
    /// and in temperature, held at the ends of each.
    fn read(&self, field: &[f64], pressure_pa: f64, blowing: f64, temperature_k: f64) -> f64 {
        let (i0, i1, fp) = bracket(&self.log_pressure, pressure_pa.ln());
        let (j0, j1, fb) = bracket(&self.blowing, blowing);
        let (k0, k1, ft) = bracket(&self.temperature_k, temperature_k);
        let (nb, nt) = (self.blowing.len(), self.temperature_k.len());
        let line = |i: usize, j: usize| {
            let at = (i * nb + j) * nt;
            field[at + k0] * (1.0 - ft) + field[at + k1] * ft
        };
        (line(i0, j0) * (1.0 - fb) + line(i0, j1) * fb) * (1.0 - fp)
            + (line(i1, j0) * (1.0 - fb) + line(i1, j1) * fb) * fp
    }

    /// B'c from the char's share of the wall gas. `None` where carbon boils.
    fn char_rate(
        &self,
        share_field: &[f64],
        pressure_pa: f64,
        blowing: f64,
        temperature_k: f64,
    ) -> Option<f64> {
        let share = self.read(share_field, pressure_pa, blowing, temperature_k);
        if share >= 1.0 - 1.0e-12 {
            return None;
        }
        let b = blowing.clamp(self.blowing[0], self.largest_blowing());
        Some(share * (1.0 + b) / (1.0 - share))
    }

    fn gas_alone(&self, pressure_pa: f64, temperature_k: f64) -> f64 {
        let nt = self.temperature_k.len();
        let (i0, i1, fp) = bracket(&self.log_pressure, pressure_pa.ln());
        let (k0, k1, ft) = bracket(&self.temperature_k, temperature_k);
        let line = |i: usize| {
            self.gas_alone_enthalpy[i * nt + k0] * (1.0 - ft)
                + self.gas_alone_enthalpy[i * nt + k1] * ft
        };
        line(i0) * (1.0 - fp) + line(i1) * fp
    }

    /// Fraction of the stream's oxygen that arrives as atoms.
    fn oxygen_atom_fraction(&self, flow_enthalpy_j_per_kg: f64, pressure_pa: f64) -> f64 {
        let nh = self.atom_enthalpy_j_per_kg.len();
        let log: Vec<f64> = self.atom_pressure_pa.iter().map(|p| p.ln()).collect();
        let (i0, i1, fp) = bracket(&log, pressure_pa.ln());
        let (k0, k1, fh) = bracket(&self.atom_enthalpy_j_per_kg, flow_enthalpy_j_per_kg);
        let line = |i: usize| {
            self.atom_fraction[i * nh + k0] * (1.0 - fh) + self.atom_fraction[i * nh + k1] * fh
        };
        line(i0) * (1.0 - fp) + line(i1) * fp
    }

    /// Carbon-vapour mass flux when diffusion (`equilibrium_flux` in all,
    /// split between the species as the equilibrium wall gas holds them) and
    /// free evaporation act one after the other, species by species.
    fn vapour_flux_limited(&self, temperature_k: f64, equilibrium_flux: f64) -> f64 {
        if equilibrium_flux <= 0.0 || temperature_k <= self.vapour_temperature_k[0] {
            return equilibrium_flux;
        }
        let ns = self.vapour_species;
        let mut flux = 0.0;
        for species in 0..ns {
            let free = held_line(
                &self.vapour_temperature_k,
                &self.vapour_log_flux,
                ns,
                species,
                temperature_k,
            )
            .exp();
            let by_diffusion = equilibrium_flux
                * held_line(
                    &self.vapour_temperature_k,
                    &self.vapour_share,
                    ns,
                    species,
                    temperature_k,
                );
            flux += free * by_diffusion / (free + by_diffusion + 1.0e-300);
        }
        flux
    }

    /// B'c and the wall-gas enthalpy at one wall temperature, pressure and
    /// B'g, with the options laid over the equilibrium table.
    /// `blown_coefficient` is `ρ_e u_e C_H Ω`.
    fn chemistry(
        &self,
        options: &SurfaceOptions,
        temperature_k: f64,
        pressure_pa: f64,
        blowing: f64,
        blown_coefficient: f64,
        flow_enthalpy_j_per_kg: f64,
    ) -> WallChemistry {
        let (t, p, bg) = (temperature_k, pressure_pa, blowing);
        let mut hw_eq = self.read(&self.wall_enthalpy, p, bg, t);
        let Some(mut bc_eq) = self.char_rate(&self.share, p, bg, t) else {
            return WallChemistry {
                char_rate: 1.0e12,
                wall_enthalpy: hw_eq,
                boiling: true,
            };
        };
        let rate_limits = options.oxidation_limited_by_wall_reaction
            || options.carbon_vapour_limited_by_evaporation;
        let mut bc_nv = bc_eq;
        let mut hw_nv = hw_eq;
        if rate_limits || options.low_pressure_rule {
            hw_nv = self.read(&self.wall_enthalpy_without_vapour, p, bg, t);
            bc_nv = self
                .char_rate(&self.share_without_vapour, p, bg, t)
                .unwrap_or(bc_eq);
        }
        let mut low_pressure_weight = 0.0;
        if options.low_pressure_rule {
            let w = ((LOW_PRESSURE_RULE_NONE_ABOVE_PA - p)
                / (LOW_PRESSURE_RULE_NONE_ABOVE_PA - LOW_PRESSURE_RULE_FULL_BELOW_PA))
                .clamp(0.0, 1.0);
            low_pressure_weight = w;
            if w > 0.0 {
                if let (Some(bc0), Some(bn0)) = (
                    self.char_rate(&self.share, p, 0.0, t),
                    self.char_rate(&self.share_without_vapour, p, 0.0, t),
                ) {
                    let hw0 = self.read(&self.wall_enthalpy, p, 0.0, t);
                    let hn0 = self.read(&self.wall_enthalpy_without_vapour, p, 0.0, t);
                    let hp = self.gas_alone(p, t);
                    let hw_i = ((1.0 + bc0) * hw0 + bg * hp) / (1.0 + bc0 + bg);
                    let hn_i = ((1.0 + bn0) * hn0 + bg * hp) / (1.0 + bn0 + bg);
                    let total_eq =
                        (1.0 - w) * (1.0 + bg + bc_eq) * hw_eq + w * (1.0 + bg + bc0) * hw_i;
                    let total_nv =
                        (1.0 - w) * (1.0 + bg + bc_nv) * hw_nv + w * (1.0 + bg + bn0) * hn_i;
                    bc_eq = (1.0 - w) * bc_eq + w * bc0;
                    bc_nv = (1.0 - w) * bc_nv + w * bn0;
                    hw_eq = total_eq / (1.0 + bg + bc_eq);
                    hw_nv = total_nv / (1.0 + bg + bc_nv);
                }
            }
        }
        if !rate_limits {
            return WallChemistry {
                char_rate: bc_eq,
                wall_enthalpy: hw_eq,
                boiling: false,
            };
        }
        let cmo = blown_coefficient.max(1.0e-12);
        let vapour = (bc_eq - bc_nv).max(0.0);
        let mut vapour_kept = 1.0;
        if options.carbon_vapour_limited_by_evaporation && vapour > 0.0 {
            vapour_kept = self.vapour_flux_limited(t, cmo * vapour) / (cmo * vapour);
        }
        let mut oxidation_kept = 1.0;
        if options.oxidation_limited_by_wall_reaction {
            let atoms = self.oxygen_atom_fraction(flow_enthalpy_j_per_kg, p);
            let speed = |molar_mass: f64| {
                (8.0 * GAS_CONSTANT_J_PER_MOL_K * t / (std::f64::consts::PI * molar_mass)).sqrt()
            };
            let atom_probability = options
                .oxygen_atom_reaction_probability
                .unwrap_or_else(|| oxygen_atom_reaction_probability(t));
            let consumed = atoms * atom_probability * speed(OXYGEN_ATOM_KG_PER_MOL) / 4.0
                + (1.0 - atoms)
                    * oxygen_molecule_reaction_probability(t)
                    * speed(OXYGEN_MOLECULE_KG_PER_MOL)
                    / 4.0;
            let damkohler = p * WALL_GAS_MOLAR_MASS_KG_PER_MOL / (GAS_CONSTANT_J_PER_MOL_K * t)
                * consumed
                / cmo;
            oxidation_kept = damkohler / (1.0 + damkohler);
        }
        if oxidation_kept >= 1.0 && vapour_kept >= 1.0 {
            return WallChemistry {
                char_rate: bc_eq,
                wall_enthalpy: hw_eq,
                boiling: false,
            };
        }
        let bc = oxidation_kept * bc_nv + vapour_kept * vapour;
        let h_none = self.read(&self.wall_enthalpy_no_exchange, p, bg, t);
        let total_nv = (1.0 + bg + bc_nv) * hw_nv;
        let total_eq = (1.0 + bg + bc_eq) * hw_eq;
        // In the share whose oxygen is unused: the gas left apart from the
        // air, each in its own equilibrium, by this weight.
        let apart = match options.gas_beside_unused_oxygen {
            GasBesideUnusedOxygen::Burns => 0.0,
            GasBesideUnusedOxygen::FollowsTheLowPressureRule => low_pressure_weight,
            GasBesideUnusedOxygen::DoesNotBurn => 1.0,
        };
        let total = if apart > 0.0 {
            let air = self.read(&self.wall_enthalpy_no_exchange, p, 0.0, t);
            let unburnt = air + bg * self.gas_alone(p, t);
            (1.0 - oxidation_kept) * ((1.0 - apart) * (1.0 + bg) * h_none + apart * unburnt)
                + oxidation_kept * total_nv
                + vapour_kept * (total_eq - total_nv)
        } else {
            (1.0 - oxidation_kept) * (1.0 + bg) * h_none
                + oxidation_kept * total_nv
                + vapour_kept * (total_eq - total_nv)
        };
        WallChemistry {
            char_rate: bc,
            wall_enthalpy: total / (1.0 + bg + bc),
            boiling: false,
        }
    }

    /// Ω, B'g, B'c and the wall-gas enthalpy for a wall temperature and a
    /// decomposition-gas flux: the root of `Ω = Ω(B'g + B'c)` with
    /// `B'g = ṁ_g / (ρ_e u_e C_H Ω)`, found inside a bracket.
    fn wall_values(
        &self,
        options: &SurfaceOptions,
        stream: &StreamNow,
        gas_flux: f64,
        wall_temperature_k: f64,
        gas_enthalpy: f64,
    ) -> Blowing {
        let target = (gas_flux / stream.coefficient).clamp(0.0, 500.0);
        let top = self.largest_blowing();
        let at = |omega: f64| {
            let b = target / omega;
            let mut gas = self.chemistry(
                options,
                wall_temperature_k,
                stream.pressure_pa,
                b,
                stream.coefficient * omega,
                stream.flow_enthalpy_j_per_kg,
            );
            if !stream.face_wears {
                gas.char_rate = 0.0;
            }
            if b > top {
                // Beyond the table's largest B'g the further decomposition gas is added unchanged.
                let share = (1.0 + top + gas.char_rate) / (1.0 + b + gas.char_rate);
                gas.wall_enthalpy = gas_enthalpy + (gas.wall_enthalpy - gas_enthalpy) * share;
            }
            (blowing_correction(b + gas.char_rate).0 - omega, b, gas)
        };
        let mut high = 1.0;
        let (mut f_high, mut b, mut gas) = at(high);
        let mut omega = high;
        if f_high < 0.0 {
            let mut low = 0.5;
            let mut f_low = at(low).0;
            while f_low < 0.0 && low > 1.0e-300 {
                high = low;
                f_high = f_low;
                low *= 0.5;
                f_low = at(low).0;
            }
            // Regula falsi with the Illinois correction, falling back on halving.
            let mut side = 0i32;
            for _ in 0..200 {
                let mut next = (low * f_high - high * f_low) / (f_high - f_low);
                if !(next > low && next < high) {
                    next = 0.5 * (low + high);
                }
                let (f, b_next, gas_next) = at(next);
                omega = next;
                b = b_next;
                gas = gas_next;
                if f == 0.0 || high - low <= 4.0e-16 * high {
                    break;
                }
                if f > 0.0 {
                    low = next;
                    f_low = f;
                    if side == 1 {
                        f_high *= 0.5;
                    }
                    side = 1;
                } else {
                    high = next;
                    f_high = f;
                    if side == -1 {
                        f_low *= 0.5;
                    }
                    side = -1;
                }
            }
        }
        Blowing {
            blowing: b,
            correction: omega,
            char_rate: gas.char_rate,
            wall_enthalpy: gas.wall_enthalpy,
            beyond_table: gas.boiling,
            past_largest_blowing: b > top,
            ..Blowing::default()
        }
    }

    /// The same with slopes in the wall temperature and the gas flux, by
    /// differences either side.
    fn wall_state(
        &self,
        options: &SurfaceOptions,
        stream: &StreamNow,
        gas_flux: f64,
        wall_temperature_k: f64,
        gas_enthalpy: (f64, f64),
    ) -> Blowing {
        let mut here = self.wall_values(
            options,
            stream,
            gas_flux,
            wall_temperature_k,
            gas_enthalpy.0,
        );
        let dt = 1.0e-3;
        let up = self.wall_values(
            options,
            stream,
            gas_flux,
            wall_temperature_k + dt,
            gas_enthalpy.0 + gas_enthalpy.1 * dt,
        );
        let down = self.wall_values(
            options,
            stream,
            gas_flux,
            wall_temperature_k - dt,
            gas_enthalpy.0 - gas_enthalpy.1 * dt,
        );
        here.correction_temperature = (up.correction - down.correction) / (2.0 * dt);
        here.char_rate_temperature = (up.char_rate - down.char_rate) / (2.0 * dt);
        here.wall_enthalpy_temperature = (up.wall_enthalpy - down.wall_enthalpy) / (2.0 * dt);
        let dm = 1.0e-6 * (gas_flux.abs() + 1.0e-3 * stream.coefficient);
        let below = (gas_flux - dm).max(0.0);
        let up = self.wall_values(
            options,
            stream,
            gas_flux + dm,
            wall_temperature_k,
            gas_enthalpy.0,
        );
        let down = self.wall_values(options, stream, below, wall_temperature_k, gas_enthalpy.0);
        let span = gas_flux + dm - below;
        here.correction_flux = (up.correction - down.correction) / span;
        here.char_rate_flux = (up.char_rate - down.char_rate) / span;
        here.wall_enthalpy_flux = (up.wall_enthalpy - down.wall_enthalpy) / span;
        here
    }
}

/// Where the face reads its chemistry: a published table as it stands, or a
/// bundle with options.
enum FaceChemistry<'m> {
    Table(SurfaceLookup<'m>),
    Bundle(&'m SurfaceBundle, SurfaceOptions),
}

impl FaceChemistry<'_> {
    fn state(
        &self,
        stream: &StreamNow,
        gas_flux: f64,
        wall_temperature_k: f64,
        gas_enthalpy: (f64, f64),
        char_rate_along_straight_lines: bool,
    ) -> Blowing {
        match self {
            Self::Table(lookup) => blowing_state(
                lookup,
                stream,
                gas_flux,
                wall_temperature_k,
                gas_enthalpy,
                char_rate_along_straight_lines,
            ),
            Self::Bundle(bundle, options) => {
                bundle.wall_state(options, stream, gas_flux, wall_temperature_k, gas_enthalpy)
            }
        }
    }

    /// 1 when the stream's own heating term is in the balance, 0 when it is supplied separately.
    fn stream_heats(&self) -> f64 {
        match self {
            Self::Bundle(_, options) if options.stream_heating_supplied_separately => 0.0,
            _ => 1.0,
        }
    }

    /// Char lost as solid for each unit consumed by reaction.
    fn solid_loss(&self) -> f64 {
        match self {
            Self::Table(_) => 0.0,
            Self::Bundle(_, options) => options.solid_char_loss_fraction,
        }
    }
}

// ---------------------------------------------------------------------------
// The slab.
// ---------------------------------------------------------------------------

/// Cell faces measured from the original heated face, metres. `face[0]` is
/// the heated face and `face[n]` the back. Every length the solver uses is
/// read from this list. Cells are finest at the original face, each
/// `cell_growth` times the one above, up to `largest_cell_m`. When the face
/// wears, `face[0]` moves inward; cells stay where they are in the material,
/// and a front cell worn thinner than [`FRONT_CELL_MERGE_FRACTION`] of its
/// neighbour is joined to it. So the cell at the face is always between that
/// fraction and one and a half times a cell that is no wider than
/// `largest_cell_m`.
fn cell_faces(thickness_m: f64, numerics: &Numerics) -> Vec<f64> {
    let mut widths = Vec::new();
    let mut width = numerics.first_cell_m;
    let mut total = 0.0;
    while total < thickness_m {
        widths.push(width);
        total += width;
        width = (width * numerics.cell_growth).min(numerics.largest_cell_m);
    }
    let scale = thickness_m / total;
    let mut faces = Vec::with_capacity(widths.len() + 1);
    let mut depth = 0.0;
    faces.push(0.0);
    for w in &widths {
        depth += w * scale;
        faces.push(depth);
    }
    let last = faces.len() - 1;
    faces[last] = thickness_m;
    faces
}

/// State at one time.
struct Slab {
    face_depth_m: Vec<f64>,
    temperature_k: Vec<f64>,
    /// What is left of resin parts A and B in each cell.
    remaining: Vec<[f64; 2]>,
    /// Gas flux toward the heated face through the upper face of each cell,
    /// kg/m²/s, over the last step. One more entry than cells: the back wall,
    /// which is closed.
    gas_flux_kg_per_m2_s: Vec<f64>,
    /// What is left of the resin at the heated face itself.
    face_remaining: [f64; 2],
    face_temperature_k: f64,
    /// Char leaving the face over the last step, kg/m²/s.
    char_flux_kg_per_m2_s: f64,
}

impl Slab {
    fn cells(&self) -> usize {
        self.temperature_k.len()
    }
    fn width_m(&self, i: usize) -> f64 {
        self.face_depth_m[i + 1] - self.face_depth_m[i]
    }
    fn centre_m(&self, i: usize) -> f64 {
        0.5 * (self.face_depth_m[i + 1] + self.face_depth_m[i])
    }

    /// Temperature at a depth below the original face: straight lines between
    /// the face, the cell centres and the back wall. Zero for a depth the
    /// face has passed.
    fn temperature_at_k(&self, depth_m: f64) -> f64 {
        let n = self.cells();
        let mut above_depth = self.face_depth_m[0];
        let mut above_value = self.face_temperature_k;
        if depth_m < above_depth {
            return 0.0;
        }
        for i in 0..n {
            let centre = self.centre_m(i);
            if depth_m <= centre {
                let span = centre - above_depth;
                if span <= 0.0 {
                    return self.temperature_k[i];
                }
                let w = (depth_m - above_depth) / span;
                return above_value + (self.temperature_k[i] - above_value) * w;
            }
            above_depth = centre;
            above_value = self.temperature_k[i];
        }
        above_value
    }

    /// Depth, from the original face, at which the density first reaches
    /// `level` going inward. The face's depth when the face itself is at or
    /// above `level`; the back wall's when no point reaches it.
    fn front_depth_m(&self, material: &Material, level_kg_per_m3: f64) -> f64 {
        let n = self.cells();
        let mut above_depth = self.face_depth_m[0];
        let mut above_value = material.solid_density(self.face_remaining);
        if above_value >= level_kg_per_m3 {
            return above_depth;
        }
        for i in 0..n {
            let centre = self.centre_m(i);
            let value = material.solid_density(self.remaining[i]);
            if value >= level_kg_per_m3 {
                let w = (level_kg_per_m3 - above_value) / (value - above_value);
                return above_depth + (centre - above_depth) * w;
            }
            above_depth = centre;
            above_value = value;
        }
        self.face_depth_m[n]
    }

    /// Joins the cell at the face to the one below it. The joined cell holds
    /// the two cells' resin and the two cells' enthalpy exactly: what is left
    /// of each resin part is their mean by width, and the temperature is the
    /// one at which the joined cell's enthalpy equals the sum.
    fn join_front_cells(&mut self, nodes: &SolidEnthalpyNodes, frozen_at_k: Option<f64>) {
        let material = nodes.material;
        let w0 = self.width_m(0);
        let w1 = self.width_m(1);
        let mut enthalpy = 0.0;
        for (i, w) in [(0, w0), (1, w1)] {
            let point = solid_point(nodes, self.temperature_k[i], frozen_at_k);
            enthalpy +=
                solid_enthalpy_per_volume(&point, material.solid_density(self.remaining[i])) * w;
        }
        let joined = [
            (self.remaining[0][0] * w0 + self.remaining[1][0] * w1) / (w0 + w1),
            (self.remaining[0][1] * w0 + self.remaining[1][1] * w1) / (w0 + w1),
        ];
        let density = material.solid_density(joined);
        let want = enthalpy / (w0 + w1);
        let mut low = self.temperature_k[0].min(self.temperature_k[1]);
        let mut high = self.temperature_k[0].max(self.temperature_k[1]);
        let mut t = (self.temperature_k[0] * w0 + self.temperature_k[1] * w1) / (w0 + w1);
        for _ in 0..100 {
            let point = solid_point(nodes, t, frozen_at_k);
            let f = solid_enthalpy_per_volume(&point, density) - want;
            if f < 0.0 {
                low = t;
            } else {
                high = t;
            }
            let mut next = t - f / heat_capacity_per_volume(&point, density);
            if !(next >= low && next <= high) {
                next = 0.5 * (low + high);
            }
            let moved = (next - t).abs();
            t = next;
            if moved <= 1.0e-13 * t {
                break;
            }
        }
        self.temperature_k[1] = t;
        self.remaining[1] = joined;
        self.temperature_k.remove(0);
        self.remaining.remove(0);
        self.gas_flux_kg_per_m2_s.remove(1);
        self.face_depth_m.remove(1);
    }
}

/// A banded set of linear equations, solved by elimination with row
/// exchanges. Three entries below the diagonal and three above.
struct Band {
    n: usize,
    /// Column by column; each column holds the rows from six above the
    /// diagonal to three below it.
    entries: Vec<f64>,
    right: Vec<f64>,
}

const BAND_BELOW: usize = 3;
const BAND_ABOVE: usize = 3;
const BAND_COLUMN: usize = 2 * BAND_BELOW + BAND_ABOVE + 1;
/// The diagonal entry is the pivot unless it is smaller than this fraction
/// of the largest entry below it.
const BAND_PIVOT_KEEP: f64 = 1.0e-6;
/// The mass rows are multiplied by this (J/kg, the size of a gas enthalpy)
/// so that they are of like size to the energy rows when pivots are chosen.
const FLUX_ROW_SCALE: f64 = 1.0e7;

impl Band {
    fn new(n: usize) -> Self {
        Self {
            n,
            entries: vec![0.0; n * BAND_COLUMN],
            right: vec![0.0; n],
        }
    }
    fn clear(&mut self, n: usize) {
        self.n = n;
        for v in &mut self.entries[..n * BAND_COLUMN] {
            *v = 0.0;
        }
        for v in &mut self.right[..n] {
            *v = 0.0;
        }
    }
    fn at(&self, row: usize, column: usize) -> usize {
        column * BAND_COLUMN + BAND_BELOW + BAND_ABOVE + row - column
    }
    fn add(&mut self, row: usize, column: usize, value: f64) {
        let at = self.at(row, column);
        self.entries[at] += value;
    }
    /// Solves in place; the answer is left in `right`. `false` when a pivot
    /// is zero or not a number.
    fn solve(&mut self) -> bool {
        let n = self.n;
        for j in 0..n {
            let last_row = (j + BAND_BELOW).min(n - 1);
            let last_column = (j + BAND_BELOW + BAND_ABOVE).min(n - 1);
            // The row on the diagonal is kept unless another is far larger,
            // so that rows which are exactly zero stay exactly zero.
            let own = self.entries[self.at(j, j)].abs();
            let mut pivot = j;
            let mut size = own;
            for i in j + 1..=last_row {
                let here = self.entries[self.at(i, j)].abs();
                if here > size {
                    size = here;
                    pivot = i;
                }
            }
            if !(size > 0.0) || !size.is_finite() {
                return false;
            }
            if own >= BAND_PIVOT_KEEP * size {
                pivot = j;
            }
            if pivot != j {
                for c in j..=last_column {
                    let a = self.at(j, c);
                    let b = self.at(pivot, c);
                    self.entries.swap(a, b);
                }
                self.right.swap(j, pivot);
            }
            let diagonal = self.entries[self.at(j, j)];
            for i in j + 1..=last_row {
                let factor = self.entries[self.at(i, j)] / diagonal;
                if factor != 0.0 {
                    for c in j + 1..=last_column {
                        let from = self.entries[self.at(j, c)];
                        let at = self.at(i, c);
                        self.entries[at] -= factor * from;
                    }
                    self.right[i] -= factor * self.right[j];
                }
            }
        }
        for j in (0..n).rev() {
            let last_column = (j + BAND_BELOW + BAND_ABOVE).min(n - 1);
            let mut sum = self.right[j];
            for c in j + 1..=last_column {
                sum -= self.entries[self.at(j, c)] * self.right[c];
            }
            self.right[j] = sum / self.entries[self.at(j, j)];
        }
        true
    }
}

/// Scratch arrays for one time step.
struct Work {
    trial_temperature_k: Vec<f64>,
    trial_gas_flux: Vec<f64>,
    trial_remaining: Vec<[f64; 2]>,
    density: Vec<f64>,
    density_slope: Vec<f64>,
    enthalpy: Vec<f64>,
    enthalpy_slope: Vec<f64>,
    conductivity: Vec<f64>,
    conductivity_slope: Vec<f64>,
    old_density: Vec<f64>,
    old_enthalpy: Vec<f64>,
    active: Vec<[bool; 2]>,
    band: Band,
}

impl Work {
    fn new(n: usize) -> Self {
        Self {
            trial_temperature_k: vec![0.0; n],
            trial_gas_flux: vec![0.0; n + 1],
            trial_remaining: vec![[0.0; 2]; n],
            density: vec![0.0; n],
            density_slope: vec![0.0; n],
            enthalpy: vec![0.0; n],
            enthalpy_slope: vec![0.0; n],
            conductivity: vec![0.0; n],
            conductivity_slope: vec![0.0; n],
            old_density: vec![0.0; n],
            old_enthalpy: vec![0.0; n],
            active: vec![[false; 2]; n],
            band: Band::new(2 * n + 2),
        }
    }
}

/// What one accepted step moved, per square metre of face.
#[derive(Clone, Copy, Debug, Default)]
struct StepAccount {
    heat_in_j: f64,
    heat_stored_j: f64,
    heat_taken_by_gas_j: f64,
    heat_given_off_j: f64,
    solid_enthalpy_change_j: f64,
    gas_enthalpy_out_j: f64,
    solid_mass_lost_kg: f64,
    gas_mass_out_kg: f64,
    /// What the energy account was allowed to be open by.
    energy_tolerance_j: f64,
    /// What the mass account was allowed to be open by.
    mass_tolerance_kg: f64,
    passes: u32,
}

/// What one accepted step moved at the face, per square metre.
#[derive(Clone, Copy, Debug, Default)]
struct FaceAccount {
    /// Heat from the stream, with the blowing correction.
    heat_from_stream_j: f64,
    heat_absorbed_j: f64,
    heat_radiated_j: f64,
    /// Heat taken at the face by the decomposition gas and the char as they
    /// become wall gas: `ṁ_g (h_w − h_g) + ṁ_c (h_w − h_char)`.
    heat_taken_by_gases_j: f64,
    char_removed_kg: f64,
    /// Enthalpy of the char removed, at the face temperature.
    char_enthalpy_removed_j: f64,
    /// The part of that enthalpy above the front cell's own.
    char_enthalpy_above_cell_j: f64,
    /// `conducted in − (stream + absorbed − radiated − taken by gases)`.
    balance_residual_j: f64,
    balance_tolerance_j: f64,
}

/// Residual of the working form of the energy equation over a step or a run:
/// `stored − in − given off + taken by gas`.
fn energy_residual_j(account: &StepAccount) -> f64 {
    account.heat_stored_j - account.heat_in_j - account.heat_given_off_j
        + account.heat_taken_by_gas_j
}

fn ensure_energy_account(time_s: f64, account: &StepAccount) -> Result<(), SlabReject> {
    let tolerance = account.energy_tolerance_j;
    let working = energy_residual_j(account);
    let state = account.solid_enthalpy_change_j - account.heat_in_j + account.gas_enthalpy_out_j;
    for residual in [working, state] {
        if !residual.is_finite() {
            return Err(SlabReject::NonFiniteValue { time_s });
        }
        if residual.abs() > tolerance {
            return Err(SlabReject::EnergyAccountBroken {
                time_s,
                residual_j_per_m2: residual,
                tolerance_j_per_m2: tolerance,
            });
        }
    }
    Ok(())
}

fn ensure_mass_account(time_s: f64, account: &StepAccount) -> Result<(), SlabReject> {
    let residual = account.solid_mass_lost_kg - account.gas_mass_out_kg;
    if !residual.is_finite() {
        return Err(SlabReject::NonFiniteValue { time_s });
    }
    if residual.abs() > account.mass_tolerance_kg {
        return Err(SlabReject::MassAccountBroken {
            time_s,
            solid_mass_lost_kg_per_m2: account.solid_mass_lost_kg,
            gas_mass_out_kg_per_m2: account.gas_mass_out_kg,
        });
    }
    Ok(())
}

/// The heat balance at the face must close: what is conducted into the slab
/// is what the stream and radiation bring, less what is radiated away and
/// what the gases take.
fn ensure_face_balance(time_s: f64, face: &FaceAccount) -> Result<(), SlabReject> {
    if !face.balance_residual_j.is_finite() {
        return Err(SlabReject::NonFiniteValue { time_s });
    }
    if face.balance_residual_j.abs() > face.balance_tolerance_j {
        return Err(SlabReject::EnergyAccountBroken {
            time_s,
            residual_j_per_m2: face.balance_residual_j,
            tolerance_j_per_m2: face.balance_tolerance_j,
        });
    }
    Ok(())
}

/// The resin may only be used up: what is left of a part is never more than
/// before and never negative.
fn ensure_one_way(
    time_s: f64,
    depth_m: f64,
    before: [f64; 2],
    after: [f64; 2],
) -> Result<(), SlabReject> {
    for part in 0..2 {
        if !after[part].is_finite() {
            return Err(SlabReject::NonFiniteValue { time_s });
        }
        if after[part] > before[part] || after[part] < 0.0 {
            return Err(SlabReject::DecompositionRanBackwards { time_s, depth_m });
        }
    }
    Ok(())
}

fn ensure_temperature_in_band(
    time_s: f64,
    depth_m: f64,
    temperature_k: f64,
    lowest_k: f64,
    highest_k: f64,
) -> Result<(), SlabReject> {
    if !temperature_k.is_finite() {
        return Err(SlabReject::NonFiniteValue { time_s });
    }
    if temperature_k < lowest_k - TEMPERATURE_BOUND_TOLERANCE_K
        || temperature_k > highest_k + TEMPERATURE_BOUND_TOLERANCE_K
    {
        return Err(SlabReject::TemperatureOutOfBounds {
            time_s,
            depth_m,
            temperature_k,
        });
    }
    Ok(())
}

/// The face may only move inward, and not through the whole of its cell in
/// one step. `width_before_m` and `width_after_m` are the front cell's.
fn ensure_face_moved_inward(
    time_s: f64,
    char_flux_kg_per_m2_s: f64,
    width_before_m: f64,
    width_after_m: f64,
) -> Result<(), SlabReject> {
    if !char_flux_kg_per_m2_s.is_finite() || !width_after_m.is_finite() {
        return Err(SlabReject::NonFiniteValue { time_s });
    }
    if char_flux_kg_per_m2_s < -CHAR_FLUX_TOLERANCE_KG_PER_M2_S {
        return Err(SlabReject::FaceMovedBackward { time_s });
    }
    if width_after_m < FRONT_CELL_LEFT_AFTER_A_STEP * width_before_m {
        return Err(SlabReject::FaceOutranItsCell { time_s });
    }
    Ok(())
}

/// What is done to the face during one step.
#[derive(Clone, Copy, Debug)]
enum FaceDrive {
    /// Its temperature at the end of the step is given, kelvin.
    Held { temperature_k: f64 },
    /// A gas stream, as it is at the end of the step.
    Stream(StreamNow),
}

/// One backward-Euler step from `time_s − step_s` to `time_s`.
///
/// Unknowns: the face temperature `T_w`, the char flux leaving the face
/// `ṁ_c`, and in each cell its temperature and the gas flux through its
/// upper face. Equations in each cell:
///
/// mass:   `M_i − M_{i+1} − (ρ_old − ρ_new) w / Δt = 0`
/// energy: `(H_new − H_old) w / Δt − q_in + q_out − M_{i+1} h_g(T_below) + M_i h_g(T_above) = 0`
///
/// with `H = ρ h` of the solid, `ρ_new` the closed-form decomposition at the
/// new temperature, and the gas enthalpy read at the face temperatures. This
/// is the conserved form `∂(ρh)/∂t = ∂/∂x(k ∂T/∂x) + ∂(ṁ_g h_g)/∂x`; the
/// working form with `(h_g − h̄) ∂ρ/∂t` and `ṁ_g ∂h_g/∂x` is the same
/// equation, and the step's account reports its terms.
///
/// At the face, for a held temperature: `T_w` is the given one and
/// `ṁ_c = 0`. For a gas stream:
///
/// balance: `k (T_w − T_0)/(w_0/2) = ρ_e u_e C_H Ω (h_e − h_w) + α q_rad − ε σ (T_w⁴ − T_∞⁴) + M_0 (h_g − h_w) + ṁ_c (h_char − h_w)`
/// char:    `ṁ_c = B'c ρ_e u_e C_H Ω`
///
/// and the front cell loses `ṁ_c Δt / ρ` of its width, the char leaving at
/// the face temperature. All of it is solved together by Newton's method;
/// each pass is one banded solve.
#[allow(clippy::too_many_arguments)]
fn step(
    slab: &mut Slab,
    work: &mut Work,
    nodes: &SolidEnthalpyNodes,
    chemistry: &FaceChemistry,
    terms: &Terms,
    pass_limit: u32,
    time_s: f64,
    step_s: f64,
    drive: &FaceDrive,
    lowest_k: f64,
    highest_k: f64,
) -> Result<(StepAccount, FaceAccount), SlabReject> {
    let material = nodes.material;
    let n = slab.cells();
    let frozen = terms.properties_frozen_at_k;
    let straight = terms.char_rate_along_straight_lines;

    // A held face has resin of its own, which follows its temperature at
    // once. A face in a stream is the top of the front cell.
    let mut held_face_remaining = slab.face_remaining;
    if let FaceDrive::Held { temperature_k } = *drive {
        if terms.decomposition {
            for part in 0..2 {
                let active =
                    slab.face_temperature_k > material.resin_parts[part].threshold_temperature_k;
                held_face_remaining[part] = decomposed(
                    &material.resin_parts[part],
                    slab.face_remaining[part],
                    temperature_k,
                    active,
                    step_s,
                )
                .0;
            }
        }
    }
    let held_face_density = material.solid_density(held_face_remaining);

    // Start of step.
    for i in 0..n {
        let point = solid_point(nodes, slab.temperature_k[i], frozen);
        let density = material.solid_density(slab.remaining[i]);
        work.old_density[i] = density;
        work.old_enthalpy[i] = solid_enthalpy_per_volume(&point, density);
        // A part is switched on for the step by the temperature the step
        // starts from, so the switch cannot flicker inside the Newton passes.
        for part in 0..2 {
            work.active[i][part] = terms.decomposition
                && slab.temperature_k[i] > material.resin_parts[part].threshold_temperature_k;
        }
        work.trial_temperature_k[i] = slab.temperature_k[i];
        work.trial_gas_flux[i] = slab.gas_flux_kg_per_m2_s[i];
    }
    work.trial_gas_flux[n] = 0.0;
    // The pores are at the wall pressure through the thickness.
    if let FaceDrive::Stream(stream) = *drive {
        nodes.pore_pressure_pa.set(stream.pressure_pa);
    }
    let old_front_width = slab.width_m(0);
    let mut face_temperature_k = match *drive {
        FaceDrive::Held { temperature_k } => temperature_k,
        FaceDrive::Stream(_) => slab.face_temperature_k,
    };
    let mut char_flux = match *drive {
        FaceDrive::Held { .. } => 0.0,
        FaceDrive::Stream(stream) => {
            if stream.present && stream.face_wears {
                slab.char_flux_kg_per_m2_s
            } else {
                0.0
            }
        }
    };
    // A test may stop the face at a depth: the step that would pass it lands
    // on it, and no char leaves after.
    let face_depth_before = slab.face_depth_m[0];
    let landing = match terms.face_stops_at_m {
        Some(stop) if face_depth_before < stop => {
            let reach = slab.char_flux_kg_per_m2_s * step_s / work.old_density[0];
            if face_depth_before + 1.5 * reach >= stop {
                Some(stop - face_depth_before)
            } else {
                None
            }
        }
        _ => None,
    };
    let stopped = matches!(terms.face_stops_at_m, Some(stop) if face_depth_before >= stop);

    let unknowns = 2 * n + 2;
    let mut passes = 0u32;
    let mut settled = false;
    while passes < pass_limit {
        passes += 1;

        // Cell quantities at the trial temperatures.
        for i in 0..n {
            let t = work.trial_temperature_k[i];
            let mut density_slope = 0.0;
            for part in 0..2 {
                let (left, slope) = decomposed(
                    &material.resin_parts[part],
                    slab.remaining[i][part],
                    t,
                    work.active[i][part],
                    step_s,
                );
                work.trial_remaining[i][part] = left;
                density_slope += material.part_weight(part) * slope;
            }
            let density = material.solid_density(work.trial_remaining[i]);
            let point = solid_point(nodes, t, frozen);
            let (k, k_temperature, k_density) = conductivity(&point, density);
            work.density[i] = density;
            work.density_slope[i] = density_slope;
            work.enthalpy[i] = solid_enthalpy_per_volume(&point, density);
            work.enthalpy_slope[i] = heat_capacity_per_volume(&point, density)
                + enthalpy_of_solid_lost(&point) * density_slope;
            work.conductivity[i] = k;
            work.conductivity_slope[i] = k_temperature + k_density * density_slope;
        }

        work.band.clear(unknowns);
        let band = &mut work.band;
        const FACE_T: usize = 0;
        const CHAR: usize = 1;
        let cell_t = |i: usize| 2 + 2 * i;
        let cell_m = |i: usize| 3 + 2 * i;

        // The front cell's width at the end of the step.
        let density_0 = work.density[0];
        let front_width = old_front_width - char_flux * step_s / density_0;
        let front_width_char = -step_s / density_0;
        let half = 0.5 * front_width;
        let t0 = work.trial_temperature_k[0];
        let m0 = work.trial_gas_flux[0];

        // Conduction from the face into the front cell.
        let face_point = solid_point(nodes, face_temperature_k, frozen);
        let face_density = match *drive {
            FaceDrive::Held { .. } => held_face_density,
            FaceDrive::Stream(_) => density_0,
        };
        let (k_face, k_face_t, _) = conductivity(&face_point, face_density);
        let k_mean = 0.5 * (k_face + work.conductivity[0]);
        let drop = face_temperature_k - t0;
        let conducted = k_mean * drop / half;
        let conducted_face_t = (0.5 * k_face_t * drop + k_mean) / half;
        let conducted_t0 = (0.5 * work.conductivity_slope[0] * drop - k_mean) / half;
        let conducted_char = -conducted / half * 0.5 * front_width_char;
        let (face_gas_enthalpy, face_gas_enthalpy_slope) =
            material.gas_enthalpy(face_temperature_k);
        // Enthalpy of the solid at the face, per kilogram, with the front
        // cell's make-up at the face temperature; and of the front cell.
        let face_solid = solid_enthalpy_per_volume(&face_point, density_0);
        let face_solid_t = heat_capacity_per_volume(&face_point, density_0);
        let char_enthalpy = face_solid / density_0;
        let char_enthalpy_face_t = face_solid_t / density_0;
        let char_enthalpy_t0 = (enthalpy_of_solid_lost(&face_point) - char_enthalpy)
            * work.density_slope[0]
            / density_0;

        // The two equations of the face.
        match *drive {
            FaceDrive::Held { temperature_k } => {
                band.add(FACE_T, FACE_T, 1.0);
                band.right[FACE_T] = -(face_temperature_k - temperature_k);
                band.add(CHAR, CHAR, 1.0);
                band.right[CHAR] = -char_flux;
            }
            FaceDrive::Stream(stream) => {
                let (emissivity, emissivity_density) = material.emissivity(density_0);
                let emissivity_t0 = emissivity_density * work.density_slope[0];
                let fourth = face_temperature_k.powi(4) - stream.surroundings_temperature_k.powi(4);
                let sigma = if terms.reradiation {
                    STEFAN_BOLTZMANN_W_PER_M2_K4
                } else {
                    0.0
                };
                let radiated = emissivity * sigma * fourth;
                let radiated_face_t = 4.0 * emissivity * sigma * face_temperature_k.powi(3);
                let absorbed = emissivity * stream.radiative_heating_w_per_m2;
                let radiation_t0 =
                    emissivity_t0 * (sigma * fourth - stream.radiative_heating_w_per_m2);
                let mut balance = conducted - absorbed + radiated;
                let mut balance_face_t = conducted_face_t + radiated_face_t;
                let mut balance_t0 = conducted_t0 + radiation_t0;
                let mut balance_m0 = 0.0;
                let mut balance_char = conducted_char;
                let mut char_row = char_flux;
                let mut char_row_face_t = 0.0;
                let mut char_row_m0 = 0.0;
                let mut char_row_char = 1.0;
                let mut char_row_t0 = 0.0;
                if stream.present {
                    let reacting = 1.0 / (1.0 + chemistry.solid_loss());
                    let b = chemistry.state(
                        &stream,
                        m0,
                        face_temperature_k,
                        (face_gas_enthalpy, face_gas_enthalpy_slope),
                        straight,
                    );
                    let c = stream.coefficient;
                    let reach = stream.flow_enthalpy_j_per_kg - b.wall_enthalpy;
                    let heats = chemistry.stream_heats();
                    let stream_heat = heats
                        * if terms.blowing_correction {
                            b.correction
                        } else {
                            1.0
                        };
                    let (stream_heat_t, stream_heat_m) = if terms.blowing_correction {
                        (heats * b.correction_temperature, heats * b.correction_flux)
                    } else {
                        (0.0, 0.0)
                    };
                    // Heating from the stream.
                    balance -= c * stream_heat * reach;
                    balance_face_t -=
                        c * (stream_heat_t * reach - stream_heat * b.wall_enthalpy_temperature);
                    balance_m0 -= c * (stream_heat_m * reach - stream_heat * b.wall_enthalpy_flux);
                    if terms.gases_take_heat_at_face {
                        // The decomposition gas becomes wall gas.
                        balance -= m0 * (face_gas_enthalpy - b.wall_enthalpy);
                        balance_face_t -=
                            m0 * (face_gas_enthalpy_slope - b.wall_enthalpy_temperature);
                        balance_m0 -=
                            face_gas_enthalpy - b.wall_enthalpy - m0 * b.wall_enthalpy_flux;
                        // The char becomes wall gas.
                        let char_flux = char_flux * reacting;
                        balance -= char_flux * (char_enthalpy - b.wall_enthalpy);
                        balance_face_t -=
                            char_flux * (char_enthalpy_face_t - b.wall_enthalpy_temperature);
                        balance_t0 -= char_flux * char_enthalpy_t0;
                        balance_m0 += char_flux * b.wall_enthalpy_flux;
                        balance_char -= reacting * (char_enthalpy - b.wall_enthalpy);
                    }
                    if stream.face_wears && !stopped {
                        if let Some(distance) = landing {
                            // Test-only: the face lands on a given depth.
                            char_row = char_flux * step_s - distance * density_0;
                            char_row_char = step_s;
                            char_row_t0 = -distance * work.density_slope[0];
                        } else {
                            // ṁ_c = B'c ρ_e u_e C_H Ω.
                            let c = c / reacting;
                            char_row -= b.char_rate * c * b.correction;
                            char_row_face_t -= c
                                * (b.char_rate_temperature * b.correction
                                    + b.char_rate * b.correction_temperature);
                            char_row_m0 -= c
                                * (b.char_rate_flux * b.correction
                                    + b.char_rate * b.correction_flux);
                        }
                    }
                }
                band.add(FACE_T, FACE_T, balance_face_t);
                band.add(FACE_T, CHAR, balance_char);
                band.add(FACE_T, cell_t(0), balance_t0);
                band.add(FACE_T, cell_m(0), balance_m0);
                band.right[FACE_T] = -balance;
                band.add(CHAR, FACE_T, char_row_face_t * FLUX_ROW_SCALE);
                band.add(CHAR, CHAR, char_row_char * FLUX_ROW_SCALE);
                band.add(CHAR, cell_t(0), char_row_t0 * FLUX_ROW_SCALE);
                band.add(CHAR, cell_m(0), char_row_m0 * FLUX_ROW_SCALE);
                band.right[CHAR] = -char_row * FLUX_ROW_SCALE;
            }
        }

        // Each cell: storage, decomposition and its own gas.
        for i in 0..n {
            let w = if i == 0 {
                old_front_width
            } else {
                slab.width_m(i)
            };
            let t = work.trial_temperature_k[i];
            let source = (work.old_density[i] - work.density[i]) * w / step_s;
            let source_slope = -work.density_slope[i] * w / step_s;
            let mut energy = (work.enthalpy[i] - work.old_enthalpy[i]) * w / step_s;
            let mut energy_t = work.enthalpy_slope[i] * w / step_s;
            if !terms.gas_carries_heat {
                // Test-only: the gas made in the cell leaves it at once, taking
                // either its own enthalpy or exactly what the solid gave up.
                let point = solid_point(nodes, t, frozen);
                let (h, h_slope) = if terms.decomposition_heat {
                    material.gas_enthalpy(t)
                } else {
                    (
                        enthalpy_of_solid_lost(&point),
                        enthalpy_of_solid_lost_slope(&point),
                    )
                };
                energy += h * source;
                energy_t += h_slope * source + h * source_slope;
            }
            let mut mass = work.trial_gas_flux[i] - work.trial_gas_flux[i + 1] - source;
            if i + 1 == n {
                energy -= terms.unaccounted_heat_w_per_m2;
                mass -= terms.unaccounted_gas_kg_per_m2_s;
            }
            band.add(cell_t(i), cell_t(i), energy_t);
            band.right[cell_t(i)] = -energy;
            band.add(cell_m(i), cell_t(i), -source_slope * FLUX_ROW_SCALE);
            band.add(cell_m(i), cell_m(i), FLUX_ROW_SCALE);
            if i + 1 < n {
                band.add(cell_m(i), cell_m(i + 1), -FLUX_ROW_SCALE);
            }
            band.right[cell_m(i)] = -mass * FLUX_ROW_SCALE;
        }

        // The front cell and the face: conduction in, gas out, and the char
        // leaving at the face temperature rather than the cell's.
        {
            let row = cell_t(0);
            band.right[row] += conducted;
            band.add(row, FACE_T, -conducted_face_t);
            band.add(row, cell_t(0), -conducted_t0);
            band.add(row, CHAR, -conducted_char);
            if terms.gas_carries_heat {
                band.right[row] -= m0 * face_gas_enthalpy;
                band.add(row, cell_m(0), face_gas_enthalpy);
                band.add(row, FACE_T, m0 * face_gas_enthalpy_slope);
            }
            let above_cell = (face_solid - work.enthalpy[0]) / density_0;
            band.right[row] -= char_flux * above_cell;
            band.add(row, CHAR, above_cell);
            band.add(row, FACE_T, char_flux * face_solid_t / density_0);
            band.add(
                row,
                cell_t(0),
                char_flux
                    * ((enthalpy_of_solid_lost(&face_point) * work.density_slope[0]
                        - work.enthalpy_slope[0])
                        / density_0
                        - above_cell * work.density_slope[0] / density_0),
            );
        }

        // Each face between two cells: conduction down, gas up.
        for i in 0..n - 1 {
            let w_above = if i == 0 { front_width } else { slab.width_m(i) };
            let w_below = slab.width_m(i + 1);
            let t_above = work.trial_temperature_k[i];
            let t_below = work.trial_temperature_k[i + 1];
            let distance = 0.5 * (w_above + w_below);
            let share_above = w_below / (w_above + w_below);
            let k =
                share_above * work.conductivity[i] + (1.0 - share_above) * work.conductivity[i + 1];
            let drop = t_above - t_below;
            let q = k * drop / distance;
            let q_above = (k + share_above * work.conductivity_slope[i] * drop) / distance;
            let q_below =
                (-k + (1.0 - share_above) * work.conductivity_slope[i + 1] * drop) / distance;
            // Heat leaves the cell above and enters the cell below.
            band.right[cell_t(i)] -= q;
            band.add(cell_t(i), cell_t(i), q_above);
            band.add(cell_t(i), cell_t(i + 1), q_below);
            band.right[cell_t(i + 1)] += q;
            band.add(cell_t(i + 1), cell_t(i), -q_above);
            band.add(cell_t(i + 1), cell_t(i + 1), -q_below);
            // Only the front cell's width depends on the char flux.
            let share_width = -w_below / ((w_above + w_below) * (w_above + w_below));
            if i == 0 {
                let k_width = share_width * (work.conductivity[0] - work.conductivity[1]);
                let q_char = (k_width * drop - 0.5 * q) / distance * front_width_char;
                band.add(cell_t(0), CHAR, q_char);
                band.add(cell_t(1), CHAR, -q_char);
            }
            if terms.gas_carries_heat {
                // The gas crosses at the face's temperature.
                let gas_share = terms.gas_face_share_of_cell_above.unwrap_or(share_above);
                let t_face = gas_share * t_above + (1.0 - gas_share) * t_below;
                let (h, h_slope) = material.gas_enthalpy(t_face);
                let m = work.trial_gas_flux[i + 1];
                // It enters the cell above and leaves the cell below.
                band.right[cell_t(i)] += m * h;
                band.add(cell_t(i), cell_m(i + 1), -h);
                band.add(cell_t(i), cell_t(i), -m * h_slope * gas_share);
                band.add(cell_t(i), cell_t(i + 1), -m * h_slope * (1.0 - gas_share));
                band.right[cell_t(i + 1)] -= m * h;
                band.add(cell_t(i + 1), cell_m(i + 1), h);
                band.add(cell_t(i + 1), cell_t(i), m * h_slope * gas_share);
                band.add(
                    cell_t(i + 1),
                    cell_t(i + 1),
                    m * h_slope * (1.0 - gas_share),
                );
                if i == 0 && terms.gas_face_share_of_cell_above.is_none() {
                    let g_char = m * h_slope * share_width * drop * front_width_char;
                    band.add(cell_t(0), CHAR, -g_char);
                    band.add(cell_t(1), CHAR, g_char);
                }
            }
        }

        if !band.solve() {
            return Err(SlabReject::NonFiniteValue { time_s });
        }
        let mut largest_temperature_change = band.right[FACE_T].abs();
        let mut largest_flux_change = band.right[CHAR].abs();
        if !band.right[FACE_T].is_finite() || !band.right[CHAR].is_finite() {
            return Err(SlabReject::NonFiniteValue { time_s });
        }
        face_temperature_k += band.right[FACE_T];
        char_flux += band.right[CHAR];
        for i in 0..n {
            let dt = band.right[cell_t(i)];
            let dm = band.right[cell_m(i)];
            if !dt.is_finite() || !dm.is_finite() {
                return Err(SlabReject::NonFiniteValue { time_s });
            }
            work.trial_temperature_k[i] += dt;
            work.trial_gas_flux[i] += dm;
            largest_temperature_change = largest_temperature_change.max(dt.abs());
            largest_flux_change = largest_flux_change.max(dm.abs());
        }
        if largest_temperature_change <= NEWTON_TEMPERATURE_TOLERANCE_K
            && largest_flux_change <= NEWTON_GAS_FLUX_TOLERANCE_KG_PER_M2_S
        {
            settled = true;
            break;
        }
    }
    if !settled {
        return Err(SlabReject::StepNotConverged { time_s, passes });
    }

    // The settled state, checked, and the account of the step.
    let mut account = StepAccount {
        passes,
        ..StepAccount::default()
    };
    let mut gas_enthalpy_at_birth_w = 0.0;
    let mut energy_moved_j = 0.0;
    let mut enthalpy_held_j = 0.0;
    let mut mass_held_kg = 0.0;
    ensure_temperature_in_band(
        time_s,
        face_depth_before,
        face_temperature_k,
        lowest_k,
        highest_k,
    )?;
    for i in 0..n {
        let w = if i == 0 {
            old_front_width
        } else {
            slab.width_m(i)
        };
        let depth = slab.centre_m(i);
        let t = work.trial_temperature_k[i];
        ensure_temperature_in_band(time_s, depth, t, lowest_k, highest_k)?;
        let mut left = [0.0; 2];
        for part in 0..2 {
            left[part] = decomposed(
                &material.resin_parts[part],
                slab.remaining[i][part],
                t,
                work.active[i][part],
                step_s,
            )
            .0;
        }
        ensure_one_way(time_s, depth, slab.remaining[i], left)?;
        let density = material.solid_density(left);
        let point = solid_point(nodes, t, frozen);
        let enthalpy = solid_enthalpy_per_volume(&point, density);
        let lost = (work.old_density[i] - density) * w;
        let stored =
            (solid_enthalpy_per_volume(&point, work.old_density[i]) - work.old_enthalpy[i]) * w;
        let solid_lost_enthalpy = enthalpy_of_solid_lost(&point);
        let gas_here = material.gas_enthalpy(t).0;
        let given_off = if terms.decomposition_heat {
            (solid_lost_enthalpy - gas_here) * lost
        } else {
            0.0
        };
        let birth = if terms.decomposition_heat {
            gas_here
        } else {
            solid_lost_enthalpy
        };
        gas_enthalpy_at_birth_w += birth * lost / step_s;
        account.solid_mass_lost_kg += lost;
        account.heat_stored_j += stored;
        account.heat_given_off_j += given_off;
        account.solid_enthalpy_change_j += (enthalpy - work.old_enthalpy[i]) * w;
        energy_moved_j += stored.abs() + given_off.abs();
        enthalpy_held_j += enthalpy.abs() * w;
        mass_held_kg += density * w;
        slab.temperature_k[i] = t;
        slab.remaining[i] = left;
        work.density[i] = density;
        work.enthalpy[i] = enthalpy;
    }
    for i in 0..=n {
        if !work.trial_gas_flux[i].is_finite() {
            return Err(SlabReject::NonFiniteValue { time_s });
        }
        slab.gas_flux_kg_per_m2_s[i] = work.trial_gas_flux[i];
    }

    // The face. A char flux that settled a rounding error below zero is zero.
    let density_0 = work.density[0];
    ensure_face_moved_inward(
        time_s,
        char_flux,
        old_front_width,
        old_front_width - char_flux * step_s / density_0,
    )?;
    let char_flux = char_flux.max(0.0);
    let front_width = old_front_width - char_flux * step_s / density_0;
    let face_remaining = match *drive {
        FaceDrive::Held { .. } => held_face_remaining,
        FaceDrive::Stream(_) => slab.remaining[0],
    };
    if let FaceDrive::Held { .. } = *drive {
        ensure_one_way(
            time_s,
            face_depth_before,
            slab.face_remaining,
            face_remaining,
        )?;
    }
    slab.face_remaining = face_remaining;
    slab.face_temperature_k = face_temperature_k;
    slab.char_flux_kg_per_m2_s = char_flux;
    slab.face_depth_m[0] = match landing {
        Some(distance) => face_depth_before + distance,
        None => slab.face_depth_m[1] - front_width,
    };

    let face_point = solid_point(nodes, face_temperature_k, frozen);
    let face_density = match *drive {
        FaceDrive::Held { .. } => held_face_density,
        FaceDrive::Stream(_) => density_0,
    };
    let k_mean = 0.5
        * (conductivity(&face_point, face_density).0
            + conductivity(
                &solid_point(nodes, slab.temperature_k[0], frozen),
                density_0,
            )
            .0);
    let half = 0.5 * slab.width_m(0);
    let heat_in_w = k_mean * (face_temperature_k - slab.temperature_k[0]) / half;
    let face_gas_enthalpy = material.gas_enthalpy(face_temperature_k).0;
    let gas_out = slab.gas_flux_kg_per_m2_s[0];
    let gas_enthalpy_out_w = if terms.gas_carries_heat {
        gas_out * face_gas_enthalpy
    } else {
        gas_enthalpy_at_birth_w
    };
    let char_enthalpy = solid_enthalpy_per_volume(&face_point, density_0) / density_0;
    let char_above_cell_w = char_flux * (char_enthalpy - work.enthalpy[0] / density_0);

    let mut face = FaceAccount {
        char_removed_kg: char_flux * step_s,
        char_enthalpy_removed_j: char_flux * char_enthalpy * step_s,
        char_enthalpy_above_cell_j: char_above_cell_w * step_s,
        ..FaceAccount::default()
    };
    if let FaceDrive::Stream(stream) = *drive {
        let (emissivity, _) = material.emissivity(density_0);
        let sigma = if terms.reradiation {
            STEFAN_BOLTZMANN_W_PER_M2_K4
        } else {
            0.0
        };
        let radiated = emissivity
            * sigma
            * (face_temperature_k.powi(4) - stream.surroundings_temperature_k.powi(4));
        let absorbed = emissivity * stream.radiative_heating_w_per_m2;
        let mut from_stream = 0.0;
        let mut taken = 0.0;
        if stream.present {
            let b = chemistry.state(
                &stream,
                gas_out,
                face_temperature_k,
                material.gas_enthalpy(face_temperature_k),
                straight,
            );
            if b.beyond_table {
                return Err(SlabReject::FaceBeyondSurfaceTable {
                    time_s,
                    face_temperature_k,
                });
            }
            let stream_heat = chemistry.stream_heats()
                * if terms.blowing_correction {
                    b.correction
                } else {
                    1.0
                };
            from_stream = stream.coefficient
                * stream_heat
                * (stream.flow_enthalpy_j_per_kg - b.wall_enthalpy);
            if terms.gases_take_heat_at_face {
                taken = gas_out * (b.wall_enthalpy - face_gas_enthalpy)
                    + char_flux / (1.0 + chemistry.solid_loss())
                        * (b.wall_enthalpy - char_enthalpy);
            }
        }
        face.heat_from_stream_j = from_stream * step_s;
        face.heat_absorbed_j = absorbed * step_s;
        face.heat_radiated_j = radiated * step_s;
        face.heat_taken_by_gases_j = taken * step_s;
        face.balance_residual_j = (heat_in_w - from_stream - absorbed + radiated + taken) * step_s;
        face.balance_tolerance_j = (ENERGY_ACCOUNT_TOLERANCE
            * (heat_in_w.abs()
                + from_stream.abs()
                + absorbed.abs()
                + radiated.abs()
                + taken.abs())
            * step_s)
            .max(ACCOUNT_ROUNDING_FLOOR * enthalpy_held_j);
        ensure_face_balance(time_s, &face)?;
    }

    account.heat_in_j = heat_in_w * step_s;
    account.gas_mass_out_kg = gas_out * step_s;
    account.gas_enthalpy_out_j = gas_enthalpy_out_w * step_s;
    account.heat_taken_by_gas_j = (gas_enthalpy_out_w - gas_enthalpy_at_birth_w) * step_s;
    energy_moved_j += account.heat_in_j.abs()
        + account.heat_taken_by_gas_j.abs()
        + face.char_enthalpy_above_cell_j.abs();
    account.energy_tolerance_j =
        (ENERGY_ACCOUNT_TOLERANCE * energy_moved_j).max(ACCOUNT_ROUNDING_FLOOR * enthalpy_held_j);
    account.mass_tolerance_kg = (MASS_ACCOUNT_TOLERANCE
        * (account.solid_mass_lost_kg.abs() + account.gas_mass_out_kg.abs()))
    .max(ACCOUNT_ROUNDING_FLOOR * mass_held_kg);

    ensure_mass_account(time_s, &account)?;
    // Within the step the cells are counted at the widths they started with,
    // so the char's enthalpy above the front cell's own is the only trace of
    // the wear in the energy of the cells.
    let cells_only = StepAccount {
        heat_in_j: account.heat_in_j - face.char_enthalpy_above_cell_j,
        ..account
    };
    ensure_energy_account(time_s, &cells_only)?;
    Ok((account, face))
}

/// Every material name [`answer`] takes, the one a caller is pointed to for
/// PICA second.
pub const MATERIAL_NAMES: [&str; 5] = [
    MATERIAL_TACOT,
    MATERIAL_PICA_VERSION_5,
    MATERIAL_PICA_VERSION_4,
    MATERIAL_PICA_VERSION_3,
    MATERIAL_PICA,
];

/// Lowest and highest wall pressure a heating history may hold for the
/// material named, Pa: the ends of that material's surface table. An
/// unknown name is refused as [`answer`] refuses it.
pub fn stream_pressure_range_pa(material_name: &str) -> Result<(f64, f64), SlabReject> {
    if material_name == MATERIAL_TACOT {
        Ok((STREAM_PRESSURE_MIN_PA, STREAM_PRESSURE_MAX_PA))
    } else if MATERIAL_NAMES.contains(&material_name) {
        Ok((PICA_STREAM_PRESSURE_MIN_PA, PICA_STREAM_PRESSURE_MAX_PA))
    } else {
        Err(SlabReject::UnknownMaterial {
            name: material_name.to_string(),
        })
    }
}

/// Wall temperature at which an arc-jet's cold-wall heating is referred to
/// its stream's enthalpy, kelvin.
pub const COLD_WALL_TEMPERATURE_K: f64 = 300.0;

/// Enthalpy, J/kg, of a stream of this make-up at a wall at
/// [`COLD_WALL_TEMPERATURE_K`] (in equilibrium with itself at 1 bar; at that
/// temperature it is oxygen, nitrogen and argon as they are). An arc-jet
/// test's heat-transfer coefficient is its cold-wall heating divided by its
/// flow enthalpy less this: the rule the PICA model was developed with.
pub fn stream_enthalpy_at_cold_wall_j_per_kg(stream_gas: &StreamGas) -> Result<f64, SlabReject> {
    if !(STREAM_OXYGEN_MASS_FRACTION_MIN..=STREAM_OXYGEN_MASS_FRACTION_MAX)
        .contains(&stream_gas.oxygen_mass_fraction)
    {
        return Err(SlabReject::StreamOxygenOutOfRange {
            oxygen_mass_fraction: stream_gas.oxygen_mass_fraction,
        });
    }
    if !(0.0..=STREAM_ARGON_MASS_FRACTION_MAX).contains(&stream_gas.argon_mass_fraction) {
        return Err(SlabReject::StreamArgonOutOfRange {
            argon_mass_fraction: stream_gas.argon_mass_fraction,
        });
    }
    crate::wall_gas::stream_enthalpy_at_wall(
        stream_gas.oxygen_mass_fraction,
        stream_gas.argon_mass_fraction,
        COLD_WALL_TEMPERATURE_K,
    )
    .ok_or(SlabReject::SurfaceChemistryNotConverged {
        pressure_pa: 1.0e5,
        blowing: 0.0,
        temperature_k: COLD_WALL_TEMPERATURE_K,
    })
}

/// Whether a question for the material named is inside the declared range,
/// decided before anything is computed. For a PICA material the declared
/// range is what was compared with measurement (see
/// [`ARC_JET_THICKNESS_M`] and the envelopes after it). `Ok` means
/// [`answer`] will start computing; it may still refuse later for what
/// happens in the run (a slab worn away, a PICA temperature above
/// [`COMPARED_TEMPERATURE_MAX_K`], a TACOT temperature above its tables). An
/// `Err` is the same refusal, with the same name and sentence, that
/// [`answer`] gives for the same material and question.
pub fn check_question(material_name: &str, question: &SlabQuestion) -> Result<(), SlabReject> {
    compared_envelopes(material_name, question).map(|_| ())
}

/// The check of [`check_question`], and for a PICA material whether the
/// question lies inside the arc-jet envelope, inside the Stardust one and
/// inside the thermocouple-block one.
fn compared_envelopes(material_name: &str, question: &SlabQuestion) -> Result<(bool, bool, bool), SlabReject> {
    ensure_question(
        question,
        stream_pressure_range_pa(material_name)?,
        material_name != MATERIAL_TACOT,
    )?;
    if material_name == MATERIAL_TACOT {
        Ok((false, false, false))
    } else {
        ensure_compared(question)
    }
}

/// `any_stream_gas`: whether the material's surface can be computed for a
/// stream that is not air.
fn ensure_question(
    question: &SlabQuestion,
    pressure_range_pa: (f64, f64),
    any_stream_gas: bool,
) -> Result<(), SlabReject> {
    if !(THICKNESS_MIN_M..=THICKNESS_MAX_M).contains(&question.thickness_m) {
        return Err(SlabReject::ThicknessOutOfRange {
            thickness_m: question.thickness_m,
        });
    }
    if !(START_TEMPERATURE_MIN_K..=START_TEMPERATURE_MAX_K).contains(&question.start_temperature_k)
    {
        return Err(SlabReject::StartTemperatureOutOfRange {
            start_temperature_k: question.start_temperature_k,
        });
    }
    match &question.surface {
        SurfaceCondition::HeldTemperature {
            held_temperature_k,
            hold_duration_s,
        } => {
            if !(HELD_TEMPERATURE_MIN_K..=HELD_TEMPERATURE_MAX_K).contains(held_temperature_k) {
                return Err(SlabReject::HeldTemperatureOutOfRange {
                    held_temperature_k: *held_temperature_k,
                });
            }
            if !(HOLD_DURATION_MIN_S..=HOLD_DURATION_MAX_S).contains(hold_duration_s) {
                return Err(SlabReject::HoldDurationOutOfRange {
                    hold_duration_s: *hold_duration_s,
                });
            }
            if !(0.0..=CONTINUE_AFTER_MAX_S).contains(&question.continue_after_s) {
                return Err(SlabReject::ContinueAfterOutOfRange {
                    continue_after_s: question.continue_after_s,
                });
            }
        }
        SurfaceCondition::GasStream {
            history,
            surroundings_temperature_k,
            stream_gas,
            ..
        } => {
            if !(STREAM_OXYGEN_MASS_FRACTION_MIN..=STREAM_OXYGEN_MASS_FRACTION_MAX)
                .contains(&stream_gas.oxygen_mass_fraction)
            {
                return Err(SlabReject::StreamOxygenOutOfRange {
                    oxygen_mass_fraction: stream_gas.oxygen_mass_fraction,
                });
            }
            if !(0.0..=STREAM_ARGON_MASS_FRACTION_MAX).contains(&stream_gas.argon_mass_fraction) {
                return Err(SlabReject::StreamArgonOutOfRange {
                    argon_mass_fraction: stream_gas.argon_mass_fraction,
                });
            }
            if !any_stream_gas && !stream_gas.is_air() {
                return Err(SlabReject::StreamNotAirForThisMaterial {
                    stream_gas: *stream_gas,
                });
            }
            if !(HISTORY_ROWS_MIN..=HISTORY_ROWS_MAX).contains(&history.len()) {
                return Err(SlabReject::HistoryRowsOutOfRange {
                    rows: history.len(),
                });
            }
            let (lowest_pressure, highest_pressure) = pressure_range_pa;
            for (row, entry) in history.iter().enumerate() {
                let in_order = if row == 0 {
                    entry.time_s == 0.0
                } else {
                    entry.time_s > history[row - 1].time_s
                };
                if !in_order || !entry.time_s.is_finite() {
                    return Err(SlabReject::HistoryTimesOutOfOrder {
                        row,
                        time_s: entry.time_s,
                    });
                }
                if !(lowest_pressure..=highest_pressure).contains(&entry.pressure_pa) {
                    return Err(SlabReject::StreamPressureOutOfRange {
                        row,
                        pressure_pa: entry.pressure_pa,
                        lowest_pressure_pa: lowest_pressure,
                        highest_pressure_pa: highest_pressure,
                    });
                }
                if !(0.0..=HEAT_TRANSFER_COEFFICIENT_MAX_KG_PER_M2_S)
                    .contains(&entry.heat_transfer_coefficient_kg_per_m2_s)
                {
                    return Err(SlabReject::HeatTransferCoefficientOutOfRange {
                        row,
                        heat_transfer_coefficient_kg_per_m2_s: entry
                            .heat_transfer_coefficient_kg_per_m2_s,
                    });
                }
                if !(0.0..=FLOW_ENTHALPY_MAX_J_PER_KG).contains(&entry.flow_enthalpy_j_per_kg) {
                    return Err(SlabReject::FlowEnthalpyOutOfRange {
                        row,
                        flow_enthalpy_j_per_kg: entry.flow_enthalpy_j_per_kg,
                    });
                }
                if !(0.0..=RADIATIVE_HEATING_MAX_W_PER_M2)
                    .contains(&entry.radiative_heating_w_per_m2)
                {
                    return Err(SlabReject::RadiativeHeatingOutOfRange {
                        row,
                        radiative_heating_w_per_m2: entry.radiative_heating_w_per_m2,
                    });
                }
            }
            let duration_s = history[history.len() - 1].time_s;
            if !(HISTORY_DURATION_MIN_S..=HISTORY_DURATION_MAX_S).contains(&duration_s) {
                return Err(SlabReject::HistoryDurationOutOfRange { duration_s });
            }
            // A row a vanishing time after the one before it is a jump, which
            // the stepping would go through in steps as small as its span.
            // The rows are in order here, so every span is above zero.
            for row in 1..history.len() {
                let span_s = history[row].time_s - history[row - 1].time_s;
                if span_s < HISTORY_ROW_SPAN_MIN_S * (1.0 - COMPARED_ROUNDING) {
                    return Err(SlabReject::HistoryRowsTooClose { row, span_s });
                }
            }
            if !(SURROUNDINGS_TEMPERATURE_MIN_K..=SURROUNDINGS_TEMPERATURE_MAX_K)
                .contains(surroundings_temperature_k)
            {
                return Err(SlabReject::SurroundingsTemperatureOutOfRange {
                    surroundings_temperature_k: *surroundings_temperature_k,
                });
            }
            if question.continue_after_s != 0.0 {
                return Err(SlabReject::ContinueAfterWithStream {
                    continue_after_s: question.continue_after_s,
                });
            }
            // A stream colder than the coldest wall the band allows draws
            // heat out of the face below its band: refused before computing.
            if history
                .iter()
                .any(|entry| entry.heat_transfer_coefficient_kg_per_m2_s > 0.0)
            {
                let wall_temperature_k = question.start_temperature_k.min(*surroundings_temperature_k);
                let wall_enthalpy_j_per_kg = crate::wall_gas::stream_enthalpy_at_wall(
                    stream_gas.oxygen_mass_fraction,
                    stream_gas.argon_mass_fraction,
                    wall_temperature_k,
                )
                .ok_or(SlabReject::SurfaceChemistryNotConverged {
                    pressure_pa: 1.0e5,
                    blowing: 0.0,
                    temperature_k: wall_temperature_k,
                })?;
                for (row, entry) in history.iter().enumerate() {
                    if entry.heat_transfer_coefficient_kg_per_m2_s > 0.0
                        && entry.flow_enthalpy_j_per_kg < wall_enthalpy_j_per_kg
                    {
                        return Err(SlabReject::FlowEnthalpyBelowWall {
                            row,
                            flow_enthalpy_j_per_kg: entry.flow_enthalpy_j_per_kg,
                            wall_enthalpy_j_per_kg,
                            wall_temperature_k,
                        });
                    }
                }
            }
        }
    }
    if question.thermocouple_depths_m.len() > THERMOCOUPLES_MAX {
        return Err(SlabReject::TooManyThermocouples {
            count: question.thermocouple_depths_m.len(),
        });
    }
    for (index, &depth_m) in question.thermocouple_depths_m.iter().enumerate() {
        if !(0.0..=question.thickness_m).contains(&depth_m) {
            return Err(SlabReject::ThermocoupleOutsideSlab { index, depth_m });
        }
    }
    Ok(())
}

/// Times the run must land on exactly: the corners of the face condition
/// (flagged) and the report times.
fn landing_times(question: &SlabQuestion, end_s: f64) -> Vec<(f64, bool, bool)> {
    // (time, is a corner, is reported)
    let mut times: Vec<(f64, bool, bool)> = Vec::new();
    let mut whole = 1u32;
    loop {
        let t = whole as f64 * REPORT_INTERVAL_S;
        if t >= end_s {
            break;
        }
        times.push((t, false, true));
        whole += 1;
    }
    let heating_ends = question.surface.heating_ends_s();
    times.push((heating_ends, false, true));
    times.push((end_s, false, true));
    for corner in question.surface.corner_times_s() {
        if corner > 0.0 && corner <= end_s {
            times.push((corner, true, false));
        }
    }
    times.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, bool, bool)> = Vec::with_capacity(times.len());
    for entry in times {
        match merged.last_mut() {
            Some(last) if last.0 == entry.0 => {
                last.1 |= entry.1;
                last.2 |= entry.2;
            }
            _ => merged.push(entry),
        }
    }
    merged
}

/// The stream at `time_s`: straight lines between the rows of the history.
/// Also how fast it is changing there: the largest change of any column per
/// second, as a fraction of that column's largest value in the history.
fn stream_at(history: &[StreamRow], time_s: f64, row_before: usize) -> (StreamRow, f64) {
    let a = &history[row_before];
    let b = &history[row_before + 1];
    let span = b.time_s - a.time_s;
    let w = ((time_s - a.time_s) / span).clamp(0.0, 1.0);
    let line = |from: f64, to: f64| from + (to - from) * w;
    let mut rate = 0.0f64;
    let columns: [fn(&StreamRow) -> f64; 4] = [
        |r| r.pressure_pa,
        |r| r.heat_transfer_coefficient_kg_per_m2_s,
        |r| r.flow_enthalpy_j_per_kg,
        |r| r.radiative_heating_w_per_m2,
    ];
    for column in columns {
        let largest = history.iter().map(|r| column(r).abs()).fold(0.0, f64::max);
        if largest > 0.0 {
            rate = rate.max((column(b) - column(a)).abs() / (largest * span));
        }
    }
    (
        StreamRow {
            time_s,
            pressure_pa: line(a.pressure_pa, b.pressure_pa),
            heat_transfer_coefficient_kg_per_m2_s: line(
                a.heat_transfer_coefficient_kg_per_m2_s,
                b.heat_transfer_coefficient_kg_per_m2_s,
            ),
            flow_enthalpy_j_per_kg: line(a.flow_enthalpy_j_per_kg, b.flow_enthalpy_j_per_kg),
            radiative_heating_w_per_m2: line(
                a.radiative_heating_w_per_m2,
                b.radiative_heating_w_per_m2,
            ),
        },
        rate,
    )
}

/// How sharply the heating history turns at `row`: the largest change of
/// slope of any column, per second, as a fraction of that column's largest
/// value in the history.
fn corner_sharpness(history: &[StreamRow], row: usize) -> f64 {
    let columns: [fn(&StreamRow) -> f64; 4] = [
        |r| r.pressure_pa,
        |r| r.heat_transfer_coefficient_kg_per_m2_s,
        |r| r.flow_enthalpy_j_per_kg,
        |r| r.radiative_heating_w_per_m2,
    ];
    let (a, b, c) = (&history[row - 1], &history[row], &history[row + 1]);
    let mut sharpness = 0.0f64;
    for column in columns {
        let largest = history.iter().map(|r| column(r).abs()).fold(0.0, f64::max);
        if largest > 0.0 {
            let before = (column(b) - column(a)) / (b.time_s - a.time_s);
            let after = (column(c) - column(b)) / (c.time_s - b.time_s);
            sharpness = sharpness.max((after - before).abs() / largest);
        }
    }
    sharpness
}

/// The slab's response to the question. The material, the grid, the time
/// steps and the tolerances are the constants in this file and in `tacot.rs`.
#[cfg(test)]
pub(crate) fn solve(question: &SlabQuestion) -> Result<SlabResult, SlabReject> {
    run(question, &NUMERICS, &Terms::ALL)
}

/// The frozen PICA model: the block in `pica.rs` with its surface bundle and
/// options. Built once.
fn frozen_pica() -> &'static (Material, SurfaceBundle) {
    static MODEL: std::sync::OnceLock<(Material, SurfaceBundle)> = std::sync::OnceLock::new();
    MODEL.get_or_init(|| {
        (
            crate::pica::material(&crate::pica::FROZEN),
            SurfaceBundle::from_bytes(crate::pica::SURFACE_BUNDLE)
                .expect("the locked surface bundle"),
        )
    })
}

/// Name of the first material: TACOT 3.0 with its published surface table.
pub const MATERIAL_TACOT: &str = "TACOT 3.0";
/// Name of the second material: the PICA stand-in of `pica.rs`.
pub const MATERIAL_PICA: &str = "PICA, open record";
/// Name of the PICA stand-in's third version: the same block with the
/// conductivity depending on pressure.
pub const MATERIAL_PICA_VERSION_3: &str = "PICA, open record, version 3";
/// Name of the fourth version: PICA's own measured heat capacity and char
/// conductivity curve (times one fitted number), and no decomposition below
/// 556 K. Callable; not the served default.
pub const MATERIAL_PICA_VERSION_4: &str = "PICA, open record, version 4";
/// Name of the fifth version: version four with the decomposition gas left
/// unburnt, at low pressure, in the share of the wall gas whose oxygen the
/// wall reaction leaves unused.
pub const MATERIAL_PICA_VERSION_5: &str = "PICA, open record, version 5";
/// The PICA model a caller is pointed to: version five. The earlier
/// versions stay callable by their own names so that recorded runs can be
/// made again.
pub const MATERIAL_PICA_SERVED: &str = MATERIAL_PICA_VERSION_5;

/// The slab's response to the question, for a material picked by name:
/// [`MATERIAL_TACOT`], [`MATERIAL_PICA_SERVED`] or an earlier PICA version by
/// its own name. Nothing else about the material is a caller's input.
#[cfg(test)]
pub(crate) fn solve_material(
    material_name: &str,
    question: &SlabQuestion,
) -> Result<SlabResult, SlabReject> {
    solve_material_with(material_name, question, &Terms::ALL)
}

/// The same with the terms given.
fn solve_material_with(
    material_name: &str,
    question: &SlabQuestion,
    terms: &Terms,
) -> Result<SlabResult, SlabReject> {
    if material_name == MATERIAL_TACOT {
        run(question, &NUMERICS, terms)
    } else if material_name == MATERIAL_PICA_VERSION_5 || material_name == MATERIAL_PICA_VERSION_4 {
        static MODEL: std::sync::OnceLock<Material> = std::sync::OnceLock::new();
        let material = MODEL.get_or_init(|| crate::pica::material(&crate::pica::VERSION_4));
        let (_, bundle) = frozen_pica();
        let options = if material_name == MATERIAL_PICA_VERSION_5 {
            crate::pica::SURFACE_OPTIONS_VERSION_5
        } else {
            crate::pica::SURFACE_OPTIONS
        };
        run_model(
            material,
            Some((bundle, options)),
            question,
            &NUMERICS,
            terms,
        )
    } else if material_name == MATERIAL_PICA_VERSION_3 {
        static MODEL: std::sync::OnceLock<Material> = std::sync::OnceLock::new();
        let material = MODEL.get_or_init(|| crate::pica::material(&crate::pica::VERSION_3));
        let (_, bundle) = frozen_pica();
        run_model(
            material,
            Some((bundle, crate::pica::SURFACE_OPTIONS)),
            question,
            &NUMERICS,
            terms,
        )
    } else if material_name == MATERIAL_PICA {
        let (material, bundle) = frozen_pica();
        run_model(
            material,
            Some((bundle, crate::pica::SURFACE_OPTIONS)),
            question,
            &NUMERICS,
            terms,
        )
    } else {
        Err(SlabReject::UnknownMaterial {
            name: material_name.to_string(),
        })
    }
}

/// For studies, not for serving: the PICA block with stated departures,
/// surface options chosen one by one, and optionally another surface bundle
/// (another stream gas, another decomposition gas, other carbon data).
#[cfg(test)]
pub(crate) fn solve_pica_study(
    departures: &crate::pica::Departures,
    options: &SurfaceOptions,
    surface_bundle: Option<&[u8]>,
    question: &SlabQuestion,
) -> Result<SlabResult, SlabReject> {
    solve_pica_study_refined(departures, options, surface_bundle, false, false, question)
}

/// The same with every cell halved, every time step halved, or both: for
/// showing how far an answer moves.
#[cfg(test)]
pub(crate) fn solve_pica_study_refined(
    departures: &crate::pica::Departures,
    options: &SurfaceOptions,
    surface_bundle: Option<&[u8]>,
    cells_halved: bool,
    steps_halved: bool,
    question: &SlabQuestion,
) -> Result<SlabResult, SlabReject> {
    let mut numerics = NUMERICS;
    if cells_halved {
        numerics.first_cell_m *= 0.5;
        numerics.largest_cell_m *= 0.5;
        numerics.cell_growth = numerics.cell_growth.sqrt();
    }
    if steps_halved {
        numerics.first_step_s *= 0.5;
        numerics.step_fraction *= 0.5;
        numerics.largest_step_s *= 0.5;
        numerics.wear_per_step *= 0.5;
    }
    let material = crate::pica::material(departures);
    let bundle = SurfaceBundle::from_bytes(surface_bundle.unwrap_or(crate::pica::SURFACE_BUNDLE))
        .map_err(|why| SlabReject::SurfaceBundleUnreadable { why })?;
    run_model(
        &material,
        Some((&bundle, *options)),
        question,
        &numerics,
        &Terms::ALL,
    )
}

// ---------------------------------------------------------------------------
// The served entry.
// ---------------------------------------------------------------------------

/// What the PICA model has been compared with measurement on. A PICA
/// question is answered only inside it; outside it the question is refused
/// by name. Three envelopes were compared: the arc-jet tests, the Stardust
/// flight and the thermocouple blocks. A gas-stream question is answered
/// when it lies wholly inside one of them, its slab, start and surroundings
/// included, and the answer says which ([`Summary::inside_arc_jet_comparison`],
/// [`Summary::inside_stardust_comparison`],
/// [`Summary::inside_thermocouple_block_comparison`]). TACOT, compared with
/// other codes only, is not held to them.
///
/// Each envelope has its own slab, start and surroundings: the arc-jet
/// tests were 0.06 m thick, started at 300 K and radiated to 300 K; the
/// Stardust cores were 0.0582 m thick, started at 260 K and radiated to
/// 266.48 K; the thermocouple blocks were 0.0224 to 0.0325 m thick, started
/// at 303.15 K and radiated to 300 K. Every one wore its face, and none held
/// its face at a temperature. A value inside no envelope at all is refused
/// by its own name ([`SlabReject::ThicknessNotCompared`],
/// [`SlabReject::StartTemperatureNotCompared`],
/// [`SlabReject::SurroundingsNotCompared`]); a question whose values each lie
/// in some envelope but not all in one is refused as
/// [`SlabReject::QuestionNotCompared`].
pub const ARC_JET_THICKNESS_M: f64 = 0.06;
pub const ARC_JET_START_TEMPERATURE_K: f64 = 300.0;
pub const ARC_JET_SURROUNDINGS_TEMPERATURE_K: f64 = 300.0;
pub const STARDUST_THICKNESS_M: f64 = 0.0582;
pub const STARDUST_START_TEMPERATURE_K: f64 = 260.0;
pub const STARDUST_SURROUNDINGS_TEMPERATURE_K: f64 = 266.48;
pub const THERMOCOUPLE_BLOCK_THICKNESS_MIN_M: f64 = 0.0224;
pub const THERMOCOUPLE_BLOCK_THICKNESS_MAX_M: f64 = 0.0325;
pub const THERMOCOUPLE_BLOCK_START_TEMPERATURE_K: f64 = 303.15;
pub const THERMOCOUPLE_BLOCK_SURROUNDINGS_TEMPERATURE_K: f64 = 300.0;
/// The hottest temperature any accepted step reached in the compared runs
/// (the 67 arc-jet tests and the two Stardust cores, as the entry asks them;
/// the thermocouple blocks stay below 3,000 K), kelvin: the Stardust
/// near-nose run. A PICA run hotter than this anywhere is refused
/// ([`SlabReject::TemperatureAboveCompared`]). From 3333 K, where TACOT's
/// tables stop, to this limit the PICA tables hold the specific heat at its
/// 3333 K value and continue the conductivity along its last straight line
/// (`pica.rs`); nothing measured stands there.
pub const COMPARED_TEMPERATURE_MAX_K: f64 = 3412.366092194615;
/// A value within this share of a compared end counts as at that end, so
/// that a compared test made into a question by its recipe lands inside.
pub const COMPARED_ROUNDING: f64 = 1.0e-9;

/// The arc-jet envelope: the 36 development tests (Milos and Chen 2010) and
/// the 31 held-back tests of 2026-10-09, as the recipe asks them. Wall
/// pressure 2.3 to 84.4 kPa.
pub const ARC_JET_PRESSURE_MIN_PA: f64 = 2.3e3;
pub const ARC_JET_PRESSURE_MAX_PA: f64 = 84.4e3;
/// Cold-wall heating, the coefficient times the flow enthalpy less the
/// stream's enthalpy at a cold wall ([`stream_enthalpy_at_cold_wall_j_per_kg`]):
/// 107 to 1,102 W/cm². Below the lowest it may be only in the first
/// [`ARC_JET_SWING_IN_S`], while the stream swings onto the sample.
pub const ARC_JET_HEATING_MIN_W_PER_M2: f64 = 1.07e6;
pub const ARC_JET_HEATING_MAX_W_PER_M2: f64 = 1.102e7;
/// Seconds from the start in which an arc-jet stream may be below the lowest
/// compared heating: the recipe's swing-in.
pub const ARC_JET_SWING_IN_S: f64 = 0.5;
/// Flow enthalpy, 7.3 to 29.2 MJ/kg, in every row.
pub const ARC_JET_FLOW_ENTHALPY_MIN_J_PER_KG: f64 = 7.3e6;
pub const ARC_JET_FLOW_ENTHALPY_MAX_J_PER_KG: f64 = 29.2e6;
/// The longest exposure, seconds: the heating history ends by then.
pub const ARC_JET_DURATION_MAX_S: f64 = 400.0;
/// The stream's gas: air with up to 27.6 percent argon by mass, or nitrogen
/// with 10 to 30 percent oxygen. No arc-jet test had radiative heating or a
/// cooling phase, so neither is in this envelope.
pub const ARC_JET_STREAM_OXYGEN_MASS_FRACTION_MIN: f64 = 0.10;
pub const ARC_JET_STREAM_OXYGEN_MASS_FRACTION_MAX: f64 = 0.30;
pub const ARC_JET_STREAM_ARGON_MASS_FRACTION_MAX: f64 = 0.276;

/// The Stardust envelope: the two flight comparisons, near-nose and flank
/// cores, with NASA's bare-wall heating, in air, the stream on from the
/// start and off by 134 s, then cooling by radiation to 600 s. Their heating
/// is NASA's computation, not a measurement; wear and core density were
/// measured. Wall pressure, Pa.
pub const STARDUST_PRESSURE_MIN_PA: f64 = 1.42452812;
pub const STARDUST_PRESSURE_MAX_PA: f64 = 34009.7;
/// Flow enthalpy in every row, J/kg.
pub const STARDUST_FLOW_ENTHALPY_MIN_J_PER_KG: f64 = 84956.8984;
pub const STARDUST_FLOW_ENTHALPY_MAX_J_PER_KG: f64 = 78313424.0;
/// Largest heat-transfer coefficient, kg/m²/s, and largest cold-wall
/// heating between rows, W/m² (the near-nose history at 50 to 51 s).
pub const STARDUST_COEFFICIENT_MAX_KG_PER_M2_S: f64 = 0.1971;
pub const STARDUST_HEATING_MAX_W_PER_M2: f64 = 10016024.33;
/// Largest radiative heating, W/m².
pub const STARDUST_RADIATIVE_HEATING_MAX_W_PER_M2: f64 = 740000.0;
/// The stream is on in one stretch that ends by this time, seconds.
pub const STARDUST_STREAM_ENDS_BY_S: f64 = 134.0;

/// The thermocouple-block envelope: the four PICA blocks of Tran and others
/// 1997 (9B, 7B, 3B, 6A), as the development runs asked them, the data the
/// one fitted number was fitted to. In air, at one wall pressure held
/// through the whole history, from 0.2 to 0.45 atm, Pa.
pub const THERMOCOUPLE_BLOCK_PRESSURE_MIN_PA: f64 = 20265.0;
pub const THERMOCOUPLE_BLOCK_PRESSURE_MAX_PA: f64 = 45596.25;
/// Flow enthalpy in every row, J/kg.
pub const THERMOCOUPLE_BLOCK_FLOW_ENTHALPY_J_PER_KG: f64 = 29.5e6;
/// Cold-wall heating while the stream is held, 400 to 580 W/cm², W/m².
/// Below the lowest it may be only in the first
/// [`THERMOCOUPLE_BLOCK_SWING_IN_S`]. The top is 580 W/cm² as the recipe
/// made the coefficient, with its surface table's cold-wall enthalpy of air
/// (1,871.40 J/kg) where this solver computes 1,871.06 J/kg: 0.07 W/m² more.
pub const THERMOCOUPLE_BLOCK_HEATING_MIN_W_PER_M2: f64 = 4.0e6;
pub const THERMOCOUPLE_BLOCK_HEATING_MAX_W_PER_M2: f64 = 5_800_000.07;
/// Seconds from the start in which the stream swings onto the block.
pub const THERMOCOUPLE_BLOCK_SWING_IN_S: f64 = 0.5;
/// The stream is held from the swing-in until a time in this span, seconds
/// (the exposures, 15 to 40 s), then falls to zero in a straight line over
/// [`THERMOCOUPLE_BLOCK_RAMP_DOWN_S`], and the block cools.
pub const THERMOCOUPLE_BLOCK_EXPOSURE_MIN_S: f64 = 15.0;
pub const THERMOCOUPLE_BLOCK_EXPOSURE_MAX_S: f64 = 40.0;
pub const THERMOCOUPLE_BLOCK_RAMP_DOWN_S: f64 = 0.5;
/// The longest block history, seconds (block 3B: its exposure and the
/// thermocouple record after it).
pub const THERMOCOUPLE_BLOCK_HISTORY_ENDS_BY_S: f64 = 152.0;

fn within(value: f64, lowest: f64, highest: f64) -> bool {
    value >= lowest - COMPARED_ROUNDING * lowest.abs()
        && value <= highest + COMPARED_ROUNDING * highest.abs()
}

/// Lowest and highest cold-wall heating on `[from_s, to_s]` of the segment
/// between rows `a` and `b`; `None` when the two do not overlap.
fn heating_between(a: &StreamRow, b: &StreamRow, cold_wall_j_per_kg: f64, from_s: f64) -> Option<(f64, f64)> {
    let start = a.time_s.max(from_s);
    if start > b.time_s {
        return None;
    }
    let span = b.time_s - a.time_s;
    let dc = (b.heat_transfer_coefficient_kg_per_m2_s - a.heat_transfer_coefficient_kg_per_m2_s) / span;
    let de = (b.flow_enthalpy_j_per_kg - a.flow_enthalpy_j_per_kg) / span;
    let q = |t: f64| {
        let s = t - a.time_s;
        (a.heat_transfer_coefficient_kg_per_m2_s + dc * s)
            * (a.flow_enthalpy_j_per_kg - cold_wall_j_per_kg + de * s)
    };
    let mut at = vec![start, b.time_s];
    if dc * de != 0.0 {
        let s = -(dc * (a.flow_enthalpy_j_per_kg - cold_wall_j_per_kg)
            + a.heat_transfer_coefficient_kg_per_m2_s * de)
            / (2.0 * dc * de);
        let t = a.time_s + s;
        if t > start && t < b.time_s {
            at.push(t);
        }
    }
    let values: Vec<f64> = at.into_iter().map(q).collect();
    Some((
        values.iter().copied().fold(f64::INFINITY, f64::min),
        values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    ))
}

/// Why a gas stream is outside the arc-jet envelope, the first reason found;
/// `None` when it is inside.
fn arc_jet_miss(history: &[StreamRow], stream_gas: &StreamGas) -> Result<Option<String>, SlabReject> {
    if !within(
        stream_gas.oxygen_mass_fraction,
        ARC_JET_STREAM_OXYGEN_MASS_FRACTION_MIN,
        ARC_JET_STREAM_OXYGEN_MASS_FRACTION_MAX,
    ) {
        return Ok(Some(format!(
            "oxygen mass fraction {} is outside {ARC_JET_STREAM_OXYGEN_MASS_FRACTION_MIN}..={ARC_JET_STREAM_OXYGEN_MASS_FRACTION_MAX}",
            stream_gas.oxygen_mass_fraction
        )));
    }
    if !within(stream_gas.argon_mass_fraction, 0.0, ARC_JET_STREAM_ARGON_MASS_FRACTION_MAX) {
        return Ok(Some(format!(
            "argon mass fraction {} is above {ARC_JET_STREAM_ARGON_MASS_FRACTION_MAX}",
            stream_gas.argon_mass_fraction
        )));
    }
    let end_s = history[history.len() - 1].time_s;
    if end_s > ARC_JET_DURATION_MAX_S {
        return Ok(Some(format!(
            "the heating history lasts {end_s} s, longer than {ARC_JET_DURATION_MAX_S} s"
        )));
    }
    for (row, entry) in history.iter().enumerate() {
        if !within(entry.pressure_pa, ARC_JET_PRESSURE_MIN_PA, ARC_JET_PRESSURE_MAX_PA) {
            return Ok(Some(format!(
                "row {row} has wall pressure {} Pa, outside {ARC_JET_PRESSURE_MIN_PA}..={ARC_JET_PRESSURE_MAX_PA} Pa",
                entry.pressure_pa
            )));
        }
        if !within(
            entry.flow_enthalpy_j_per_kg,
            ARC_JET_FLOW_ENTHALPY_MIN_J_PER_KG,
            ARC_JET_FLOW_ENTHALPY_MAX_J_PER_KG,
        ) {
            return Ok(Some(format!(
                "row {row} has flow enthalpy {} J/kg, outside {ARC_JET_FLOW_ENTHALPY_MIN_J_PER_KG}..={ARC_JET_FLOW_ENTHALPY_MAX_J_PER_KG} J/kg",
                entry.flow_enthalpy_j_per_kg
            )));
        }
        if entry.radiative_heating_w_per_m2 != 0.0 {
            return Ok(Some(format!(
                "row {row} has radiative heating {} W/m2; no arc-jet test had any",
                entry.radiative_heating_w_per_m2
            )));
        }
    }
    let cold_wall = stream_enthalpy_at_cold_wall_j_per_kg(stream_gas)?;
    for pair in history.windows(2) {
        if let Some((_, highest)) = heating_between(&pair[0], &pair[1], cold_wall, 0.0) {
            if !within(highest, 0.0, ARC_JET_HEATING_MAX_W_PER_M2) {
                return Ok(Some(format!(
                    "the cold-wall heating reaches {highest} W/m2 between {} and {} s, above {ARC_JET_HEATING_MAX_W_PER_M2} W/m2",
                    pair[0].time_s, pair[1].time_s
                )));
            }
        }
        if let Some((lowest, _)) = heating_between(&pair[0], &pair[1], cold_wall, ARC_JET_SWING_IN_S) {
            if !within(lowest, ARC_JET_HEATING_MIN_W_PER_M2, f64::INFINITY) {
                return Ok(Some(format!(
                    "the cold-wall heating falls to {lowest} W/m2 between {} and {} s, below {ARC_JET_HEATING_MIN_W_PER_M2} W/m2 after the first {ARC_JET_SWING_IN_S} s",
                    pair[0].time_s.max(ARC_JET_SWING_IN_S), pair[1].time_s
                )));
            }
        }
    }
    Ok(None)
}

/// Why a gas stream is outside the Stardust envelope, the first reason
/// found; `None` when it is inside.
fn stardust_miss(history: &[StreamRow], stream_gas: &StreamGas) -> Result<Option<String>, SlabReject> {
    if !stream_gas.is_air() {
        return Ok(Some("the stream is not air".to_string()));
    }
    for (row, entry) in history.iter().enumerate() {
        if !within(entry.pressure_pa, STARDUST_PRESSURE_MIN_PA, STARDUST_PRESSURE_MAX_PA) {
            return Ok(Some(format!(
                "row {row} has wall pressure {} Pa, outside {STARDUST_PRESSURE_MIN_PA}..={STARDUST_PRESSURE_MAX_PA} Pa",
                entry.pressure_pa
            )));
        }
        if !within(
            entry.flow_enthalpy_j_per_kg,
            STARDUST_FLOW_ENTHALPY_MIN_J_PER_KG,
            STARDUST_FLOW_ENTHALPY_MAX_J_PER_KG,
        ) {
            return Ok(Some(format!(
                "row {row} has flow enthalpy {} J/kg, outside {STARDUST_FLOW_ENTHALPY_MIN_J_PER_KG}..={STARDUST_FLOW_ENTHALPY_MAX_J_PER_KG} J/kg",
                entry.flow_enthalpy_j_per_kg
            )));
        }
        if !within(entry.heat_transfer_coefficient_kg_per_m2_s, 0.0, STARDUST_COEFFICIENT_MAX_KG_PER_M2_S) {
            return Ok(Some(format!(
                "row {row} has heat-transfer coefficient {} kg/m2/s, above {STARDUST_COEFFICIENT_MAX_KG_PER_M2_S}",
                entry.heat_transfer_coefficient_kg_per_m2_s
            )));
        }
        if !within(entry.radiative_heating_w_per_m2, 0.0, STARDUST_RADIATIVE_HEATING_MAX_W_PER_M2) {
            return Ok(Some(format!(
                "row {row} has radiative heating {} W/m2, above {STARDUST_RADIATIVE_HEATING_MAX_W_PER_M2} W/m2",
                entry.radiative_heating_w_per_m2
            )));
        }
    }
    let cold_wall = stream_enthalpy_at_cold_wall_j_per_kg(stream_gas)?;
    for pair in history.windows(2) {
        if let Some((_, highest)) = heating_between(&pair[0], &pair[1], cold_wall, 0.0) {
            if !within(highest, 0.0, STARDUST_HEATING_MAX_W_PER_M2) {
                return Ok(Some(format!(
                    "the cold-wall heating reaches {highest} W/m2 between {} and {} s, above {STARDUST_HEATING_MAX_W_PER_M2} W/m2",
                    pair[0].time_s, pair[1].time_s
                )));
            }
        }
    }
    let on: Vec<usize> = (0..history.len())
        .filter(|&row| history[row].heat_transfer_coefficient_kg_per_m2_s > 0.0)
        .collect();
    if let (Some(&first), Some(&last)) = (on.first(), on.last()) {
        if let Some(gap) = (first..=last).find(|row| !on.contains(row)) {
            return Ok(Some(format!(
                "the stream stops at row {gap} ({} s) and comes back on; the flight had one stretch of stream",
                history[gap].time_s
            )));
        }
        let ends_s = history[(last + 1).min(history.len() - 1)].time_s;
        if ends_s > STARDUST_STREAM_ENDS_BY_S {
            return Ok(Some(format!(
                "the stream is on until {ends_s} s, later than {STARDUST_STREAM_ENDS_BY_S} s"
            )));
        }
    }
    Ok(None)
}

/// Why a gas stream is outside the thermocouple-block envelope, the first
/// reason found; `None` when it is inside.
fn thermocouple_block_miss(history: &[StreamRow], stream_gas: &StreamGas) -> Result<Option<String>, SlabReject> {
    if !stream_gas.is_air() {
        return Ok(Some("the stream is not air".to_string()));
    }
    let pressure_pa = history[0].pressure_pa;
    if !within(pressure_pa, THERMOCOUPLE_BLOCK_PRESSURE_MIN_PA, THERMOCOUPLE_BLOCK_PRESSURE_MAX_PA) {
        return Ok(Some(format!(
            "row 0 has wall pressure {pressure_pa} Pa, outside {THERMOCOUPLE_BLOCK_PRESSURE_MIN_PA}..={THERMOCOUPLE_BLOCK_PRESSURE_MAX_PA} Pa"
        )));
    }
    let end_s = history[history.len() - 1].time_s;
    if end_s > THERMOCOUPLE_BLOCK_HISTORY_ENDS_BY_S {
        return Ok(Some(format!(
            "the heating history lasts {end_s} s, longer than {THERMOCOUPLE_BLOCK_HISTORY_ENDS_BY_S} s"
        )));
    }
    for (row, entry) in history.iter().enumerate() {
        if entry.pressure_pa != pressure_pa {
            return Ok(Some(format!(
                "row {row} has wall pressure {} Pa, not the {pressure_pa} Pa of row 0; each block was held at one pressure",
                entry.pressure_pa
            )));
        }
        if !within(
            entry.flow_enthalpy_j_per_kg,
            THERMOCOUPLE_BLOCK_FLOW_ENTHALPY_J_PER_KG,
            THERMOCOUPLE_BLOCK_FLOW_ENTHALPY_J_PER_KG,
        ) {
            return Ok(Some(format!(
                "row {row} has flow enthalpy {} J/kg, not the {THERMOCOUPLE_BLOCK_FLOW_ENTHALPY_J_PER_KG} J/kg compared",
                entry.flow_enthalpy_j_per_kg
            )));
        }
        if entry.radiative_heating_w_per_m2 != 0.0 {
            return Ok(Some(format!(
                "row {row} has radiative heating {} W/m2; no block had any",
                entry.radiative_heating_w_per_m2
            )));
        }
    }
    let on: Vec<usize> = (0..history.len())
        .filter(|&row| history[row].heat_transfer_coefficient_kg_per_m2_s > 0.0)
        .collect();
    let (Some(&first), Some(&last)) = (on.first(), on.last()) else {
        return Ok(Some("the stream is never on".to_string()));
    };
    if let Some(gap) = (first..=last).find(|row| !on.contains(row)) {
        return Ok(Some(format!(
            "the stream stops at row {gap} ({} s) and comes back on; each block had one stretch of stream",
            history[gap].time_s
        )));
    }
    if last == history.len() - 1 {
        return Ok(Some(format!(
            "the stream is still on at {end_s} s, the end of the history; each block's stream fell to zero over {THERMOCOUPLE_BLOCK_RAMP_DOWN_S} s and the block cooled"
        )));
    }
    let held_until_s = history[last].time_s;
    if !within(held_until_s, THERMOCOUPLE_BLOCK_EXPOSURE_MIN_S, THERMOCOUPLE_BLOCK_EXPOSURE_MAX_S) {
        return Ok(Some(format!(
            "the stream is held until {held_until_s} s, outside the exposures compared, {THERMOCOUPLE_BLOCK_EXPOSURE_MIN_S}..={THERMOCOUPLE_BLOCK_EXPOSURE_MAX_S} s"
        )));
    }
    let ramp_s = history[last + 1].time_s - held_until_s;
    if !within(ramp_s, THERMOCOUPLE_BLOCK_RAMP_DOWN_S, THERMOCOUPLE_BLOCK_RAMP_DOWN_S) {
        return Ok(Some(format!(
            "the stream falls to zero over {ramp_s} s from {held_until_s} s; each block's fell over {THERMOCOUPLE_BLOCK_RAMP_DOWN_S} s"
        )));
    }
    let cold_wall = stream_enthalpy_at_cold_wall_j_per_kg(stream_gas)?;
    for (row, pair) in history.windows(2).enumerate() {
        if let Some((_, highest)) = heating_between(&pair[0], &pair[1], cold_wall, 0.0) {
            if !within(highest, 0.0, THERMOCOUPLE_BLOCK_HEATING_MAX_W_PER_M2) {
                return Ok(Some(format!(
                    "the cold-wall heating reaches {highest} W/m2 between {} and {} s, above {THERMOCOUPLE_BLOCK_HEATING_MAX_W_PER_M2} W/m2",
                    pair[0].time_s, pair[1].time_s
                )));
            }
        }
        if row + 1 > last {
            continue;
        }
        if let Some((lowest, _)) = heating_between(&pair[0], &pair[1], cold_wall, THERMOCOUPLE_BLOCK_SWING_IN_S) {
            if !within(lowest, THERMOCOUPLE_BLOCK_HEATING_MIN_W_PER_M2, f64::INFINITY) {
                return Ok(Some(format!(
                    "the cold-wall heating falls to {lowest} W/m2 between {} and {} s, below {THERMOCOUPLE_BLOCK_HEATING_MIN_W_PER_M2} W/m2 while the stream is held",
                    pair[0].time_s.max(THERMOCOUPLE_BLOCK_SWING_IN_S), pair[1].time_s
                )));
            }
        }
    }
    Ok(None)
}

/// Why a slab, start and surroundings are not those of an envelope, the
/// first reason found; `None` when they are.
fn slab_miss(
    question: &SlabQuestion,
    surroundings_temperature_k: f64,
    (thickness_min_m, thickness_max_m): (f64, f64),
    start_temperature_k: f64,
    compared_surroundings_k: f64,
) -> Option<String> {
    if !within(question.thickness_m, thickness_min_m, thickness_max_m) {
        return Some(if thickness_min_m == thickness_max_m {
            format!("thickness {} m is not the {thickness_min_m} m compared", question.thickness_m)
        } else {
            format!(
                "thickness {} m is outside {thickness_min_m}..={thickness_max_m} m",
                question.thickness_m
            )
        });
    }
    if !within(question.start_temperature_k, start_temperature_k, start_temperature_k) {
        return Some(format!(
            "start temperature {} K is not the {start_temperature_k} K compared",
            question.start_temperature_k
        ));
    }
    if !within(surroundings_temperature_k, compared_surroundings_k, compared_surroundings_k) {
        return Some(format!(
            "surroundings temperature {surroundings_temperature_k} K is not the {compared_surroundings_k} K compared"
        ));
    }
    None
}

/// Which compared envelopes a PICA gas-stream question lies inside, its
/// slab, start and surroundings included: the arc-jet one, the Stardust one
/// and the thermocouple-block one, or why it is outside each.
fn envelope_misses(
    question: &SlabQuestion,
    history: &[StreamRow],
    surroundings_temperature_k: f64,
    stream_gas: &StreamGas,
) -> Result<(Option<String>, Option<String>, Option<String>), SlabReject> {
    let arc_jet = match slab_miss(
        question,
        surroundings_temperature_k,
        (ARC_JET_THICKNESS_M, ARC_JET_THICKNESS_M),
        ARC_JET_START_TEMPERATURE_K,
        ARC_JET_SURROUNDINGS_TEMPERATURE_K,
    ) {
        Some(why) => Some(why),
        None => arc_jet_miss(history, stream_gas)?,
    };
    let stardust = match slab_miss(
        question,
        surroundings_temperature_k,
        (STARDUST_THICKNESS_M, STARDUST_THICKNESS_M),
        STARDUST_START_TEMPERATURE_K,
        STARDUST_SURROUNDINGS_TEMPERATURE_K,
    ) {
        Some(why) => Some(why),
        None => stardust_miss(history, stream_gas)?,
    };
    let thermocouple_blocks = match slab_miss(
        question,
        surroundings_temperature_k,
        (THERMOCOUPLE_BLOCK_THICKNESS_MIN_M, THERMOCOUPLE_BLOCK_THICKNESS_MAX_M),
        THERMOCOUPLE_BLOCK_START_TEMPERATURE_K,
        THERMOCOUPLE_BLOCK_SURROUNDINGS_TEMPERATURE_K,
    ) {
        Some(why) => Some(why),
        None => thermocouple_block_miss(history, stream_gas)?,
    };
    Ok((arc_jet, stardust, thermocouple_blocks))
}

/// The limits of what was compared with measurement, for a PICA material:
/// refused by name outside them. Asked after [`ensure_question`]. Returns
/// whether the question lies inside the arc-jet, the Stardust and the
/// thermocouple-block envelope.
fn ensure_compared(question: &SlabQuestion) -> Result<(bool, bool, bool), SlabReject> {
    let (history, surroundings_temperature_k, face_wears, stream_gas) = match &question.surface {
        SurfaceCondition::HeldTemperature { .. } => return Err(SlabReject::HeldFaceNotCompared),
        SurfaceCondition::GasStream {
            history,
            surroundings_temperature_k,
            face_wears,
            stream_gas,
        } => (history, *surroundings_temperature_k, *face_wears, stream_gas),
    };
    if !face_wears {
        return Err(SlabReject::FaceNotWearingNotCompared);
    }
    let thickness_m = question.thickness_m;
    if !(within(thickness_m, ARC_JET_THICKNESS_M, ARC_JET_THICKNESS_M)
        || within(thickness_m, STARDUST_THICKNESS_M, STARDUST_THICKNESS_M)
        || within(thickness_m, THERMOCOUPLE_BLOCK_THICKNESS_MIN_M, THERMOCOUPLE_BLOCK_THICKNESS_MAX_M))
    {
        return Err(SlabReject::ThicknessNotCompared { thickness_m });
    }
    let start_temperature_k = question.start_temperature_k;
    if ![
        ARC_JET_START_TEMPERATURE_K,
        STARDUST_START_TEMPERATURE_K,
        THERMOCOUPLE_BLOCK_START_TEMPERATURE_K,
    ]
    .iter()
    .any(|&compared| within(start_temperature_k, compared, compared))
    {
        return Err(SlabReject::StartTemperatureNotCompared { start_temperature_k });
    }
    if ![
        ARC_JET_SURROUNDINGS_TEMPERATURE_K,
        STARDUST_SURROUNDINGS_TEMPERATURE_K,
        THERMOCOUPLE_BLOCK_SURROUNDINGS_TEMPERATURE_K,
    ]
    .iter()
    .any(|&compared| within(surroundings_temperature_k, compared, compared))
    {
        return Err(SlabReject::SurroundingsNotCompared {
            surroundings_temperature_k,
        });
    }
    match envelope_misses(question, history, surroundings_temperature_k, stream_gas)? {
        (Some(arc_jet), Some(stardust), Some(thermocouple_blocks)) => Err(SlabReject::QuestionNotCompared {
            arc_jet,
            stardust,
            thermocouple_blocks,
        }),
        (arc_jet, stardust, thermocouple_blocks) => {
            Ok((arc_jet.is_none(), stardust.is_none(), thermocouple_blocks.is_none()))
        }
    }
}

/// Most points in a history that comes back, and most points in a profile.
/// Longer ones are thinned to evenly spaced points that keep the first and
/// the last.
pub const HISTORY_POINTS_MAX: usize = 121;
pub const PROFILE_POINTS_MAX: usize = 201;

/// What the caller wants back beside the summary, by name.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Wanted {
    /// Face temperature, recession, the two fronts, and the gas and char fluxes against time.
    pub face_history: bool,
    /// Temperature at each thermocouple against time.
    pub thermocouple_histories: bool,
    /// Temperature and density through the thickness at the end.
    pub final_profile: bool,
}

/// The short answer. It always comes back.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    /// The material, by the name it was asked for.
    pub material: String,
    /// The PICA model has been compared with measurements; TACOT, a made-up
    /// material, with other codes only.
    pub material_compared_with_measurement: bool,
    /// How long the run was, seconds.
    pub duration_s: f64,
    /// How far the face wore inward, metres.
    pub wear_m: f64,
    /// Hottest the face was at a report time (every second), kelvin, and when.
    pub peak_face_temperature_k: f64,
    pub peak_face_temperature_at_s: f64,
    pub final_face_temperature_k: f64,
    /// Temperature at the back wall at the end, kelvin.
    pub final_back_temperature_k: f64,
    /// Depth below the face as it ended at which the density is half way
    /// from char to virgin, and at which it is within 2 percent of virgin
    /// density, metres.
    pub depth_half_way_to_virgin_m: f64,
    pub depth_within_2_percent_of_virgin_m: f64,
    /// The heat account, J/m²: see the fields of the same names in the header.
    pub heat_from_stream_j_per_m2: f64,
    pub heat_absorbed_from_radiation_j_per_m2: f64,
    pub heat_radiated_away_j_per_m2: f64,
    pub heat_taken_at_face_by_gases_j_per_m2: f64,
    pub heat_in_through_face_j_per_m2: f64,
    pub heat_stored_in_solid_j_per_m2: f64,
    pub heat_taken_by_gas_j_per_m2: f64,
    pub heat_given_off_by_decomposition_j_per_m2: f64,
    pub char_enthalpy_removed_j_per_m2: f64,
    /// How far the energy account and the face's heat balance are open, and
    /// what they were allowed, J/m².
    pub energy_account_residual_j_per_m2: f64,
    pub energy_account_tolerance_j_per_m2: f64,
    pub face_balance_residual_j_per_m2: f64,
    pub face_balance_tolerance_j_per_m2: f64,
    /// The mass account, kg/m²: solid lost = gas out + char worn off.
    pub solid_mass_lost_kg_per_m2: f64,
    pub gas_mass_out_kg_per_m2: f64,
    pub char_mass_removed_kg_per_m2: f64,
    pub mass_account_residual_kg_per_m2: f64,
    pub mass_account_tolerance_kg_per_m2: f64,
    /// Whether the question lies wholly inside the arc-jet envelope compared
    /// with measurement, whether inside the Stardust one, and whether inside
    /// the thermocouple-block one. A PICA answer has at least one; a question
    /// inside none is refused. All false for TACOT, which was compared with
    /// other codes only.
    pub inside_arc_jet_comparison: bool,
    pub inside_stardust_comparison: bool,
    pub inside_thermocouple_block_comparison: bool,
    /// Cells at the start and at the end, time steps, and the Newton passes
    /// of the step that needed the most.
    pub cells: u32,
    pub cells_at_end: u32,
    pub time_steps: u32,
    pub hardest_step_newton_passes: u32,
}

/// Face quantities against time, at most [`HISTORY_POINTS_MAX`] points.
#[derive(Clone, Debug, PartialEq)]
pub struct FaceHistory {
    pub time_s: Vec<f64>,
    pub face_temperature_k: Vec<f64>,
    pub recession_m: Vec<f64>,
    pub char_front_depth_m: Vec<f64>,
    pub virgin_front_depth_m: Vec<f64>,
    pub gas_mass_flux_kg_per_m2_s: Vec<f64>,
    pub char_mass_flux_kg_per_m2_s: Vec<f64>,
}

/// One thermocouple's record. It ends where the face passed it.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermocoupleHistory {
    /// Depth below the original face, metres.
    pub depth_m: f64,
    /// Whether the face wore past it, and when; the end of the run if it did not. Seconds.
    pub passed_by_face: bool,
    pub covered_until_s: f64,
    /// At most [`HISTORY_POINTS_MAX`] points, none after it was passed.
    pub time_s: Vec<f64>,
    pub temperature_k: Vec<f64>,
}

/// Temperature and density through the thickness at the end, at most
/// [`PROFILE_POINTS_MAX`] points.
#[derive(Clone, Debug, PartialEq)]
pub struct FinalProfile {
    /// Depth below the original face, metres.
    pub depth_m: Vec<f64>,
    pub temperature_k: Vec<f64>,
    pub density_kg_per_m3: Vec<f64>,
}

/// The answer: the summary, and what was asked for by name.
#[derive(Clone, Debug, PartialEq)]
pub struct Answer {
    pub summary: Summary,
    pub face_history: Option<FaceHistory>,
    pub thermocouple_histories: Option<Vec<ThermocoupleHistory>>,
    pub final_profile: Option<FinalProfile>,
}

/// Evenly spaced positions in a list of `n`, at most `cap`, with the first and the last.
fn thinned(n: usize, cap: usize) -> Vec<usize> {
    if n <= cap {
        return (0..n).collect();
    }
    (0..cap).map(|k| k * (n - 1) / (cap - 1)).collect()
}

/// The one entry for a caller: a material by name ([`MATERIAL_TACOT`],
/// [`MATERIAL_PICA_SERVED`], or an earlier PICA version by its own name, as
/// it was flown), a slab and what is done to its face, and what to
/// return beside the summary. A converged, checked answer or a refusal by name.
pub fn answer(
    material_name: &str,
    question: &SlabQuestion,
    wanted: &Wanted,
) -> Result<Answer, SlabReject> {
    let inside = compared_envelopes(material_name, question)?;
    let terms = if material_name == MATERIAL_TACOT {
        Terms::ALL
    } else {
        Terms {
            compared_temperature_top_k: Some(COMPARED_TEMPERATURE_MAX_K),
            ..Terms::ALL
        }
    };
    answer_with(material_name, question, wanted, &terms, inside)
}

/// For studies of the physics, not for serving: [`answer`] without the
/// limits of what was compared with measurement, as the entry was before
/// engine 0.7.0. Both envelope flags are false.
#[cfg(test)]
pub(crate) fn answer_beyond_compared(
    material_name: &str,
    question: &SlabQuestion,
    wanted: &Wanted,
) -> Result<Answer, SlabReject> {
    answer_with(material_name, question, wanted, &Terms::ALL, (false, false, false))
}

/// For studies: [`check_question`] without the limits of what was compared.
#[cfg(test)]
pub(crate) fn check_beyond_compared(
    material_name: &str,
    question: &SlabQuestion,
) -> Result<(), SlabReject> {
    ensure_question(
        question,
        stream_pressure_range_pa(material_name)?,
        material_name != MATERIAL_TACOT,
    )
}

fn answer_with(
    material_name: &str,
    question: &SlabQuestion,
    wanted: &Wanted,
    terms: &Terms,
    (inside_arc_jet_comparison, inside_stardust_comparison, inside_thermocouple_block_comparison): (
        bool,
        bool,
        bool,
    ),
) -> Result<Answer, SlabReject> {
    let r = solve_material_with(material_name, question, terms)?;
    let last = r.report_time_s.len() - 1;
    let mut peak = 0;
    for k in 0..=last {
        if r.surface_temperature_k[k] > r.surface_temperature_k[peak] {
            peak = k;
        }
    }
    let summary = Summary {
        material: material_name.to_string(),
        material_compared_with_measurement: material_name != MATERIAL_TACOT,
        duration_s: r.report_time_s[last],
        wear_m: r.recession_m[last],
        peak_face_temperature_k: r.surface_temperature_k[peak],
        peak_face_temperature_at_s: r.report_time_s[peak],
        final_face_temperature_k: r.surface_temperature_k[last],
        final_back_temperature_k: r.final_temperature_k[r.final_temperature_k.len() - 1],
        depth_half_way_to_virgin_m: r.depth_half_way_to_virgin_m,
        depth_within_2_percent_of_virgin_m: r.depth_within_2_percent_of_virgin_m,
        heat_from_stream_j_per_m2: r.heat_from_stream_j_per_m2,
        heat_absorbed_from_radiation_j_per_m2: r.heat_absorbed_from_radiation_j_per_m2,
        heat_radiated_away_j_per_m2: r.heat_radiated_away_j_per_m2,
        heat_taken_at_face_by_gases_j_per_m2: r.heat_taken_at_face_by_gases_j_per_m2,
        heat_in_through_face_j_per_m2: r.heat_in_through_face_j_per_m2,
        heat_stored_in_solid_j_per_m2: r.heat_stored_in_solid_j_per_m2,
        heat_taken_by_gas_j_per_m2: r.heat_taken_by_gas_j_per_m2,
        heat_given_off_by_decomposition_j_per_m2: r.heat_given_off_by_decomposition_j_per_m2,
        char_enthalpy_removed_j_per_m2: r.char_enthalpy_removed_j_per_m2,
        energy_account_residual_j_per_m2: r.energy_account_residual_j_per_m2,
        energy_account_tolerance_j_per_m2: r.energy_account_tolerance_j_per_m2,
        face_balance_residual_j_per_m2: r.face_balance_residual_j_per_m2,
        face_balance_tolerance_j_per_m2: r.face_balance_tolerance_j_per_m2,
        solid_mass_lost_kg_per_m2: r.solid_mass_lost_kg_per_m2,
        gas_mass_out_kg_per_m2: r.gas_mass_out_kg_per_m2,
        char_mass_removed_kg_per_m2: r.char_mass_removed_kg_per_m2,
        mass_account_residual_kg_per_m2: r.mass_account_residual_kg_per_m2,
        mass_account_tolerance_kg_per_m2: r.mass_account_tolerance_kg_per_m2,
        inside_arc_jet_comparison,
        inside_stardust_comparison,
        inside_thermocouple_block_comparison,
        cells: r.cells,
        cells_at_end: r.cells_at_end,
        time_steps: r.time_steps,
        hardest_step_newton_passes: r.hardest_step_newton_passes,
    };
    let pick = |values: &[f64], at: &[usize]| at.iter().map(|&k| values[k]).collect::<Vec<f64>>();
    let face_history = wanted.face_history.then(|| {
        let at = thinned(last + 1, HISTORY_POINTS_MAX);
        FaceHistory {
            time_s: pick(&r.report_time_s, &at),
            face_temperature_k: pick(&r.surface_temperature_k, &at),
            recession_m: pick(&r.recession_m, &at),
            char_front_depth_m: pick(&r.char_front_depth_m, &at),
            virgin_front_depth_m: pick(&r.virgin_front_depth_m, &at),
            gas_mass_flux_kg_per_m2_s: pick(&r.gas_mass_flux_at_surface_kg_per_m2_s, &at),
            char_mass_flux_kg_per_m2_s: pick(&r.char_mass_flux_kg_per_m2_s, &at),
        }
    });
    let thermocouple_histories = wanted.thermocouple_histories.then(|| {
        r.thermocouple_depth_m
            .iter()
            .zip(&r.thermocouple_temperature_k)
            .map(|(&depth_m, history)| {
                // The record ends at the last report before the face passed it.
                let covered = r.recession_m.iter().take_while(|&&s| s <= depth_m).count();
                let passed_by_face = covered <= last;
                let covered_until_s = if passed_by_face {
                    let (s0, s1) = (r.recession_m[covered - 1], r.recession_m[covered]);
                    let (t0, t1) = (r.report_time_s[covered - 1], r.report_time_s[covered]);
                    t0 + (t1 - t0) * (depth_m - s0) / (s1 - s0)
                } else {
                    r.report_time_s[last]
                };
                let at = thinned(covered, HISTORY_POINTS_MAX);
                ThermocoupleHistory {
                    depth_m,
                    passed_by_face,
                    covered_until_s,
                    time_s: pick(&r.report_time_s, &at),
                    temperature_k: pick(history, &at),
                }
            })
            .collect()
    });
    let final_profile = wanted.final_profile.then(|| {
        let at = thinned(r.final_depth_m.len(), PROFILE_POINTS_MAX);
        FinalProfile {
            depth_m: pick(&r.final_depth_m, &at),
            temperature_k: pick(&r.final_temperature_k, &at),
            density_kg_per_m3: pick(&r.final_density_kg_per_m3, &at),
        }
    });
    Ok(Answer {
        summary,
        face_history,
        thermocouple_histories,
        final_profile,
    })
}

/// TACOT with the cell layout given; the stage-one tests call it.
fn run(
    question: &SlabQuestion,
    numerics: &Numerics,
    terms: &Terms,
) -> Result<SlabResult, SlabReject> {
    run_material(&TACOT, question, numerics, terms)
}

fn run_material(
    material: &Material,
    question: &SlabQuestion,
    numerics: &Numerics,
    terms: &Terms,
) -> Result<SlabResult, SlabReject> {
    run_model(material, None, question, numerics, terms)
}

/// A material with its surface read from its published table, or from a
/// bundle with options.
fn run_model(
    material: &Material,
    bundle: Option<(&SurfaceBundle, SurfaceOptions)>,
    question: &SlabQuestion,
    numerics: &Numerics,
    terms: &Terms,
) -> Result<SlabResult, SlabReject> {
    ensure_question(
        question,
        match bundle {
            Some((bundle, _)) => bundle.pressure_range_pa(),
            None => (
                material.surface.pressure_pa[0],
                material.surface.pressure_pa[material.surface.pressure_pa.len() - 1],
            ),
        },
        bundle.is_some(),
    )?;
    // A stream that is not air has its surface table computed here, at the
    // pressures of its own history. Air reads the locked table.
    let computed = match (&question.surface, bundle) {
        (
            SurfaceCondition::GasStream {
                history,
                stream_gas,
                ..
            },
            Some((bundle, options)),
        ) if !stream_gas.is_air() || options.air_table_computed_afresh => {
            Some(bundle.for_stream(stream_gas, history)?)
        }
        _ => None,
    };
    let lookup = match (bundle, &computed) {
        (Some((_, options)), Some(computed)) => FaceChemistry::Bundle(computed, options),
        (Some((bundle, options)), None) => FaceChemistry::Bundle(bundle, options),
        (None, _) => FaceChemistry::Table(SurfaceLookup::of(&material.surface)),
    };
    let nodes = SolidEnthalpyNodes::of(material);
    let start_k = question.start_temperature_k;
    let table_top_k = material.solid_table_k[material.solid_table_k.len() - 1];
    let (lowest_k, highest_k, streamed) = match &question.surface {
        SurfaceCondition::HeldTemperature {
            held_temperature_k, ..
        } => (
            start_k,
            held_temperature_k.max(heat_release_ends_k(&nodes)),
            false,
        ),
        SurfaceCondition::GasStream {
            surroundings_temperature_k,
            ..
        } => (
            start_k.min(*surroundings_temperature_k),
            match terms.compared_temperature_top_k {
                Some(top) => top.min(table_top_k),
                None => table_top_k,
            },
            true,
        ),
    };
    let end_s = question.surface.heating_ends_s() + question.continue_after_s;

    let largest_cell_m = if streamed {
        numerics.largest_cell_m * LARGEST_CELL_IN_A_STREAM
    } else {
        numerics.largest_cell_m
    };
    let face_depth_m = cell_faces(
        question.thickness_m,
        &Numerics {
            largest_cell_m,
            ..*numerics
        },
    );
    let n = face_depth_m.len() - 1;
    let mut slab = Slab {
        face_depth_m,
        temperature_k: vec![start_k; n],
        remaining: vec![material.virgin_remaining(); n],
        gas_flux_kg_per_m2_s: vec![0.0; n + 1],
        face_remaining: material.virgin_remaining(),
        face_temperature_k: start_k,
        char_flux_kg_per_m2_s: 0.0,
    };
    let mut work = Work::new(n);

    let span = material.virgin_density_kg_per_m3 - material.char_density_kg_per_m3;
    let char_front_level = material.char_density_kg_per_m3 + FRONT_FRACTION * span;
    let virgin_front_level = material.char_density_kg_per_m3 + (1.0 - FRONT_FRACTION) * span;

    let tc = question.thermocouple_depths_m.len();
    let mut result = SlabResult {
        report_time_s: Vec::new(),
        surface_temperature_k: Vec::new(),
        thermocouple_depth_m: question.thermocouple_depths_m.clone(),
        thermocouple_temperature_k: vec![Vec::new(); tc],
        gas_mass_flux_at_surface_kg_per_m2_s: Vec::new(),
        char_front_depth_m: Vec::new(),
        virgin_front_depth_m: Vec::new(),
        final_depth_m: Vec::new(),
        final_temperature_k: Vec::new(),
        final_density_kg_per_m3: Vec::new(),
        final_surface_density_kg_per_m3: 0.0,
        heat_in_through_face_j_per_m2: 0.0,
        heat_stored_in_solid_j_per_m2: 0.0,
        heat_taken_by_gas_j_per_m2: 0.0,
        heat_given_off_by_decomposition_j_per_m2: 0.0,
        energy_account_residual_j_per_m2: 0.0,
        energy_account_tolerance_j_per_m2: 0.0,
        solid_enthalpy_change_j_per_m2: 0.0,
        gas_enthalpy_out_j_per_m2: 0.0,
        solid_mass_lost_kg_per_m2: 0.0,
        gas_mass_out_kg_per_m2: 0.0,
        mass_account_residual_kg_per_m2: 0.0,
        mass_account_tolerance_kg_per_m2: 0.0,
        cells: n as u32,
        time_steps: 0,
        hardest_step_newton_passes: 0,
        hardest_step_time_s: 0.0,
        recession_m: Vec::new(),
        char_mass_flux_kg_per_m2_s: Vec::new(),
        char_mass_removed_kg_per_m2: 0.0,
        char_enthalpy_removed_j_per_m2: 0.0,
        heat_from_stream_j_per_m2: 0.0,
        heat_absorbed_from_radiation_j_per_m2: 0.0,
        heat_radiated_away_j_per_m2: 0.0,
        heat_taken_at_face_by_gases_j_per_m2: 0.0,
        face_balance_residual_j_per_m2: 0.0,
        face_balance_tolerance_j_per_m2: 0.0,
        cells_at_end: n as u32,
        depth_half_way_to_virgin_m: 0.0,
        depth_within_2_percent_of_virgin_m: 0.0,
    };

    // State functions at the start, for the accounts at the end.
    let mut mass_at_start = 0.0;
    let mut enthalpy_at_start = 0.0;
    for i in 0..n {
        let density = material.solid_density(slab.remaining[i]);
        let point = solid_point(&nodes, slab.temperature_k[i], terms.properties_frozen_at_k);
        mass_at_start += density * slab.width_m(i);
        enthalpy_at_start += solid_enthalpy_per_volume(&point, density) * slab.width_m(i);
    }

    let record = |slab: &Slab, result: &mut SlabResult, time_s: f64| {
        result.report_time_s.push(time_s);
        result.surface_temperature_k.push(slab.face_temperature_k);
        for (slot, &depth) in question.thermocouple_depths_m.iter().enumerate() {
            result.thermocouple_temperature_k[slot].push(slab.temperature_at_k(depth));
        }
        result
            .gas_mass_flux_at_surface_kg_per_m2_s
            .push(slab.gas_flux_kg_per_m2_s[0]);
        result
            .char_front_depth_m
            .push(slab.front_depth_m(material, char_front_level));
        result
            .virgin_front_depth_m
            .push(slab.front_depth_m(material, virgin_front_level));
        result.recession_m.push(slab.face_depth_m[0]);
        result
            .char_mass_flux_kg_per_m2_s
            .push(slab.char_flux_kg_per_m2_s);
    };
    record(&slab, &mut result, 0.0);

    let landings = landing_times(question, end_s);
    let mut total = StepAccount::default();
    let mut total_face = FaceAccount::default();
    let mut time_s = 0.0f64;
    let mut last_corner_s = 0.0f64;
    // For a stream: the step the schedule would take next, and the row of
    // the history that the time is in.
    let mut unforced_step_s = numerics.first_step_s;
    let mut row_before = 0usize;
    for &(target_s, is_corner, is_reported) in &landings {
        while time_s < target_s {
            let natural = match &question.surface {
                SurfaceCondition::HeldTemperature { .. } => (numerics.step_fraction
                    * (time_s - last_corner_s))
                    .max(numerics.first_step_s)
                    .min(numerics.largest_step_s),
                SurfaceCondition::GasStream {
                    history,
                    face_wears,
                    ..
                } => {
                    while row_before + 2 < history.len() && time_s >= history[row_before + 1].time_s
                    {
                        row_before += 1;
                        let corner = corner_sharpness(history, row_before);
                        if corner > 0.0 {
                            unforced_step_s = unforced_step_s.min(
                                STEP_AFTER_A_CORNER * numerics.step_fraction
                                    / STEP_FRACTION
                                    / corner,
                            );
                        }
                    }
                    // A front cell worn thin is joined to its neighbour.
                    if *face_wears {
                        while slab.width_m(0) < FRONT_CELL_MERGE_FRACTION * slab.width_m(1) {
                            if slab.cells() <= CELLS_MIN {
                                return Err(SlabReject::SlabWornAway {
                                    time_s,
                                    recession_m: slab.face_depth_m[0],
                                });
                            }
                            slab.join_front_cells(&nodes, terms.properties_frozen_at_k);
                        }
                    }
                    let (_, rate) = stream_at(history, time_s, row_before);
                    let mut natural = unforced_step_s.min(numerics.largest_step_s);
                    if rate > 0.0 {
                        natural = natural.min(
                            FORCING_CHANGE_PER_STEP * numerics.step_fraction / STEP_FRACTION / rate,
                        );
                    }
                    // The face may not cross more than a set share of its cell in a step.
                    let wear_speed =
                        slab.char_flux_kg_per_m2_s / material.solid_density(slab.remaining[0]);
                    if wear_speed > 0.0 {
                        natural =
                            natural.min(numerics.wear_per_step * slab.width_m(0) / wear_speed);
                    }
                    natural
                }
            };
            let left = target_s - time_s;
            let next_s = if left <= natural * 1.000001 {
                target_s
            } else if left < 2.0 * natural {
                time_s + 0.5 * left
            } else {
                time_s + natural
            };
            let step_s = next_s - time_s;
            let drive = match &question.surface {
                SurfaceCondition::HeldTemperature { .. } => FaceDrive::Held {
                    temperature_k: question.surface.face_temperature_k(start_k, next_s),
                },
                SurfaceCondition::GasStream {
                    history,
                    surroundings_temperature_k,
                    face_wears,
                    ..
                } => {
                    let (now, _) = stream_at(history, next_s, row_before);
                    let mut stream = StreamNow {
                        pressure_pa: now.pressure_pa,
                        coefficient: now.heat_transfer_coefficient_kg_per_m2_s,
                        flow_enthalpy_j_per_kg: now.flow_enthalpy_j_per_kg,
                        radiative_heating_w_per_m2: now.radiative_heating_w_per_m2,
                        surroundings_temperature_k: *surroundings_temperature_k,
                        present: true,
                        face_wears: *face_wears,
                    };
                    stream.present = stream.coefficient > 0.0;
                    FaceDrive::Stream(stream)
                }
            };
            let (account, face) = step(
                &mut slab,
                &mut work,
                &nodes,
                &lookup,
                terms,
                numerics.newton_pass_limit,
                next_s,
                step_s,
                &drive,
                lowest_k,
                highest_k,
            )
            .map_err(|reject| match (reject, terms.compared_temperature_top_k) {
                (
                    SlabReject::TemperatureOutOfBounds {
                        time_s,
                        depth_m,
                        temperature_k,
                    },
                    Some(top),
                ) if temperature_k > top => SlabReject::TemperatureAboveCompared {
                    time_s,
                    depth_m,
                    temperature_k,
                },
                (reject, _) => reject,
            })?;
            total.heat_in_j += account.heat_in_j;
            total.heat_stored_j += account.heat_stored_j;
            total.heat_taken_by_gas_j += account.heat_taken_by_gas_j;
            total.heat_given_off_j += account.heat_given_off_j;
            total.gas_enthalpy_out_j += account.gas_enthalpy_out_j;
            total.gas_mass_out_kg += account.gas_mass_out_kg;
            total.energy_tolerance_j += account.energy_tolerance_j;
            total.mass_tolerance_kg += account.mass_tolerance_kg;
            total_face.heat_from_stream_j += face.heat_from_stream_j;
            total_face.heat_absorbed_j += face.heat_absorbed_j;
            total_face.heat_radiated_j += face.heat_radiated_j;
            total_face.heat_taken_by_gases_j += face.heat_taken_by_gases_j;
            total_face.char_removed_kg += face.char_removed_kg;
            total_face.char_enthalpy_removed_j += face.char_enthalpy_removed_j;
            total_face.char_enthalpy_above_cell_j += face.char_enthalpy_above_cell_j;
            total_face.balance_residual_j += face.balance_residual_j;
            total_face.balance_tolerance_j += face.balance_tolerance_j;
            result.time_steps += 1;
            if account.passes > result.hardest_step_newton_passes {
                result.hardest_step_newton_passes = account.passes;
                result.hardest_step_time_s = next_s;
            }
            unforced_step_s =
                (unforced_step_s * (1.0 + numerics.step_fraction)).min(numerics.largest_step_s);
            time_s = next_s;
        }
        if is_corner {
            last_corner_s = target_s;
        }
        if is_reported {
            record(&slab, &mut result, target_s);
        }
    }

    // The accounts of the whole run. The solid's mass and enthalpy are read
    // from the first and last states alone; the rest was summed step by step.
    let n = slab.cells();
    let mut mass_at_end = 0.0;
    let mut enthalpy_at_end = 0.0;
    for i in 0..n {
        let density = material.solid_density(slab.remaining[i]);
        let point = solid_point(&nodes, slab.temperature_k[i], terms.properties_frozen_at_k);
        mass_at_end += density * slab.width_m(i);
        enthalpy_at_end += solid_enthalpy_per_volume(&point, density) * slab.width_m(i);
        result.final_depth_m.push(slab.centre_m(i));
        result.final_temperature_k.push(slab.temperature_k[i]);
        result.final_density_kg_per_m3.push(density);
    }
    let solid_lost = mass_at_start - mass_at_end;
    let solid_enthalpy_change = enthalpy_at_end - enthalpy_at_start;
    // Solid lost to decomposition is what was lost less the char worn away;
    // the solid's enthalpy changed by heat conducted in, less what the gas
    // and the worn char carried out.
    total.solid_mass_lost_kg = solid_lost - total_face.char_removed_kg;
    total.solid_enthalpy_change_j = solid_enthalpy_change + total_face.char_enthalpy_removed_j
        - total_face.char_enthalpy_above_cell_j;
    ensure_mass_account(end_s, &total)?;
    ensure_energy_account(
        end_s,
        &StepAccount {
            heat_in_j: total.heat_in_j - total_face.char_enthalpy_above_cell_j,
            ..total
        },
    )?;
    ensure_face_balance(end_s, &total_face)?;

    result.final_surface_density_kg_per_m3 = material.solid_density(slab.face_remaining);
    result.heat_in_through_face_j_per_m2 = total.heat_in_j;
    result.heat_stored_in_solid_j_per_m2 = total.heat_stored_j;
    result.heat_taken_by_gas_j_per_m2 = total.heat_taken_by_gas_j;
    result.heat_given_off_by_decomposition_j_per_m2 = total.heat_given_off_j;
    result.energy_account_residual_j_per_m2 =
        energy_residual_j(&total) + total_face.char_enthalpy_above_cell_j;
    result.energy_account_tolerance_j_per_m2 = total.energy_tolerance_j;
    result.solid_enthalpy_change_j_per_m2 = solid_enthalpy_change;
    result.gas_enthalpy_out_j_per_m2 = total.gas_enthalpy_out_j;
    result.solid_mass_lost_kg_per_m2 = solid_lost;
    result.gas_mass_out_kg_per_m2 = total.gas_mass_out_kg;
    result.mass_account_residual_kg_per_m2 =
        solid_lost - total.gas_mass_out_kg - total_face.char_removed_kg;
    result.mass_account_tolerance_kg_per_m2 = total.mass_tolerance_kg;
    result.char_mass_removed_kg_per_m2 = total_face.char_removed_kg;
    result.char_enthalpy_removed_j_per_m2 = total_face.char_enthalpy_removed_j;
    result.heat_from_stream_j_per_m2 = total_face.heat_from_stream_j;
    result.heat_absorbed_from_radiation_j_per_m2 = total_face.heat_absorbed_j;
    result.heat_radiated_away_j_per_m2 = total_face.heat_radiated_j;
    result.heat_taken_at_face_by_gases_j_per_m2 = total_face.heat_taken_by_gases_j;
    result.face_balance_residual_j_per_m2 = total_face.balance_residual_j;
    result.face_balance_tolerance_j_per_m2 = total_face.balance_tolerance_j;
    result.cells_at_end = n as u32;
    let virgin = material.virgin_density_kg_per_m3;
    result.depth_half_way_to_virgin_m = slab
        .front_depth_m(material, 0.5 * (virgin + material.char_density_kg_per_m3))
        - slab.face_depth_m[0];
    result.depth_within_2_percent_of_virgin_m =
        slab.front_depth_m(material, 0.98 * virgin) - slab.face_depth_m[0];
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    // -----------------------------------------------------------------------
    // Reference numbers, copied from files. Nothing below was computed here.
    // -----------------------------------------------------------------------

    /// FIAT on Test Case 1. File `tutorials/1D/AblationTestCase_1.0/data/ref/FIAT/T`
    /// of PATO 3.1 ([`REFERENCE_URL`]), as transcribed in
    /// `reference/case1_FIAT_temperature.csv`. Time (s), then temperature (K)
    /// at the face and at 1, 2, 4, 8, 16 and 50 mm.
    const FIAT_TEMPERATURE: [[f64; 8]; 61] = [
        [0.0, 298.0, 298.0, 298.0, 298.0, 298.0, 298.0, 298.0],
        [1.0, 1644.0, 949.72, 556.62, 318.01, 298.01, 298.0, 298.0],
        [2.0, 1644.0, 1121.74, 757.83, 397.23, 299.19, 298.0, 298.0],
        [3.0, 1644.0, 1205.63, 871.95, 483.82, 305.32, 298.0, 298.0],
        [4.0, 1644.0, 1258.06, 947.23, 559.25, 317.42, 298.01, 298.0],
        [5.0, 1644.0, 1294.2, 1004.14, 622.42, 334.08, 298.05, 298.0],
        [6.0, 1644.0, 1321.44, 1048.78, 675.54, 353.67, 298.2, 298.0],
        [7.0, 1644.0, 1342.44, 1085.41, 720.48, 374.97, 298.53, 298.0],
        [8.0, 1644.0, 1359.72, 1116.03, 758.64, 397.12, 299.13, 298.0],
        [9.0, 1644.0, 1374.14, 1141.4, 791.46, 419.58, 300.05, 298.0],
        [
            10.0, 1644.0, 1386.43, 1163.12, 819.87, 441.99, 301.36, 298.0,
        ],
        [
            11.0, 1644.0, 1397.15, 1182.14, 845.01, 464.02, 303.06, 298.0,
        ],
        [
            12.0, 1644.0, 1406.41, 1198.66, 867.45, 485.32, 305.18, 298.0,
        ],
        [
            13.0, 1644.0, 1414.62, 1213.43, 888.01, 505.83, 307.69, 298.0,
        ],
        [
            14.0, 1644.0, 1422.06, 1226.73, 906.88, 525.49, 310.57, 298.0,
        ],
        [15.0, 1644.0, 1428.8, 1238.71, 924.35, 544.29, 313.79, 298.0],
        [
            16.0, 1644.0, 1434.91, 1249.56, 940.66, 562.27, 317.33, 298.0,
        ],
        [17.0, 1644.0, 1440.5, 1259.55, 955.9, 579.49, 321.16, 298.0],
        [18.0, 1644.0, 1445.62, 1268.74, 970.1, 595.93, 325.27, 298.0],
        [19.0, 1644.0, 1450.38, 1277.28, 983.66, 611.69, 329.6, 298.0],
        [
            20.0, 1644.0, 1454.72, 1285.13, 996.31, 626.64, 334.17, 298.0,
        ],
        [
            21.0, 1644.0, 1458.85, 1292.62, 1008.42, 641.06, 338.89, 298.0,
        ],
        [22.0, 1644.0, 1462.7, 1299.6, 1019.82, 654.74, 343.79, 298.0],
        [
            23.0, 1644.0, 1466.29, 1306.12, 1030.57, 667.74, 348.85, 298.0,
        ],
        [
            24.0, 1644.0, 1469.66, 1312.25, 1040.82, 680.15, 354.03, 298.0,
        ],
        [
            25.0, 1644.0, 1472.77, 1317.93, 1050.47, 691.81, 359.35, 298.0,
        ],
        [
            26.0, 1644.0, 1475.71, 1323.31, 1059.67, 702.9, 364.76, 298.0,
        ],
        [
            27.0, 1644.0, 1478.5, 1328.42, 1068.43, 713.43, 370.26, 298.0,
        ],
        [28.0, 1644.0, 1481.15, 1333.3, 1076.7, 723.45, 375.82, 298.0],
        [
            29.0, 1644.0, 1483.67, 1337.94, 1084.58, 733.03, 381.44, 298.0,
        ],
        [
            30.0, 1644.0, 1486.08, 1342.36, 1092.13, 742.17, 387.11, 298.0,
        ],
        [
            31.0, 1644.0, 1488.36, 1346.59, 1099.37, 750.9, 392.82, 298.0,
        ],
        [
            32.0, 1644.0, 1490.55, 1350.63, 1106.32, 759.27, 398.56, 298.0,
        ],
        [
            33.0, 1644.0, 1492.64, 1354.51, 1112.99, 767.31, 404.33, 298.0,
        ],
        [
            34.0, 1644.0, 1494.64, 1358.21, 1119.36, 775.01, 410.11, 298.0,
        ],
        [
            35.0, 1644.0, 1496.56, 1361.76, 1125.5, 782.43, 415.91, 298.0,
        ],
        [
            36.0, 1644.0, 1498.4, 1365.17, 1131.39, 789.59, 421.72, 298.01,
        ],
        [
            37.0, 1644.0, 1500.18, 1368.47, 1137.06, 796.51, 427.53, 298.01,
        ],
        [
            38.0, 1644.0, 1501.89, 1371.65, 1142.54, 803.16, 433.34, 298.01,
        ],
        [
            39.0, 1644.0, 1503.54, 1374.72, 1147.82, 809.6, 439.15, 298.01,
        ],
        [
            40.0, 1644.0, 1505.13, 1377.67, 1152.9, 815.85, 444.94, 298.01,
        ],
        [
            41.0, 1644.0, 1506.66, 1380.52, 1157.8, 821.92, 450.72, 298.02,
        ],
        [
            42.0, 1644.0, 1508.13, 1383.26, 1162.55, 827.85, 456.46, 298.02,
        ],
        [
            43.0, 1644.0, 1509.56, 1385.91, 1167.15, 833.63, 462.17, 298.03,
        ],
        [
            44.0, 1644.0, 1510.93, 1388.48, 1171.62, 839.25, 467.84, 298.03,
        ],
        [
            45.0, 1644.0, 1512.27, 1390.97, 1175.95, 844.73, 473.47, 298.04,
        ],
        [
            46.0, 1644.0, 1513.56, 1393.38, 1180.12, 850.1, 479.06, 298.05,
        ],
        [
            47.0, 1644.0, 1514.81, 1395.72, 1184.16, 855.37, 484.6, 298.06,
        ],
        [
            48.0, 1644.0, 1516.02, 1397.99, 1188.08, 860.51, 490.09, 298.07,
        ],
        [
            49.0, 1644.0, 1517.2, 1400.2, 1191.89, 865.53, 495.54, 298.09,
        ],
        [
            50.0, 1644.0, 1518.35, 1402.34, 1195.6, 870.44, 500.93, 298.1,
        ],
        [
            51.0, 1644.0, 1519.46, 1404.43, 1199.21, 875.26, 506.27, 298.12,
        ],
        [
            52.0, 1644.0, 1520.54, 1406.46, 1202.73, 880.0, 511.56, 298.14,
        ],
        [
            53.0, 1644.0, 1521.59, 1408.43, 1206.15, 884.65, 516.8, 298.16,
        ],
        [
            54.0, 1644.0, 1522.61, 1410.35, 1209.47, 889.21, 521.98, 298.18,
        ],
        [
            55.0, 1644.0, 1523.61, 1412.21, 1212.72, 893.68, 527.11, 298.21,
        ],
        [
            56.0, 1644.0, 1524.58, 1414.02, 1215.88, 898.07, 532.18, 298.24,
        ],
        [
            57.0, 1644.0, 1525.52, 1415.8, 1218.97, 902.41, 537.2, 298.27,
        ],
        [
            58.0, 1644.0, 1526.44, 1417.53, 1221.98, 906.67, 542.15, 298.31,
        ],
        [
            59.0, 1644.0, 1527.34, 1419.22, 1224.93, 910.85, 547.05, 298.35,
        ],
        [
            60.0, 1644.0, 1528.22, 1420.87, 1227.8, 914.97, 551.89, 298.39,
        ],
    ];

    /// FIAT on Test Case 1. File `.../data/ref/FIAT/pyrolysisFront` of PATO
    /// 3.1, as transcribed in `reference/case1_FIAT_pyrolysis.csv`. Time (s),
    /// gas mass flux at the face (kg/m²/s), depth of the char front (m), depth
    /// of the virgin front (m).
    const FIAT_GAS_AND_FRONTS: [[f64; 4]; 61] = [
        [0.0, 0.0, 0.0, 0.0],
        [1.0, 0.0281, 0.0, 0.001433],
        [2.0, 0.0217, 0.0, 0.002071],
        [3.0, 0.0185, 0.0, 0.002671],
        [4.0, 0.0165, 0.0, 0.003161],
        [5.0, 0.0151, 0.0, 0.003593],
        [6.0, 0.014, 0.0, 0.003989],
        [7.0, 0.0132, 4e-06, 0.004355],
        [8.0, 0.0125, 5.8e-05, 0.004687],
        [9.0, 0.0119, 0.000116, 0.005002],
        [10.0, 0.0114, 0.000178, 0.005345],
        [11.0, 0.0109, 0.000243, 0.005654],
        [12.0, 0.0106, 0.000309, 0.005928],
        [13.0, 0.0102, 0.000378, 0.006215],
        [14.0, 0.0099, 0.00044, 0.006502],
        [15.0, 0.00962, 0.000493, 0.00674],
        [16.0, 0.00937, 0.000547, 0.007014],
        [17.0, 0.00913, 0.000604, 0.007262],
        [18.0, 0.00892, 0.000663, 0.007493],
        [19.0, 0.00872, 0.000724, 0.007746],
        [20.0, 0.00855, 0.000786, 0.007953],
        [21.0, 0.00837, 0.000847, 0.008202],
        [22.0, 0.00821, 0.000896, 0.008398],
        [23.0, 0.00806, 0.000946, 0.00864],
        [24.0, 0.00792, 0.000998, 0.008833],
        [25.0, 0.00779, 0.00105, 0.009069],
        [26.0, 0.00767, 0.001104, 0.009268],
        [27.0, 0.00755, 0.001159, 0.009489],
        [28.0, 0.00744, 0.001216, 0.009677],
        [29.0, 0.00733, 0.001269, 0.009892],
        [30.0, 0.00723, 0.001314, 0.01007],
        [31.0, 0.00713, 0.001359, 0.01028],
        [32.0, 0.00703, 0.001406, 0.01045],
        [33.0, 0.00694, 0.001455, 0.01065],
        [34.0, 0.00685, 0.001504, 0.01083],
        [35.0, 0.00677, 0.001555, 0.01102],
        [36.0, 0.00669, 0.001606, 0.0112],
        [37.0, 0.00662, 0.001659, 0.01136],
        [38.0, 0.00654, 0.001703, 0.01155],
        [39.0, 0.00647, 0.001746, 0.01171],
        [40.0, 0.0064, 0.001789, 0.01189],
        [41.0, 0.00634, 0.001833, 0.01206],
        [42.0, 0.00627, 0.001878, 0.01222],
        [43.0, 0.00621, 0.001924, 0.0124],
        [44.0, 0.00615, 0.001971, 0.01254],
        [45.0, 0.00609, 0.002019, 0.01272],
        [46.0, 0.00604, 0.002067, 0.01288],
        [47.0, 0.00599, 0.002112, 0.01303],
        [48.0, 0.00593, 0.002151, 0.0132],
        [49.0, 0.00588, 0.002192, 0.01335],
        [50.0, 0.00583, 0.002233, 0.01351],
        [51.0, 0.00578, 0.002275, 0.01367],
        [52.0, 0.00574, 0.002317, 0.0138],
        [53.0, 0.00569, 0.002361, 0.01397],
        [54.0, 0.00565, 0.002405, 0.01412],
        [55.0, 0.00561, 0.00245, 0.01426],
        [56.0, 0.00556, 0.002495, 0.01442],
        [57.0, 0.00552, 0.002536, 0.01456],
        [58.0, 0.00548, 0.002573, 0.0147],
        [59.0, 0.00544, 0.002611, 0.01486],
        [60.0, 0.00541, 0.00265, 0.015],
    ];

    /// PATO 3.1's own stored result for the same case, started at 300 K. File
    /// `src/applications/utilities/tests/testsuites/tutorials/ref/1D/AblationTestCase_1.0/output/porousMat/scalar/Ta_plot`
    /// of PATO 3.1, as transcribed in `reference/case1_PATO_temperature.csv`,
    /// whole seconds only. Time (s), then temperature (K) at 1, 2, 4, 8, 12,
    /// 16 and 24 mm.
    const PATO_TEMPERATURE: [[f64; 8]; 61] = [
        [0.0, 300.0, 300.0, 300.0, 300.0, 300.0, 300.0, 300.0],
        [
            1.0, 963.479, 557.868, 316.772, 300.004, 300.001, 300.001, 300.001,
        ],
        [
            2.0, 1131.52, 767.975, 397.214, 300.834, 300.002, 300.002, 300.002,
        ],
        [
            3.0, 1211.44, 884.334, 487.273, 306.31, 300.051, 300.002, 300.002,
        ],
        [
            4.0, 1260.94, 960.441, 564.976, 318.081, 300.422, 300.005, 300.002,
        ],
        [
            5.0, 1295.73, 1016.27, 629.5, 334.802, 301.581, 300.029, 300.002,
        ],
        [
            6.0, 1321.99, 1059.55, 683.439, 354.709, 303.89, 300.128, 300.002,
        ],
        [
            7.0, 1342.77, 1094.31, 728.78, 376.45, 307.492, 300.381, 300.002,
        ],
        [
            8.0, 1359.75, 1122.96, 767.043, 399.108, 312.353, 300.878, 300.003,
        ],
        [
            9.0, 1373.98, 1147.1, 799.662, 422.089, 318.35, 301.694, 300.005,
        ],
        [
            10.0, 1386.14, 1167.81, 827.961, 444.999, 325.32, 302.884, 300.011,
        ],
        [
            11.0, 1396.69, 1185.85, 852.983, 467.447, 333.1, 304.478, 300.025,
        ],
        [
            12.0, 1405.95, 1201.77, 875.472, 489.145, 341.537, 306.485, 300.051,
        ],
        [
            13.0, 1414.18, 1215.98, 895.95, 509.979, 350.498, 308.9, 300.097,
        ],
        [
            14.0, 1421.56, 1228.77, 914.762, 529.909, 359.869, 311.705, 300.169,
        ],
        [
            15.0, 1428.21, 1240.37, 932.185, 548.935, 369.554, 314.877, 300.277,
        ],
        [
            16.0, 1434.26, 1250.97, 948.399, 567.091, 379.474, 318.387, 300.427,
        ],
        [
            17.0, 1439.79, 1260.69, 963.513, 584.438, 389.564, 322.208, 300.627,
        ],
        [
            18.0, 1444.89, 1269.68, 977.687, 600.998, 399.768, 326.309, 300.885,
        ],
        [
            19.0, 1449.59, 1278.0, 990.959, 616.788, 410.042, 330.661, 301.206,
        ],
        [
            20.0, 1453.94, 1285.75, 1003.45, 631.829, 420.35, 335.238, 301.597,
        ],
        [
            21.0, 1458.0, 1293.0, 1015.19, 646.136, 430.664, 340.015, 302.061,
        ],
        [
            22.0, 1461.78, 1299.77, 1026.28, 659.739, 440.944, 344.967, 302.602,
        ],
        [
            23.0, 1465.33, 1306.13, 1036.73, 672.665, 451.157, 350.074, 303.222,
        ],
        [
            24.0, 1468.66, 1312.13, 1046.63, 684.936, 461.266, 355.315, 303.924,
        ],
        [
            25.0, 1471.81, 1317.8, 1056.0, 696.578, 471.244, 360.674, 304.708,
        ],
        [
            26.0, 1474.77, 1323.16, 1064.89, 707.618, 481.074, 366.133, 305.574,
        ],
        [
            27.0, 1477.57, 1328.24, 1073.36, 718.093, 490.744, 371.679, 306.522,
        ],
        [
            28.0, 1480.24, 1333.08, 1081.39, 728.035, 500.248, 377.299, 307.552,
        ],
        [
            29.0, 1482.77, 1337.67, 1089.06, 737.49, 509.579, 382.98, 308.661,
        ],
        [
            30.0, 1485.18, 1342.05, 1096.4, 746.49, 518.734, 388.712, 309.849,
        ],
        [
            31.0, 1487.47, 1346.23, 1103.38, 755.078, 527.71, 394.487, 311.114,
        ],
        [
            32.0, 1489.66, 1350.23, 1110.08, 763.291, 536.504, 400.294, 312.453,
        ],
        [
            33.0, 1491.75, 1354.06, 1116.51, 771.162, 545.121, 406.129, 313.864,
        ],
        [
            34.0, 1493.76, 1357.73, 1122.67, 778.717, 553.558, 411.98, 315.345,
        ],
        [
            35.0, 1495.68, 1361.25, 1128.58, 785.993, 561.821, 417.847, 316.893,
        ],
        [
            36.0, 1497.52, 1364.64, 1134.27, 793.013, 569.915, 423.718, 318.506,
        ],
        [
            37.0, 1499.3, 1367.9, 1139.75, 799.791, 577.838, 429.596, 320.181,
        ],
        [
            38.0, 1501.0, 1371.03, 1145.02, 806.354, 585.593, 435.465, 321.916,
        ],
        [
            39.0, 1502.64, 1374.05, 1150.11, 812.722, 593.183, 441.327, 323.708,
        ],
        [
            40.0, 1504.22, 1376.97, 1155.03, 818.904, 600.597, 447.171, 325.555,
        ],
        [
            41.0, 1505.75, 1379.78, 1159.78, 824.913, 607.846, 452.988, 327.454,
        ],
        [
            42.0, 1507.23, 1382.5, 1164.36, 830.763, 614.923, 458.772, 329.403,
        ],
        [
            43.0, 1508.65, 1385.13, 1168.81, 836.466, 621.824, 464.522, 331.4,
        ],
        [
            44.0, 1510.03, 1387.68, 1173.12, 842.031, 628.559, 470.227, 333.441,
        ],
        [
            45.0, 1511.36, 1390.14, 1177.29, 847.461, 635.122, 475.887, 335.525,
        ],
        [
            46.0, 1512.65, 1392.53, 1181.33, 852.764, 641.514, 481.503, 337.651,
        ],
        [
            47.0, 1513.9, 1394.85, 1185.25, 857.953, 647.741, 487.067, 339.814,
        ],
        [
            48.0, 1515.12, 1397.1, 1189.07, 863.032, 653.804, 492.581, 342.015,
        ],
        [
            49.0, 1516.3, 1399.29, 1192.78, 868.003, 659.705, 498.046, 344.25,
        ],
        [
            50.0, 1517.44, 1401.41, 1196.39, 872.872, 665.447, 503.454, 346.518,
        ],
        [
            51.0, 1518.55, 1403.47, 1199.9, 877.648, 671.033, 508.808, 348.817,
        ],
        [
            52.0, 1519.63, 1405.48, 1203.32, 882.335, 676.47, 514.109, 351.145,
        ],
        [
            53.0, 1520.68, 1407.43, 1206.65, 886.934, 681.757, 519.35, 353.502,
        ],
        [
            54.0, 1521.71, 1409.33, 1209.89, 891.449, 686.901, 524.535, 355.884,
        ],
        [
            55.0, 1522.7, 1411.18, 1213.05, 895.883, 691.906, 529.664, 358.291,
        ],
        [
            56.0, 1523.67, 1412.99, 1216.14, 900.245, 696.777, 534.73, 360.721,
        ],
        [
            57.0, 1524.62, 1414.75, 1219.16, 904.536, 701.518, 539.739, 363.173,
        ],
        [
            58.0, 1525.54, 1416.47, 1222.11, 908.756, 706.134, 544.691, 365.645,
        ],
        [
            59.0, 1526.44, 1418.14, 1224.98, 912.904, 710.633, 549.582, 368.137,
        ],
        [
            60.0, 1527.32, 1419.78, 1227.79, 916.984, 715.017, 554.413, 370.646,
        ],
    ];

    /// The enthalpy columns of `tacot_virgin_thermal.csv` and
    /// `tacot_char_thermal.csv`, J/kg.
    const VIRGIN_ENTHALPY_COLUMN_J_PER_KG: [f64; SOLID_NODES] = [
        -896682.5311428572,
        -857142.8571428572,
        -690063.9511428571,
        -536547.9511428571,
        -401639.951142857,
        -91235.25114285719,
        405947.24885714287,
        933367.748857143,
        1477070.2488571426,
        2028332.2488571429,
        2582501.748857143,
        3696655.7488571433,
        4813135.748857142,
    ];
    const CHAR_ENTHALPY_COLUMN_J_PER_KG: [f64; SOLID_NODES] = [
        -32164.85840000001,
        0.0,
        137341.92640000003,
        271319.5264,
        393574.08640000003,
        686975.7263999999,
        1175435.7264,
        1704600.7264,
        2262840.7263999996,
        2838525.7264,
        3422351.7264,
        4601633.7264,
        5792545.726399999,
    ];

    // -----------------------------------------------------------------------
    // Helpers.
    // -----------------------------------------------------------------------

    fn question(
        thickness_m: f64,
        start_temperature_k: f64,
        held_temperature_k: f64,
        hold_duration_s: f64,
        continue_after_s: f64,
        thermocouple_depths_m: &[f64],
    ) -> SlabQuestion {
        SlabQuestion {
            thickness_m,
            start_temperature_k,
            surface: SurfaceCondition::HeldTemperature {
                held_temperature_k,
                hold_duration_s,
            },
            continue_after_s,
            thermocouple_depths_m: thermocouple_depths_m.to_vec(),
        }
    }

    /// Test Case 1, solved once for every test that reads it.
    fn test_case_1() -> &'static SlabResult {
        static RESULT: OnceLock<SlabResult> = OnceLock::new();
        RESULT.get_or_init(|| solve(&ablation_workshop_test_case_1()).expect("test case 1"))
    }

    /// Test Case 1 as PATO's tutorial sets it up: from 300 K, with PATO's
    /// probe depths.
    fn test_case_1_from_300_k() -> &'static SlabResult {
        static RESULT: OnceLock<SlabResult> = OnceLock::new();
        RESULT.get_or_init(|| {
            let mut q = ablation_workshop_test_case_1();
            q.start_temperature_k = 300.0;
            q.thermocouple_depths_m = vec![0.001, 0.002, 0.004, 0.008, 0.012, 0.016, 0.024];
            solve(&q).expect("test case 1 from 300 K")
        })
    }

    fn slot_at(result: &SlabResult, time_s: f64) -> usize {
        result
            .report_time_s
            .iter()
            .position(|&t| t == time_s)
            .unwrap_or_else(|| panic!("no report at {time_s} s"))
    }

    fn percent(value: f64, reference: f64) -> f64 {
        100.0 * (value - reference) / reference
    }

    /// Largest absolute difference between two runs over every thermocouple
    /// and every report time, kelvin, and where it is.
    fn largest_movement_k(a: &SlabResult, b: &SlabResult) -> (f64, f64, f64) {
        assert_eq!(a.report_time_s, b.report_time_s);
        let mut worst = (0.0f64, 0.0, 0.0);
        for (slot, depth) in a.thermocouple_depth_m.iter().enumerate() {
            for (k, &t) in a.report_time_s.iter().enumerate() {
                let d = (a.thermocouple_temperature_k[slot][k]
                    - b.thermocouple_temperature_k[slot][k])
                    .abs();
                if d > worst.0 {
                    worst = (d, t, *depth);
                }
            }
        }
        worst
    }

    fn every_number(result: &SlabResult) -> Vec<f64> {
        let mut all = Vec::new();
        all.extend(&result.report_time_s);
        all.extend(&result.surface_temperature_k);
        all.extend(&result.thermocouple_depth_m);
        for history in &result.thermocouple_temperature_k {
            all.extend(history);
        }
        all.extend(&result.gas_mass_flux_at_surface_kg_per_m2_s);
        all.extend(&result.char_front_depth_m);
        all.extend(&result.virgin_front_depth_m);
        all.extend(&result.final_depth_m);
        all.extend(&result.final_temperature_k);
        all.extend(&result.final_density_kg_per_m3);
        all.extend([
            result.final_surface_density_kg_per_m3,
            result.heat_in_through_face_j_per_m2,
            result.heat_stored_in_solid_j_per_m2,
            result.heat_taken_by_gas_j_per_m2,
            result.heat_given_off_by_decomposition_j_per_m2,
            result.energy_account_residual_j_per_m2,
            result.energy_account_tolerance_j_per_m2,
            result.solid_enthalpy_change_j_per_m2,
            result.gas_enthalpy_out_j_per_m2,
            result.solid_mass_lost_kg_per_m2,
            result.gas_mass_out_kg_per_m2,
            result.mass_account_residual_kg_per_m2,
            result.mass_account_tolerance_kg_per_m2,
            result.cells as f64,
            result.time_steps as f64,
            result.hardest_step_newton_passes as f64,
            result.hardest_step_time_s,
        ]);
        all
    }

    // -----------------------------------------------------------------------
    // The material as locked in.
    // -----------------------------------------------------------------------

    #[test]
    fn virgin_is_280_and_char_is_220_kg_per_m3() {
        assert_eq!(solid_density(virgin_remaining()), VIRGIN_DENSITY_KG_PER_M3);
        assert_eq!(solid_density([0.0, 0.0]), CHAR_DENSITY_KG_PER_M3);
        // Each part gives up 30 kg per cubic metre of material.
        let without_a = solid_density([0.0, virgin_remaining()[1]]);
        let without_b = solid_density([virgin_remaining()[0], 0.0]);
        assert!((VIRGIN_DENSITY_KG_PER_M3 - without_a - 30.0).abs() < 1.0e-12);
        assert!((VIRGIN_DENSITY_KG_PER_M3 - without_b - 30.0).abs() < 1.0e-12);
    }

    #[test]
    fn enthalpy_integrated_from_specific_heat_equals_the_tables_enthalpy_columns() {
        let nodes = SolidEnthalpyNodes::new();
        for j in 0..SOLID_NODES {
            let v = nodes.virgin_j_per_kg[j] - VIRGIN_ENTHALPY_COLUMN_J_PER_KG[j];
            let c = nodes.char_j_per_kg[j] - CHAR_ENTHALPY_COLUMN_J_PER_KG[j];
            assert!(
                v.abs() < 1.0e-6 && c.abs() < 1.0e-6,
                "node {j}: {v} {c} J/kg"
            );
            // And the interpolant passes through the node.
            let point = solid_point(&nodes, SOLID_TABLE_K[j], None);
            assert!((point.virgin_enthalpy - nodes.virgin_j_per_kg[j]).abs() < 1.0e-6);
            assert!((point.char_enthalpy - nodes.char_j_per_kg[j]).abs() < 1.0e-6);
            assert!(
                (point.virgin_specific_heat - VIRGIN_SPECIFIC_HEAT_J_PER_KG_K[j]).abs() < 1.0e-9
            );
            assert!((point.char_conductivity - CHAR_CONDUCTIVITY_W_PER_M_K[j]).abs() < 1.0e-12);
        }
    }

    #[test]
    fn decomposition_gives_off_heat_below_939_k_and_takes_it_up_above() {
        let nodes = SolidEnthalpyNodes::new();
        let heat =
            |t: f64| gas_enthalpy(t).0 - enthalpy_of_solid_lost(&solid_point(&nodes, t, None));
        // `MODEL_NEEDED.md`, section 5: h_g − h̄ in MJ/kg.
        for (t, expect) in [
            (298.0, -3.09),
            (500.0, -3.10),
            (700.0, -2.80),
            (800.0, -2.00),
            (1000.0, 0.46),
            (1400.0, 2.23),
            (1644.0, 4.48),
        ] {
            let got = heat(t) / 1.0e6;
            assert!(
                (got - expect).abs() < 0.02,
                "{t} K: {got} against {expect} MJ/kg"
            );
        }
        let crossing = heat_release_ends_k(&nodes);
        assert!((crossing - 939.4).abs() < 0.1, "{crossing}");
        assert!(heat(crossing - 1.0) < 0.0 && heat(crossing + 1.0) > 0.0);
    }

    #[test]
    fn the_closed_form_decomposition_step_equals_many_small_steps() {
        for (part, temperature_k, step_s) in [
            (0usize, 700.0, 2.0),
            (0, 1644.0, 0.05),
            (1, 900.0, 1.0),
            (1, 1644.0, 0.001),
            (1, 3300.0, 0.05),
        ] {
            let p = &RESIN_PARTS[part];
            let start = virgin_remaining()[part];
            let (left, _) = decomposed(p, start, temperature_k, true, step_s);
            let rate = p.rate_constant_per_s * (-p.activation_temperature_k / temperature_k).exp();
            let f = |u: f64| -rate * u * u * u;
            let pieces = 200_000;
            let h = step_s / pieces as f64;
            let mut u = start;
            for _ in 0..pieces {
                let k1 = f(u);
                let k2 = f(u + 0.5 * h * k1);
                let k3 = f(u + 0.5 * h * k2);
                let k4 = f(u + h * k3);
                u += h * (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0;
            }
            assert!(
                (left - u).abs() < 1.0e-9 * start,
                "part {part} at {temperature_k} K: {left} against {u}"
            );
            assert!(left <= start && left >= 0.0);
        }
        // Below its threshold, or switched off, a part does not move.
        assert_eq!(decomposed(&RESIN_PARTS[1], 0.2, 3000.0, false, 10.0).0, 0.2);
    }

    #[test]
    fn the_slopes_newton_uses_match_differences() {
        let nodes = SolidEnthalpyNodes::new();
        for t in [320.0, 610.0, 905.0, 1500.0, 2400.0, 3200.0] {
            let dt = 1.0e-3;
            let (h, slope) = gas_enthalpy(t);
            assert!(((gas_enthalpy(t + dt).0 - h) / dt - slope).abs() < 1.0e-3 * slope.abs());
            for density in [225.0, 250.0, 279.0] {
                let here = solid_point(&nodes, t, None);
                let ahead = solid_point(&nodes, t + dt, None);
                let behind = solid_point(&nodes, t - dt, None);
                let numeric = (solid_enthalpy_per_volume(&ahead, density)
                    - solid_enthalpy_per_volume(&behind, density))
                    / (2.0 * dt);
                let exact = heat_capacity_per_volume(&here, density);
                assert!(
                    (numeric - exact).abs() < 1.0e-6 * exact,
                    "ρc_p at {t} K: {numeric} {exact}"
                );
                let dr = 1.0e-4;
                let numeric = (solid_enthalpy_per_volume(&here, density + dr)
                    - solid_enthalpy_per_volume(&here, density - dr))
                    / (2.0 * dr);
                let exact = enthalpy_of_solid_lost(&here);
                assert!((numeric - exact).abs() < 1.0e-6 * exact.abs(), "h̄ at {t} K");
                let numeric =
                    (enthalpy_of_solid_lost(&ahead) - enthalpy_of_solid_lost(&behind)) / (2.0 * dt);
                let exact = enthalpy_of_solid_lost_slope(&here);
                assert!(
                    (numeric - exact).abs() < 1.0e-6 * exact.abs(),
                    "slope of h̄ at {t} K"
                );
                let (_, k_t, k_r) = conductivity(&here, density);
                let numeric = (conductivity(&ahead, density).0 - conductivity(&behind, density).0)
                    / (2.0 * dt);
                assert!((numeric - k_t).abs() < 1.0e-9, "k slope in T at {t} K");
                let numeric = (conductivity(&here, density + dr).0
                    - conductivity(&here, density - dr).0)
                    / (2.0 * dr);
                assert!((numeric - k_r).abs() < 1.0e-9, "k slope in ρ at {t} K");
            }
            for part in 0..2 {
                let p = &RESIN_PARTS[part];
                let wide = 0.1;
                let (_, slope) = decomposed(p, 0.3, t, true, 0.01);
                let numeric = (decomposed(p, 0.3, t + wide, true, 0.01).0
                    - decomposed(p, 0.3, t - wide, true, 0.01).0)
                    / (2.0 * wide);
                assert!(
                    (numeric - slope).abs() <= 2.0e-4 * slope.abs() + 1.0e-13,
                    "part {part} at {t} K: {numeric} {slope}"
                );
            }
        }
    }

    #[test]
    fn the_cells_fill_the_slab_exactly_and_are_finest_at_the_heated_face() {
        for thickness in [THICKNESS_MIN_M, 0.0237, 0.05, THICKNESS_MAX_M] {
            let faces = cell_faces(thickness, &NUMERICS);
            assert_eq!(faces[0], 0.0);
            assert_eq!(*faces.last().unwrap(), thickness);
            let mut previous = 0.0;
            for pair in faces.windows(2) {
                let width = pair[1] - pair[0];
                assert!(width > 0.0 && width >= previous * (1.0 - 1.0e-9));
                assert!(width <= LARGEST_CELL_M * (1.0 + 1.0e-9));
                previous = width;
            }
            assert!(faces[1] <= FIRST_CELL_M);
        }
    }

    // -----------------------------------------------------------------------
    // The test that matters: Test Case 1 against FIAT.
    // -----------------------------------------------------------------------

    #[test]
    fn test_case_1_against_fiat() {
        let r = test_case_1();
        let mut sum = 0.0;
        let mut count = 0.0;
        let mut worst = (0.0f64, 0.0, 0usize);
        let mut beyond_one_percent = 0;
        eprintln!("Test Case 1, this solver minus FIAT, K (percent of FIAT)");
        eprintln!(
            "   t s      1 mm            2 mm            4 mm            8 mm           16 mm           50 mm"
        );
        for row in FIAT_TEMPERATURE.iter().skip(1) {
            let k = slot_at(r, row[0]);
            let mut line = format!("{:6.0}", row[0]);
            for slot in 0..6 {
                let ours = r.thermocouple_temperature_k[slot][k];
                let pct = percent(ours, row[2 + slot]);
                line += &format!("  {:+7.2} ({:+5.2})", ours - row[2 + slot], pct);
                if slot < 5 {
                    sum += pct * pct;
                    count += 1.0;
                    if pct.abs() > worst.0.abs() {
                        worst = (pct, row[0], slot);
                    }
                    if pct.abs() > 1.0 {
                        beyond_one_percent += 1;
                    }
                }
                // No single value beyond 3 percent.
                assert!(pct.abs() < 3.0, "t {} s slot {slot}: {pct} %", row[0]);
            }
            if [
                1.0, 2.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 30.0, 40.0, 50.0, 60.0,
            ]
            .contains(&row[0])
            {
                eprintln!("{line}");
            }
        }
        let rms = (sum / count).sqrt();
        eprintln!(
            "five depths, sixty times: root mean square {rms:.3} %, worst {:+.2} % at {} s, slot {}; {beyond_one_percent} of 300 values beyond 1 %",
            worst.0, worst.1, worst.2
        );
        // Root mean square inside 1 percent of temperature.
        assert!(rms < 1.0, "{rms}");

        // At 60 s, every probe inside 1 percent.
        let end = slot_at(r, 60.0);
        let fiat = FIAT_TEMPERATURE[60];
        eprintln!("at 60 s: depth, FIAT, this solver, difference");
        for (slot, depth_mm) in [1, 2, 4, 8, 16, 50].iter().enumerate() {
            let ours = r.thermocouple_temperature_k[slot][end];
            let pct = percent(ours, fiat[2 + slot]);
            eprintln!(
                "  {depth_mm:2} mm  {:8.2} K  {:8.2} K  {:+6.2} K ({pct:+.2} %)",
                fiat[2 + slot],
                ours,
                ours - fiat[2 + slot]
            );
            assert!(pct.abs() < 1.0, "{depth_mm} mm at 60 s: {pct} %");
        }
        assert_eq!(r.surface_temperature_k[end], 1644.0);

        // Gas flux inside 2 percent, virgin front inside 1 percent, char front
        // inside 5 percent, at 60 s.
        let fronts = FIAT_GAS_AND_FRONTS[60];
        let gas = percent(r.gas_mass_flux_at_surface_kg_per_m2_s[end], fronts[1]);
        let char_front = percent(r.char_front_depth_m[end], fronts[2]);
        let virgin_front = percent(r.virgin_front_depth_m[end], fronts[3]);
        eprintln!(
            "at 60 s: gas flux {:.5} against {:.5} kg/m2/s ({gas:+.2} %); char front {:.3} against {:.3} mm ({char_front:+.2} %); virgin front {:.3} against {:.3} mm ({virgin_front:+.2} %)",
            r.gas_mass_flux_at_surface_kg_per_m2_s[end],
            fronts[1],
            r.char_front_depth_m[end] * 1.0e3,
            fronts[2] * 1.0e3,
            r.virgin_front_depth_m[end] * 1.0e3,
            fronts[3] * 1.0e3
        );
        assert!(gas.abs() < 2.0, "{gas}");
        assert!(virgin_front.abs() < 1.0, "{virgin_front}");
        assert!(char_front.abs() < 5.0, "{char_front}");
        for row in FIAT_GAS_AND_FRONTS.iter().skip(1) {
            let k = slot_at(r, row[0]);
            if [1.0, 2.0, 5.0, 10.0, 20.0, 30.0, 40.0, 50.0].contains(&row[0]) {
                eprintln!(
                    "{:4.0} s: gas flux {:.5} / {:.5} kg/m2/s; char front {:.3} / {:.3} mm; virgin front {:.3} / {:.3} mm",
                    row[0],
                    r.gas_mass_flux_at_surface_kg_per_m2_s[k],
                    row[1],
                    r.char_front_depth_m[k] * 1.0e3,
                    row[2] * 1.0e3,
                    r.virgin_front_depth_m[k] * 1.0e3,
                    row[3] * 1.0e3
                );
            }
        }
    }

    #[test]
    fn test_case_1_from_300_k_against_pato() {
        let r = test_case_1_from_300_k();
        let mut sum = 0.0;
        let mut count = 0.0;
        let mut worst = (0.0f64, 0.0, 0usize);
        for row in PATO_TEMPERATURE.iter().skip(1) {
            let k = slot_at(r, row[0]);
            for slot in 0..7 {
                let pct = percent(r.thermocouple_temperature_k[slot][k], row[1 + slot]);
                sum += pct * pct;
                count += 1.0;
                if pct.abs() > worst.0.abs() {
                    worst = (pct, row[0], slot);
                }
            }
        }
        let rms = (sum / count).sqrt();
        eprintln!(
            "against PATO's stored result, seven depths, sixty times: root mean square {rms:.3} %, worst {:+.2} % at {} s, slot {}",
            worst.0, worst.1, worst.2
        );
        // Every value inside 1 percent.
        assert!(worst.0.abs() < 1.0, "{worst:?}");
    }

    /// Where this solver is more than 1 percent from FIAT, PATO's stored
    /// result is on the same side of FIAT, and is nearer to this solver (run
    /// from PATO's 300 K) than it is to FIAT.
    #[test]
    fn where_this_differs_from_fiat_by_more_than_1_percent_pato_differs_the_same_way() {
        let r = test_case_1();
        let from_300 = test_case_1_from_300_k();
        let pato_column = [1usize, 2, 3, 4, 6];
        let slot_from_300 = [0usize, 1, 2, 3, 5];
        let mut found = 0;
        for (row, pato) in FIAT_TEMPERATURE.iter().zip(PATO_TEMPERATURE.iter()).skip(1) {
            let k = slot_at(r, row[0]);
            for slot in 0..5 {
                let ours = r.thermocouple_temperature_k[slot][k] - row[2 + slot];
                if 100.0 * ours.abs() / row[2 + slot] > 1.0 {
                    found += 1;
                    let theirs = pato[pato_column[slot]] - row[2 + slot];
                    let between = from_300.thermocouple_temperature_k[slot_from_300[slot]][k]
                        - pato[pato_column[slot]];
                    assert!(
                        theirs * ours > 0.0 && between.abs() < theirs.abs(),
                        "t {} s slot {slot}: this solver {ours:+.2} K and PATO {theirs:+.2} K from FIAT; this solver from 300 K {between:+.2} K from PATO",
                        row[0]
                    );
                }
            }
        }
        eprintln!("{found} values more than 1 % from FIAT; PATO is on the same side at every one");
        assert!(found > 0);
    }

    /// The FIAT file's char front advances in a sawtooth: its speed drops by
    /// more than 8 percent from one second to the next at depths about
    /// 0.42 mm apart. That is the print of nodes about 0.42 mm apart. This
    /// solver's front has no such drops.
    #[test]
    fn the_fiat_file_carries_the_print_of_a_grid_about_0_42_mm() {
        let drops = |depth_m: &dyn Fn(usize) -> f64| {
            let mut at = Vec::new();
            for t in 10..60 {
                let before = depth_m(t) - depth_m(t - 1);
                let after = depth_m(t + 1) - depth_m(t);
                if after < 0.92 * before {
                    at.push(depth_m(t));
                }
            }
            at
        };
        let fiat = drops(&|t| FIAT_GAS_AND_FRONTS[t][2]);
        let r = test_case_1();
        let ours = drops(&|t| r.char_front_depth_m[slot_at(r, t as f64)]);
        assert!(ours.is_empty(), "{ours:?}");
        assert!(fiat.len() >= 6, "{fiat:?}");
        let span = fiat[fiat.len() - 1] - fiat[0];
        // Two of the drops straddle one node, so count the distinct ones.
        let mut distinct = 1;
        for pair in fiat.windows(2) {
            if pair[1] - pair[0] > 2.0e-4 {
                distinct += 1;
            }
        }
        let spacing = span / (distinct - 1) as f64;
        eprintln!(
            "FIAT's char front slows at {fiat:?} m: {distinct} places, mean spacing {:.3} mm",
            spacing * 1.0e3
        );
        assert!((spacing - 0.42e-3).abs() < 0.03e-3, "{spacing}");
    }

    // -----------------------------------------------------------------------
    // Each term earns its place.
    // -----------------------------------------------------------------------

    #[test]
    fn without_the_heat_the_gas_carries_the_8_mm_temperature_is_far_too_hot() {
        let q = ablation_workshop_test_case_1();
        let no_gas = Terms {
            gas_carries_heat: false,
            ..Terms::ALL
        };
        let neither = Terms {
            gas_carries_heat: false,
            decomposition_heat: false,
            ..Terms::ALL
        };
        let fiat = FIAT_TEMPERATURE[60][5];
        let full = test_case_1();
        let end = slot_at(full, 60.0);
        let with_all = full.thermocouple_temperature_k[3][end] - fiat;
        let without_gas = run(&q, &NUMERICS, &no_gas)
            .expect("no gas term")
            .thermocouple_temperature_k[3][end]
            - fiat;
        let without_either = run(&q, &NUMERICS, &neither)
            .expect("neither term")
            .thermocouple_temperature_k[3][end]
            - fiat;
        eprintln!(
            "8 mm at 60 s, minus FIAT: every term {with_all:+.1} K; without the gas term {without_gas:+.1} K; without the gas term and the heat of decomposition {without_either:+.1} K"
        );
        assert!(with_all.abs() < 9.15, "{with_all}");
        assert!(without_gas > 100.0, "{without_gas}");
        assert!(without_either > 100.0, "{without_either}");
    }

    /// Temperature in a slab with constant properties, face ramped and held,
    /// back insulated: the series solution.
    fn ramp_and_hold_exact_k(
        depth_m: f64,
        time_s: f64,
        thickness_m: f64,
        diffusivity_m2_per_s: f64,
        start_k: f64,
        held_k: f64,
    ) -> f64 {
        let rate = (held_k - start_k) / SURFACE_RAMP_S;
        let ramped = time_s.min(SURFACE_RAMP_S);
        let mut t = start_k + rate * ramped;
        for n in 0..4000 {
            let lambda = (n as f64 + 0.5) * std::f64::consts::PI / thickness_m;
            let decay = diffusivity_m2_per_s * lambda * lambda;
            let integral = ((-decay * (time_s - ramped)).exp() - (-decay * time_s).exp()) / decay;
            t -= 2.0 / (lambda * thickness_m) * (lambda * depth_m).sin() * rate * integral;
        }
        t
    }

    #[test]
    fn without_decomposition_and_with_constant_properties_it_is_the_exact_conduction_solution() {
        let frozen = Terms {
            decomposition: false,
            properties_frozen_at_k: Some(298.0),
            ..Terms::ALL
        };
        let thickness = 0.02;
        let depths = [0.001, 0.002, 0.005, 0.01, 0.02];
        let q = question(thickness, 298.0, 1644.0, 60.0, 0.0, &depths);
        let diffusivity = VIRGIN_CONDUCTIVITY_W_PER_M_K[SOLID_ANCHOR_NODE]
            / (VIRGIN_DENSITY_KG_PER_M3 * VIRGIN_SPECIFIC_HEAT_J_PER_KG_K[SOLID_ANCHOR_NODE]);
        let worst_of = |r: &SlabResult| {
            let mut worst = (0.0f64, 0.0, 0.0);
            for (slot, &depth) in depths.iter().enumerate() {
                for (k, &t) in r.report_time_s.iter().enumerate().skip(1) {
                    let exact =
                        ramp_and_hold_exact_k(depth, t, thickness, diffusivity, 298.0, 1644.0);
                    let d = (r.thermocouple_temperature_k[slot][k] - exact).abs();
                    if d > worst.0 {
                        worst = (d, t, depth);
                    }
                }
            }
            worst
        };
        let r = run(&q, &NUMERICS, &frozen).expect("constant properties");
        let worst = worst_of(&r);
        let mut finer = NUMERICS;
        finer.first_step_s *= 0.25;
        finer.step_fraction *= 0.25;
        finer.largest_step_s *= 0.25;
        finer.first_cell_m *= 0.5;
        finer.largest_cell_m *= 0.5;
        finer.cell_growth = finer.cell_growth.sqrt();
        let worst_finer = worst_of(&run(&q, &finer, &frozen).expect("constant properties, finer"));
        eprintln!(
            "against the exact conduction solution: worst {:.3} K (at {} s, {} m); with a quarter of the step and half the cell {:.3} K",
            worst.0, worst.1, worst.2, worst_finer.0
        );
        // The back wall has warmed by hundreds of kelvin, so it is tested too.
        let back = *r.thermocouple_temperature_k[4].last().unwrap();
        assert!(back > 600.0, "{back}");
        assert!(worst.0 < 0.5, "{worst:?}");
        assert!(worst_finer.0 < 0.3 * worst.0, "{worst_finer:?}");
        // Nothing decomposed, no gas, and heat in equals heat stored.
        assert_eq!(r.solid_mass_lost_kg_per_m2, 0.0);
        assert_eq!(r.gas_mass_out_kg_per_m2, 0.0);
        assert_eq!(r.heat_given_off_by_decomposition_j_per_m2, 0.0);
        assert_eq!(r.heat_taken_by_gas_j_per_m2, 0.0);
        assert!(r
            .final_density_kg_per_m3
            .iter()
            .all(|&d| d == VIRGIN_DENSITY_KG_PER_M3));
        assert!(
            (r.heat_in_through_face_j_per_m2 - r.heat_stored_in_solid_j_per_m2).abs()
                < 1.0e-9 * r.heat_in_through_face_j_per_m2
        );
    }

    // -----------------------------------------------------------------------
    // Charring goes one way.
    // -----------------------------------------------------------------------

    #[test]
    fn a_slab_that_cools_keeps_its_char() {
        let depths = [0.0005, 0.002, 0.004];
        let heated = solve(&question(0.05, 298.0, 1644.0, 20.0, 0.0, &depths)).expect("heated");
        let cooled =
            solve(&question(0.05, 298.0, 1644.0, 20.0, 20.0, &depths)).expect("heated then cooled");
        assert_eq!(heated.final_depth_m, cooled.final_depth_m);
        let mut cooled_cells = 0;
        let mut lost_more = 0;
        for i in 0..heated.final_depth_m.len() {
            let before = heated.final_density_kg_per_m3[i];
            let after = cooled.final_density_kg_per_m3[i];
            assert!(after <= before, "cell {i}: {before} then {after}");
            assert!(after >= CHAR_DENSITY_KG_PER_M3);
            if cooled.final_temperature_k[i] < heated.final_temperature_k[i] - 100.0 {
                cooled_cells += 1;
            }
            if after < before {
                lost_more += 1;
            }
        }
        assert!(cooled_cells > 50, "{cooled_cells}");
        assert!(lost_more > 0);
        // The cell at the face is back near the start temperature and is still char.
        assert!(
            cooled.final_temperature_k[0] < 300.0,
            "{}",
            cooled.final_temperature_k[0]
        );
        assert!(heated.final_temperature_k[0] > 1600.0);
        assert!(cooled.final_density_kg_per_m3[0] < 221.0);
        assert!(cooled.final_surface_density_kg_per_m3 <= heated.final_surface_density_kg_per_m3);
        // The fronts never move back toward the face.
        for history in [&cooled.char_front_depth_m, &cooled.virgin_front_depth_m] {
            for pair in history.windows(2) {
                assert!(pair[1] >= pair[0], "{pair:?}");
            }
        }
        // The thermocouples did cool.
        let last = cooled.report_time_s.len() - 1;
        let at_20 = slot_at(&cooled, 20.0);
        for slot in 0..3 {
            assert!(
                cooled.thermocouple_temperature_k[slot][last]
                    < cooled.thermocouple_temperature_k[slot][at_20] - 300.0
            );
        }
        assert_eq!(cooled.surface_temperature_k[last], 298.0);
    }

    #[test]
    fn resin_that_grows_back_is_refused() {
        ensure_one_way(1.0, 0.001, [0.5, 0.2], [0.5, 0.2]).expect("unchanged");
        ensure_one_way(1.0, 0.001, [0.5, 0.2], [0.4, 0.0]).expect("used up");
        for after in [[0.5000001, 0.2], [0.5, 0.21], [-1.0e-9, 0.2]] {
            let err = ensure_one_way(1.0, 0.001, [0.5, 0.2], after).expect_err("backwards");
            assert_eq!(
                err,
                SlabReject::DecompositionRanBackwards {
                    time_s: 1.0,
                    depth_m: 0.001
                }
            );
        }
        let err =
            ensure_one_way(1.0, 0.001, [0.5, 0.2], [f64::NAN, 0.2]).expect_err("not a number");
        assert_eq!(err, SlabReject::NonFiniteValue { time_s: 1.0 });
    }

    // -----------------------------------------------------------------------
    // The accounts.
    // -----------------------------------------------------------------------

    #[test]
    fn the_energy_and_mass_accounts_close() {
        let r = test_case_1();
        eprintln!(
            "energy, J/m2: in through the face {:.1}; stored {:.1}; taken by the gas {:.1}; given off by decomposition {:.1}; open by {:.3e} (allowed {:.3e})",
            r.heat_in_through_face_j_per_m2,
            r.heat_stored_in_solid_j_per_m2,
            r.heat_taken_by_gas_j_per_m2,
            r.heat_given_off_by_decomposition_j_per_m2,
            r.energy_account_residual_j_per_m2,
            r.energy_account_tolerance_j_per_m2
        );
        eprintln!(
            "mass, kg/m2: solid lost {:.9}; gas out {:.9}; open by {:.3e} (allowed {:.3e})",
            r.solid_mass_lost_kg_per_m2,
            r.gas_mass_out_kg_per_m2,
            r.mass_account_residual_kg_per_m2,
            r.mass_account_tolerance_kg_per_m2
        );
        // The working form: stored = in + given off − taken by the gas.
        let working = r.heat_stored_in_solid_j_per_m2
            - r.heat_in_through_face_j_per_m2
            - r.heat_given_off_by_decomposition_j_per_m2
            + r.heat_taken_by_gas_j_per_m2;
        assert_eq!(working, r.energy_account_residual_j_per_m2);
        assert!(working.abs() <= r.energy_account_tolerance_j_per_m2);
        assert!(working.abs() < 1.0e-9 * r.heat_in_through_face_j_per_m2);
        // The conserved form, from the first and last states alone.
        let conserved = r.solid_enthalpy_change_j_per_m2 - r.heat_in_through_face_j_per_m2
            + r.gas_enthalpy_out_j_per_m2;
        assert!(
            conserved.abs() < 1.0e-9 * r.heat_in_through_face_j_per_m2,
            "{conserved}"
        );
        // Mass, recomputed here from the final density profile.
        let mut face = 0.0;
        let mut lost = 0.0;
        for (centre, density) in r.final_depth_m.iter().zip(&r.final_density_kg_per_m3) {
            let width = 2.0 * (centre - face);
            lost += (VIRGIN_DENSITY_KG_PER_M3 - density) * width;
            face += width;
        }
        assert!((face - 0.05).abs() < 1.0e-12);
        assert!(
            (lost - r.gas_mass_out_kg_per_m2).abs() < 1.0e-9 * lost,
            "{lost} {}",
            r.gas_mass_out_kg_per_m2
        );
        assert!(r.mass_account_residual_kg_per_m2.abs() <= r.mass_account_tolerance_kg_per_m2);
        // Sizes: most of the heat is stored, the gas takes a large share, and
        // decomposition gives a little back.
        assert!(r.heat_in_through_face_j_per_m2 > 7.0e6 && r.heat_in_through_face_j_per_m2 < 9.0e6);
        assert!(r.heat_taken_by_gas_j_per_m2 > 0.3 * r.heat_in_through_face_j_per_m2);
        assert!(r.heat_given_off_by_decomposition_j_per_m2 > 0.0);
    }

    #[test]
    fn heat_or_gas_that_no_account_knows_about_is_refused() {
        let q = question(0.02, 298.0, 1644.0, 2.0, 0.0, &[0.001]);
        run(&q, &NUMERICS, &Terms::ALL).expect("sound");
        // A ten-thousandth, and a millionth, of the heat that goes through the face.
        for leak in [100.0, 1.0] {
            let err = run(
                &q,
                &NUMERICS,
                &Terms {
                    unaccounted_heat_w_per_m2: leak,
                    ..Terms::ALL
                },
            )
            .expect_err("heat leak");
            assert!(
                matches!(err, SlabReject::EnergyAccountBroken { .. }),
                "{err:?}"
            );
        }
        // A hundred-thousandth, and a millionth, of the gas that leaves the face.
        for leak in [1.0e-6, 1.0e-7] {
            let err = run(
                &q,
                &NUMERICS,
                &Terms {
                    unaccounted_gas_kg_per_m2_s: leak,
                    ..Terms::ALL
                },
            )
            .expect_err("gas leak");
            assert!(
                matches!(err, SlabReject::MassAccountBroken { .. }),
                "{err:?}"
            );
        }
        // The checks themselves.
        let sound = StepAccount {
            heat_in_j: 10.0,
            heat_stored_j: 9.0,
            heat_taken_by_gas_j: 2.0,
            heat_given_off_j: 1.0,
            solid_enthalpy_change_j: 4.0,
            gas_enthalpy_out_j: 6.0,
            solid_mass_lost_kg: 1.0e-3,
            gas_mass_out_kg: 1.0e-3,
            energy_tolerance_j: 1.0e-8,
            mass_tolerance_kg: 1.0e-12,
            passes: 1,
        };
        ensure_energy_account(3.0, &sound).expect("closes");
        ensure_mass_account(3.0, &sound).expect("closes");
        let err = ensure_energy_account(
            3.0,
            &StepAccount {
                heat_stored_j: 9.0 + 2.0e-8,
                ..sound
            },
        )
        .expect_err("working form open");
        assert!(
            matches!(err, SlabReject::EnergyAccountBroken { time_s, .. } if time_s == 3.0),
            "{err:?}"
        );
        let err = ensure_energy_account(
            3.0,
            &StepAccount {
                gas_enthalpy_out_j: 6.0 + 2.0e-8,
                ..sound
            },
        )
        .expect_err("conserved form open");
        assert!(
            matches!(err, SlabReject::EnergyAccountBroken { .. }),
            "{err:?}"
        );
        let err = ensure_mass_account(
            3.0,
            &StepAccount {
                gas_mass_out_kg: 1.0e-3 + 2.0e-12,
                ..sound
            },
        )
        .expect_err("mass open");
        assert_eq!(
            err,
            SlabReject::MassAccountBroken {
                time_s: 3.0,
                solid_mass_lost_kg_per_m2: 1.0e-3,
                gas_mass_out_kg_per_m2: 1.0e-3 + 2.0e-12
            }
        );
    }

    // -----------------------------------------------------------------------
    // Refusals.
    // -----------------------------------------------------------------------

    #[test]
    fn a_question_outside_the_declared_range_is_refused_by_name() {
        let base = || question(0.05, 298.0, 1644.0, 60.0, 0.0, &[0.001]);
        for thickness_m in [
            0.0,
            THICKNESS_MIN_M * 0.999,
            THICKNESS_MAX_M * 1.001,
            -0.05,
            f64::NAN,
            f64::INFINITY,
        ] {
            let mut q = base();
            q.thickness_m = thickness_m;
            q.thermocouple_depths_m.clear();
            let err = solve(&q).expect_err("thickness");
            assert!(
                matches!(err, SlabReject::ThicknessOutOfRange { .. }),
                "{err:?}"
            );
        }
        for start in [
            START_TEMPERATURE_MIN_K - 0.01,
            START_TEMPERATURE_MAX_K + 0.01,
            0.0,
            f64::NAN,
        ] {
            let mut q = base();
            q.start_temperature_k = start;
            let err = solve(&q).expect_err("start temperature");
            assert!(
                matches!(err, SlabReject::StartTemperatureOutOfRange { .. }),
                "{err:?}"
            );
        }
        for held in [
            HELD_TEMPERATURE_MIN_K - 0.01,
            HELD_TEMPERATURE_MAX_K + 0.01,
            298.0,
            f64::NAN,
        ] {
            let err =
                solve(&question(0.05, 298.0, held, 60.0, 0.0, &[])).expect_err("held temperature");
            assert!(
                matches!(err, SlabReject::HeldTemperatureOutOfRange { .. }),
                "{err:?}"
            );
        }
        for hold in [
            HOLD_DURATION_MIN_S * 0.999,
            HOLD_DURATION_MAX_S + 0.01,
            0.0,
            -1.0,
            f64::NAN,
        ] {
            let err = solve(&question(0.05, 298.0, 1644.0, hold, 0.0, &[])).expect_err("hold");
            assert!(
                matches!(err, SlabReject::HoldDurationOutOfRange { .. }),
                "{err:?}"
            );
        }
        for after in [-1.0e-9, CONTINUE_AFTER_MAX_S + 0.01, f64::NAN] {
            let err = solve(&question(0.05, 298.0, 1644.0, 60.0, after, &[])).expect_err("after");
            assert!(
                matches!(err, SlabReject::ContinueAfterOutOfRange { .. }),
                "{err:?}"
            );
        }
        let err = solve(&question(0.05, 298.0, 1644.0, 60.0, 0.0, &[0.001; 9]))
            .expect_err("nine thermocouples");
        assert_eq!(err, SlabReject::TooManyThermocouples { count: 9 });
        for depth in [-1.0e-6, 0.0500001, f64::NAN] {
            let err = solve(&question(0.05, 298.0, 1644.0, 60.0, 0.0, &[0.001, depth]))
                .expect_err("thermocouple");
            assert!(
                matches!(err, SlabReject::ThermocoupleOutsideSlab { index: 1, .. }),
                "{err:?}"
            );
        }
        // A refusal names itself in words.
        let said = SlabReject::ThicknessOutOfRange { thickness_m: 0.5 }.to_string();
        assert!(said.contains("thickness") && said.contains("0.5"), "{said}");
    }

    #[test]
    fn the_edges_of_the_declared_range_are_answered() {
        // No thermocouples, eight thermocouples, the face and the back wall.
        let r = solve(&question(
            THICKNESS_MIN_M,
            START_TEMPERATURE_MIN_K,
            HELD_TEMPERATURE_MAX_K,
            HOLD_DURATION_MIN_S,
            0.0,
            &[],
        ))
        .expect("corner");
        assert!(r.thermocouple_temperature_k.is_empty());
        let depths = [
            0.0,
            0.001,
            0.002,
            0.003,
            0.004,
            0.005,
            0.006,
            THICKNESS_MIN_M,
        ];
        let r = solve(&question(
            THICKNESS_MIN_M,
            START_TEMPERATURE_MAX_K,
            HELD_TEMPERATURE_MIN_K,
            HOLD_DURATION_MIN_S,
            1.5,
            &depths,
        ))
        .expect("held at the start temperature");
        // Held at the temperature it started at: a trace of resin decomposes
        // and warms the slab by about a thousandth of a kelvin.
        assert_eq!(r.report_time_s, vec![0.0, 1.0, 2.0, 2.5]);
        for history in &r.thermocouple_temperature_k {
            for &t in history {
                assert!((400.0..400.01).contains(&t), "{t}");
            }
        }
        assert!(r.solid_mass_lost_kg_per_m2 >= 0.0 && r.solid_mass_lost_kg_per_m2 < 1.0e-5);
    }

    #[test]
    fn too_few_newton_passes_are_refused() {
        let q = question(0.02, 298.0, 1644.0, 1.0, 0.0, &[]);
        let mut short = NUMERICS;
        short.newton_pass_limit = 1;
        let err = run(&q, &short, &Terms::ALL).expect_err("one pass");
        assert!(
            matches!(err, SlabReject::StepNotConverged { passes: 1, .. }),
            "{err:?}"
        );
        // Test Case 1 needs four in its hardest step; the limit is three times that.
        let hardest = test_case_1().hardest_step_newton_passes;
        assert!(hardest <= 4, "{hardest}");
        assert!(NEWTON_PASS_LIMIT >= 3 * hardest);
    }

    #[test]
    fn a_value_that_is_not_a_number_or_leaves_its_band_is_refused() {
        let q = question(0.02, 298.0, 1644.0, 1.0, 0.0, &[]);
        let err = run(
            &q,
            &NUMERICS,
            &Terms {
                unaccounted_heat_w_per_m2: f64::NAN,
                ..Terms::ALL
            },
        )
        .expect_err("not a number");
        assert!(matches!(err, SlabReject::NonFiniteValue { .. }), "{err:?}");
        // Heat drawn out of the back cell takes it below the start temperature.
        let err = run(
            &q,
            &NUMERICS,
            &Terms {
                unaccounted_heat_w_per_m2: -1.0e5,
                ..Terms::ALL
            },
        )
        .expect_err("too cold");
        assert!(
            matches!(err, SlabReject::TemperatureOutOfBounds { temperature_k, .. } if temperature_k < 298.0),
            "{err:?}"
        );
        ensure_temperature_in_band(1.0, 0.0, 298.0 - 0.5e-6, 298.0, 1644.0)
            .expect("inside the tolerance");
        ensure_temperature_in_band(1.0, 0.0, 298.0 - 2.0e-6, 298.0, 1644.0).expect_err("below");
        ensure_temperature_in_band(1.0, 0.0, 1644.0 + 2.0e-6, 298.0, 1644.0).expect_err("above");
    }

    // -----------------------------------------------------------------------
    // Same input, same bits; every number finite and in its band.
    // -----------------------------------------------------------------------

    #[test]
    fn the_same_question_twice_is_bit_identical() {
        let q = question(0.03, 310.0, 2000.0, 5.0, 3.0, &[0.0, 0.0015, 0.007, 0.03]);
        let first = solve(&q).expect("first");
        let second = solve(&q).expect("second");
        let a: Vec<u64> = every_number(&first).iter().map(|v| v.to_bits()).collect();
        let b: Vec<u64> = every_number(&second).iter().map(|v| v.to_bits()).collect();
        assert_eq!(a, b);
        assert!(a.len() > 500);
        // And it is the stored Test Case 1 again when asked again.
        let again = solve(&ablation_workshop_test_case_1()).expect("again");
        let a: Vec<u64> = every_number(test_case_1())
            .iter()
            .map(|v| v.to_bits())
            .collect();
        let b: Vec<u64> = every_number(&again).iter().map(|v| v.to_bits()).collect();
        assert_eq!(a, b);
    }

    #[test]
    fn every_number_is_finite_and_inside_its_band() {
        let cooled = solve(&question(
            0.02,
            298.0,
            1644.0,
            10.0,
            10.0,
            &[0.0, 0.001, 0.02],
        ))
        .expect("cooled");
        for (r, start, top) in [(test_case_1(), 298.0, 1644.0), (&cooled, 298.0, 1644.0)] {
            assert!(every_number(r).iter().all(|v| v.is_finite()));
            let low = start - TEMPERATURE_BOUND_TOLERANCE_K;
            let high = top + TEMPERATURE_BOUND_TOLERANCE_K;
            for history in &r.thermocouple_temperature_k {
                assert!(history.iter().all(|&t| t >= low && t <= high));
            }
            assert!(r.final_temperature_k.iter().all(|&t| t >= low && t <= high));
            assert!(r
                .surface_temperature_k
                .iter()
                .all(|&t| t >= start && t <= top));
            assert!(r
                .final_density_kg_per_m3
                .iter()
                .all(|&d| (CHAR_DENSITY_KG_PER_M3..=VIRGIN_DENSITY_KG_PER_M3).contains(&d)));
            assert!(r
                .gas_mass_flux_at_surface_kg_per_m2_s
                .iter()
                .all(|&m| m >= 0.0));
            assert!(r
                .char_front_depth_m
                .iter()
                .zip(&r.virgin_front_depth_m)
                .all(|(c, v)| c <= v));
            assert_eq!(r.report_time_s.len(), r.surface_temperature_k.len());
            assert_eq!(r.report_time_s.len(), r.char_front_depth_m.len());
        }
        assert_eq!(test_case_1().report_time_s.len(), 61);
    }

    /// TACOT's decomposition gives off heat below about 939 K. A thin slab
    /// held below that soaks through and then warms itself: the back runs
    /// hotter than the face. That is what the equations and TACOT's numbers
    /// say, so the band's top is 939 K there and the answer is given.
    #[test]
    fn a_thin_slab_held_below_939_k_runs_warmer_inside_than_at_its_face() {
        let r = solve(&question(0.01, 298.0, 600.0, 200.0, 0.0, &[0.005, 0.01]))
            .expect("held at 600 K");
        let hottest = r.thermocouple_temperature_k[1]
            .iter()
            .cloned()
            .fold(0.0f64, f64::max);
        eprintln!("a 10 mm slab held at 600 K: the back wall reaches {hottest:.1} K");
        assert!(hottest > 620.0 && hottest < 939.4, "{hottest}");
        assert!(r.heat_given_off_by_decomposition_j_per_m2 > 0.0);
        // Held above 939 K, nothing inside is hotter than the face.
        let r = solve(&question(0.01, 298.0, 1000.0, 100.0, 0.0, &[0.005, 0.01]))
            .expect("held at 1000 K");
        let hottest = r.final_temperature_k.iter().cloned().fold(0.0f64, f64::max);
        assert!(
            hottest <= 1000.0 + TEMPERATURE_BOUND_TOLERANCE_K,
            "{hottest}"
        );
    }

    // -----------------------------------------------------------------------
    // The grid and the time step.
    // -----------------------------------------------------------------------

    #[test]
    fn halving_the_cells_or_the_time_step_moves_no_temperature_by_half_a_kelvin() {
        let q = ablation_workshop_test_case_1();
        let base = test_case_1();
        let mut cells = NUMERICS;
        cells.first_cell_m *= 0.5;
        cells.largest_cell_m *= 0.5;
        cells.cell_growth = cells.cell_growth.sqrt();
        let mut steps = NUMERICS;
        steps.first_step_s *= 0.5;
        steps.step_fraction *= 0.5;
        steps.largest_step_s *= 0.5;
        let finer_cells = run(&q, &cells, &Terms::ALL).expect("half the cell");
        let finer_steps = run(&q, &steps, &Terms::ALL).expect("half the step");
        assert_eq!(finer_cells.cells, 2 * base.cells - 1);
        assert!(finer_steps.time_steps > 19 * base.time_steps / 10);
        let end = slot_at(base, 60.0);
        for (label, finer) in [
            ("cells halved", &finer_cells),
            ("time step halved", &finer_steps),
        ] {
            let (worst, at_s, at_m) = largest_movement_k(base, finer);
            let at_60: Vec<String> = (0..6)
                .map(|slot| {
                    format!(
                        "{:+.3}",
                        finer.thermocouple_temperature_k[slot][end]
                            - base.thermocouple_temperature_k[slot][end]
                    )
                })
                .collect();
            eprintln!(
                "{label}: {} cells, {} steps; largest movement {worst:.3} K (at {at_s} s, {at_m} m); at 60 s {} K; gas flux {:+.3} %, char front {:+.4} mm, virgin front {:+.4} mm",
                finer.cells,
                finer.time_steps,
                at_60.join(" "),
                percent(
                    finer.gas_mass_flux_at_surface_kg_per_m2_s[end],
                    base.gas_mass_flux_at_surface_kg_per_m2_s[end]
                ),
                (finer.char_front_depth_m[end] - base.char_front_depth_m[end]) * 1.0e3,
                (finer.virgin_front_depth_m[end] - base.virgin_front_depth_m[end]) * 1.0e3
            );
            // One percent of the coldest FIAT value that has moved is 3 K;
            // of the hottest, 15 K.
            assert!(worst < 0.5, "{label}: {worst} K");
            for slot in 0..6 {
                let moved = (finer.thermocouple_temperature_k[slot][end]
                    - base.thermocouple_temperature_k[slot][end])
                    .abs();
                assert!(moved < 0.1, "{label} slot {slot}: {moved} K at 60 s");
            }
        }
    }

    // -----------------------------------------------------------------------
    // The declared range.
    // -----------------------------------------------------------------------

    /// Evidence about the reference, not a pass line for the solver. FIAT is
    /// listed as first order in space, and its file shows nodes about 0.42 mm
    /// apart. Run this solver that way (uniform 0.42 mm cells, the gas crossing
    /// each face given the enthalpy of the cell on the hot side, a first-order
    /// form) and it lands on FIAT: the differences at 8 and 16 mm and 60 s go
    /// from +5.45 and +4.24 K to under 1 K, and the root mean square falls
    /// from 0.63 to 0.27 percent. With cells of 0.21 mm the shift is about
    /// half. So FIAT's distance from this solver is the size and sign of a
    /// first-order error on FIAT's own grid.
    #[test]
    fn run_first_order_on_a_0_42_mm_grid_this_solver_lands_on_fiat() {
        let q = ablation_workshop_test_case_1();
        let hot_side = Terms {
            gas_face_share_of_cell_above: Some(1.0),
            ..Terms::ALL
        };
        let uniform = |cell_m: f64| {
            let mut numerics = NUMERICS;
            numerics.first_cell_m = cell_m;
            numerics.largest_cell_m = cell_m;
            numerics.cell_growth = 1.0;
            numerics
        };
        let against_fiat = |r: &SlabResult| {
            let mut sum = 0.0;
            let mut count = 0.0;
            for row in FIAT_TEMPERATURE.iter().skip(1) {
                let k = slot_at(r, row[0]);
                for slot in 0..5 {
                    let pct = percent(r.thermocouple_temperature_k[slot][k], row[2 + slot]);
                    sum += pct * pct;
                    count += 1.0;
                }
            }
            let end = slot_at(r, 60.0);
            (
                (sum / count).sqrt(),
                r.thermocouple_temperature_k[3][end] - FIAT_TEMPERATURE[60][5],
                r.thermocouple_temperature_k[4][end] - FIAT_TEMPERATURE[60][6],
                percent(r.char_front_depth_m[end], FIAT_GAS_AND_FRONTS[60][2]),
            )
        };
        let solver = against_fiat(test_case_1());
        let as_fiat =
            against_fiat(&run(&q, &uniform(0.42e-3), &hot_side).expect("0.42 mm, first order"));
        let half_cell =
            against_fiat(&run(&q, &uniform(0.21e-3), &hot_side).expect("0.21 mm, first order"));
        let central =
            against_fiat(&run(&q, &uniform(0.42e-3), &Terms::ALL).expect("0.42 mm, central"));
        for (label, v) in [
            ("this solver", solver),
            ("0.42 mm cells, gas enthalpy from the hot side", as_fiat),
            ("0.21 mm cells, gas enthalpy from the hot side", half_cell),
            ("0.42 mm cells, gas enthalpy at the face", central),
        ] {
            eprintln!(
                "{label}: root mean square {:.3} % from FIAT; at 60 s {:+.2} K at 8 mm, {:+.2} K at 16 mm, char front {:+.2} %",
                v.0, v.1, v.2, v.3
            );
        }
        assert!(solver.1 > 5.0 && solver.2 > 4.0);
        assert!(
            as_fiat.1.abs() < 1.0 && as_fiat.2.abs() < 1.0,
            "{as_fiat:?}"
        );
        assert!(as_fiat.0 < 0.5 * solver.0, "{as_fiat:?}");
        assert!(as_fiat.3.abs() < 1.0 && solver.3 > 2.0);
        // First order: half the cell, about half the shift.
        let shift = solver.1 - as_fiat.1;
        let half_shift = solver.1 - half_cell.1;
        assert!(
            half_shift > 0.4 * shift && half_shift < 0.65 * shift,
            "{shift} {half_shift}"
        );
        // The coarse cells alone do not do it.
        assert!(central.1 > 4.5, "{central:?}");
    }

    /// 888 questions across the declared range. About 95 seconds with
    /// `--release`, about 32 minutes without.
    #[test]
    #[ignore]
    fn a_sweep_of_the_declared_range_refuses_nothing() {
        let mut questions = Vec::new();
        for &thickness in &[THICKNESS_MIN_M, 0.02, 0.05, THICKNESS_MAX_M] {
            for &start in &[START_TEMPERATURE_MIN_K, 298.0, START_TEMPERATURE_MAX_K] {
                for &held in &[
                    HELD_TEMPERATURE_MIN_K,
                    600.0,
                    900.0,
                    1200.0,
                    1644.0,
                    2200.0,
                    2800.0,
                    HELD_TEMPERATURE_MAX_K,
                ] {
                    for &(hold, after) in &[
                        (HOLD_DURATION_MIN_S, 0.0),
                        (1.0, 1.0),
                        (10.0, 10.0),
                        (60.0, 0.0),
                        (60.0, 60.0),
                        (2.5, 0.05),
                        (7.3, 3.7),
                    ] {
                        questions.push(question(
                            thickness,
                            start,
                            held,
                            hold,
                            after,
                            &[0.0, 0.001, thickness],
                        ));
                    }
                }
            }
        }
        // The longest runs, at the corners.
        for &thickness in &[THICKNESS_MIN_M, THICKNESS_MAX_M] {
            for &start in &[START_TEMPERATURE_MIN_K, START_TEMPERATURE_MAX_K] {
                for &held in &[
                    HELD_TEMPERATURE_MIN_K,
                    900.0,
                    1644.0,
                    HELD_TEMPERATURE_MAX_K,
                ] {
                    questions.push(question(
                        thickness,
                        start,
                        held,
                        HOLD_DURATION_MAX_S,
                        CONTINUE_AFTER_MAX_S,
                        &[0.0, 0.001, thickness],
                    ));
                }
            }
        }
        // Two hundred more, spread through the inside of the range by a
        // fixed low-discrepancy sequence (no random numbers).
        let spread = |index: u32, base: u32| {
            let mut value = 0.0;
            let mut scale = 1.0 / base as f64;
            let mut left = index;
            while left > 0 {
                value += (left % base) as f64 * scale;
                left /= base;
                scale /= base as f64;
            }
            value
        };
        for index in 1..=200u32 {
            let thickness =
                THICKNESS_MIN_M + (THICKNESS_MAX_M - THICKNESS_MIN_M) * spread(index, 2);
            let start = START_TEMPERATURE_MIN_K
                + (START_TEMPERATURE_MAX_K - START_TEMPERATURE_MIN_K) * spread(index, 3);
            let held = HELD_TEMPERATURE_MIN_K
                + (HELD_TEMPERATURE_MAX_K - HELD_TEMPERATURE_MIN_K) * spread(index, 5);
            let hold = HOLD_DURATION_MIN_S * (60.0f64 / HOLD_DURATION_MIN_S).powf(spread(index, 7));
            let after = 60.0 * spread(index, 11) * spread(index, 11);
            let depth = thickness * spread(index, 13);
            questions.push(question(
                thickness,
                start,
                held,
                hold,
                after,
                &[depth, 0.5 * depth, thickness],
            ));
        }
        let mut refused = Vec::new();
        let mut passes = [0u32; NEWTON_PASS_LIMIT as usize + 1];
        let mut most_steps = 0;
        let mut energy_open = 0.0f64;
        let mut mass_open = 0.0f64;
        for q in &questions {
            match solve(q) {
                Ok(r) => {
                    passes[r.hardest_step_newton_passes as usize] += 1;
                    most_steps = most_steps.max(r.time_steps);
                    assert!(every_number(&r).iter().all(|v| v.is_finite()));
                    assert!(r
                        .gas_mass_flux_at_surface_kg_per_m2_s
                        .iter()
                        .all(|&m| m >= -1.0e-12));
                    assert!(r.final_density_kg_per_m3.iter().all(|&d| {
                        (CHAR_DENSITY_KG_PER_M3..=VIRGIN_DENSITY_KG_PER_M3).contains(&d)
                    }));
                    assert!(r
                        .char_front_depth_m
                        .iter()
                        .zip(&r.virgin_front_depth_m)
                        .all(|(c, v)| c <= v));
                    for history in [&r.char_front_depth_m, &r.virgin_front_depth_m] {
                        assert!(history.windows(2).all(|pair| pair[1] >= pair[0]), "{q:?}");
                    }
                    let low = q.start_temperature_k - TEMPERATURE_BOUND_TOLERANCE_K;
                    assert!(r.final_temperature_k.iter().all(|&t| t >= low), "{q:?}");
                    for history in &r.thermocouple_temperature_k {
                        assert!(history.iter().all(|&t| t >= low), "{q:?}");
                    }
                    energy_open = energy_open.max(
                        r.energy_account_residual_j_per_m2.abs()
                            / r.energy_account_tolerance_j_per_m2,
                    );
                    mass_open = mass_open.max(
                        r.mass_account_residual_kg_per_m2.abs()
                            / r.mass_account_tolerance_kg_per_m2,
                    );
                }
                Err(err) => refused.push(format!("{q:?}: {err}")),
            }
        }
        eprintln!(
            "{} questions, {} refused; runs by Newton passes in their hardest step {:?}; most time steps {most_steps}; accounts open by at most {energy_open:.1e} (energy) and {mass_open:.1e} (mass) of what is allowed",
            questions.len(),
            refused.len(),
            passes
        );
        assert!(refused.is_empty(), "{refused:#?}");
        let hardest = passes.iter().rposition(|&count| count > 0).unwrap() as u32;
        assert!(3 * hardest <= NEWTON_PASS_LIMIT, "{hardest}");
    }
}

/// Stage two: the face in a gas stream. The stage-one tests above are as
/// they were.
#[cfg(test)]
mod stream_tests {
    use super::*;
    use crate::workshop_series_2::*;
    use std::sync::OnceLock;

    fn slot_at(result: &SlabResult, time_s: f64) -> usize {
        result
            .report_time_s
            .iter()
            .position(|&t| t == time_s)
            .unwrap_or_else(|| panic!("no report at {time_s} s"))
    }

    /// TACOT with the older 1-atm surface table that ships in PATO 3.1.
    fn tacot_with_the_older_surface_table() -> Material {
        static PRESSURE: [f64; 1] = [101325.0];
        Material {
            surface: SurfaceTable {
                pressure_pa: &PRESSURE,
                blowing: &PATO_1ATM_BLOWING,
                temperature_k: &PATO_1ATM_TEMPERATURE_K,
                char_rate: &PATO_1ATM_CHAR_RATE,
                wall_enthalpy_j_per_kg: &PATO_1ATM_WALL_ENTHALPY_J_PER_KG,
            },
            ..TACOT
        }
    }

    fn case_2_1() -> &'static SlabResult {
        static RESULT: OnceLock<SlabResult> = OnceLock::new();
        RESULT.get_or_init(|| solve(&ablation_workshop_test_case_2_1()).expect("case 2.1"))
    }
    fn case_2_2() -> &'static SlabResult {
        static RESULT: OnceLock<SlabResult> = OnceLock::new();
        RESULT.get_or_init(|| solve(&ablation_workshop_test_case_2_2()).expect("case 2.2"))
    }
    fn case_2_3() -> &'static SlabResult {
        static RESULT: OnceLock<SlabResult> = OnceLock::new();
        RESULT.get_or_init(|| solve(&ablation_workshop_test_case_2_3()).expect("case 2.3"))
    }
    fn case_2_3_with_the_older_table() -> &'static SlabResult {
        static RESULT: OnceLock<SlabResult> = OnceLock::new();
        RESULT.get_or_init(|| {
            run_material(
                &tacot_with_the_older_surface_table(),
                &ablation_workshop_test_case_2_3(),
                &NUMERICS,
                &Terms::ALL,
            )
            .expect("case 2.3 with the older table")
        })
    }

    /// A stream switched on over 0.1 s and held to `end_s`.
    fn stream(
        thickness_m: f64,
        coefficient: f64,
        flow_enthalpy_j_per_kg: f64,
        pressure_pa: f64,
        radiative_heating_w_per_m2: f64,
        end_s: f64,
        face_wears: bool,
    ) -> SlabQuestion {
        let row = |time_s: f64, on: f64| StreamRow {
            time_s,
            pressure_pa,
            heat_transfer_coefficient_kg_per_m2_s: coefficient * on,
            flow_enthalpy_j_per_kg: flow_enthalpy_j_per_kg * on,
            radiative_heating_w_per_m2: radiative_heating_w_per_m2 * on,
        };
        SlabQuestion {
            thickness_m,
            start_temperature_k: 300.0,
            surface: SurfaceCondition::GasStream {
                history: vec![row(0.0, 0.0), row(0.1, 1.0), row(end_s, 1.0)],
                surroundings_temperature_k: 300.0,
                face_wears,
                stream_gas: StreamGas::AIR,
            },
            continue_after_s: 0.0,
            thermocouple_depths_m: vec![0.1 * thickness_m, 0.4 * thickness_m, thickness_m],
        }
    }

    /// How a run sits against a reference: printed, and returned as
    /// (largest wall difference 10 to 60 s in K, largest wall difference
    /// 62 to 120 s in K, largest percent at the 16 and 24 mm probes over the
    /// whole run, percent on recession at 60 s).
    fn against(
        label: &str,
        r: &SlabResult,
        temperature: &[[f64; 10]; 121],
        mass: &[[f64; 6]; 121],
    ) -> (f64, f64, f64, f64) {
        eprintln!("{label}: this solver minus the reference");
        eprintln!(
            "    t s    wall K    1 mm    2 mm    4 mm    8 mm   12 mm   16 mm   24 mm   50 mm | gas flux %  char flux %  recession mm"
        );
        let mut heated = 0.0f64;
        let mut heated_sum = 0.0;
        let mut cooling = 0.0f64;
        let mut deep = 0.0f64;
        let mut covered = [0.0f64; 8];
        for row in temperature.iter().skip(1) {
            let t = row[0];
            let k = slot_at(r, t);
            let wall = r.surface_temperature_k[k] - row[1];
            if (10.0..=60.0).contains(&t) {
                heated = heated.max(wall.abs());
                heated_sum += wall.abs() / 51.0;
            }
            // The first second after the stream is cut is reported and not
            // judged: the wall is falling by hundreds of kelvin a second.
            if t >= 62.0 {
                cooling = cooling.max(wall.abs());
            }
            let m = mass[t as usize];
            let mut line = format!("  {t:5.0}  {wall:+8.2}");
            for slot in 0..8 {
                let ours = r.thermocouple_temperature_k[slot][k];
                let theirs = row[2 + slot];
                // A probe is compared while half a millimetre still covers it in both.
                let depth = r.thermocouple_depth_m[slot];
                if ours == 0.0
                    || theirs == 0.0
                    || depth - r.recession_m[k] < 0.5e-3
                    || depth - m[5] < 0.5e-3
                {
                    line += "    gone";
                    continue;
                }
                line += &format!(" {:+7.2}", ours - theirs);
                let pct = 100.0 * (ours - theirs).abs() / theirs;
                covered[slot] = covered[slot].max(pct);
                if slot == 5 || slot == 6 {
                    deep = deep.max(pct);
                }
            }
            let pct = |ours: f64, theirs: f64| {
                if theirs != 0.0 {
                    format!("{:+10.2}", 100.0 * (ours / theirs - 1.0))
                } else {
                    "         -".to_string()
                }
            };
            line += &format!(
                " | {} {}   {:.4} / {:.4}",
                pct(r.gas_mass_flux_at_surface_kg_per_m2_s[k], m[1]),
                pct(r.char_mass_flux_kg_per_m2_s[k], m[2]),
                r.recession_m[k] * 1.0e3,
                m[5] * 1.0e3
            );
            if [
                1.0, 2.0, 5.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 61.0, 65.0, 80.0, 100.0, 120.0,
            ]
            .contains(&t)
            {
                eprintln!("{line}");
            }
        }
        let at_60 = slot_at(r, 60.0);
        let recession = if mass[60][5] > 0.0 {
            100.0 * (r.recession_m[at_60] / mass[60][5] - 1.0)
        } else {
            0.0
        };
        eprintln!(
            "  wall, 10 to 60 s: mean {heated_sum:.2} K, largest {heated:.2} K; wall, 62 to 120 s: largest {cooling:.2} K; probes while covered, largest percent at 1, 2, 4, 8, 12, 16, 24, 50 mm: {}; recession at 60 s {recession:+.3} %",
            covered
                .iter()
                .map(|p| format!("{p:.2}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
        eprintln!(
            "  at 60 s: wall {:.1} / {:.1} K; gas flux {:.5} / {:.5}; char flux {:.5} / {:.5} kg/m2/s; virgin front {:.3} / {:.3} mm; char front {:.3} / {:.3} mm; at 120 s: recession {:.4} / {:.4} mm",
            r.surface_temperature_k[at_60],
            temperature[60][1],
            r.gas_mass_flux_at_surface_kg_per_m2_s[at_60],
            mass[60][1],
            r.char_mass_flux_kg_per_m2_s[at_60],
            mass[60][2],
            r.virgin_front_depth_m[at_60] * 1.0e3,
            mass[60][3] * 1.0e3,
            r.char_front_depth_m[at_60] * 1.0e3,
            mass[60][4] * 1.0e3,
            r.recession_m[r.recession_m.len() - 1] * 1.0e3,
            mass[120][5] * 1.0e3
        );
        (heated, cooling, deep, recession)
    }

    // -----------------------------------------------------------------------
    // The surface table as locked in.
    // -----------------------------------------------------------------------

    #[test]
    fn the_surface_table_is_read_back_at_its_own_nodes() {
        let table = &TACOT.surface;
        let lookup = SurfaceLookup::of(table);
        assert_eq!(
            table.pressure_pa,
            &[
                STREAM_PRESSURE_MIN_PA,
                1013.2499999999999,
                10132.5,
                STREAM_PRESSURE_MAX_PA
            ]
        );
        assert_eq!((table.blowing.len(), table.temperature_k.len()), (25, 151));
        assert_eq!(table.char_rate.len(), 15_100);
        assert_eq!(table.wall_enthalpy_j_per_kg.len(), 15_100);
        let mut checked = 0;
        for (ip, &p) in table.pressure_pa.iter().enumerate() {
            for (ib, &b) in table.blowing.iter().enumerate().take(24) {
                for (it, &t) in table.temperature_k.iter().enumerate() {
                    let at = (ip * 25 + ib) * 151 + it;
                    for straight in [false, true] {
                        let gas = lookup.read(p, b, t, straight);
                        let want = table.char_rate[at];
                        assert!(
                            (gas.char_rate - want).abs() <= 1.0e-9 * want.max(1.0),
                            "{p} {b} {t}: {} {want}",
                            gas.char_rate
                        );
                        assert!((gas.enthalpy - table.wall_enthalpy_j_per_kg[at]).abs() < 1.0e-6);
                    }
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 4 * 24 * 151);
        // With no decomposition gas the table is carbon in air: all the
        // oxygen leaves as CO2 when cold and as CO when hot.
        let cold = lookup.read(101325.0, 0.0, 300.0, false).char_rate;
        let hot = lookup.read(101325.0, 0.0, 1500.0, false).char_rate;
        assert!(
            (cold - 0.08743).abs() < 1.0e-5 && (hot - 0.1748).abs() < 1.0e-4,
            "{cold} {hot}"
        );
        // Where the sheet stops giving values of its own (carbon has boiled),
        // with no decomposition gas: 3075, 3325, 3625 and 3950 K.
        for (ip, want) in [3075.0, 3325.0, 3625.0, 3950.0].iter().enumerate() {
            let last = table.temperature_k[lookup.last_own_node[ip * 25]];
            assert_eq!(last, *want, "pressure {ip}");
            assert!(
                !lookup
                    .read(table.pressure_pa[ip], 0.0, *want, false)
                    .beyond_table
            );
            assert!(
                lookup
                    .read(table.pressure_pa[ip], 0.0, *want + 1.0, false)
                    .beyond_table
            );
        }
        // B'g above the table's largest is read at the largest.
        let top = lookup.read(101325.0, 10.0, 2000.0, false);
        let above = lookup.read(101325.0, 500.0, 2000.0, false);
        assert_eq!(top.enthalpy, above.enthalpy);
        assert_eq!(above.enthalpy_blowing, 0.0);
    }

    #[test]
    fn the_slopes_of_the_wall_gas_and_the_blowing_match_differences() {
        let lookup = SurfaceLookup::of(&TACOT.surface);
        for &(p, b, t) in &[
            (101325.0, 0.045, 1570.0),
            (101325.0, 0.058, 3160.0),
            (23811.0, 0.31, 2960.0),
            (300.0, 1.3, 2410.0),
            (5000.0, 0.012, 640.0),
        ] {
            for straight in [false, true] {
                let here = lookup.read(p, b, t, straight);
                let (db, dt) = (1.0e-7, 1.0e-4);
                let by_b = |f: &dyn Fn(&WallGas) -> f64| {
                    (f(&lookup.read(p, b + db, t, straight))
                        - f(&lookup.read(p, b - db, t, straight)))
                        / (2.0 * db)
                };
                let by_t = |f: &dyn Fn(&WallGas) -> f64| {
                    (f(&lookup.read(p, b, t + dt, straight))
                        - f(&lookup.read(p, b, t - dt, straight)))
                        / (2.0 * dt)
                };
                assert!(
                    (by_b(&|g| g.char_rate) - here.char_rate_blowing).abs()
                        < 1.0e-5 * (1.0 + here.char_rate_blowing.abs()),
                    "{p} {b} {t}"
                );
                assert!(
                    (by_t(&|g| g.char_rate) - here.char_rate_temperature).abs()
                        < 1.0e-5 * (1.0e-3 + here.char_rate_temperature.abs()),
                    "{p} {b} {t}"
                );
                assert!(
                    (by_b(&|g| g.enthalpy) - here.enthalpy_blowing).abs()
                        < 1.0e-5 * here.enthalpy_blowing.abs() + 1.0,
                    "{p} {b} {t}"
                );
                assert!(
                    (by_t(&|g| g.enthalpy) - here.enthalpy_temperature).abs()
                        < 1.0e-5 * here.enthalpy_temperature.abs() + 1.0e-3,
                    "{p} {b} {t}"
                );
            }
            for wears in [true, false] {
                let stream = StreamNow {
                    pressure_pa: p,
                    coefficient: 0.3,
                    flow_enthalpy_j_per_kg: 1.5e6,
                    radiative_heating_w_per_m2: 0.0,
                    surroundings_temperature_k: 300.0,
                    present: true,
                    face_wears: wears,
                };
                let flux = b * 0.27;
                let here = blowing_state(&lookup, &stream, flux, t, TACOT.gas_enthalpy(t), false);
                // It solves its equation: B'g Ω = gas flux over the coefficient.
                assert!((here.blowing * here.correction - flux / 0.3).abs() < 1.0e-13);
                let (omega, _) = blowing_correction(here.blowing + here.char_rate);
                assert!((omega - here.correction).abs() < 1.0e-15);
                let (dm, dt) = (1.0e-8, 1.0e-4);
                let at = |m: f64, t: f64| {
                    blowing_state(&lookup, &stream, m, t, TACOT.gas_enthalpy(t), false)
                };
                let checks: [(&dyn Fn(&Blowing) -> f64, f64, f64); 3] = [
                    (
                        &|s| s.correction,
                        here.correction_flux,
                        here.correction_temperature,
                    ),
                    (
                        &|s| s.char_rate,
                        here.char_rate_flux,
                        here.char_rate_temperature,
                    ),
                    (
                        &|s| s.wall_enthalpy,
                        here.wall_enthalpy_flux,
                        here.wall_enthalpy_temperature,
                    ),
                ];
                for (f, by_flux, by_temperature) in checks {
                    let numeric = (f(&at(flux + dm, t)) - f(&at(flux - dm, t))) / (2.0 * dm);
                    assert!(
                        (numeric - by_flux).abs()
                            < 1.0e-5 * by_flux.abs() + 1.0e-6 * numeric.abs().max(1.0),
                        "{p} {b} {t}: {numeric} {by_flux}"
                    );
                    let numeric = (f(&at(flux, t + dt)) - f(&at(flux, t - dt))) / (2.0 * dt);
                    assert!(
                        (numeric - by_temperature).abs()
                            < 1.0e-5 * by_temperature.abs() + 1.0e-6 * numeric.abs().max(1.0e-3),
                        "{p} {b} {t}: {numeric} {by_temperature}"
                    );
                }
            }
        }
        // The blowing correction at its ends and across its change of formula.
        assert_eq!(blowing_correction(0.0).0, 1.0);
        let (a, _) = blowing_correction(0.99e-4);
        let (b, _) = blowing_correction(1.01e-4);
        assert!((a - b - 1.0e-6).abs() < 1.0e-9, "{a} {b}");
        assert!((blowing_correction(0.2105).0 - (1.2105f64).ln() / 0.2105).abs() < 1.0e-15);
    }

    /// With the wall temperature and gas flux that Amaryllis printed for case
    /// 2.2 at 60 s, the table and the blowing correction give Amaryllis' char
    /// flux. (`MODEL_NEEDED.md`, check B: Ω 0.9075, B'g 0.0454, B'c 0.1651.)
    #[test]
    fn the_wall_chemistry_gives_the_references_char_flux_from_its_own_wall_state() {
        let lookup = SurfaceLookup::of(&TACOT.surface);
        let stream = StreamNow {
            pressure_pa: 101325.0,
            coefficient: 0.3,
            flow_enthalpy_j_per_kg: 1.5e6,
            radiative_heating_w_per_m2: 0.0,
            surroundings_temperature_k: 300.0,
            present: true,
            face_wears: true,
        };
        let wall = AMARYLLIS_2_2_TEMPERATURE[60][1];
        let gas_flux = AMARYLLIS_2_2_MASS[60][1];
        let state = blowing_state(
            &lookup,
            &stream,
            gas_flux,
            wall,
            TACOT.gas_enthalpy(wall),
            false,
        );
        assert!(
            (state.correction - 0.9075).abs() < 2.0e-4,
            "{}",
            state.correction
        );
        assert!((state.blowing - 0.0454).abs() < 1.0e-4, "{}", state.blowing);
        assert!(
            (state.char_rate - 0.1651).abs() < 1.0e-4,
            "{}",
            state.char_rate
        );
        let char_flux = state.char_rate * 0.3 * state.correction;
        assert!(
            (char_flux / AMARYLLIS_2_2_MASS[60][2] - 1.0).abs() < 2.0e-4,
            "{char_flux}"
        );
        // A face that does not wear: B'c is left out of the blowing as well.
        let own = TACOT.gas_enthalpy(wall);
        let bare = blowing_state(
            &lookup,
            &StreamNow {
                face_wears: false,
                ..stream
            },
            gas_flux,
            wall,
            own,
            false,
        );
        assert_eq!(bare.char_rate, 0.0);
        assert!(bare.correction > state.correction);
        assert!(!state.past_largest_blowing);
        // As the stream fades the blowing passes the table's largest B'g. The
        // wall gas then runs smoothly to the decomposition gas's own enthalpy,
        // and the stream's heating and the char flux run to nothing.
        let mut before = state.wall_enthalpy;
        let mut heating = f64::INFINITY;
        for coefficient in [
            1.0e-2, 4.0e-3, 3.4e-3, 3.3e-3, 2.0e-3, 1.0e-3, 3.0e-4, 1.0e-4, 1.0e-5,
        ] {
            let faint = blowing_state(
                &lookup,
                &StreamNow {
                    coefficient,
                    ..stream
                },
                gas_flux,
                wall,
                own,
                false,
            );
            assert!(faint.wall_enthalpy.is_finite(), "{coefficient}");
            assert!(faint.correction * coefficient < heating);
            before = faint.wall_enthalpy;
            heating = faint.correction * coefficient;
            assert_eq!(faint.past_largest_blowing, faint.blowing > 10.0);
            if coefficient >= 1.0e-2 {
                assert!(!faint.past_largest_blowing);
            }
            if coefficient <= 1.0e-3 {
                assert!(
                    faint.past_largest_blowing,
                    "{coefficient}: B'g {}",
                    faint.blowing
                );
            }
        }
        assert!(
            (before - own.0).abs() < 1.0e-6 * own.0.abs() && heating < 1.0e-40,
            "{before} {} {heating}",
            own.0
        );
    }

    /// How the reading in pressure behaves between 0.1 and 1 atm, where the
    /// table has nothing: at 0.235 atm and B'g = 0.1, against the values
    /// computed at that pressure in
    /// `research/heatshield_surface_model/tables/summary_Bc_between_0p1_and_1atm.csv`.
    /// Not a pass line: it measures what a table with more pressures would
    /// take away. The table type takes any number of pressures.
    #[test]
    fn between_0_1_and_1_atm_the_four_pressure_table_overstates_char_loss_near_boiling() {
        let lookup = SurfaceLookup::of(&TACOT.surface);
        let pressure = 0.235 * 101325.0;
        let mut over = Vec::new();
        for (t, computed_there) in [
            (2800.0, 0.180779),
            (3000.0, 0.194117),
            (3200.0, 0.227315),
            (3300.0, 0.272277),
            (3400.0, 0.373122),
            (3500.0, 0.618163),
        ] {
            let share = lookup.read(pressure, 0.1, t, false).char_rate;
            let straight = lookup.read(pressure, 0.1, t, true).char_rate;
            eprintln!(
                "0.235 atm, B'g 0.1, {t} K: B'c computed at that pressure {computed_there:.4}; read from the four-pressure table {share:.4} ({:+.1} %); with B'c along straight lines {straight:.4} ({:+.1} %)",
                100.0 * (share / computed_there - 1.0),
                100.0 * (straight / computed_there - 1.0)
            );
            over.push((
                100.0 * (share / computed_there - 1.0),
                100.0 * (straight / computed_there - 1.0),
            ));
        }
        // Below 3000 K the reading is within 1 percent; by 3500 K it is tens
        // of percent high, and less high than straight lines in B'c.
        assert!(over[0].0.abs() < 1.0 && over[1].0.abs() < 1.0, "{over:?}");
        assert!(over[5].0 > 10.0 && over[5].0 < over[5].1, "{over:?}");
        // A table with one pressure, and one with more, are read by the same code.
        let older = tacot_with_the_older_surface_table();
        let one = SurfaceLookup::of(&older.surface);
        assert!(one.read(101325.0, 0.05, 1500.0, false).char_rate > 0.15);
    }

    // -----------------------------------------------------------------------
    // The tests that matter: workshop cases 2.1, 2.2 and 2.3.
    // -----------------------------------------------------------------------

    #[test]
    fn case_2_1_against_amaryllis() {
        let r = case_2_1();
        let (heated, cooling, deep, _) = against(
            "case 2.1, against Amaryllis",
            r,
            &AMARYLLIS_2_1_TEMPERATURE,
            &AMARYLLIS_2_1_MASS,
        );
        assert!(heated < 5.0, "{heated}");
        assert!(deep < 1.5, "{deep}");
        assert!(cooling < 10.0, "{cooling}");
        assert!(r.recession_m.iter().all(|&s| s == 0.0));
        assert!(r.char_mass_flux_kg_per_m2_s.iter().all(|&m| m == 0.0));
        assert_eq!(r.cells, r.cells_at_end);
    }

    #[test]
    fn case_2_2_against_amaryllis() {
        let r = case_2_2();
        let (heated, cooling, deep, recession) = against(
            "case 2.2, against Amaryllis",
            r,
            &AMARYLLIS_2_2_TEMPERATURE,
            &AMARYLLIS_2_2_MASS,
        );
        // The lines set before the run: wall inside 5 K from 10 to 60 s, the
        // 16 and 24 mm probes inside 1.5 percent throughout, recession inside
        // 1 percent.
        assert!(heated < 5.0, "{heated}");
        assert!(deep < 1.5, "{deep}");
        assert!(recession.abs() < 1.0, "{recession}");
        assert!(cooling < 10.0, "{cooling}");
        let at_60 = slot_at(r, 60.0);
        assert!(
            (r.char_mass_flux_kg_per_m2_s[at_60] / AMARYLLIS_2_2_MASS[60][2] - 1.0).abs() < 0.01
        );
        assert!(
            (r.gas_mass_flux_at_surface_kg_per_m2_s[at_60] / AMARYLLIS_2_2_MASS[60][1] - 1.0).abs()
                < 0.02
        );
    }

    #[test]
    fn case_2_2_against_pato() {
        let r = case_2_2();
        let mut heated = 0.0f64;
        let mut deep = 0.0f64;
        eprintln!("case 2.2, this solver minus PATO's stored result");
        eprintln!(
            "    t s    wall K    1 mm    2 mm    4 mm    8 mm   12 mm   16 mm   24 mm    back | recession mm"
        );
        for row in PATO_2_2_PROBES.iter().skip(1) {
            let t = row[0];
            let k = slot_at(r, t);
            let theirs_mass = PATO_2_2_MASS[t as usize];
            let wall = r.surface_temperature_k[k] - PATO_2_2_WALL_AND_BACK[t as usize][1];
            if (10.0..=60.0).contains(&t) {
                heated = heated.max(wall.abs());
            }
            let mut line = format!("  {t:5.0}  {wall:+8.2}");
            for slot in 0..7 {
                let ours = r.thermocouple_temperature_k[slot][k];
                let depth = r.thermocouple_depth_m[slot];
                if ours == 0.0
                    || depth - r.recession_m[k] < 0.5e-3
                    || depth - theirs_mass[5] < 0.5e-3
                {
                    line += "    gone";
                    continue;
                }
                line += &format!(" {:+7.2}", ours - row[1 + slot]);
                if slot >= 5 {
                    deep = deep.max(100.0 * (ours - row[1 + slot]).abs() / row[1 + slot]);
                }
            }
            line += &format!(
                " {:+7.2} | {:.4} / {:.4}",
                r.thermocouple_temperature_k[7][k] - PATO_2_2_WALL_AND_BACK[t as usize][2],
                r.recession_m[k] * 1.0e3,
                theirs_mass[5] * 1.0e3
            );
            if [
                1.0, 5.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 61.0, 65.0, 80.0, 100.0, 120.0,
            ]
            .contains(&t)
            {
                eprintln!("{line}");
            }
        }
        let at_60 = slot_at(r, 60.0);
        let recession = 100.0 * (r.recession_m[at_60] / PATO_2_2_MASS[60][5] - 1.0);
        eprintln!(
            "  wall, 10 to 60 s: largest {heated:.2} K; 16 and 24 mm probes: largest {deep:.2} %; recession at 60 s {recession:+.3} %"
        );
        assert!(heated < 5.0, "{heated}");
        assert!(deep < 1.5, "{deep}");
        assert!(recession.abs() < 1.0, "{recession}");
    }

    /// Case 2.3 was run in 2012 with the surface table of TACOT 2.2. Read
    /// from the older 1-atm table that ships in PATO, this solver lands on
    /// the reference. Read from TACOT 3.0's table, which is the solver's own,
    /// it is about 51 K hotter with 1.3 percent more recession: the two
    /// tables differ in wall-gas enthalpy above 2500 K. Case 2.2 does not
    /// tell the tables apart.
    #[test]
    fn case_2_3_against_amaryllis_with_the_surface_table_of_its_day() {
        let older = case_2_3_with_the_older_table();
        let (heated, cooling, deep, recession) = against(
            "case 2.3 with the older surface table, against Amaryllis",
            older,
            &AMARYLLIS_2_3_TEMPERATURE,
            &AMARYLLIS_2_3_MASS,
        );
        assert!(heated < 5.0, "{heated}");
        assert!(deep < 1.5, "{deep}");
        assert!(recession.abs() < 1.0, "{recession}");
        assert!(cooling < 10.0, "{cooling}");
        let own = case_2_3();
        let (heated_own, _, _, recession_own) = against(
            "case 2.3 with TACOT 3.0's surface table, against Amaryllis",
            own,
            &AMARYLLIS_2_3_TEMPERATURE,
            &AMARYLLIS_2_3_MASS,
        );
        assert!(heated_own > 35.0 && heated_own < 60.0, "{heated_own}");
        assert!(
            recession_own > 1.0 && recession_own < 1.6,
            "{recession_own}"
        );
        // The two tables at the reference's own wall state at 60 s.
        let wall = AMARYLLIS_2_3_TEMPERATURE[60][1];
        let new = SurfaceLookup::of(&TACOT.surface).read(101325.0, 0.05, wall, false);
        let material = tacot_with_the_older_surface_table();
        let old = SurfaceLookup::of(&material.surface).read(101325.0, 0.05, wall, false);
        eprintln!(
            "at {wall} K and B'g 0.05: wall-gas enthalpy {:.3} MJ/kg in the older table, {:.3} in TACOT 3.0; B'c {:.4} and {:.4}",
            old.enthalpy / 1.0e6,
            new.enthalpy / 1.0e6,
            old.char_rate,
            new.char_rate
        );
        assert!(old.enthalpy - new.enthalpy > 1.0e6);
        // Case 2.2 with the older table: the same answer.
        let q = ablation_workshop_test_case_2_2();
        let with_older = run_material(&material, &q, &NUMERICS, &Terms::ALL)
            .expect("case 2.2 with the older table");
        let at_60 = slot_at(case_2_2(), 60.0);
        let wall_moved =
            with_older.surface_temperature_k[at_60] - case_2_2().surface_temperature_k[at_60];
        let recession_moved =
            100.0 * (with_older.recession_m[at_60] / case_2_2().recession_m[at_60] - 1.0);
        eprintln!(
            "case 2.2 with the older table: wall at 60 s {wall_moved:+.3} K, recession {recession_moved:+.4} %"
        );
        assert!(wall_moved.abs() < 0.5 && recession_moved.abs() < 0.05);
    }

    // -----------------------------------------------------------------------
    // Each term earns its place.
    // -----------------------------------------------------------------------

    #[test]
    fn each_term_of_the_face_balance_moves_the_answer() {
        let q = stream(0.03, 0.3, 1.5e6, 101325.0, 0.0, 15.0, true);
        let end = |r: &SlabResult| {
            let last = r.report_time_s.len() - 1;
            (r.surface_temperature_k[last], r.recession_m[last] * 1.0e3)
        };
        let all = end(&run(&q, &NUMERICS, &Terms::ALL).expect("every term"));
        let no_blowing = end(&run(
            &q,
            &NUMERICS,
            &Terms {
                blowing_correction: false,
                ..Terms::ALL
            },
        )
        .expect("no blowing correction"));
        let no_gases = end(&run(
            &q,
            &NUMERICS,
            &Terms {
                gases_take_heat_at_face: false,
                ..Terms::ALL
            },
        )
        .expect("gases take no heat"));
        let mut bare = q.clone();
        if let SurfaceCondition::GasStream { face_wears, .. } = &mut bare.surface {
            *face_wears = false;
        }
        let no_wear = end(&run(&bare, &NUMERICS, &Terms::ALL).expect("no wear"));
        let no_radiation = run(
            &q,
            &NUMERICS,
            &Terms {
                reradiation: false,
                ..Terms::ALL
            },
        );
        eprintln!(
            "after 15 s of case 2.2's stream: wall {:.1} K, recession {:.3} mm; without the blowing correction on the heating {:.1} K, {:.3} mm; without the heat the gases take at the face {:.1} K, {:.3} mm; without wear {:.1} K; without radiating: {}",
            all.0,
            all.1,
            no_blowing.0,
            no_blowing.1,
            no_gases.0,
            no_gases.1,
            no_wear.0,
            match &no_radiation {
                Ok(r) => format!("{:.1} K", end(r).0),
                Err(e) => format!("refused: {e}"),
            }
        );
        assert!(no_blowing.0 > all.0 + 20.0);
        assert!((no_gases.0 - all.0).abs() > 20.0);
        assert!(no_wear.0 > all.0 + 10.0 && no_wear.1 == 0.0);
        // A face that cannot radiate runs hundreds of kelvin hotter.
        match no_radiation {
            Ok(r) => assert!(end(&r).0 > all.0 + 300.0),
            Err(err) => assert!(
                matches!(err, SlabReject::TemperatureOutOfBounds { .. }),
                "{err:?}"
            ),
        }
        // Radiative heating is taken up: the face runs hotter and wears no less.
        let lit = end(
            &solve(&stream(0.03, 0.3, 1.5e6, 101325.0, 5.0e5, 15.0, true))
                .expect("with radiative heating"),
        );
        assert!(lit.0 > all.0 + 50.0 && lit.1 >= all.1);
    }

    // -----------------------------------------------------------------------
    // The face only moves inward, and the accounts close through it.
    // -----------------------------------------------------------------------

    #[test]
    fn the_face_never_moves_backward() {
        for r in [case_2_1(), case_2_2(), case_2_3()] {
            for pair in r.recession_m.windows(2) {
                assert!(pair[1] >= pair[0], "{pair:?}");
            }
            assert!(r.char_mass_flux_kg_per_m2_s.iter().all(|&m| m >= 0.0));
            for history in [&r.char_front_depth_m, &r.virgin_front_depth_m] {
                assert!(history.windows(2).all(|pair| pair[1] >= pair[0]));
            }
            // The char front is never above the face.
            assert!(r
                .char_front_depth_m
                .iter()
                .zip(&r.recession_m)
                .all(|(c, s)| c >= s));
        }
        // After the stream stops, the face stays where it is.
        let r = case_2_2();
        let at_61 = slot_at(r, 61.0);
        assert!(r.recession_m[at_61..]
            .iter()
            .all(|&s| s == r.recession_m[at_61]));
        assert!(r.cells_at_end < r.cells);
        ensure_face_moved_inward(1.0, 0.04, 1.0e-4, 0.9e-4).expect("inward");
        ensure_face_moved_inward(1.0, 0.0, 1.0e-4, 1.0e-4).expect("still");
        ensure_face_moved_inward(1.0, -0.5 * CHAR_FLUX_TOLERANCE_KG_PER_M2_S, 1.0e-4, 1.0e-4)
            .expect("rounding");
        assert_eq!(
            ensure_face_moved_inward(1.0, -1.0e-9, 1.0e-4, 1.0e-4).expect_err("backward"),
            SlabReject::FaceMovedBackward { time_s: 1.0 }
        );
        assert_eq!(
            ensure_face_moved_inward(1.0, 0.04, 1.0e-4, 0.2e-4).expect_err("too far"),
            SlabReject::FaceOutranItsCell { time_s: 1.0 }
        );
        assert_eq!(
            ensure_face_moved_inward(1.0, f64::NAN, 1.0e-4, 1.0e-4).expect_err("not a number"),
            SlabReject::NonFiniteValue { time_s: 1.0 }
        );
    }

    /// Mass lost by the solid, recomputed from a result's final profile.
    fn mass_lost_from_profile(r: &SlabResult, thickness_m: f64) -> f64 {
        let mut face = r.recession_m[r.recession_m.len() - 1];
        let mut left = 0.0;
        for (centre, density) in r.final_depth_m.iter().zip(&r.final_density_kg_per_m3) {
            let width = 2.0 * (centre - face);
            left += density * width;
            face += width;
        }
        assert!((face - thickness_m).abs() < 1.0e-12, "{face}");
        VIRGIN_DENSITY_KG_PER_M3 * thickness_m - left
    }

    fn assert_accounts_close(label: &str, r: &SlabResult, thickness_m: f64) {
        eprintln!(
            "{label}: heat, J/m2: from the stream {:.0}, absorbed {:.0}, radiated {:.0}, taken at the face by gas and char {:.0}, conducted in {:.0} (face balance open by {:.1e}, allowed {:.1e}); stored {:.0}, taken by the gas inside {:.0}, given off by decomposition {:.0}, left with the char {:.0} (open by {:.1e}, allowed {:.1e}). mass, kg/m2: solid lost {:.6}, gas out {:.6}, char worn {:.6} (open by {:.1e}, allowed {:.1e})",
            r.heat_from_stream_j_per_m2,
            r.heat_absorbed_from_radiation_j_per_m2,
            r.heat_radiated_away_j_per_m2,
            r.heat_taken_at_face_by_gases_j_per_m2,
            r.heat_in_through_face_j_per_m2,
            r.face_balance_residual_j_per_m2,
            r.face_balance_tolerance_j_per_m2,
            r.heat_stored_in_solid_j_per_m2,
            r.heat_taken_by_gas_j_per_m2,
            r.heat_given_off_by_decomposition_j_per_m2,
            r.char_enthalpy_removed_j_per_m2,
            r.energy_account_residual_j_per_m2,
            r.energy_account_tolerance_j_per_m2,
            r.solid_mass_lost_kg_per_m2,
            r.gas_mass_out_kg_per_m2,
            r.char_mass_removed_kg_per_m2,
            r.mass_account_residual_kg_per_m2,
            r.mass_account_tolerance_kg_per_m2
        );
        let scale = r.heat_from_stream_j_per_m2.abs()
            + r.heat_absorbed_from_radiation_j_per_m2
            + r.heat_in_through_face_j_per_m2.abs();
        // The face: conducted in = from the stream + absorbed − radiated − taken.
        let face = r.heat_in_through_face_j_per_m2
            - r.heat_from_stream_j_per_m2
            - r.heat_absorbed_from_radiation_j_per_m2
            + r.heat_radiated_away_j_per_m2
            + r.heat_taken_at_face_by_gases_j_per_m2;
        assert!(face.abs() < 1.0e-9 * scale, "{label}: face {face}");
        assert!(r.face_balance_residual_j_per_m2.abs() <= r.face_balance_tolerance_j_per_m2);
        // The solid, from its first and last states: its enthalpy changed by
        // the heat conducted in, less what the gas and the worn char carried out.
        let solid = r.solid_enthalpy_change_j_per_m2 - r.heat_in_through_face_j_per_m2
            + r.gas_enthalpy_out_j_per_m2
            + r.char_enthalpy_removed_j_per_m2;
        assert!(solid.abs() < 1.0e-9 * scale, "{label}: solid {solid}");
        assert!(r.energy_account_residual_j_per_m2.abs() <= r.energy_account_tolerance_j_per_m2);
        // Mass: solid lost = gas out + char worn, with the solid's loss
        // recomputed here from the final profile and the final recession.
        let lost = mass_lost_from_profile(r, thickness_m);
        assert!(
            (lost - r.gas_mass_out_kg_per_m2 - r.char_mass_removed_kg_per_m2).abs() < 1.0e-9 * lost,
            "{label}: {lost}"
        );
        assert!((lost - r.solid_mass_lost_kg_per_m2).abs() < 1.0e-9 * lost);
        assert!(r.mass_account_residual_kg_per_m2.abs() <= r.mass_account_tolerance_kg_per_m2);
    }

    #[test]
    fn the_accounts_close_through_heating_wear_and_cooling() {
        assert_accounts_close("case 2.1", case_2_1(), 0.05);
        assert_accounts_close("case 2.2", case_2_2(), 0.05);
        assert_accounts_close("case 2.3", case_2_3(), 0.05);
        let r = case_2_2();
        // Most of the stream's heat is radiated away; the char worn off is
        // most of the mass lost.
        assert!(r.heat_radiated_away_j_per_m2 > 0.5 * r.heat_from_stream_j_per_m2);
        assert!(r.char_mass_removed_kg_per_m2 > 2.0 * r.gas_mass_out_kg_per_m2);
        assert_eq!(case_2_1().char_mass_removed_kg_per_m2, 0.0);
    }

    #[test]
    fn heat_or_gas_that_no_account_knows_about_is_refused_under_a_stream() {
        let q = stream(0.02, 0.3, 1.5e6, 101325.0, 0.0, 2.0, true);
        run(&q, &NUMERICS, &Terms::ALL).expect("sound");
        let err = run(
            &q,
            &NUMERICS,
            &Terms {
                unaccounted_heat_w_per_m2: 100.0,
                ..Terms::ALL
            },
        )
        .expect_err("heat leak");
        assert!(
            matches!(err, SlabReject::EnergyAccountBroken { .. }),
            "{err:?}"
        );
        let err = run(
            &q,
            &NUMERICS,
            &Terms {
                unaccounted_gas_kg_per_m2_s: 1.0e-6,
                ..Terms::ALL
            },
        )
        .expect_err("gas leak");
        assert!(
            matches!(err, SlabReject::MassAccountBroken { .. }),
            "{err:?}"
        );
        // The face balance itself.
        let sound = FaceAccount {
            heat_from_stream_j: 10.0,
            heat_radiated_j: 6.0,
            balance_residual_j: 1.0e-9,
            balance_tolerance_j: 1.0e-8,
            ..FaceAccount::default()
        };
        ensure_face_balance(2.0, &sound).expect("closes");
        let err = ensure_face_balance(
            2.0,
            &FaceAccount {
                balance_residual_j: -2.0e-8,
                ..sound
            },
        )
        .expect_err("open");
        assert_eq!(
            err,
            SlabReject::EnergyAccountBroken {
                time_s: 2.0,
                residual_j_per_m2: -2.0e-8,
                tolerance_j_per_m2: 1.0e-8
            }
        );
        let err = ensure_face_balance(
            2.0,
            &FaceAccount {
                balance_residual_j: f64::NAN,
                ..sound
            },
        )
        .expect_err("not a number");
        assert_eq!(err, SlabReject::NonFiniteValue { time_s: 2.0 });
    }

    /// The face is made to stop exactly on what was a cell boundary, and
    /// exactly on what was a cell centre, of the layout the run started with.
    /// The accounts close in both, and the answers either side of each
    /// landing agree.
    #[test]
    fn a_face_that_stops_on_a_cell_boundary_or_a_cell_centre_keeps_its_accounts() {
        let q = stream(0.03, 0.3, 1.5e6, 101325.0, 0.0, 12.0, true);
        let layout = cell_faces(
            0.03,
            &Numerics {
                largest_cell_m: LARGEST_CELL_M * LARGEST_CELL_IN_A_STREAM,
                ..NUMERICS
            },
        );
        let boundary = layout[40];
        let centre = 0.5 * (layout[40] + layout[41]);
        assert!(boundary > 1.0e-3 && centre < 2.0e-3, "{boundary} {centre}");
        let free = run(&q, &NUMERICS, &Terms::ALL).expect("not stopped");
        assert!(*free.recession_m.last().unwrap() > centre + 2.0e-4);
        let mut landed = Vec::new();
        for target in [
            boundary,
            centre,
            boundary * (1.0 + 1.0e-12),
            boundary * (1.0 - 1.0e-12),
            centre * (1.0 + 1.0e-12),
        ] {
            let r = run(
                &q,
                &NUMERICS,
                &Terms {
                    face_stops_at_m: Some(target),
                    ..Terms::ALL
                },
            )
            .expect("stopped");
            let end = *r.recession_m.last().unwrap();
            assert!(
                (end - target).abs() <= 4.0 * f64::EPSILON * target,
                "{end} {target}"
            );
            assert_accounts_close("face stopped", &r, 0.03);
            // No char left after it stopped, and the face stayed.
            assert_eq!(*r.char_mass_flux_kg_per_m2_s.last().unwrap(), 0.0);
            assert!(
                (r.char_mass_removed_kg_per_m2 - free.char_mass_removed_kg_per_m2).abs() > 0.05
            );
            landed.push(r);
        }
        assert_eq!(
            landed[0].recession_m.last().unwrap().to_bits(),
            boundary.to_bits()
        );
        assert_eq!(
            landed[1].recession_m.last().unwrap().to_bits(),
            centre.to_bits()
        );
        // A hair either side of the boundary gives the same slab to a hair.
        let last = landed[0].report_time_s.len() - 1;
        for near in [&landed[2], &landed[3]] {
            assert!(
                (near.surface_temperature_k[last] - landed[0].surface_temperature_k[last]).abs()
                    < 1.0e-6
            );
            assert!(
                (near.heat_in_through_face_j_per_m2 / landed[0].heat_in_through_face_j_per_m2
                    - 1.0)
                    .abs()
                    < 1.0e-9
            );
            assert!(
                (near.char_mass_removed_kg_per_m2 / landed[0].char_mass_removed_kg_per_m2 - 1.0)
                    .abs()
                    < 1.0e-9
            );
        }
        assert!(
            (landed[4].heat_in_through_face_j_per_m2 / landed[1].heat_in_through_face_j_per_m2
                - 1.0)
                .abs()
                < 1.0e-9
        );
    }

    /// Joining the front cell to its neighbour keeps the resin and the
    /// enthalpy of the two, whatever the front cell's width: a rounding
    /// error wide, exactly at the joining threshold, or wider than its
    /// neighbour.
    #[test]
    fn joining_the_front_cell_to_its_neighbour_loses_nothing() {
        let nodes = SolidEnthalpyNodes::new();
        for front_width in [
            1.0e-18,
            1.0e-12,
            0.5e-4 * (1.0 - 1.0e-15),
            0.5e-4,
            0.9e-4,
            3.0e-4,
        ] {
            let mut slab = Slab {
                face_depth_m: vec![1.0e-3 - front_width, 1.0e-3, 1.1e-3, 1.2e-3],
                temperature_k: vec![1570.0, 1490.0, 1400.0],
                remaining: vec![[0.001, 0.0], [0.02, 0.001], [0.2, 0.05]],
                gas_flux_kg_per_m2_s: vec![0.012, 0.0119, 0.0117, 0.0],
                face_remaining: [0.001, 0.0],
                face_temperature_k: 1575.0,
                char_flux_kg_per_m2_s: 0.045,
            };
            let totals = |slab: &Slab| {
                let mut mass = 0.0;
                let mut enthalpy = 0.0;
                for i in 0..slab.cells() {
                    let density = TACOT.solid_density(slab.remaining[i]);
                    mass += density * slab.width_m(i);
                    enthalpy += solid_enthalpy_per_volume(
                        &solid_point(&nodes, slab.temperature_k[i], None),
                        density,
                    ) * slab.width_m(i);
                }
                (mass, enthalpy)
            };
            let before = totals(&slab);
            slab.join_front_cells(&nodes, None);
            let after = totals(&slab);
            assert_eq!(slab.cells(), 2);
            assert_eq!(
                slab.face_depth_m,
                vec![1.0e-3 - front_width, 1.1e-3, 1.2e-3]
            );
            assert_eq!(slab.gas_flux_kg_per_m2_s, vec![0.012, 0.0117, 0.0]);
            assert!(
                (after.0 - before.0).abs() <= 1.0e-14 * before.0,
                "{front_width}: mass {} {}",
                before.0,
                after.0
            );
            assert!(
                (after.1 - before.1).abs() <= 1.0e-12 * before.1.abs(),
                "{front_width}: enthalpy {} {}",
                before.1,
                after.1
            );
            assert!(slab.temperature_k[0] >= 1490.0 && slab.temperature_k[0] <= 1570.0);
            assert!(slab.remaining[0][0] >= 0.001 && slab.remaining[0][0] <= 0.02);
        }
    }

    // -----------------------------------------------------------------------
    // Refusals, bits, bands.
    // -----------------------------------------------------------------------

    #[test]
    fn a_heating_history_outside_the_declared_range_is_refused_by_name() {
        let good = || ablation_workshop_test_case_2_2();
        let with = |change: &dyn Fn(&mut Vec<StreamRow>, &mut f64)| {
            let mut q = good();
            if let SurfaceCondition::GasStream {
                history,
                surroundings_temperature_k,
                ..
            } = &mut q.surface
            {
                change(history, surroundings_temperature_k);
            }
            solve(&q).expect_err("out of range")
        };
        assert_eq!(
            with(&|h, _| h.truncate(1)),
            SlabReject::HistoryRowsOutOfRange { rows: 1 }
        );
        assert_eq!(
            with(&|h, _| h.clear()),
            SlabReject::HistoryRowsOutOfRange { rows: 0 }
        );
        let long = with(&|h, _| {
            let last = h[4];
            for extra in 0..HISTORY_ROWS_MAX {
                h.push(StreamRow {
                    time_s: 121.0 + extra as f64,
                    ..last
                });
            }
        });
        assert_eq!(
            long,
            SlabReject::HistoryRowsOutOfRange {
                rows: HISTORY_ROWS_MAX + 5
            }
        );
        assert_eq!(
            with(&|h, _| h[0].time_s = 0.01),
            SlabReject::HistoryTimesOutOfOrder {
                row: 0,
                time_s: 0.01
            }
        );
        assert_eq!(
            with(&|h, _| h[3].time_s = 60.0),
            SlabReject::HistoryTimesOutOfOrder {
                row: 3,
                time_s: 60.0
            }
        );
        assert!(matches!(
            with(&|h, _| h[2].time_s = f64::NAN),
            SlabReject::HistoryTimesOutOfOrder { row: 2, .. }
        ));
        assert_eq!(
            with(&|h, _| h[4].time_s = 600.5),
            SlabReject::HistoryDurationOutOfRange { duration_s: 600.5 }
        );
        assert_eq!(
            with(&|h, _| h.truncate(2)),
            SlabReject::HistoryDurationOutOfRange { duration_s: 0.1 }
        );
        for pressure in [
            STREAM_PRESSURE_MIN_PA * 0.999,
            STREAM_PRESSURE_MAX_PA * 1.001,
            0.0,
            f64::NAN,
        ] {
            assert!(matches!(
                with(&|h, _| h[1].pressure_pa = pressure),
                SlabReject::StreamPressureOutOfRange { row: 1, .. }
            ));
        }
        for coefficient in [
            -1.0e-9,
            HEAT_TRANSFER_COEFFICIENT_MAX_KG_PER_M2_S * 1.001,
            f64::NAN,
        ] {
            assert!(matches!(
                with(&|h, _| h[2].heat_transfer_coefficient_kg_per_m2_s = coefficient),
                SlabReject::HeatTransferCoefficientOutOfRange { row: 2, .. }
            ));
        }
        for enthalpy in [-1.0, FLOW_ENTHALPY_MAX_J_PER_KG * 1.001, f64::INFINITY] {
            assert!(matches!(
                with(&|h, _| h[1].flow_enthalpy_j_per_kg = enthalpy),
                SlabReject::FlowEnthalpyOutOfRange { row: 1, .. }
            ));
        }
        for heating in [-1.0, RADIATIVE_HEATING_MAX_W_PER_M2 * 1.001, f64::NAN] {
            assert!(matches!(
                with(&|h, _| h[4].radiative_heating_w_per_m2 = heating),
                SlabReject::RadiativeHeatingOutOfRange { row: 4, .. }
            ));
        }
        for surroundings in [
            SURROUNDINGS_TEMPERATURE_MIN_K - 0.1,
            SURROUNDINGS_TEMPERATURE_MAX_K + 0.1,
            f64::NAN,
        ] {
            assert!(matches!(
                with(&|_, s| *s = surroundings),
                SlabReject::SurroundingsTemperatureOutOfRange { .. }
            ));
        }
        let mut q = good();
        q.continue_after_s = 1.0;
        assert_eq!(
            solve(&q).expect_err("continue after"),
            SlabReject::ContinueAfterWithStream {
                continue_after_s: 1.0
            }
        );
        let mut q = good();
        q.thickness_m = 0.2;
        assert!(matches!(
            solve(&q).expect_err("thickness"),
            SlabReject::ThicknessOutOfRange { .. }
        ));
        let mut q = good();
        q.thermocouple_depths_m = vec![0.06];
        assert!(matches!(
            solve(&q).expect_err("thermocouple"),
            SlabReject::ThermocoupleOutsideSlab { index: 0, .. }
        ));
        // The declared pressure range is the surface table's own.
        assert_eq!(TACOT.surface.pressure_pa[0], STREAM_PRESSURE_MIN_PA);
        assert_eq!(TACOT.surface.pressure_pa[3], STREAM_PRESSURE_MAX_PA);
        for refusal in [
            long,
            with(&|h, _| h[0].time_s = 0.01),
            with(&|h, _| h[1].pressure_pa = 1.0),
        ] {
            assert!(refusal.to_string().contains("heating history"), "{refusal}");
        }
    }

    /// A row of the heating history less than a millisecond after the row
    /// before it is refused by its own name, by the check that computes
    /// nothing, where until version 0.8.0 of this crate the stepping went
    /// through the jump in steps as small as its span (139,626 time steps
    /// for the first history here). A row a millisecond after the one
    /// before is answered, with the numbers it had before the floor was
    /// written. Rows at the same time or out of order, a history too short
    /// and a row outside its range keep the refusal they had.
    #[test]
    fn a_history_row_under_a_millisecond_after_the_one_before_is_refused_by_name() {
        assert_eq!(HISTORY_ROW_SPAN_MIN_S, 1.0e-3);
        let row = |time_s: f64, coefficient: f64| StreamRow {
            time_s,
            pressure_pa: 1000.0,
            heat_transfer_coefficient_kg_per_m2_s: coefficient,
            flow_enthalpy_j_per_kg: 2.0e6,
            radiative_heating_w_per_m2: 0.0,
        };
        let under = |history: Vec<StreamRow>| SlabQuestion {
            thickness_m: 0.05,
            start_temperature_k: 300.0,
            surface: SurfaceCondition::GasStream {
                history,
                surroundings_temperature_k: 300.0,
                face_wears: true,
                stream_gas: StreamGas::AIR,
            },
            continue_after_s: 0.0,
            thermocouple_depths_m: vec![],
        };
        // The stream comes on between the first row and a second row a
        // given time after it, and is held to 10 s.
        let jump = |second_row_s: f64| under(vec![row(0.0, 0.0), row(second_row_s, 0.3), row(10.0, 0.3)]);
        for span_s in [1.0e-300, 1.0e-100, 1.0e-12, 1.0e-4, 0.000999999] {
            let refused = SlabReject::HistoryRowsTooClose { row: 1, span_s };
            // `check_question` decides before anything is computed.
            assert_eq!(check_question(MATERIAL_TACOT, &jump(span_s)), Err(refused.clone()));
            assert_eq!(solve(&jump(span_s)).expect_err("too close"), refused);
            assert_eq!(
                answer(MATERIAL_TACOT, &jump(span_s), &Wanted::default()).expect_err("too close"),
                refused
            );
            // A PICA question is refused by the same name, before its envelopes are asked.
            assert_eq!(check_question(MATERIAL_PICA_SERVED, &jump(span_s)), Err(refused));
        }
        let refused = SlabReject::HistoryRowsTooClose { row: 1, span_s: 1.0e-300 };
        assert_eq!(refused.name(), "HistoryRowsTooClose");
        assert_eq!(
            refused.to_string(),
            "row 1 of the heating history is 1e-300 s after the row before it; the least time between two rows is 0.001 s"
        );

        // A millisecond exactly is answered, with the numbers the crate
        // gave before this floor: 2,511 time steps, and these bits.
        assert_eq!(check_question(MATERIAL_TACOT, &jump(HISTORY_ROW_SPAN_MIN_S)), Ok(()));
        let exact = answer(MATERIAL_TACOT, &jump(HISTORY_ROW_SPAN_MIN_S), &Wanted::default()).expect("a millisecond is answered");
        assert_eq!(exact.summary.time_steps, 2511);
        assert_eq!((exact.summary.cells, exact.summary.cells_at_end), (528, 483));
        for (value, bits) in [
            (exact.summary.wear_m, 0x3f5eb54587526ad7_u64),
            (exact.summary.peak_face_temperature_k, 0x40993fe2c203b36c),
            (exact.summary.final_face_temperature_k, 0x40993fe2c203b36c),
            (exact.summary.final_back_temperature_k, 0x4072c00000000000),
            (exact.summary.heat_from_stream_j_per_m2, 0x41539ae6e8b4cdd4),
            (exact.summary.heat_stored_in_solid_j_per_m2, 0x414311ecb9c197eb),
            (exact.summary.solid_mass_lost_kg_per_m2, 0x3fe48235fb32abf0),
        ] {
            assert_eq!(value.to_bits(), bits, "{value}");
        }

        // The row and the span are those of the first pair too close.
        let late = under(vec![row(0.0, 0.0), row(0.5, 0.3), row(5.0, 0.3), row(5.0005, 0.1), row(10.0, 0.1)]);
        assert_eq!(
            check_question(MATERIAL_TACOT, &late),
            Err(SlabReject::HistoryRowsTooClose { row: 3, span_s: 5.0005 - 5.0 })
        );
        // The span is the difference of the two times as binary numbers, and
        // two times written a millisecond apart can be a hair under one
        // apart: they are taken, at every time a history may hold.
        let apart = |earlier_s: f64, later_s: f64| {
            check_question(MATERIAL_TACOT, &under(vec![row(0.0, 0.0), row(earlier_s, 0.3), row(later_s, 0.1), row(HISTORY_DURATION_MAX_S, 0.1)]))
        };
        assert_eq!(1.001 - 1.0, 0.0009999999999998899);
        for earlier_s in [0.001, 0.002, 1.0, 5.0, 10.0, 59.999, 123.456, 599.0, 599.998] {
            assert_eq!(apart(earlier_s, earlier_s + 0.001), Ok(()), "{earlier_s}");
        }
        let mut whole = 0.0_f64;
        while whole < 599.0 {
            // every whole second and the millisecond after it, as a decimal would be read
            let later: f64 = format!("{}.001", whole as u64).parse().unwrap();
            assert_eq!(apart(whole.max(0.5), later.max(0.501)), Ok(()), "{whole}");
            whole += 1.0;
        }
        // A span a millionth of a millisecond short is refused.
        assert_eq!(
            apart(1.0, 1.000999999),
            Err(SlabReject::HistoryRowsTooClose { row: 2, span_s: 1.000999999 - 1.0 })
        );

        // What was refused before is refused as before. Rows at the same
        // time or out of order:
        assert_eq!(
            check_question(MATERIAL_TACOT, &under(vec![row(0.0, 0.0), row(5.0, 0.3), row(5.0, 0.3), row(10.0, 0.3)])),
            Err(SlabReject::HistoryTimesOutOfOrder { row: 2, time_s: 5.0 })
        );
        assert_eq!(
            check_question(MATERIAL_TACOT, &under(vec![row(0.0, 0.0), row(1.0e-300, 0.3), row(0.0, 0.3), row(10.0, 0.3)])),
            Err(SlabReject::HistoryTimesOutOfOrder { row: 2, time_s: 0.0 })
        );
        // A history that ends too soon, though its rows are too close as well:
        assert_eq!(
            check_question(MATERIAL_TACOT, &under(vec![row(0.0, 0.0), row(1.0e-300, 0.3), row(0.5, 0.3)])),
            Err(SlabReject::HistoryDurationOutOfRange { duration_s: 0.5 })
        );
        // A row outside its range, though it is too close as well:
        let mut thin = jump(1.0e-300);
        if let SurfaceCondition::GasStream { history, .. } = &mut thin.surface {
            history[1].pressure_pa = 1.0;
        }
        assert!(matches!(
            check_question(MATERIAL_TACOT, &thin),
            Err(SlabReject::StreamPressureOutOfRange { row: 1, .. })
        ));
    }

    /// A flight history is a legitimate question: 134 rows a second apart,
    /// the size of the Stardust table. (Shaped like a flight, not Stardust's
    /// numbers, and on TACOT.) What stands in the way of the real Stardust
    /// table is named: its first rows are at 1.66 Pa, below the surface
    /// table's lowest pressure.
    #[test]
    fn a_flight_history_of_134_rows_is_answered() {
        let mut history = Vec::new();
        for second in 0..134 {
            let t = second as f64;
            let pulse = (-((t - 55.0) / 18.0).powi(2)).exp();
            history.push(StreamRow {
                time_s: t,
                pressure_pa: 150.0 + 30_000.0 * (-((t - 62.0) / 20.0).powi(2)).exp(),
                heat_transfer_coefficient_kg_per_m2_s: 0.0005 + 0.05 * pulse,
                flow_enthalpy_j_per_kg: 6.0e7 * (1.0 - t / 160.0),
                radiative_heating_w_per_m2: 2.0e5 * pulse,
            });
        }
        let q = SlabQuestion {
            thickness_m: 0.058,
            start_temperature_k: 266.48,
            surface: SurfaceCondition::GasStream {
                history: history.clone(),
                surroundings_temperature_k: 266.48,
                face_wears: true,
                stream_gas: StreamGas::AIR,
            },
            continue_after_s: 0.0,
            thermocouple_depths_m: vec![0.005, 0.02, 0.058],
        };
        let r = solve(&q).expect("a flight-shaped history");
        assert_eq!(r.report_time_s.len(), 134);
        let hottest = r
            .surface_temperature_k
            .iter()
            .cloned()
            .fold(0.0f64, f64::max);
        let worn = *r.recession_m.last().unwrap();
        eprintln!(
            "134 rows: {} steps, hardest step {} passes; hottest face {hottest:.0} K; recession {:.2} mm; cells {} -> {}",
            r.time_steps,
            r.hardest_step_newton_passes,
            worn * 1.0e3,
            r.cells,
            r.cells_at_end
        );
        assert!(hottest > 2000.0 && worn > 1.0e-4);
        assert_accounts_close("flight-shaped history", &r, 0.058);
        // The real table's first row.
        let mut low = q.clone();
        if let SurfaceCondition::GasStream { history, .. } = &mut low.surface {
            history[0].pressure_pa = 1.65787;
        }
        assert_eq!(
            solve(&low).expect_err("1.66 Pa"),
            SlabReject::StreamPressureOutOfRange {
                row: 0,
                pressure_pa: 1.65787,
                lowest_pressure_pa: STREAM_PRESSURE_MIN_PA,
                highest_pressure_pa: STREAM_PRESSURE_MAX_PA,
            }
        );
    }

    #[test]
    fn a_face_or_a_slab_taken_past_what_the_tables_hold_is_refused_by_name() {
        // Hotter than the material's tables go (3333 K).
        let err = solve(&stream(0.05, 1.0, 1.0e8, 101325.0, 0.0, 5.0, true))
            .expect_err("too hot for the material table");
        assert!(
            matches!(err, SlabReject::TemperatureOutOfBounds { temperature_k, .. } if temperature_k > 3333.0),
            "{err:?}"
        );
        // At 0.001 atm the table's carbon boils at 3075 K: a face driven
        // there is past what the surface table holds.
        let err = solve(&stream(0.05, 0.003, 1.0e4, 101.325, 6.0e6, 20.0, false))
            .expect_err("past the surface table");
        assert!(
            matches!(err, SlabReject::FaceBeyondSurfaceTable { face_temperature_k, .. } if face_temperature_k > 2900.0),
            "{err:?}"
        );
        // A thin slab under case 2.3's stream wears through.
        let err =
            solve(&stream(0.01, 0.3, 2.5e7, 101325.0, 0.0, 60.0, true)).expect_err("worn away");
        assert!(
            matches!(err, SlabReject::SlabWornAway { recession_m, .. } if recession_m > 0.008),
            "{err:?}"
        );
        for refusal in [
            SlabReject::FaceBeyondSurfaceTable {
                time_s: 1.0,
                face_temperature_k: 3100.0,
            },
            SlabReject::SlabWornAway {
                time_s: 1.0,
                recession_m: 0.009,
            },
            SlabReject::FaceMovedBackward { time_s: 1.0 },
            SlabReject::FaceOutranItsCell { time_s: 1.0 },
            SlabReject::ContinueAfterWithStream {
                continue_after_s: 1.0,
            },
        ] {
            assert!(!refusal.to_string().is_empty());
        }
    }

    #[test]
    fn the_same_heating_history_twice_is_bit_identical() {
        let q = stream(0.03, 0.3, 2.5e7, 101325.0, 1.0e5, 6.0, true);
        let numbers = |r: &SlabResult| {
            let mut all: Vec<u64> = Vec::new();
            for list in [
                &r.report_time_s,
                &r.surface_temperature_k,
                &r.gas_mass_flux_at_surface_kg_per_m2_s,
                &r.char_front_depth_m,
                &r.virgin_front_depth_m,
                &r.recession_m,
                &r.char_mass_flux_kg_per_m2_s,
                &r.final_depth_m,
                &r.final_temperature_k,
                &r.final_density_kg_per_m3,
            ] {
                all.extend(list.iter().map(|v| v.to_bits()));
            }
            for history in &r.thermocouple_temperature_k {
                all.extend(history.iter().map(|v| v.to_bits()));
            }
            for value in [
                r.heat_in_through_face_j_per_m2,
                r.heat_from_stream_j_per_m2,
                r.heat_radiated_away_j_per_m2,
                r.heat_absorbed_from_radiation_j_per_m2,
                r.heat_taken_at_face_by_gases_j_per_m2,
                r.char_mass_removed_kg_per_m2,
                r.char_enthalpy_removed_j_per_m2,
                r.solid_mass_lost_kg_per_m2,
                r.energy_account_residual_j_per_m2,
                r.face_balance_residual_j_per_m2,
            ] {
                all.push(value.to_bits());
            }
            all.push(r.time_steps as u64);
            all.push(r.cells_at_end as u64);
            all
        };
        let first = solve(&q).expect("first");
        let second = solve(&q).expect("second");
        assert_eq!(numbers(&first), numbers(&second));
        assert!(numbers(&first).len() > 500);
        assert!(numbers(&first)
            .iter()
            .all(|bits| f64::from_bits(*bits).is_finite()));
        assert!(*first.recession_m.last().unwrap() > 1.0e-3);
    }

    #[test]
    fn every_number_under_a_stream_is_finite_and_in_its_band() {
        for r in [case_2_1(), case_2_2(), case_2_3()] {
            for list in [&r.surface_temperature_k, &r.final_temperature_k] {
                assert!(list.iter().all(|&t| t.is_finite()
                    && t >= 300.0 - TEMPERATURE_BOUND_TOLERANCE_K
                    && t < 3333.4));
            }
            for history in &r.thermocouple_temperature_k {
                // A thermocouple the face has passed reads zero from then on.
                let mut passed = false;
                for &t in history {
                    if t == 0.0 {
                        passed = true;
                    } else {
                        assert!(
                            !passed && t >= 300.0 - TEMPERATURE_BOUND_TOLERANCE_K && t < 3333.4,
                            "{t}"
                        );
                    }
                }
            }
            assert!(r
                .final_density_kg_per_m3
                .iter()
                .all(|&d| (CHAR_DENSITY_KG_PER_M3..=VIRGIN_DENSITY_KG_PER_M3).contains(&d)));
            assert_eq!(r.report_time_s.len(), 121);
            assert_eq!(r.recession_m.len(), 121);
            assert_eq!(r.final_depth_m.len(), r.cells_at_end as usize);
            assert!(r.hardest_step_newton_passes * 2 <= NEWTON_PASS_LIMIT);
        }
        // In case 2.2 the face passes the 1, 2, 4 and 8 mm thermocouples.
        let r = case_2_2();
        for slot in 0..4 {
            assert_eq!(*r.thermocouple_temperature_k[slot].last().unwrap(), 0.0);
        }
        assert!(*r.thermocouple_temperature_k[4].last().unwrap() > 300.0);
    }

    // -----------------------------------------------------------------------
    // The grid, the time step and the table reading.
    // -----------------------------------------------------------------------

    /// About 7 seconds with `--release`.
    #[test]
    #[ignore]
    fn halving_the_cells_or_the_time_step_under_a_stream_moves_little() {
        for (label, q, base) in [
            ("case 2.2", ablation_workshop_test_case_2_2(), case_2_2()),
            ("case 2.3", ablation_workshop_test_case_2_3(), case_2_3()),
        ] {
            let mut cells = NUMERICS;
            cells.first_cell_m *= 0.5;
            cells.largest_cell_m *= 0.5;
            cells.cell_growth = cells.cell_growth.sqrt();
            let mut steps = NUMERICS;
            steps.first_step_s *= 0.5;
            steps.step_fraction *= 0.5;
            steps.largest_step_s *= 0.5;
            steps.wear_per_step *= 0.5;
            let straight = Terms {
                char_rate_along_straight_lines: true,
                ..Terms::ALL
            };
            for (name, numerics, terms) in [
                ("cells halved", cells, Terms::ALL),
                ("time step halved", steps, Terms::ALL),
                ("B'c read along straight lines", NUMERICS, straight),
            ] {
                let finer = run(&q, &numerics, &terms).expect(name);
                let mut wall = (0.0f64, 0.0);
                let mut probe = (0.0f64, 0.0, 0.0);
                for (k, &t) in base.report_time_s.iter().enumerate() {
                    let d = (finer.surface_temperature_k[k] - base.surface_temperature_k[k]).abs();
                    if t >= 2.0 && d > wall.0 {
                        wall = (d, t);
                    }
                    for (slot, &depth) in base.thermocouple_depth_m.iter().enumerate() {
                        let (a, b) = (
                            base.thermocouple_temperature_k[slot][k],
                            finer.thermocouple_temperature_k[slot][k],
                        );
                        if a == 0.0 || b == 0.0 || depth - base.recession_m[k] < 0.5e-3 {
                            continue;
                        }
                        if (a - b).abs() > probe.0 {
                            probe = ((a - b).abs(), t, depth);
                        }
                    }
                }
                let at_60 = slot_at(base, 60.0);
                let recession = 100.0 * (finer.recession_m[at_60] / base.recession_m[at_60] - 1.0);
                eprintln!(
                    "{label}, {name}: {} cells, {} steps; wall moves at most {:.3} K (at {} s, from 2 s on), {:+.3} K at 60 s; a covered probe at most {:.3} K (at {} s, {} m); recession at 60 s {recession:+.4} %",
                    finer.cells,
                    finer.time_steps,
                    wall.0,
                    wall.1,
                    finer.surface_temperature_k[at_60] - base.surface_temperature_k[at_60],
                    probe.0,
                    probe.1,
                    probe.2
                );
                assert!(wall.0 < 1.0, "{label} {name}: {wall:?}");
                assert!(probe.0 < 1.5, "{label} {name}: {probe:?}");
                assert!(recession.abs() < 0.05, "{label} {name}: {recession}");
            }
        }
    }

    // -----------------------------------------------------------------------
    // The declared range.
    // -----------------------------------------------------------------------

    /// 3,456 heating histories across the declared range. About 17 minutes
    /// with `--release`. Measured: 2,145 answered; 751 refused because a
    /// temperature passed the top of the material's tables, 402 because the
    /// face passed the top of the surface table at its pressure, 158 because
    /// the slab wore away, none for any other reason; hardest step 6 passes.
    #[test]
    #[ignore]
    fn a_sweep_of_heating_histories_refuses_only_what_the_tables_cannot_hold() {
        let mut asked = 0;
        let mut answered = 0;
        let mut passes = [0u32; NEWTON_PASS_LIMIT as usize + 1];
        let mut too_hot = 0;
        let mut past_surface_table = 0;
        let mut worn_away = 0;
        let mut other = Vec::new();
        for &thickness in &[THICKNESS_MIN_M, 0.05, THICKNESS_MAX_M] {
            for &coefficient in &[0.003, 0.03, 0.3, HEAT_TRANSFER_COEFFICIENT_MAX_KG_PER_M2_S] {
                for &enthalpy in &[0.0, 1.5e6, 2.5e7, FLOW_ENTHALPY_MAX_J_PER_KG] {
                    for &pressure in &[
                        STREAM_PRESSURE_MIN_PA,
                        2000.0,
                        23811.0,
                        STREAM_PRESSURE_MAX_PA,
                    ] {
                        for &heating in &[0.0, 1.0e6, RADIATIVE_HEATING_MAX_W_PER_M2] {
                            for &(end, cools) in &[(3.0, false), (40.0, true), (40.0, false)] {
                                for &wears in &[true, false] {
                                    let mut q = stream(
                                        thickness,
                                        coefficient,
                                        enthalpy,
                                        pressure,
                                        heating,
                                        end,
                                        wears,
                                    );
                                    if cools {
                                        if let SurfaceCondition::GasStream { history, .. } =
                                            &mut q.surface
                                        {
                                            let last = history[2];
                                            history.push(StreamRow {
                                                time_s: end + 0.1,
                                                heat_transfer_coefficient_kg_per_m2_s: 0.0,
                                                flow_enthalpy_j_per_kg: 0.0,
                                                radiative_heating_w_per_m2: 0.0,
                                                ..last
                                            });
                                            history.push(StreamRow {
                                                time_s: end + 40.0,
                                                ..history[3]
                                            });
                                        }
                                    }
                                    asked += 1;
                                    match solve(&q) {
                                        Ok(r) => {
                                            answered += 1;
                                            passes[r.hardest_step_newton_passes as usize] += 1;
                                            assert!(r
                                                .recession_m
                                                .windows(2)
                                                .all(|pair| pair[1] >= pair[0]));
                                            assert!(
                                                r.mass_account_residual_kg_per_m2.abs()
                                                    <= r.mass_account_tolerance_kg_per_m2
                                            );
                                            assert!(
                                                r.energy_account_residual_j_per_m2.abs()
                                                    <= r.energy_account_tolerance_j_per_m2
                                            );
                                            assert!(r
                                                .final_temperature_k
                                                .iter()
                                                .all(|t| t.is_finite()));
                                        }
                                        Err(SlabReject::TemperatureOutOfBounds {
                                            temperature_k,
                                            ..
                                        }) if temperature_k > 3333.0 => too_hot += 1,
                                        Err(SlabReject::FaceBeyondSurfaceTable { .. }) => {
                                            past_surface_table += 1
                                        }
                                        Err(SlabReject::SlabWornAway { .. }) => worn_away += 1,
                                        Err(err) => other.push(format!("{q:?}: {err}")),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        eprintln!(
            "{asked} heating histories: {answered} answered; refused because a temperature passed the top of the material's tables (3333 K): {too_hot}; because the face passed the top of the surface table at its pressure: {past_surface_table}; because the slab wore away: {worn_away}; for any other reason: {}. Answered runs by Newton passes in their hardest step: {passes:?}",
            other.len()
        );
        assert!(other.is_empty(), "{:#?}", &other[..other.len().min(10)]);
        let hardest = passes.iter().rposition(|&count| count > 0).unwrap() as u32;
        assert!(hardest < NEWTON_PASS_LIMIT, "{hardest}");
    }
}

/// Stage three: the surface beyond plain equilibrium, and the PICA block.
#[cfg(test)]
mod pica_tests {
    use super::*;
    use crate::pica;

    fn stream_now(pressure_pa: f64, coefficient: f64, flow_enthalpy_j_per_kg: f64) -> StreamNow {
        StreamNow {
            pressure_pa,
            coefficient,
            flow_enthalpy_j_per_kg,
            radiative_heating_w_per_m2: 0.0,
            surroundings_temperature_k: 300.0,
            present: true,
            face_wears: true,
        }
    }

    /// Wall states computed by the surface work's own Python
    /// (`evidence/research/heatshield_surface_model/analysis`, classes
    /// `SurfaceBC` and `make_modify`, on `bundle_air_tacotgas.npz`), for five
    /// settings of the options: low-pressure rule, wall-reaction limit,
    /// evaporation limit; then wall temperature (K), pressure (Pa),
    /// coefficient (kg/m²/s), flow enthalpy (J/kg), gas flux (kg/m²/s); then
    /// the Python's Ω, B'g, B'c and wall-gas enthalpy (J/kg).
    #[rustfmt::skip]
    const PYTHON_WALL_STATES: [(bool, bool, bool, f64, f64, f64, f64, f64, f64, f64, f64, f64); 60] = [
        (false, false, false, 1570.0, 101325.0, 0.3, 1500000.0, 0.0124, 0.9074654521366369, 0.0455481067763602, 0.16509953585428086, 132199.2464654565),
        (false, false, false, 3160.0, 101325.0, 0.3, 25000000.0, 0.0156, 0.8890282181926595, 0.05849083176828326, 0.20113528730340247, 3333818.743636064),
        (false, false, false, 3350.0, 23811.0, 0.17, 59000000.0, 0.02, 0.819007802769593, 0.1436458338152152, 0.32871438006922205, 7622965.897291201),
        (false, false, false, 2400.0, 5000.0, 0.05, 40000000.0, 0.004, 0.890604984145362, 0.08982658006672917, 0.16550677753939563, 1677952.4951497659),
        (false, false, false, 2062.0, 2300.0, 0.07, 15200000.0, 0.002, 0.9110756424056401, 0.03136010580685665, 0.17000061903790994, 824199.7485730744),
        (false, false, false, 1300.0, 30000.0, 0.02, 2000000.0, 0.0005, 0.913109197511409, 0.02737898166579795, 0.16879302356323572, -232510.26056737406),
        (false, false, false, 900.0, 12000.0, 0.01, 800000.0, 0.0002, 0.9247566784346816, 0.021627310677459165, 0.145401980958158, -1055626.3611189583),
        (false, false, false, 2900.0, 9000.0, 0.1, 30000000.0, 0.01, 0.8743495419211427, 0.1143707351751229, 0.1861934622534464, 3615921.7027374664),
        (false, false, false, 3450.0, 40000.0, 0.2, 50000000.0, 0.03, 0.7831298923090103, 0.19153910648925423, 0.4092549921886949, 9241534.12476755),
        (false, false, false, 1800.0, 500.0, 0.004, 60000000.0, 0.0, 0.9215923636839467, 0.0, 0.17485078219563122, 385497.16738565604),
        (false, false, false, 700.0, 20000.0, 0.005, 1000000.0, 2e-05, 0.9564005579489296, 0.00418234803815252, 0.08835650741480204, -2180190.8791293437),
        (false, false, false, 3050.0, 101325.0, 0.4, 26000000.0, 0.3, 0.632372033869405, 1.1860107008678211, 0.1674316227766736, 7406391.533318367),
        (true, false, false, 1570.0, 101325.0, 0.3, 1500000.0, 0.0124, 0.9074654521366369, 0.0455481067763602, 0.16509953585428086, 132199.2464654565),
        (true, false, false, 3160.0, 101325.0, 0.3, 25000000.0, 0.0156, 0.8890282181926595, 0.05849083176828326, 0.20113528730340247, 3333818.743636064),
        (true, false, false, 3350.0, 23811.0, 0.17, 59000000.0, 0.02, 0.819007802769593, 0.1436458338152152, 0.32871438006922205, 7622965.897291201),
        (true, false, false, 2400.0, 5000.0, 0.05, 40000000.0, 0.004, 0.887006981288207, 0.09019094734944141, 0.17496724336351077, 1628709.912309832),
        (true, false, false, 2062.0, 2300.0, 0.07, 15200000.0, 0.002, 0.9091576561144598, 0.031426264025015284, 0.17485626236888527, 846605.1915492825),
        (true, false, false, 1300.0, 30000.0, 0.02, 2000000.0, 0.0005, 0.913109197511409, 0.02737898166579795, 0.16879302356323572, -232510.26056737406),
        (true, false, false, 900.0, 12000.0, 0.01, 800000.0, 0.0002, 0.9243247479539769, 0.021637416967979977, 0.14645535828871092, -1056246.2573988314),
        (true, false, false, 2900.0, 9000.0, 0.1, 30000000.0, 0.01, 0.8758868699950205, 0.11416999538962655, 0.18202249608159388, 3285860.180526174),
        (true, false, false, 3450.0, 40000.0, 0.2, 50000000.0, 0.03, 0.7831298923090103, 0.19153910648925423, 0.4092549921886949, 9241534.12476755),
        (true, false, false, 1800.0, 500.0, 0.004, 60000000.0, 0.0, 0.9215923636839467, 0.0, 0.17485078219563122, 385497.16738565604),
        (true, false, false, 700.0, 20000.0, 0.005, 1000000.0, 2e-05, 0.9564005579489296, 0.00418234803815252, 0.08835650741480204, -2180190.8791293437),
        (true, false, false, 3050.0, 101325.0, 0.4, 26000000.0, 0.3, 0.632372033869405, 1.1860107008678211, 0.1674316227766736, 7406391.533318367),
        (true, true, false, 1570.0, 101325.0, 0.3, 1500000.0, 0.0124, 0.9504014207766976, 0.043490395099714786, 0.0626683815169771, 149863.02908466686),
        (true, true, false, 3160.0, 101325.0, 0.3, 25000000.0, 0.0156, 0.8898294659862257, 0.05843816364208011, 0.19900406144497393, 3338038.0256101806),
        (true, true, false, 3350.0, 23811.0, 0.17, 59000000.0, 0.02, 0.8207132773996155, 0.14334733209038503, 0.3232669365041617, 7640383.851699755),
        (true, true, false, 2400.0, 5000.0, 0.05, 40000000.0, 0.004, 0.8892753605905683, 0.08996088668479439, 0.16899106764879676, 1603274.8196369722),
        (true, true, false, 2062.0, 2300.0, 0.07, 15200000.0, 0.002, 0.9161613810615163, 0.03118602152995252, 0.15725494328509038, 881392.5179645064),
        (true, true, false, 1300.0, 30000.0, 0.02, 2000000.0, 0.0005, 0.9428734619505756, 0.02651469257015299, 0.09705989175923621, -23959.31121283656),
        (true, true, false, 900.0, 12000.0, 0.01, 800000.0, 0.0002, 0.9894419673252646, 0.02021341386791716, 0.0012036150371559319, -2006.9779407587353),
        (true, true, false, 2900.0, 9000.0, 0.1, 30000000.0, 0.01, 0.8784109529418584, 0.1138419319221842, 0.17521612487540564, 3297356.1311156168),
        (true, true, false, 3450.0, 40000.0, 0.2, 50000000.0, 0.03, 0.7843082000564587, 0.19125134713965827, 0.4050833125267944, 9247072.716410482),
        (true, true, false, 1800.0, 500.0, 0.004, 60000000.0, 0.0, 0.9236418663781139, 0.0, 0.16977681396418046, 419042.1869356842),
        (true, true, false, 700.0, 20000.0, 0.005, 1000000.0, 2e-05, 0.9979813417321396, 0.004008090963950335, 4.01178374909723e-05, 290249.0749666662),
        (true, true, false, 3050.0, 101325.0, 0.4, 26000000.0, 0.3, 0.6327204866418407, 1.1853575392515274, 0.16581311968323928, 7400018.526974413),
        (false, true, true, 1570.0, 101325.0, 0.3, 1500000.0, 0.0124, 0.9504014207766976, 0.043490395099714786, 0.06266838151697707, 149863.02908466588),
        (false, true, true, 3160.0, 101325.0, 0.3, 25000000.0, 0.0156, 0.8901130121134312, 0.058419548123788426, 0.19825107173031933, 3322158.511157434),
        (false, true, true, 3350.0, 23811.0, 0.17, 59000000.0, 0.02, 0.836619089149444, 0.1406220109698626, 0.2738587011019595, 6834763.969842729),
        (false, true, true, 2400.0, 5000.0, 0.05, 40000000.0, 0.004, 0.892770047509453, 0.08960874094691319, 0.15986253187788, 1651136.040673163),
        (false, true, true, 2062.0, 2300.0, 0.07, 15200000.0, 0.002, 0.9179174172769367, 0.031126360610964507, 0.15289725391860862, 861601.8081728375),
        (false, true, true, 1300.0, 30000.0, 0.02, 2000000.0, 0.0005, 0.9428734619505756, 0.02651469257015299, 0.09705989175923621, -23959.31121283656),
        (false, true, true, 900.0, 12000.0, 0.01, 800000.0, 0.0002, 0.9894459480405416, 0.020213332545802976, 0.0011955066135577116, -1990.8411142266982),
        (false, true, true, 2900.0, 9000.0, 0.1, 30000000.0, 0.01, 0.8774345682631298, 0.11396861204467408, 0.1778428818267956, 3587220.050591771),
        (false, true, true, 3450.0, 40000.0, 0.2, 50000000.0, 0.03, 0.8040734946220909, 0.18655011121286375, 0.33747656750320065, 8289368.087884316),
        (false, true, true, 1800.0, 500.0, 0.004, 60000000.0, 0.0, 0.9236418663859176, 0.0, 0.16977681394491703, 419042.18627084297),
        (false, true, true, 700.0, 20000.0, 0.005, 1000000.0, 2e-05, 0.9979813417321396, 0.004008090963950335, 4.01178374909723e-05, 290249.0749666662),
        (false, true, true, 3050.0, 101325.0, 0.4, 26000000.0, 0.3, 0.6330588053196395, 1.1847240616238377, 0.16424399756080457, 7383455.927572016),
        (true, true, true, 1570.0, 101325.0, 0.3, 1500000.0, 0.0124, 0.9504014207766976, 0.043490395099714786, 0.06266838151697707, 149863.02908466588),
        (true, true, true, 3160.0, 101325.0, 0.3, 25000000.0, 0.0156, 0.8901130121134312, 0.058419548123788426, 0.19825107173031933, 3322158.511157434),
        (true, true, true, 3350.0, 23811.0, 0.17, 59000000.0, 0.02, 0.836619089149444, 0.1406220109698626, 0.2738587011019595, 6834763.969842729),
        (true, true, true, 2400.0, 5000.0, 0.05, 40000000.0, 0.004, 0.8892759017100993, 0.08996083194408579, 0.16898964684049458, 1603241.4478109803),
        (true, true, true, 2062.0, 2300.0, 0.07, 15200000.0, 0.002, 0.9161613838122952, 0.031186021436316345, 0.1572549364416343, 881392.3259371561),
        (true, true, true, 1300.0, 30000.0, 0.02, 2000000.0, 0.0005, 0.9428734619505756, 0.02651469257015299, 0.09705989175923621, -23959.31121283656),
        (true, true, true, 900.0, 12000.0, 0.01, 800000.0, 0.0002, 0.9894419673252646, 0.02021341386791716, 0.0012036150371559319, -2006.9779407587353),
        (true, true, true, 2900.0, 9000.0, 0.1, 30000000.0, 0.01, 0.8788230035669264, 0.11378855525921504, 0.1741099030924437, 3274219.0690647173),
        (true, true, true, 3450.0, 40000.0, 0.2, 50000000.0, 0.03, 0.8040734946220909, 0.18655011121286375, 0.33747656750320065, 8289368.087884316),
        (true, true, true, 1800.0, 500.0, 0.004, 60000000.0, 0.0, 0.9236418663859176, 0.0, 0.16977681394491703, 419042.18627084297),
        (true, true, true, 700.0, 20000.0, 0.005, 1000000.0, 2e-05, 0.9979813417321396, 0.004008090963950335, 4.01178374909723e-05, 290249.0749666662),
        (true, true, true, 3050.0, 101325.0, 0.4, 26000000.0, 0.3, 0.6330588053196395, 1.1847240616238377, 0.16424399756080457, 7383455.927572016),
    ];

    #[test]
    fn the_rust_surface_gives_the_python_surfaces_wall_states() {
        let (_, bundle) = frozen_pica();
        let mut worst = [0.0f64; 4];
        for &(low, oxidation, vapour, t, p, c, h, flux, omega, bg, bc, hw) in &PYTHON_WALL_STATES {
            let options = SurfaceOptions {
                low_pressure_rule: low,
                oxidation_limited_by_wall_reaction: oxidation,
                oxygen_atom_reaction_probability: None,
                carbon_vapour_limited_by_evaporation: vapour,
                solid_char_loss_fraction: 0.0,
                stream_heating_supplied_separately: false,
                gas_beside_unused_oxygen: GasBesideUnusedOxygen::Burns,
                air_table_computed_afresh: false,
            };
            let state = bundle.wall_values(&options, &stream_now(p, c, h), flux, t, 0.0);
            let off = [
                (state.correction / omega - 1.0).abs(),
                (state.blowing - bg).abs() / bg.max(1.0e-3),
                (state.char_rate - bc).abs() / bc.max(1.0e-3),
                (state.wall_enthalpy - hw).abs() / hw.abs().max(1.0e5),
            ];
            for q in 0..4 {
                worst[q] = worst[q].max(off[q]);
            }
            // The Python stops its iteration at 1e-9 in Ω.
            assert!(
                off.iter().all(|d| *d < 2.0e-7),
                "{low} {oxidation} {vapour} {t} {p}: {off:?} {state:?}"
            );
        }
        eprintln!(
            "60 wall states against the Python: largest relative differences in Ω, B'g, B'c, wall-gas enthalpy {worst:?}"
        );
    }

    #[test]
    fn the_slopes_of_the_bundle_surface_match_wider_differences() {
        let (_, bundle) = frozen_pica();
        let options = pica::SURFACE_OPTIONS;
        for &(t, p, c, h, flux) in &[
            (3160.0, 23811.0, 0.17, 5.9e7, 0.02),
            (1400.0, 30000.0, 0.02, 2.0e6, 0.0005),
            (2412.0, 5000.0, 0.05, 4.0e7, 0.004),
        ] {
            let stream = stream_now(p, c, h);
            let here = bundle.wall_state(&options, &stream, flux, t, (1.0e6, 3000.0));
            let wide = |dt: f64, dm: f64| {
                let up =
                    bundle.wall_values(&options, &stream, flux + dm, t + dt, 1.0e6 + 3000.0 * dt);
                let down =
                    bundle.wall_values(&options, &stream, flux - dm, t - dt, 1.0e6 - 3000.0 * dt);
                let span = 2.0 * (dt + dm);
                (
                    (up.correction - down.correction) / span,
                    (up.char_rate - down.char_rate) / span,
                    (up.wall_enthalpy - down.wall_enthalpy) / span,
                )
            };
            let by_t = wide(0.02, 0.0);
            let by_m = wide(0.0, 1.0e-5 * flux);
            assert!(
                (by_t.0 - here.correction_temperature).abs()
                    < 1.0e-3 * here.correction_temperature.abs() + 1.0e-9,
                "{t}"
            );
            assert!(
                (by_t.1 - here.char_rate_temperature).abs()
                    < 1.0e-3 * here.char_rate_temperature.abs() + 1.0e-9,
                "{t}"
            );
            assert!(
                (by_t.2 - here.wall_enthalpy_temperature).abs()
                    < 1.0e-3 * here.wall_enthalpy_temperature.abs() + 1.0e-3,
                "{t}"
            );
            assert!(
                (by_m.0 - here.correction_flux).abs()
                    < 1.0e-3 * here.correction_flux.abs() + 1.0e-6,
                "{t}"
            );
            assert!(
                (by_m.2 - here.wall_enthalpy_flux).abs()
                    < 1.0e-3 * here.wall_enthalpy_flux.abs() + 1.0,
                "{t}"
            );
        }
    }

    #[test]
    fn the_pica_block_is_what_its_file_says() {
        let m = pica::material(&pica::FROZEN);
        assert!((m.solid_density(m.virgin_remaining()) - 274.0).abs() < 1.0e-10);
        assert!((m.solid_density([0.0, 0.0]) - 274.0 * 220.0 / 280.0).abs() < 1.0e-10);
        let nodes = SolidEnthalpyNodes::of(&m);
        let room = solid_point(&nodes, 298.0, None);
        assert!((room.virgin_conductivity - 0.174).abs() < 1.0e-12);
        assert!((room.char_conductivity - 0.745 * 0.40249965408).abs() < 1.0e-12);
        assert_eq!(pica::SURFACE_OPTIONS.solid_char_loss_fraction, 0.05);
        assert_eq!(
            pica::SURFACE_OPTIONS.oxygen_atom_reaction_probability,
            Some(0.9)
        );
        assert!(
            pica::SURFACE_OPTIONS.low_pressure_rule
                && pica::SURFACE_OPTIONS.oxidation_limited_by_wall_reaction
        );
        assert!(!pica::SURFACE_OPTIONS.carbon_vapour_limited_by_evaporation);
        assert_eq!(room.virgin_specific_heat, 983.8980000000001);
        assert!((room.virgin_enthalpy - (-2.0e6 * 114.0 / 274.0)).abs() < 1.0e-6);
        // Above 3333 K: specific heat held, conductivity continued along its line.
        let top = solid_point(&nodes, 3975.0, None);
        let end = solid_point(&nodes, 3333.333333333333, None);
        let before = solid_point(&nodes, 2777.777777777778, None);
        assert_eq!(top.char_specific_heat, end.char_specific_heat);
        let slope = (end.char_conductivity - before.char_conductivity)
            / (3333.333333333333 - 2777.777777777778);
        assert!(
            (top.char_conductivity - end.char_conductivity - slope * (3975.0 - 3333.333333333333))
                .abs()
                < 1.0e-12
        );
        let held = pica::material(&pica::Departures {
            conductivity_held_above_3333_k: true,
            ..pica::FROZEN
        });
        let nodes_held = SolidEnthalpyNodes::of(&held);
        assert_eq!(
            solid_point(&nodes_held, 3975.0, None).char_conductivity,
            end.char_conductivity
        );
        // The bundle: 23 pressures from 0.00001 atm to 1 atm.
        let (_, bundle) = frozen_pica();
        let (low, high) = bundle.pressure_range_pa();
        assert!((low - 1.01325).abs() < 1.0e-9 && high == 101325.0);
        assert_eq!(
            (
                bundle.pressure_pa.len(),
                bundle.blowing.len(),
                bundle.temperature_k.len()
            ),
            (23, 26, 243)
        );
        assert!(SurfaceBundle::from_bytes(b"not a bundle").is_err());
        assert!(SurfaceBundle::from_bytes(&pica::SURFACE_BUNDLE[..1000]).is_err());
    }

    #[test]
    fn the_study_switches_do_what_they_say() {
        let mut q = arc_jet_like(6.0);
        if let SurfaceCondition::GasStream { history, .. } = &mut q.surface {
            for row in history.iter_mut().skip(1) {
                row.radiative_heating_w_per_m2 = 3.0e6;
            }
        }
        // Heating supplied separately: the stream's own heating term is gone,
        // the oxygen it brings is not, and the accounts still close.
        let separately = SurfaceOptions {
            stream_heating_supplied_separately: true,
            gas_beside_unused_oxygen: GasBesideUnusedOxygen::Burns,
            air_table_computed_afresh: false,
            ..pica::SURFACE_OPTIONS
        };
        let r = solve_pica_study(&pica::FROZEN, &separately, None, &q)
            .expect("heating supplied separately");
        assert_eq!(r.heat_from_stream_j_per_m2, 0.0);
        assert!(
            r.heat_absorbed_from_radiation_j_per_m2 > 1.0e7
                && *r.recession_m.last().unwrap() > 1.0e-4
        );
        assert!(r.face_balance_residual_j_per_m2.abs() <= r.face_balance_tolerance_j_per_m2);
        assert!(r.energy_account_residual_j_per_m2.abs() <= r.energy_account_tolerance_j_per_m2);
        let with_stream = solve_pica_study(&pica::FROZEN, &pica::SURFACE_OPTIONS, None, &q)
            .expect("with the stream's heating");
        assert!(with_stream.heat_from_stream_j_per_m2 > 0.0);
        assert!(with_stream.surface_temperature_k.last() > r.surface_temperature_k.last());
        // The bracket on decomposition: nothing decomposes below the stated temperature.
        let late = pica::material(&pica::Departures {
            no_decomposition_below_k: 700.0,
            ..pica::FROZEN
        });
        assert_eq!(late.resin_parts[0].threshold_temperature_k, 700.0);
        assert_eq!(late.resin_parts[1].threshold_temperature_k, 700.0);
        let own = pica::material(&pica::FROZEN);
        assert_eq!(
            own.resin_parts[0].threshold_temperature_k,
            333.3333333333333
        );
        // Halved cells and steps give nearly the same answer, and not the same bits.
        let plain = arc_jet_like(6.0);
        let base =
            solve_pica_study(&pica::FROZEN, &pica::SURFACE_OPTIONS, None, &plain).expect("base");
        let finer = solve_pica_study_refined(
            &pica::FROZEN,
            &pica::SURFACE_OPTIONS,
            None,
            true,
            true,
            &plain,
        )
        .expect("finer");
        let (a, b) = (
            *base.recession_m.last().unwrap(),
            *finer.recession_m.last().unwrap(),
        );
        assert!(a != b && (a / b - 1.0).abs() < 1.0e-3, "{a} {b}");
        assert!(finer.cells > base.cells && finer.time_steps > base.time_steps);
    }

    #[test]
    fn version_three_conducts_less_at_low_pressure_by_what_was_measured() {
        let m = pica::material(&pica::VERSION_3);
        let nodes = SolidEnthalpyNodes::of(&m);
        let at = |t: f64, p: f64| {
            nodes.pore_pressure_pa.set(p);
            solid_point(&nodes, t, None)
        };
        // At room temperature, from 1 atm to 0.001 atm: virgin falls by 0.042
        // W/m/K and char by 0.024, as Monk and others 2022 tabulate.
        let (full, thin) = (at(298.0, 101325.0), at(298.0, 101.325));
        assert!(
            (full.virgin_conductivity - 0.174).abs() < 2.0e-4,
            "{}",
            full.virgin_conductivity
        );
        assert!((full.virgin_conductivity - thin.virgin_conductivity - 0.042).abs() < 1.0e-3);
        assert!((full.char_conductivity - thin.char_conductivity - 0.024).abs() < 1.0e-3);
        // At the pressure the char curve was fitted at, it is the fitted curve.
        let fitted = at(1500.0, pica::CHAR_REFERENCE_PRESSURE_PA);
        let plain = solid_point(
            &SolidEnthalpyNodes::of(&pica::material(&pica::FROZEN)),
            1500.0,
            None,
        );
        assert_eq!(fitted.char_conductivity, plain.char_conductivity);
        // Lower pressure, lower conductivity, never below zero; slopes match differences.
        for t in [300.0, 900.0, 2000.0, 3300.0] {
            let mut before = f64::INFINITY;
            for p in [101325.0, 20000.0, 2000.0, 100.0, 2.0] {
                let here = at(t, p);
                assert!(
                    here.char_conductivity < before
                        && here.char_conductivity > 0.0
                        && here.virgin_conductivity > 0.0,
                    "{t} {p}"
                );
                before = here.char_conductivity;
                let numeric =
                    (at(t + 0.01, p).char_conductivity - at(t - 0.01, p).char_conductivity) / 0.02;
                assert!(
                    (numeric - here.char_conductivity_slope).abs() < 1.0e-7,
                    "{t} {p}"
                );
                let numeric = (at(t + 0.01, p).virgin_conductivity
                    - at(t - 0.01, p).virgin_conductivity)
                    / 0.02;
                assert!(
                    (numeric - here.virgin_conductivity_slope).abs() < 1.0e-7,
                    "{t} {p}"
                );
            }
        }
        // Picked by name; version one is still version one.
        let q = arc_jet_like(5.0);
        let three = solve_material(MATERIAL_PICA_VERSION_3, &q).expect("version 3");
        let one = solve_material(MATERIAL_PICA, &q).expect("version 1");
        assert!(three.final_temperature_k != one.final_temperature_k);
        assert!(
            three.energy_account_residual_j_per_m2.abs() <= three.energy_account_tolerance_j_per_m2
        );
    }

    fn arc_jet_like(end_s: f64) -> SlabQuestion {
        let row = |time_s: f64, on: f64| StreamRow {
            time_s,
            pressure_pa: 20_000.0,
            heat_transfer_coefficient_kg_per_m2_s: 0.17 * on,
            flow_enthalpy_j_per_kg: 2.5e7,
            radiative_heating_w_per_m2: 0.0,
        };
        SlabQuestion {
            thickness_m: 0.03,
            start_temperature_k: 300.0,
            surface: SurfaceCondition::GasStream {
                history: vec![row(0.0, 0.0), row(0.5, 1.0), row(end_s, 1.0)],
                surroundings_temperature_k: 300.0,
                face_wears: true,
                stream_gas: StreamGas::AIR,
            },
            continue_after_s: 0.0,
            thermocouple_depths_m: vec![0.005, 0.03],
        }
    }

    #[test]
    fn the_materials_are_picked_by_name_and_each_option_moves_the_answer() {
        let q = arc_jet_like(10.0);
        let frozen = solve_material(MATERIAL_PICA, &q).expect("the frozen PICA model");
        let again = solve_pica_study(&pica::FROZEN, &pica::SURFACE_OPTIONS, None, &q)
            .expect("the same through the study door");
        assert_eq!(frozen, again);
        let tacot = solve_material(MATERIAL_TACOT, &q).expect("TACOT");
        assert_eq!(tacot, solve(&q).unwrap());
        assert!(frozen.recession_m != tacot.recession_m);
        assert_eq!(
            solve_material("PICA", &q).expect_err("no such name"),
            SlabReject::UnknownMaterial {
                name: "PICA".to_string()
            }
        );
        let err = solve_pica_study(&pica::FROZEN, &pica::SURFACE_OPTIONS, Some(b"ZTPSURF1"), &q)
            .expect_err("cut short");
        assert!(
            matches!(err, SlabReject::SurfaceBundleUnreadable { .. }),
            "{err:?}"
        );
        // The accounts close with the options on.
        let last = frozen.recession_m.len() - 1;
        assert!(
            frozen.energy_account_residual_j_per_m2.abs()
                <= frozen.energy_account_tolerance_j_per_m2
        );
        assert!(
            frozen.mass_account_residual_kg_per_m2.abs() <= frozen.mass_account_tolerance_kg_per_m2
        );
        assert!(
            frozen.face_balance_residual_j_per_m2.abs() <= frozen.face_balance_tolerance_j_per_m2
        );
        assert!(frozen.recession_m[last] > 5.0e-4 && frozen.surface_temperature_k[last] > 2000.0);
        let plain = SurfaceOptions {
            low_pressure_rule: false,
            oxidation_limited_by_wall_reaction: false,
            oxygen_atom_reaction_probability: None,
            carbon_vapour_limited_by_evaporation: false,
            solid_char_loss_fraction: 0.0,
            stream_heating_supplied_separately: false,
            gas_beside_unused_oxygen: GasBesideUnusedOxygen::Burns,
            air_table_computed_afresh: false,
        };
        let with = |options: SurfaceOptions, question: &SlabQuestion| {
            let r = solve_pica_study(&pica::FROZEN, &options, None, question).expect("study");
            (r.recession_m[r.recession_m.len() - 1], r)
        };
        let base = with(plain, &q).0;
        // Char lost as solid: five percent more wear, and the mass account still closes.
        let (lossy, r) = with(
            SurfaceOptions {
                solid_char_loss_fraction: 0.05,
                ..plain
            },
            &q,
        );
        assert!(lossy > 1.03 * base && lossy < 1.06 * base, "{base} {lossy}");
        assert!(r.mass_account_residual_kg_per_m2.abs() <= r.mass_account_tolerance_kg_per_m2);
        assert!(r.energy_account_residual_j_per_m2.abs() <= r.energy_account_tolerance_j_per_m2);
        // The low-pressure rule acts below 13 kPa and not at 20 kPa.
        assert_eq!(
            with(
                SurfaceOptions {
                    low_pressure_rule: true,
                    ..plain
                },
                &q
            )
            .0,
            base
        );
        let mut thin_air = q.clone();
        if let SurfaceCondition::GasStream { history, .. } = &mut thin_air.surface {
            for row in history.iter_mut() {
                row.pressure_pa = 5000.0;
            }
        }
        let low = with(plain, &thin_air).0;
        assert!(
            (with(
                SurfaceOptions {
                    low_pressure_rule: true,
                    ..plain
                },
                &thin_air
            )
            .0 / low
                - 1.0)
                .abs()
                > 0.01
        );
        // The wall-reaction limit lowers wear; the evaporation limit acts only on a boiling face.
        assert!(
            with(
                SurfaceOptions {
                    oxidation_limited_by_wall_reaction: true,
                    ..plain
                },
                &q
            )
            .0 < base
        );
        let slow = with(
            SurfaceOptions {
                carbon_vapour_limited_by_evaporation: true,
                ..plain
            },
            &q,
        )
        .0;
        assert!(slow <= base && slow > 0.9 * base);
    }
}

/// Stage six: the served entry.
#[cfg(test)]
mod served_tests {
    use super::*;

    fn stream_rows(rows: &[(f64, f64, f64, f64, f64)]) -> Vec<StreamRow> {
        rows.iter()
            .map(|&(time_s, pressure_pa, c, h, q)| StreamRow {
                time_s,
                pressure_pa,
                heat_transfer_coefficient_kg_per_m2_s: c,
                flow_enthalpy_j_per_kg: h,
                radiative_heating_w_per_m2: q,
            })
            .collect()
    }

    fn under_stream(thickness_m: f64, history: Vec<StreamRow>, depths: &[f64]) -> SlabQuestion {
        SlabQuestion {
            thickness_m,
            start_temperature_k: 300.0,
            surface: SurfaceCondition::GasStream {
                history,
                surroundings_temperature_k: 300.0,
                face_wears: true,
                stream_gas: StreamGas::AIR,
            },
            continue_after_s: 0.0,
            thermocouple_depths_m: depths.to_vec(),
        }
    }

    /// A flight in small: thin air and high enthalpy at first, a pulse with
    /// radiative heating, then the stream off.
    fn flight_shaped(seconds: usize, cooling_s: f64) -> Vec<StreamRow> {
        let mut rows = Vec::new();
        for second in 0..=seconds {
            let t = second as f64;
            let pulse = (-((t - 0.5 * seconds as f64) / (0.2 * seconds as f64)).powi(2)).exp();
            rows.push((
                t,
                2.0 + 25_000.0 * pulse,
                6.0e-5 + 0.16 * pulse,
                7.5e7 * (1.0 - t / (1.5 * seconds as f64)),
                6.0e5 * pulse,
            ));
        }
        let last = rows[rows.len() - 1];
        rows.push((last.0 + 1.0, last.1, 0.0, last.3, 0.0));
        rows.push((last.0 + 1.0 + cooling_s, last.1, 0.0, last.3, 0.0));
        stream_rows(&rows)
    }

    #[test]
    fn the_short_answer_always_comes_back_and_the_rest_only_when_asked_for() {
        let q = under_stream(
            0.03,
            stream_rows(&[
                (0.0, 101325.0, 0.0, 0.0, 0.0),
                (0.1, 101325.0, 0.3, 1.5e6, 0.0),
                (15.0, 101325.0, 0.3, 1.5e6, 0.0),
            ]),
            &[0.001, 0.02],
        );
        let full = solve_material(MATERIAL_TACOT, &q).expect("the full result");
        let short = answer(MATERIAL_TACOT, &q, &Wanted::default()).expect("the short answer");
        assert!(
            short.face_history.is_none()
                && short.thermocouple_histories.is_none()
                && short.final_profile.is_none()
        );
        let s = &short.summary;
        assert_eq!(s.wear_m, *full.recession_m.last().unwrap());
        assert_eq!(
            s.peak_face_temperature_k,
            full.surface_temperature_k
                .iter()
                .cloned()
                .fold(0.0, f64::max)
        );
        assert_eq!(
            (s.duration_s, s.cells, s.cells_at_end, s.time_steps),
            (15.0, full.cells, full.cells_at_end, full.time_steps)
        );
        assert!(!s.material_compared_with_measurement && s.material == MATERIAL_TACOT);
        assert!(
            s.depth_half_way_to_virgin_m > 0.0
                && s.depth_within_2_percent_of_virgin_m > s.depth_half_way_to_virgin_m
        );
        assert!(s.energy_account_residual_j_per_m2.abs() <= s.energy_account_tolerance_j_per_m2);
        assert!(s.mass_account_residual_kg_per_m2.abs() <= s.mass_account_tolerance_kg_per_m2);
        assert!(
            (s.solid_mass_lost_kg_per_m2
                - s.gas_mass_out_kg_per_m2
                - s.char_mass_removed_kg_per_m2)
                .abs()
                < 1.0e-9
        );
        // Everything asked for.
        let all = Wanted {
            face_history: true,
            thermocouple_histories: true,
            final_profile: true,
        };
        let long = answer(MATERIAL_TACOT, &q, &all).expect("everything");
        assert_eq!(long.summary, short.summary);
        let face = long.face_history.expect("face history");
        assert_eq!(face.time_s, full.report_time_s);
        assert_eq!(face.recession_m, full.recession_m);
        let profile = long.final_profile.expect("profile");
        assert!(profile.depth_m.len() <= PROFILE_POINTS_MAX && profile.depth_m.len() >= 150);
        assert_eq!(profile.depth_m[0], full.final_depth_m[0]);
        assert_eq!(profile.depth_m.last(), full.final_depth_m.last());
        // The thermocouple at 1 mm is passed by the face: its record ends
        // there, says when, and never reads zero.
        let histories = long.thermocouple_histories.expect("thermocouples");
        let (shallow, deep) = (&histories[0], &histories[1]);
        assert!(
            shallow.passed_by_face
                && shallow.covered_until_s > 4.0
                && shallow.covered_until_s < 7.0,
            "{}",
            shallow.covered_until_s
        );
        assert!(shallow.temperature_k.iter().all(|&t| t >= 300.0));
        assert!(*shallow.time_s.last().unwrap() <= shallow.covered_until_s);
        assert!(shallow.time_s.len() == shallow.temperature_k.len() && shallow.time_s.len() >= 5);
        assert!(!deep.passed_by_face && deep.covered_until_s == 15.0 && deep.time_s.len() == 16);
        // Thinning keeps the ends and the order.
        let at = thinned(601, HISTORY_POINTS_MAX);
        assert_eq!((at.len(), at[0], at[120]), (121, 0, 600));
        assert!(at.windows(2).all(|pair| pair[1] > pair[0]));
        assert_eq!(thinned(7, HISTORY_POINTS_MAX), vec![0, 1, 2, 3, 4, 5, 6]);
    }

    /// The first development arc-jet test as the recipe makes it (row 1 of
    /// the 36): 107 W/cm², 2.3 kPa, 15.2 MJ/kg, 55 s, argon 0.276.
    fn arc_jet_row_1(coefficient_scale: f64, end_s: f64) -> SlabQuestion {
        let c = 0.07040224291374243 * coefficient_scale;
        SlabQuestion {
            thickness_m: 0.06,
            start_temperature_k: 300.0,
            surface: SurfaceCondition::GasStream {
                history: stream_rows(&[
                    (0.0, 2300.0, 0.0, 1.52e7, 0.0),
                    (0.5, 2300.0, c, 1.52e7, 0.0),
                    (end_s, 2300.0, c, 1.52e7, 0.0),
                ]),
                surroundings_temperature_k: 300.0,
                face_wears: true,
                stream_gas: StreamGas {
                    oxygen_mass_fraction: 0.16753359999999998,
                    argon_mass_fraction: 0.276,
                },
            },
            continue_after_s: 0.0,
            thermocouple_depths_m: vec![],
        }
    }

    fn with_gas(mut q: SlabQuestion, gas: StreamGas) -> SlabQuestion {
        if let SurfaceCondition::GasStream { stream_gas, .. } = &mut q.surface {
            *stream_gas = gas;
        }
        q
    }

    /// The slab, start and surroundings of the Stardust cores.
    fn on_stardust_slab(mut q: SlabQuestion) -> SlabQuestion {
        q.thickness_m = STARDUST_THICKNESS_M;
        q.start_temperature_k = STARDUST_START_TEMPERATURE_K;
        if let SurfaceCondition::GasStream { surroundings_temperature_k, .. } = &mut q.surface {
            *surroundings_temperature_k = STARDUST_SURROUNDINGS_TEMPERATURE_K;
        }
        q
    }

    /// A thermocouple block of Tran and others 1997 as the development runs
    /// asked it (stardust_heatshield_model_run/analysis/drive.py tc_question):
    /// thickness, cold-wall heating, pressure, exposure, history end and
    /// thermocouple depths.
    fn thermocouple_block(block: &str) -> SlabQuestion {
        let (thickness_m, heating_w_per_cm2, pressure_pa, exposure_s, end_s, depths): (f64, f64, f64, f64, f64, &[f64]) =
            match block {
                "9B" => (0.0274, 400.0, 20265.0, 29.0, 125.0, &[0.0052, 0.0102, 0.0176]),
                "7B" => (0.0274, 580.0, 45596.25, 15.0, 135.0, &[0.0052, 0.0102, 0.0176]),
                "3B" => (0.0325, 580.0, 45596.25, 40.0, 152.0, &[0.0084, 0.013300000000000001, 0.0207]),
                "6A" => (0.022400000000000003, 580.0, 45596.25, 20.0, 115.0, &[0.0052, 0.0143]),
                _ => unreachable!(),
            };
        // The recipe's cold-wall enthalpy of air, from its surface table.
        let c = heating_w_per_cm2 * 1.0e4 / (29.5e6 - 1871.4044377954367);
        let h = 29.5e6;
        SlabQuestion {
            thickness_m,
            start_temperature_k: 30.0 + 273.15,
            surface: SurfaceCondition::GasStream {
                history: stream_rows(&[
                    (0.0, pressure_pa, 0.0, h, 0.0),
                    (0.5, pressure_pa, c, h, 0.0),
                    (exposure_s, pressure_pa, c, h, 0.0),
                    (exposure_s + 0.5, pressure_pa, 0.0, h, 0.0),
                    (end_s, pressure_pa, 0.0, h, 0.0),
                ]),
                surroundings_temperature_k: 300.0,
                face_wears: true,
                stream_gas: StreamGas::AIR,
            },
            continue_after_s: 0.0,
            thermocouple_depths_m: depths.to_vec(),
        }
    }

    fn named(material: &str, q: &SlabQuestion) -> &'static str {
        match check_question(material, q) {
            Ok(()) => "answered",
            Err(reject) => reject.name(),
        }
    }

    #[test]
    fn a_pica_question_outside_what_was_compared_is_refused_by_name() {
        // The recipe's question for the lowest heating compared is inside the
        // arc-jet envelope; a thousandth less heating is not, and its slab is
        // not the Stardust cores' or the blocks', so it is inside none.
        assert_eq!(named(MATERIAL_PICA_SERVED, &arc_jet_row_1(1.0, 55.0)), "answered");
        let low = arc_jet_row_1(0.999, 55.0);
        assert_eq!(named(MATERIAL_PICA_SERVED, &low), "QuestionNotCompared");
        let sentence = check_question(MATERIAL_PICA_SERVED, &low).unwrap_err().to_string();
        assert!(sentence.contains("below 1070000 W/m2") && sentence.contains("not the 0.0582 m compared"), "{sentence}");
        // 10 W/cm² in the arc-jet gas: refused.
        assert_eq!(named(MATERIAL_PICA_SERVED, &arc_jet_row_1(0.0934579, 10.0)), "QuestionNotCompared");
        // The same in air on the Stardust slab lies inside the Stardust
        // envelope, whose tails run below 107 W/cm²; on the arc-jet slab it
        // lies in none, since each envelope has its own slab.
        assert_eq!(
            named(MATERIAL_PICA_SERVED, &on_stardust_slab(with_gas(arc_jet_row_1(0.0934579, 10.0), StreamGas::AIR))),
            "answered"
        );
        let mixed = with_gas(arc_jet_row_1(0.0934579, 10.0), StreamGas::AIR);
        assert_eq!(named(MATERIAL_PICA_SERVED, &mixed), "QuestionNotCompared");
        let sentence = check_question(MATERIAL_PICA_SERVED, &mixed).unwrap_err().to_string();
        assert!(sentence.contains("thickness 0.06 m is not the 0.0582 m compared"), "{sentence}");
        // Below the lowest heating only in the swing-in: answered.
        let mut swing = arc_jet_row_1(1.0, 55.0);
        if let SurfaceCondition::GasStream { history, .. } = &mut swing.surface {
            history.insert(1, stream_rows(&[(0.25, 2300.0, 0.01, 1.52e7, 0.0)])[0]);
        }
        assert_eq!(named(MATERIAL_PICA_SERVED, &swing), "answered");
        // Longer than the longest arc-jet test, in the arc-jet gas.
        assert_eq!(named(MATERIAL_PICA_SERVED, &arc_jet_row_1(1.0, 401.0)), "QuestionNotCompared");
        // The slab, its start, its surroundings, the face: a value in no
        // envelope is refused by its own name.
        let mut q = arc_jet_row_1(1.0, 55.0);
        q.thickness_m = 0.05;
        assert_eq!(named(MATERIAL_PICA_SERVED, &q), "ThicknessNotCompared");
        // A value in another envelope is refused as a question not compared:
        // the arc-jet stream on the Stardust slab, from 260 K, or from 300 K.
        assert_eq!(named(MATERIAL_PICA_SERVED, &on_stardust_slab(arc_jet_row_1(1.0, 55.0))), "QuestionNotCompared");
        let mut q = arc_jet_row_1(1.0, 55.0);
        q.thickness_m = STARDUST_THICKNESS_M;
        assert_eq!(named(MATERIAL_PICA_SERVED, &q), "QuestionNotCompared");
        let mut q = arc_jet_row_1(1.0, 55.0);
        q.start_temperature_k = STARDUST_START_TEMPERATURE_K;
        assert_eq!(named(MATERIAL_PICA_SERVED, &q), "QuestionNotCompared");
        let mut q = arc_jet_row_1(1.0, 55.0);
        q.start_temperature_k = 280.0;
        assert_eq!(named(MATERIAL_PICA_SERVED, &q), "StartTemperatureNotCompared");
        let mut q = arc_jet_row_1(1.0, 55.0);
        q.start_temperature_k = 400.0;
        assert_eq!(named(MATERIAL_PICA_SERVED, &q), "StartTemperatureNotCompared");
        let mut q = arc_jet_row_1(1.0, 55.0);
        if let SurfaceCondition::GasStream { surroundings_temperature_k, face_wears, .. } = &mut q.surface {
            *surroundings_temperature_k = 250.0;
            assert!(*face_wears);
        }
        assert_eq!(named(MATERIAL_PICA_SERVED, &q), "SurroundingsNotCompared");
        let mut q = arc_jet_row_1(1.0, 55.0);
        if let SurfaceCondition::GasStream { face_wears, .. } = &mut q.surface {
            *face_wears = false;
        }
        assert_eq!(named(MATERIAL_PICA_SERVED, &q), "FaceNotWearingNotCompared");
        let held = SlabQuestion {
            thickness_m: 0.06,
            start_temperature_k: 300.0,
            surface: SurfaceCondition::HeldTemperature {
                held_temperature_k: 3300.0,
                hold_duration_s: 300.0,
            },
            continue_after_s: 300.0,
            thermocouple_depths_m: vec![],
        };
        for material in &MATERIAL_NAMES[1..] {
            assert_eq!(named(material, &held), "HeldFaceNotCompared");
            assert_eq!(named(material, &q), "FaceNotWearingNotCompared");
        }
        // TACOT, compared with other codes only, keeps its declared range.
        assert_eq!(named(MATERIAL_TACOT, &held), "answered");
        let mut tacot = with_gas(arc_jet_row_1(0.0934579, 10.0), StreamGas::AIR);
        tacot.thickness_m = 0.01;
        tacot.start_temperature_k = 400.0;
        if let SurfaceCondition::GasStream { history, face_wears, .. } = &mut tacot.surface {
            for row in history.iter_mut() {
                row.pressure_pa = 101325.0;
            }
            *face_wears = false;
        }
        assert_eq!(named(MATERIAL_TACOT, &tacot), "answered");
        // Radiative heating and a stream at 101 kPa: in neither envelope.
        let mut q = with_gas(arc_jet_row_1(1.0, 55.0), StreamGas::AIR);
        if let SurfaceCondition::GasStream { history, .. } = &mut q.surface {
            history[2].radiative_heating_w_per_m2 = 8.0e5;
        }
        assert_eq!(named(MATERIAL_PICA_SERVED, &q), "QuestionNotCompared");
        let b12 = with_gas(
            under_stream(
                0.06,
                stream_rows(&[(0.0, 101325.0, 0.0, 1.0e8, 0.0), (0.5, 101325.0, 1.0, 1.0e8, 0.0), (10.0, 101325.0, 1.0, 1.0e8, 0.0)]),
                &[],
            ),
            StreamGas::AIR,
        );
        assert_eq!(named(MATERIAL_PICA_SERVED, &b12), "QuestionNotCompared");
        // A stream that stops and comes back is outside the flight's one
        // stretch; one that runs past 134 s in air at 3 kPa and 50 W/cm² is
        // past the flight's and below the arc jet's heating.
        let stops = on_stardust_slab(under_stream(
            0.06,
            stream_rows(&[(0.0, 3000.0, 0.02, 2.5e7, 0.0), (10.0, 3000.0, 0.0, 2.5e7, 0.0), (20.0, 3000.0, 0.02, 2.5e7, 0.0)]),
            &[],
        ));
        let sentence = check_question(MATERIAL_PICA_SERVED, &stops).unwrap_err().to_string();
        assert!(sentence.contains("comes back on"), "{sentence}");
        let long = on_stardust_slab(under_stream(
            0.06,
            stream_rows(&[(0.0, 3000.0, 0.02, 2.5e7, 0.0), (200.0, 3000.0, 0.02, 2.5e7, 0.0)]),
            &[],
        ));
        let sentence = check_question(MATERIAL_PICA_SERVED, &long).unwrap_err().to_string();
        assert!(sentence.contains("later than 134 s"), "{sentence}");
    }

    #[test]
    fn the_four_thermocouple_blocks_stay_answered_inside_their_own_envelope() {
        for block in ["9B", "7B", "3B", "6A"] {
            assert_eq!(named(MATERIAL_PICA_SERVED, &thermocouple_block(block)), "answered", "{block}");
            let (arc_jet, stardust, blocks) = compared_envelopes(MATERIAL_PICA_SERVED, &thermocouple_block(block)).unwrap();
            assert!(!arc_jet && !stardust && blocks, "{block}");
        }
        // The shortest block through the served entry: inside its envelope,
        // its face below the compared top. evidence/research/
        // heatshield_pica_arcjet/fix_0.7.0/ runs all four and their
        // thermocouples: 31.28 K root mean square over 259 samples.
        let wanted = Wanted { thermocouple_histories: true, ..Wanted::default() };
        let a = answer(MATERIAL_PICA_SERVED, &thermocouple_block("6A"), &wanted).expect("block 6A is answered");
        assert!(a.summary.inside_thermocouple_block_comparison);
        assert!(!a.summary.inside_arc_jet_comparison && !a.summary.inside_stardust_comparison);
        assert!(a.summary.peak_face_temperature_k > 2900.0 && a.summary.peak_face_temperature_k < COMPARED_TEMPERATURE_MAX_K);
        assert_eq!(a.thermocouple_histories.as_ref().map(|h| h.len()), Some(2));
        // Each part of the envelope refuses by name what was not compared.
        let edit = |block: &str, change: &dyn Fn(&mut SlabQuestion, &mut Vec<StreamRow>)| {
            let mut q = thermocouple_block(block);
            let mut rows = match &q.surface {
                SurfaceCondition::GasStream { history, .. } => history.clone(),
                _ => unreachable!(),
            };
            change(&mut q, &mut rows);
            if let SurfaceCondition::GasStream { history, .. } = &mut q.surface {
                *history = rows;
            }
            check_question(MATERIAL_PICA_SERVED, &q)
        };
        let outside = |block: &str, change: &dyn Fn(&mut SlabQuestion, &mut Vec<StreamRow>), why: &str| {
            let reject = edit(block, change).unwrap_err();
            assert_eq!(reject.name(), "QuestionNotCompared", "{why}");
            let sentence = reject.to_string();
            assert!(sentence.contains(&format!("for the thermocouple blocks, {why}")), "{sentence}");
        };
        outside("3B", &|q, _| q.thickness_m = STARDUST_THICKNESS_M, "thickness 0.0582 m is outside 0.0224..=0.0325 m");
        outside("3B", &|_, rows| rows[4].time_s = 153.0, "the heating history lasts 153 s, longer than 152 s");
        outside("7B", &|_, rows| { rows[2].time_s = 14.0; rows[3].time_s = 14.5; }, "the stream is held until 14 s");
        outside("7B", &|_, rows| rows[3].time_s = 16.0, "the stream falls to zero over 1 s from 15 s");
        outside("9B", &|_, rows| rows[4].pressure_pa = 20000.0, "row 4 has wall pressure 20000 Pa, not the 20265 Pa of row 0");
        outside("9B", &|_, rows| rows[2].heat_transfer_coefficient_kg_per_m2_s *= 0.99, "the cold-wall heating falls to");
        outside("6A", &|_, rows| rows[1].heat_transfer_coefficient_kg_per_m2_s *= 1.01, "the cold-wall heating reaches");
        outside("6A", &|_, rows| { rows.truncate(3); }, "the stream is still on at 20 s");
        outside("6A", &|_, rows| rows[4].radiative_heating_w_per_m2 = 1.0, "row 4 has radiative heating 1 W/m2");
        outside("6A", &|_, rows| { for row in rows.iter_mut() { row.flow_enthalpy_j_per_kg = 2.0e7; } }, "row 0 has flow enthalpy 20000000 J/kg");
        // The block streams on another envelope's slab, start or surroundings
        // are refused: no comparison had them.
        assert_eq!(edit("9B", &|q, _| q.thickness_m = ARC_JET_THICKNESS_M).unwrap_err().name(), "QuestionNotCompared");
        assert_eq!(edit("9B", &|q, _| q.start_temperature_k = 300.0).unwrap_err().name(), "QuestionNotCompared");
        assert_eq!(
            edit("9B", &|q, _| if let SurfaceCondition::GasStream { surroundings_temperature_k, .. } = &mut q.surface {
                *surroundings_temperature_k = STARDUST_SURROUNDINGS_TEMPERATURE_K;
            })
            .unwrap_err()
            .name(),
            "QuestionNotCompared"
        );
        assert_eq!(edit("9B", &|q, _| q.start_temperature_k = 290.0).unwrap_err().name(), "StartTemperatureNotCompared");
        // An arc-jet stream on a block's slab from 303.15 K is refused too.
        let mut q = arc_jet_row_1(1.0, 55.0);
        q.thickness_m = 0.03;
        q.start_temperature_k = THERMOCOUPLE_BLOCK_START_TEMPERATURE_K;
        assert_eq!(named(MATERIAL_PICA_SERVED, &q), "QuestionNotCompared");
    }

    #[test]
    fn a_stream_colder_than_the_coldest_wall_is_refused_before_computing() {
        // 1,600 J/kg in air is below air's 1,871 J/kg at a wall at 300 K.
        // Refused at the door, for every material.
        let mut q = under_stream(
            0.06,
            stream_rows(&[(0.0, 101325.0, 0.0, 1600.0, 0.0), (0.5, 101325.0, 0.01, 1600.0, 0.0), (10.0, 101325.0, 0.01, 1600.0, 0.0)]),
            &[],
        );
        for material in MATERIAL_NAMES {
            let reject = check_question(material, &q).unwrap_err();
            assert_eq!(reject.name(), "FlowEnthalpyBelowWall", "{material}");
            match reject {
                SlabReject::FlowEnthalpyBelowWall { row, wall_temperature_k, wall_enthalpy_j_per_kg, .. } => {
                    assert_eq!((row, wall_temperature_k), (1, 300.0));
                    assert!((wall_enthalpy_j_per_kg - 1871.06).abs() < 0.01, "{wall_enthalpy_j_per_kg}");
                }
                _ => unreachable!(),
            }
        }
        // A row with no stream may have any enthalpy.
        if let SurfaceCondition::GasStream { history, .. } = &mut q.surface {
            for row in history.iter_mut().skip(1) {
                row.flow_enthalpy_j_per_kg = 1900.0;
            }
            history[0].flow_enthalpy_j_per_kg = 0.0;
        }
        assert_eq!(check_question(MATERIAL_TACOT, &q), Ok(()));
        // The coldest wall is the lower of the start and the surroundings.
        let mut cold = q.clone();
        if let SurfaceCondition::GasStream { surroundings_temperature_k, .. } = &mut cold.surface {
            *surroundings_temperature_k = 400.0;
        }
        cold.start_temperature_k = 400.0;
        assert_eq!(check_question(MATERIAL_TACOT, &cold).unwrap_err().name(), "FlowEnthalpyBelowWall");
        // The workshop cases ramp from no stream at zero enthalpy and are answered.
        assert_eq!(check_question(MATERIAL_TACOT, &ablation_workshop_test_case_2_2()), Ok(()));
    }

    #[test]
    fn a_pica_run_hotter_than_any_compared_run_is_refused_by_name() {
        // The limit is the Stardust near-nose run's hottest step.
        assert!(COMPARED_TEMPERATURE_MAX_K > 3412.0 && COMPARED_TEMPERATURE_MAX_K < 3413.0);
        assert!(COMPARED_TEMPERATURE_MAX_K < crate::pica::TABLE_TOP_K);
        // The mechanism, with a lower top on a short arc-jet question.
        let q = arc_jet_row_1(1.0, 3.0);
        let terms = Terms {
            compared_temperature_top_k: Some(1200.0),
            ..Terms::ALL
        };
        let reject = solve_material_with(MATERIAL_PICA_SERVED, &q, &terms).unwrap_err();
        assert!(
            matches!(reject, SlabReject::TemperatureAboveCompared { temperature_k, depth_m, .. } if temperature_k > 1200.0 && depth_m < 1.0e-4),
            "{reject:?}"
        );
        // The served entry answers it and says which envelope it lies in.
        let a = answer(MATERIAL_PICA_SERVED, &q, &Wanted::default()).expect("inside the arc jet");
        assert!(a.summary.inside_arc_jet_comparison && !a.summary.inside_stardust_comparison);
        assert!(a.summary.material_compared_with_measurement);
    }

    #[test]
    fn every_refusal_has_a_name_and_one_plain_sentence() {
        let all = [
            SlabReject::ThicknessOutOfRange { thickness_m: 0.5 },
            SlabReject::StartTemperatureOutOfRange {
                start_temperature_k: 10.0,
            },
            SlabReject::HeldTemperatureOutOfRange {
                held_temperature_k: 10.0,
            },
            SlabReject::HoldDurationOutOfRange {
                hold_duration_s: 0.0,
            },
            SlabReject::ContinueAfterOutOfRange {
                continue_after_s: -1.0,
            },
            SlabReject::TooManyThermocouples { count: 9 },
            SlabReject::ThermocoupleOutsideSlab {
                index: 1,
                depth_m: 1.0,
            },
            SlabReject::StepNotConverged {
                time_s: 1.0,
                passes: 12,
            },
            SlabReject::NonFiniteValue { time_s: 1.0 },
            SlabReject::TemperatureOutOfBounds {
                time_s: 1.0,
                depth_m: 0.001,
                temperature_k: 4000.0,
            },
            SlabReject::DecompositionRanBackwards {
                time_s: 1.0,
                depth_m: 0.001,
            },
            SlabReject::MassAccountBroken {
                time_s: 1.0,
                solid_mass_lost_kg_per_m2: 1.0,
                gas_mass_out_kg_per_m2: 2.0,
            },
            SlabReject::EnergyAccountBroken {
                time_s: 1.0,
                residual_j_per_m2: 1.0,
                tolerance_j_per_m2: 0.1,
            },
            SlabReject::HistoryRowsOutOfRange { rows: 1 },
            SlabReject::HistoryTimesOutOfOrder {
                row: 2,
                time_s: 1.0,
            },
            SlabReject::HistoryDurationOutOfRange { duration_s: 0.1 },
            SlabReject::HistoryRowsTooClose {
                row: 1,
                span_s: 1.0e-300,
            },
            SlabReject::StreamPressureOutOfRange {
                row: 0,
                pressure_pa: 0.1,
                lowest_pressure_pa: PICA_STREAM_PRESSURE_MIN_PA,
                highest_pressure_pa: PICA_STREAM_PRESSURE_MAX_PA,
            },
            SlabReject::HeatTransferCoefficientOutOfRange {
                row: 0,
                heat_transfer_coefficient_kg_per_m2_s: 2.0,
            },
            SlabReject::FlowEnthalpyOutOfRange {
                row: 0,
                flow_enthalpy_j_per_kg: 2.0e8,
            },
            SlabReject::RadiativeHeatingOutOfRange {
                row: 0,
                radiative_heating_w_per_m2: 2.0e7,
            },
            SlabReject::SurroundingsTemperatureOutOfRange {
                surroundings_temperature_k: 10.0,
            },
            SlabReject::ContinueAfterWithStream {
                continue_after_s: 1.0,
            },
            SlabReject::FaceBeyondSurfaceTable {
                time_s: 1.0,
                face_temperature_k: 3800.0,
            },
            SlabReject::SlabWornAway {
                time_s: 1.0,
                recession_m: 0.009,
            },
            SlabReject::FaceMovedBackward { time_s: 1.0 },
            SlabReject::FaceOutranItsCell { time_s: 1.0 },
            SlabReject::UnknownMaterial {
                name: "x".to_string(),
            },
            SlabReject::StreamOxygenOutOfRange {
                oxygen_mass_fraction: 0.5,
            },
            SlabReject::StreamArgonOutOfRange {
                argon_mass_fraction: 0.6,
            },
            SlabReject::StreamNotAirForThisMaterial {
                stream_gas: StreamGas::air_with_argon(0.1),
            },
            SlabReject::SurfaceChemistryNotConverged {
                pressure_pa: 2300.0,
                blowing: 0.1,
                temperature_k: 450.0,
            },
            SlabReject::SurfaceBundleUnreadable {
                why: "cut short".to_string(),
            },
            SlabReject::FlowEnthalpyBelowWall {
                row: 1,
                flow_enthalpy_j_per_kg: 1600.0,
                wall_enthalpy_j_per_kg: 1620.0,
                wall_temperature_k: 300.0,
            },
            SlabReject::HeldFaceNotCompared,
            SlabReject::FaceNotWearingNotCompared,
            SlabReject::ThicknessNotCompared { thickness_m: 0.01 },
            SlabReject::StartTemperatureNotCompared {
                start_temperature_k: 400.0,
            },
            SlabReject::SurroundingsNotCompared {
                surroundings_temperature_k: 250.0,
            },
            SlabReject::QuestionNotCompared {
                arc_jet: "the stream is too cold".to_string(),
                stardust: "the stream is not air".to_string(),
                thermocouple_blocks: "the stream is not air".to_string(),
            },
            SlabReject::TemperatureAboveCompared {
                time_s: 1.0,
                depth_m: 0.0,
                temperature_k: 3500.0,
            },
        ];
        assert_eq!(all.len(), 41);
        let mut names: Vec<&str> = all.iter().map(|r| r.name()).collect();
        for refusal in &all {
            let sentence = refusal.to_string();
            assert!(
                format!("{refusal:?}").starts_with(refusal.name()),
                "{refusal:?}"
            );
            assert!(
                sentence.len() > 20 && !sentence.contains('{') && !sentence.contains('\n'),
                "{sentence}"
            );
            assert!(
                sentence.chars().next().unwrap().is_lowercase()
                    || sentence.starts_with(char::is_numeric),
                "{sentence}"
            );
        }
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 41);
    }

    #[test]
    fn the_question_check_is_the_one_answer_uses_and_states_each_materials_pressure_range() {
        // The published ranges are the tables' own ends.
        let (_, bundle) = frozen_pica();
        assert_eq!(
            bundle.pressure_range_pa(),
            (PICA_STREAM_PRESSURE_MIN_PA, PICA_STREAM_PRESSURE_MAX_PA)
        );
        assert_eq!(
            (TACOT.surface.pressure_pa[0], TACOT.surface.pressure_pa[3]),
            (STREAM_PRESSURE_MIN_PA, STREAM_PRESSURE_MAX_PA)
        );
        assert_eq!(MATERIAL_NAMES[1], MATERIAL_PICA_SERVED);
        for name in MATERIAL_NAMES {
            let expected = if name == MATERIAL_TACOT {
                (101.325, 101325.0)
            } else {
                (1.01325, 101325.0)
            };
            assert_eq!(stream_pressure_range_pa(name), Ok(expected), "{name}");
        }
        // The Stardust slab in air at 20 MJ/kg and 100 W/cm²: for PICA,
        // inside the Stardust envelope at every pressure here that the tables
        // hold except 2e5 Pa.
        let at = |pressure: f64| {
            on_stardust_slab(under_stream(
                0.06,
                stream_rows(&[
                    (0.0, pressure, 0.0, 2.0e7, 0.0),
                    (0.5, pressure, 0.05, 2.0e7, 0.0),
                    (2.0, pressure, 0.05, 2.0e7, 0.0),
                ]),
                &[0.01],
            ))
        };
        // No such material: the check, the range and the entry refuse alike.
        let nobody = SlabReject::UnknownMaterial {
            name: "PICA".to_string(),
        };
        assert_eq!(check_question("PICA", &at(5000.0)), Err(nobody.clone()));
        assert_eq!(stream_pressure_range_pa("PICA"), Err(nobody.clone()));
        assert_eq!(
            answer("PICA", &at(5000.0), &Wanted::default()).expect_err("no such material"),
            nobody
        );
        // 50 Pa is inside PICA's table and below TACOT's; 0.5 Pa is below both.
        // Each refusal states the range of the material that was asked for.
        for name in MATERIAL_NAMES {
            for &pressure in &[5000.0, 50.0, 0.5, 2.0e5] {
                let q = at(pressure);
                let checked = check_question(name, &q);
                let answered = answer(name, &q, &Wanted::default());
                let (low, high) = stream_pressure_range_pa(name).unwrap();
                if (low..=high).contains(&pressure) {
                    assert_eq!(checked, Ok(()), "{name} {pressure}");
                    assert!(answered.is_ok(), "{name} {pressure}");
                } else {
                    let refusal = SlabReject::StreamPressureOutOfRange {
                        row: 0,
                        pressure_pa: pressure,
                        lowest_pressure_pa: low,
                        highest_pressure_pa: high,
                    };
                    assert_eq!(checked, Err(refusal.clone()), "{name} {pressure}");
                    assert_eq!(answered.expect_err("outside"), refusal, "{name} {pressure}");
                    let sentence = refusal.to_string();
                    let range = if name == MATERIAL_TACOT {
                        "101.325..=101325 Pa"
                    } else {
                        "1.01325..=101325 Pa"
                    };
                    assert!(sentence.ends_with(range), "{sentence}");
                }
            }
            // Every other refusal of a question comes from the same check.
            let mut thick = at(5000.0);
            thick.thickness_m = 0.2;
            let mut deep = at(5000.0);
            deep.thermocouple_depths_m = vec![0.07];
            let mut late = at(5000.0);
            late.continue_after_s = 1.0;
            for q in [thick, deep, late] {
                let checked = check_question(name, &q).expect_err("outside the range");
                let answered = answer(name, &q, &Wanted::default()).expect_err("outside the range");
                assert_eq!(checked, answered, "{name}");
            }
        }
        // Inside the range and refused in the run: the check passes, the
        // entry refuses by name.
        let hard = under_stream(
            THICKNESS_MIN_M,
            stream_rows(&[
                (0.0, 101_325.0, 0.0, 5.0e7, 0.0),
                (
                    0.5,
                    101_325.0,
                    HEAT_TRANSFER_COEFFICIENT_MAX_KG_PER_M2_S,
                    5.0e7,
                    0.0,
                ),
                (
                    120.0,
                    101_325.0,
                    HEAT_TRANSFER_COEFFICIENT_MAX_KG_PER_M2_S,
                    5.0e7,
                    0.0,
                ),
            ]),
            &[],
        );
        // Beyond what was compared, so the served entry refuses it at the
        // door; the physics without that limit refuses it in the run.
        assert_eq!(
            check_question(MATERIAL_PICA_SERVED, &hard).unwrap_err().name(),
            "ThicknessNotCompared"
        );
        assert_eq!(check_beyond_compared(MATERIAL_PICA_SERVED, &hard), Ok(()));
        let err = answer_beyond_compared(MATERIAL_PICA_SERVED, &hard, &Wanted::default())
            .expect_err("too much");
        eprintln!(
            "inside the range, refused in the run: {} ({err})",
            err.name()
        );
        assert!(!err.name().ends_with("OutOfRange"), "{}", err.name());
    }

    /// 294 questions for the PICA model a caller is pointed to (version 5)
    /// across the range its physics takes, beyond what was compared and so
    /// asked through [`answer_beyond_compared`], six of them flight-shaped, and the same
    /// 294 for versions 4 and 3. About 8 minutes with `--release`. Measured
    /// for version 5: 269 answered, all six flights among them; 25 refused
    /// because the slab wore away; none for any other reason; hardest step
    /// 11 passes. Versions 4 and 3: 268 answered, 26 worn away, 11 passes.
    #[test]
    #[ignore]
    fn a_sweep_of_the_pica_model_refuses_only_what_the_tables_cannot_hold() {
        let mut questions = Vec::new();
        for &thickness in &[THICKNESS_MIN_M, 0.058, THICKNESS_MAX_M] {
            for &coefficient in &[0.003, 0.3, HEAT_TRANSFER_COEFFICIENT_MAX_KG_PER_M2_S] {
                for &enthalpy in &[1.5e6, 2.5e7, FLOW_ENTHALPY_MAX_J_PER_KG] {
                    for &pressure in &[1.02, 500.0, 23_811.0, STREAM_PRESSURE_MAX_PA] {
                        for &heating in &[0.0, 3.0e6] {
                            let on = (0.5, pressure, coefficient, enthalpy, heating);
                            questions.push(under_stream(
                                thickness,
                                stream_rows(&[
                                    (0.0, pressure, 0.0, enthalpy, 0.0),
                                    on,
                                    (3.0, on.1, on.2, on.3, on.4),
                                ]),
                                &[0.5 * thickness],
                            ));
                            if thickness == 0.058 {
                                questions.push(under_stream(
                                    thickness,
                                    stream_rows(&[
                                        (0.0, pressure, 0.0, enthalpy, 0.0),
                                        on,
                                        (40.0, on.1, on.2, on.3, on.4),
                                        (40.5, pressure, 0.0, enthalpy, 0.0),
                                        (80.0, pressure, 0.0, enthalpy, 0.0),
                                    ]),
                                    &[0.5 * thickness],
                                ));
                            }
                        }
                    }
                }
            }
        }
        for &(seconds, cooling) in &[(30usize, 20.0), (60, 60.0), (133, 466.0)] {
            for &thickness in &[0.03, 0.0582] {
                questions.push(under_stream(
                    thickness,
                    flight_shaped(seconds, cooling),
                    &[0.01],
                ));
            }
        }
        for material in [
            MATERIAL_PICA_SERVED,
            MATERIAL_PICA_VERSION_4,
            MATERIAL_PICA_VERSION_3,
        ] {
            let all = Wanted {
                face_history: true,
                thermocouple_histories: true,
                final_profile: true,
            };
            let mut passes = [0u32; NEWTON_PASS_LIMIT as usize + 1];
            let (mut answered, mut too_hot, mut past_table, mut worn, mut flights) =
                (0, 0, 0, 0, 0);
            let mut other = Vec::new();
            for (index, q) in questions.iter().enumerate() {
                let flight = index + 6 >= questions.len();
                match answer_beyond_compared(material, q, &all) {
                    Ok(a) => {
                        answered += 1;
                        flights += flight as u32;
                        passes[a.summary.hardest_step_newton_passes as usize] += 1;
                        let s = &a.summary;
                        assert!(
                            s.energy_account_residual_j_per_m2.abs()
                                <= s.energy_account_tolerance_j_per_m2
                        );
                        assert!(
                            s.mass_account_residual_kg_per_m2.abs()
                                <= s.mass_account_tolerance_kg_per_m2
                        );
                        assert!(
                            s.face_balance_residual_j_per_m2.abs()
                                <= s.face_balance_tolerance_j_per_m2
                        );
                        assert!(s.wear_m >= 0.0 && s.peak_face_temperature_k.is_finite());
                        let face = a.face_history.as_ref().unwrap();
                        assert!(
                            face.time_s.len() <= HISTORY_POINTS_MAX
                                && face.recession_m.windows(2).all(|pair| pair[1] >= pair[0])
                        );
                        assert!(
                            a.final_profile.as_ref().unwrap().depth_m.len() <= PROFILE_POINTS_MAX
                        );
                        for h in a.thermocouple_histories.as_ref().unwrap() {
                            assert!(h.temperature_k.iter().all(|&t| t > 200.0));
                        }
                    }
                    Err(SlabReject::TemperatureOutOfBounds { temperature_k, .. })
                        if temperature_k > 3975.0 =>
                    {
                        too_hot += 1
                    }
                    Err(SlabReject::FaceBeyondSurfaceTable { .. }) => past_table += 1,
                    Err(SlabReject::SlabWornAway { .. }) => worn += 1,
                    Err(err) => other.push(format!("{q:?}: {} ({err})", err.name())),
                }
            }
            eprintln!(
            "{material}: {} questions: {answered} answered ({flights} of the 6 flight-shaped); refused because a temperature passed the top of the material's tables (3975 K): {too_hot}; because the face reached the table's boiling point: {past_table}; because the slab wore away: {worn}; for any other reason: {}. Answered runs by Newton passes in their hardest step: {passes:?}",
            questions.len(),
            other.len()
        );
            assert!(other.is_empty(), "{:#?}", &other[..other.len().min(8)]);
            assert_eq!(flights, 6);
            let hardest = passes.iter().rposition(|&count| count > 0).unwrap() as u32;
            assert!(hardest < NEWTON_PASS_LIMIT, "{hardest}");
        }
    }

    /// The same sweep for streams that are not air, on the PICA model a
    /// caller is pointed to: seven gases at the corners and inside of the
    /// declared range (three of them arc-jet streams), asked through
    /// [`answer_beyond_compared`], each with 48 short
    /// histories at one pressure and two flight-shaped ones over many
    /// pressures. 350 questions; about 3 minutes with `--release`.
    /// Measured: 350 answered, none refused; hardest step 9 passes.
    #[test]
    #[ignore]
    fn a_sweep_of_streams_that_are_not_air_refuses_only_what_the_tables_cannot_hold() {
        let gas = |oxygen: f64, argon: f64| StreamGas {
            oxygen_mass_fraction: oxygen,
            argon_mass_fraction: argon,
        };
        let gases = [
            StreamGas::air_with_argon(0.276),
            gas(0.10, 0.0),
            gas(0.30, 0.0),
            gas(STREAM_OXYGEN_MASS_FRACTION_MIN, 0.0),
            gas(
                STREAM_OXYGEN_MASS_FRACTION_MIN,
                STREAM_ARGON_MASS_FRACTION_MAX,
            ),
            gas(STREAM_OXYGEN_MASS_FRACTION_MAX, 0.0),
            gas(
                STREAM_OXYGEN_MASS_FRACTION_MAX,
                STREAM_ARGON_MASS_FRACTION_MAX,
            ),
        ];
        let mut passes = [0u32; NEWTON_PASS_LIMIT as usize + 1];
        let (mut asked, mut answered, mut too_hot, mut past_table, mut worn) = (0, 0, 0, 0, 0);
        let mut other = Vec::new();
        let started = std::time::Instant::now();
        for stream_gas in gases {
            let mut questions = Vec::new();
            for &coefficient in &[0.003, 0.3] {
                for &enthalpy in &[1.5e6, 2.5e7, FLOW_ENTHALPY_MAX_J_PER_KG] {
                    for &pressure in &[1.02, 500.0, 23_811.0, STREAM_PRESSURE_MAX_PA] {
                        for &heating in &[0.0, 3.0e6] {
                            let on = (0.5, pressure, coefficient, enthalpy, heating);
                            questions.push(under_stream(
                                0.058,
                                stream_rows(&[
                                    (0.0, pressure, 0.0, enthalpy, 0.0),
                                    on,
                                    (3.0, on.1, on.2, on.3, on.4),
                                ]),
                                &[0.029],
                            ));
                        }
                    }
                }
            }
            questions.push(under_stream(0.0582, flight_shaped(30, 20.0), &[0.01]));
            questions.push(under_stream(0.0582, flight_shaped(133, 466.0), &[0.01]));
            for mut q in questions {
                if let SurfaceCondition::GasStream { stream_gas: g, .. } = &mut q.surface {
                    *g = stream_gas;
                }
                asked += 1;
                assert_eq!(check_beyond_compared(MATERIAL_PICA_SERVED, &q), Ok(()));
                match answer_beyond_compared(MATERIAL_PICA_SERVED, &q, &Wanted::default()) {
                    Ok(a) => {
                        answered += 1;
                        let s = &a.summary;
                        passes[s.hardest_step_newton_passes as usize] += 1;
                        assert!(
                            s.energy_account_residual_j_per_m2.abs()
                                <= s.energy_account_tolerance_j_per_m2
                        );
                        assert!(
                            s.mass_account_residual_kg_per_m2.abs()
                                <= s.mass_account_tolerance_kg_per_m2
                        );
                        assert!(
                            s.face_balance_residual_j_per_m2.abs()
                                <= s.face_balance_tolerance_j_per_m2
                        );
                        assert!(s.wear_m >= 0.0 && s.peak_face_temperature_k.is_finite());
                    }
                    Err(SlabReject::TemperatureOutOfBounds { temperature_k, .. })
                        if temperature_k > 3975.0 =>
                    {
                        too_hot += 1
                    }
                    Err(SlabReject::FaceBeyondSurfaceTable { .. }) => past_table += 1,
                    Err(SlabReject::SlabWornAway { .. }) => worn += 1,
                    Err(err) => other.push(format!("{stream_gas:?}: {} ({err})", err.name())),
                }
            }
        }
        eprintln!(
            "{asked} questions in seven streams that are not air, in {:.0} s: {answered} answered; refused because a temperature passed the top of the material's tables: {too_hot}; because the face reached the table's boiling point: {past_table}; because the slab wore away: {worn}; for any other reason: {}. Answered runs by Newton passes in their hardest step: {passes:?}",
            started.elapsed().as_secs_f64(),
            other.len()
        );
        assert!(other.is_empty(), "{:#?}", &other[..other.len().min(8)]);
        let hardest = passes.iter().rposition(|&count| count > 0).unwrap() as u32;
        assert!(hardest < NEWTON_PASS_LIMIT, "{hardest}");
    }
}

/// Stage seven: a way to run studies from outside without a public door.
#[cfg(test)]
mod study_runner {
    use super::*;
    use crate::pica;

    /// Reads a study from the file named in `ABLATOR_STUDY` (lines of a name
    /// and numbers) and prints the result on one line. Ignored unless asked for.
    #[test]
    #[ignore]
    fn study() {
        let Ok(path) = std::env::var("ABLATOR_STUDY") else {
            return;
        };
        let text = std::fs::read_to_string(&path).expect("study file");
        let mut d = pica::FROZEN;
        let mut o = pica::SURFACE_OPTIONS;
        let (mut thickness, mut start, mut surroundings, mut wears) = (0.05, 300.0, 300.0, true);
        let (mut rows, mut depths, mut bundle) = (Vec::new(), Vec::new(), None);
        let mut stream_gas = StreamGas::AIR;
        let (mut cells_halved, mut steps_halved) = (false, false);
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            let Some(name) = parts.next() else { continue };
            let rest: Vec<&str> = parts.collect();
            let v: Vec<f64> = rest.iter().filter_map(|w| w.parse().ok()).collect();
            let flag = || v[0] != 0.0;
            match name {
                "version" => {
                    if v[0] as u32 == 5 {
                        o = pica::SURFACE_OPTIONS_VERSION_5;
                    }
                    d = match v[0] as u32 {
                        4 | 5 => pica::VERSION_4,
                        3 => pica::VERSION_3,
                        _ => pica::FROZEN,
                    }
                }
                "thickness_m" => thickness = v[0],
                "start_temperature_k" => start = v[0],
                "surroundings_temperature_k" => surroundings = v[0],
                "face_wears" => wears = flag(),
                "row" => rows.push(StreamRow {
                    time_s: v[0],
                    pressure_pa: v[1],
                    heat_transfer_coefficient_kg_per_m2_s: v[2],
                    flow_enthalpy_j_per_kg: v[3],
                    radiative_heating_w_per_m2: v[4],
                }),
                "thermocouple" => depths.push(v[0]),
                "virgin_density_kg_per_m3" => d.virgin_density_kg_per_m3 = v[0],
                "char_density_kg_per_m3" => d.char_density_kg_per_m3 = v[0],
                "virgin_conductivity_scale" => d.virgin_conductivity_scale = v[0],
                "char_conductivity_scale" => d.char_conductivity_scale = v[0],
                "conductivity_held_above_3333_k" => d.conductivity_held_above_3333_k = flag(),
                "char_emissivity" => d.char_emissivity = v[0],
                "no_decomposition_below_k" => d.no_decomposition_below_k = v[0],
                "pore_gas_half_pressure_scale" => {
                    d.pore_gas_half_pressure_scale = if v[0] > 0.0 { Some(v[0]) } else { None }
                }
                "measured_curves" => d.measured_curves = flag(),
                "low_pressure_rule" => o.low_pressure_rule = flag(),
                "oxidation_limited_by_wall_reaction" => {
                    o.oxidation_limited_by_wall_reaction = flag()
                }
                "oxygen_atom_reaction_probability" => {
                    o.oxygen_atom_reaction_probability = if v[0] > 0.0 { Some(v[0]) } else { None }
                }
                "carbon_vapour_limited_by_evaporation" => {
                    o.carbon_vapour_limited_by_evaporation = flag()
                }
                "solid_char_loss_fraction" => o.solid_char_loss_fraction = v[0],
                "stream_heating_supplied_separately" => {
                    o.stream_heating_supplied_separately = flag()
                }
                "gas_beside_unused_oxygen" => {
                    o.gas_beside_unused_oxygen = match v[0] as u32 {
                        0 => GasBesideUnusedOxygen::Burns,
                        1 => GasBesideUnusedOxygen::FollowsTheLowPressureRule,
                        _ => GasBesideUnusedOxygen::DoesNotBurn,
                    }
                }
                "air_table_computed_afresh" => o.air_table_computed_afresh = flag(),
                "cells_halved" => cells_halved = flag(),
                "steps_halved" => steps_halved = flag(),
                "surface_bundle_file" => {
                    bundle = Some(std::fs::read(rest[0]).expect("bundle file"))
                }
                "stream_gas" => {
                    stream_gas = StreamGas {
                        oxygen_mass_fraction: v[0],
                        argon_mass_fraction: v[1],
                    }
                }
                other => panic!("study file: {other}"),
            }
        }
        let q = SlabQuestion {
            thickness_m: thickness,
            start_temperature_k: start,
            surface: SurfaceCondition::GasStream {
                history: rows,
                surroundings_temperature_k: surroundings,
                face_wears: wears,
                stream_gas,
            },
            continue_after_s: 0.0,
            thermocouple_depths_m: depths,
        };
        let list = |v: &[f64]| {
            format!(
                "[{}]",
                v.iter()
                    .map(|x| format!("{x:?}"))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        match solve_pica_study_refined(&d, &o, bundle.as_deref(), cells_halved, steps_halved, &q) {
            Err(err) => println!("STUDY_RESULT {{\"refused\":\"{}: {}\"}}", err.name(), err),
            Ok(r) => println!(
                "STUDY_RESULT {{\"report_time_s\":{},\"surface_temperature_k\":{},\"recession_m\":{},\"virgin_front_depth_m\":{},\"char_front_depth_m\":{},\"gas_mass_flux_at_surface_kg_per_m2_s\":{},\"char_mass_flux_kg_per_m2_s\":{},\"thermocouple_temperature_k\":[{}],\"final_depth_m\":{},\"final_temperature_k\":{},\"final_density_kg_per_m3\":{},\"heat_from_stream_j_per_m2\":{:?},\"heat_radiated_away_j_per_m2\":{:?},\"heat_taken_at_face_by_gases_j_per_m2\":{:?},\"time_steps\":{},\"hardest_step_newton_passes\":{}}}",
                list(&r.report_time_s),
                list(&r.surface_temperature_k),
                list(&r.recession_m),
                list(&r.virgin_front_depth_m),
                list(&r.char_front_depth_m),
                list(&r.gas_mass_flux_at_surface_kg_per_m2_s),
                list(&r.char_mass_flux_kg_per_m2_s),
                r.thermocouple_temperature_k
                    .iter()
                    .map(|h| list(h))
                    .collect::<Vec<_>>()
                    .join(","),
                list(&r.final_depth_m),
                list(&r.final_temperature_k),
                list(&r.final_density_kg_per_m3),
                r.heat_from_stream_j_per_m2,
                r.heat_radiated_away_j_per_m2,
                r.heat_taken_at_face_by_gases_j_per_m2,
                r.time_steps,
                r.hardest_step_newton_passes
            ),
        }
    }

    #[test]
    fn version_four_is_made_of_the_measured_curves_and_one_fitted_number() {
        let m = pica::material(&pica::VERSION_4);
        assert!((m.solid_density(m.virgin_remaining()) - 274.0).abs() < 1.0e-10);
        assert!((m.solid_density([0.0, 0.0]) - 274.0 * 220.0 / 280.0).abs() < 1.0e-10);
        assert!(m
            .resin_parts
            .iter()
            .all(|part| part.threshold_temperature_k == 555.5555555555555));
        assert_eq!(pica::VERSION_4.char_conductivity_scale, 0.71);
        let nodes = SolidEnthalpyNodes::of(&m);
        // The measured heat capacity, for both states, at its own points.
        for &(t, cp) in &pica::MEASURED_SPECIFIC_HEAT {
            let point = solid_point(&nodes, t, None);
            assert!(
                (point.virgin_specific_heat - cp).abs() < 1.0e-9
                    && point.char_specific_heat == point.virgin_specific_heat,
                "{t}"
            );
        }
        assert_eq!(solid_point(&nodes, 3500.0, None).char_specific_heat, 2303.0);
        // The measured char conductivity at the pressure it was measured at,
        // carried to 274 kg/m3: the solid's part up, radiation's part down.
        nodes
            .pore_pressure_pa
            .set(pica::MEASURED_CONDUCTIVITY_PRESSURE_PA);
        let ratio = 274.0 / 240.3;
        assert!(
            (solid_point(&nodes, 1000.0, None).char_conductivity - 0.71 * 0.369 * ratio).abs()
                < 1.0e-12
        );
        let hot = solid_point(&nodes, 2788.0, None).char_conductivity;
        assert!(
            (hot - 0.71 * (0.369 * ratio + (1.426 - 0.369) / ratio)).abs() < 1.0e-12,
            "{hot}"
        );
        // The virgin curve is version 3's.
        nodes.pore_pressure_pa.set(101325.0);
        assert!((solid_point(&nodes, 298.0, None).virgin_conductivity - 0.174).abs() < 2.0e-4);
        // Enthalpy is still the exact integral of the heat capacity.
        let (a, b) = (
            solid_point(&nodes, 999.0, None),
            solid_point(&nodes, 1001.0, None),
        );
        let here = solid_point(&nodes, 1000.0, None);
        assert!(
            ((b.char_enthalpy - a.char_enthalpy) / 2.0 - here.char_specific_heat).abs() < 1.0e-6
        );
        // Picked by name through the served entry; versions one and three still are.
        let q = SlabQuestion {
            thickness_m: 0.03,
            start_temperature_k: 300.0,
            surface: SurfaceCondition::GasStream {
                history: vec![
                    StreamRow {
                        time_s: 0.0,
                        pressure_pa: 20_000.0,
                        heat_transfer_coefficient_kg_per_m2_s: 0.0,
                        flow_enthalpy_j_per_kg: 2.5e7,
                        radiative_heating_w_per_m2: 0.0,
                    },
                    StreamRow {
                        time_s: 0.5,
                        pressure_pa: 20_000.0,
                        heat_transfer_coefficient_kg_per_m2_s: 0.17,
                        flow_enthalpy_j_per_kg: 2.5e7,
                        radiative_heating_w_per_m2: 0.0,
                    },
                    StreamRow {
                        time_s: 6.0,
                        pressure_pa: 20_000.0,
                        heat_transfer_coefficient_kg_per_m2_s: 0.17,
                        flow_enthalpy_j_per_kg: 2.5e7,
                        radiative_heating_w_per_m2: 0.0,
                    },
                ],
                surroundings_temperature_k: 300.0,
                face_wears: true,
                stream_gas: StreamGas::AIR,
            },
            continue_after_s: 0.0,
            thermocouple_depths_m: vec![],
        };
        let four = answer_beyond_compared(MATERIAL_PICA_VERSION_4, &q, &Wanted::default()).expect("version 4");
        let three = answer_beyond_compared(MATERIAL_PICA_VERSION_3, &q, &Wanted::default()).expect("version 3");
        assert!(four.summary.wear_m > 0.0 && four.summary.wear_m != three.summary.wear_m);
        assert!(
            four.summary.energy_account_residual_j_per_m2.abs()
                <= four.summary.energy_account_tolerance_j_per_m2
        );
        assert!(
            four.summary.mass_account_residual_kg_per_m2.abs()
                <= four.summary.mass_account_tolerance_kg_per_m2
        );
    }

    /// Largest differences between a bundle and another, array by array:
    /// B'c share with and without vapour (absolute), the three enthalpies
    /// (J/kg), the gas alone (J/kg), the atomic share of oxygen (absolute).
    fn bundle_differences(a: &SurfaceBundle, b: &SurfaceBundle) -> [f64; 7] {
        assert_eq!(a.pressure_pa.len(), b.pressure_pa.len());
        for (x, y) in a.pressure_pa.iter().zip(&b.pressure_pa) {
            assert!((x - y).abs() <= 1.0e-12 * x, "{x} {y}");
        }
        assert_eq!(a.blowing, b.blowing);
        assert_eq!(a.temperature_k, b.temperature_k);
        assert_eq!(a.atom_pressure_pa, b.atom_pressure_pa);
        assert_eq!(a.atom_enthalpy_j_per_kg, b.atom_enthalpy_j_per_kg);
        let worst = |x: &[f64], y: &[f64]| {
            assert_eq!(x.len(), y.len());
            x.iter()
                .zip(y)
                .fold(0.0f64, |w, (p, q)| w.max((p - q).abs()))
        };
        [
            worst(&a.share, &b.share),
            worst(&a.share_without_vapour, &b.share_without_vapour),
            worst(&a.wall_enthalpy, &b.wall_enthalpy),
            worst(
                &a.wall_enthalpy_without_vapour,
                &b.wall_enthalpy_without_vapour,
            ),
            worst(&a.wall_enthalpy_no_exchange, &b.wall_enthalpy_no_exchange),
            worst(&a.gas_alone_enthalpy, &b.gas_alone_enthalpy),
            worst(&a.atom_fraction, &b.atom_fraction),
        ]
    }

    fn across(pressure_pa: &[f64]) -> Vec<StreamRow> {
        pressure_pa
            .iter()
            .enumerate()
            .map(|(row, &pressure_pa)| StreamRow {
                time_s: row as f64,
                pressure_pa,
                heat_transfer_coefficient_kg_per_m2_s: 0.0,
                flow_enthalpy_j_per_kg: 0.0,
                radiative_heating_w_per_m2: 0.0,
            })
            .collect()
    }

    /// Computes the surface table here for each bundle file named in
    /// `ABLATOR_BUNDLES` (lines of a path, an oxygen mass fraction and an
    /// argon mass fraction) and prints the largest differences from the
    /// file, which the surface work's Python made. Ignored unless asked for.
    #[test]
    #[ignore]
    fn computed_tables_against_bundle_files() {
        let Ok(list) = std::env::var("ABLATOR_BUNDLES") else {
            return;
        };
        let (_, locked) = frozen_pica();
        for line in std::fs::read_to_string(&list).expect("list").lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let file = SurfaceBundle::from_bytes(&std::fs::read(parts[0]).expect("bundle file"))
                .expect("bundle");
            let gas = StreamGas {
                oxygen_mass_fraction: parts[1].parse().unwrap(),
                argon_mass_fraction: parts[2].parse().unwrap(),
            };
            let (low, high) = file.pressure_range_pa();
            let started = std::time::Instant::now();
            let made = locked
                .for_stream(&gas, &across(&[low, high]))
                .expect("computed");
            let seconds = started.elapsed().as_secs_f64();
            println!(
                "BUNDLE {} oxygen {} argon {} pressures {} seconds {seconds:.2} differences {:?}",
                parts[0],
                gas.oxygen_mass_fraction,
                gas.argon_mass_fraction,
                file.pressure_pa.len(),
                bundle_differences(&made, &file)
            );
            // Where the three node tables differ by more than rounding.
            let (nb, nt) = (file.blowing.len(), file.temperature_k.len());
            for (name, a, b, floor) in [
                ("share", &made.share, &file.share, 1.0e-9),
                ("enthalpy", &made.wall_enthalpy, &file.wall_enthalpy, 1.0e-3),
                (
                    "enthalpy without vapour",
                    &made.wall_enthalpy_without_vapour,
                    &file.wall_enthalpy_without_vapour,
                    1.0e-3,
                ),
                (
                    "no exchange",
                    &made.wall_enthalpy_no_exchange,
                    &file.wall_enthalpy_no_exchange,
                    1.0e-3,
                ),
            ] {
                let off: Vec<usize> = (0..a.len())
                    .filter(|&i| (a[i] - b[i]).abs() > floor)
                    .collect();
                if !off.is_empty() {
                    let show: Vec<String> = off
                        .iter()
                        .take(6)
                        .map(|&i| {
                            format!(
                                "({} Pa, {}, {} K: {} here, {} in the file)",
                                file.pressure_pa[i / (nb * nt)],
                                file.blowing[(i / nt) % nb],
                                file.temperature_k[i % nt],
                                a[i],
                                b[i]
                            )
                        })
                        .collect();
                    println!("   {name}: {} nodes differ: {}", off.len(), show.join(" "));
                }
            }
        }
    }

    fn with_gas(choice: GasBesideUnusedOxygen) -> SurfaceOptions {
        SurfaceOptions {
            gas_beside_unused_oxygen: choice,
            ..pica::SURFACE_OPTIONS
        }
    }

    #[test]
    fn version_five_changes_only_the_heat_of_the_gas_beside_unused_oxygen_at_low_pressure() {
        let (_, bundle) = frozen_pica();
        let four = pica::SURFACE_OPTIONS;
        let five = pica::SURFACE_OPTIONS_VERSION_5;
        assert_eq!(four.gas_beside_unused_oxygen, GasBesideUnusedOxygen::Burns);
        assert_eq!(
            five,
            with_gas(GasBesideUnusedOxygen::FollowsTheLowPressureRule)
        );
        let never = with_gas(GasBesideUnusedOxygen::DoesNotBurn);
        let mut differed = 0;
        for &t in &[400.0, 650.0, 800.0, 1100.0, 1600.0, 2400.0, 3100.0] {
            for &p in &[
                101.325, 3000.0, 7000.0, 10_000.0, 13_000.0, 30_000.0, 101_325.0,
            ] {
                for &bg in &[0.0, 0.05, 0.3, 2.0] {
                    for &(c, h) in &[(0.02, 1.0e5), (0.1, 4.0e6), (0.17, 5.0e7)] {
                        let a = bundle.chemistry(&four, t, p, bg, c, h);
                        let b = bundle.chemistry(&five, t, p, bg, c, h);
                        let n = bundle.chemistry(&never, t, p, bg, c, h);
                        // The mass side is not touched.
                        assert_eq!(a.char_rate.to_bits(), b.char_rate.to_bits());
                        assert_eq!(a.char_rate.to_bits(), n.char_rate.to_bits());
                        assert!(b.wall_enthalpy.is_finite() && n.wall_enthalpy.is_finite());
                        if p >= LOW_PRESSURE_RULE_NONE_ABOVE_PA || bg == 0.0 {
                            // No rule above 13 kPa; nothing to burn without gas.
                            assert!(
                                (a.wall_enthalpy - b.wall_enthalpy).abs()
                                    <= 1.0e-9 * a.wall_enthalpy.abs() + 1.0e-6,
                                "{t} {p} {bg}"
                            );
                        } else if a.wall_enthalpy != b.wall_enthalpy {
                            differed += 1;
                        }
                        if p <= LOW_PRESSURE_RULE_FULL_BELOW_PA {
                            assert_eq!(b.wall_enthalpy.to_bits(), n.wall_enthalpy.to_bits());
                        }
                    }
                }
            }
        }
        assert!(differed > 100, "{differed}");
        // A wall too cold to react, at low pressure: the wall gas is the
        // air and the decomposition gas side by side, nothing burnt, and
        // the stream's heating term is what air alone at the wall gives.
        let (t, p, bg, c, h) = (400.0, 3000.0, 0.15, 0.02, 1.0e5);
        let b = bundle.chemistry(&five, t, p, bg, c, h);
        let air = bundle.read(&bundle.wall_enthalpy_no_exchange, p, 0.0, t);
        let apart = air + bg * bundle.gas_alone(p, t);
        assert!(b.char_rate < 1.0e-6, "{}", b.char_rate);
        let total = (1.0 + bg + b.char_rate) * b.wall_enthalpy;
        assert!(
            (total - apart).abs() < 1.0e-4 * apart.abs(),
            "{total} {apart}"
        );
        assert!(air > 0.0 && air < 2.0e5, "{air}");
        // The table as it stands has burnt the gas there: megajoules lower.
        let a = bundle.chemistry(&four, t, p, bg, c, h);
        assert!(
            a.wall_enthalpy < b.wall_enthalpy - 2.0e6,
            "{} {}",
            a.wall_enthalpy,
            b.wall_enthalpy
        );
    }

    /// A slab heated for 20 s, then left under a cold thin flow (0.1 MJ/kg,
    /// 3 kPa) or under no flow at all, to 60 s.
    fn heated_then(coefficient_after: f64) -> SlabQuestion {
        let row = |time_s: f64, c: f64, h: f64| StreamRow {
            time_s,
            pressure_pa: 3000.0,
            heat_transfer_coefficient_kg_per_m2_s: c,
            flow_enthalpy_j_per_kg: h,
            radiative_heating_w_per_m2: 0.0,
        };
        SlabQuestion {
            thickness_m: 0.05,
            start_temperature_k: 300.0,
            surface: SurfaceCondition::GasStream {
                history: vec![
                    row(0.0, 0.0, 2.0e7),
                    row(0.5, 0.1, 2.0e7),
                    row(20.0, 0.1, 2.0e7),
                    row(21.0, coefficient_after, 1.0e5),
                    row(60.0, coefficient_after, 1.0e5),
                ],
                surroundings_temperature_k: 300.0,
                face_wears: true,
                stream_gas: StreamGas::AIR,
            },
            continue_after_s: 0.0,
            thermocouple_depths_m: vec![],
        }
    }

    #[test]
    fn a_cold_flow_cools_a_warm_face_in_version_five_and_heated_it_in_version_four() {
        let end = |name: &str, coefficient_after: f64| {
            let r = solve_material(name, &heated_then(coefficient_after)).expect(name);
            (
                *r.surface_temperature_k.last().unwrap(),
                r.recession_m.last().copied().unwrap(),
            )
        };
        let (five_flow, five_wear) = end(MATERIAL_PICA_VERSION_5, 0.02);
        let (five_none, _) = end(MATERIAL_PICA_VERSION_5, 0.0);
        let (four_flow, four_wear) = end(MATERIAL_PICA_VERSION_4, 0.02);
        let (four_none, _) = end(MATERIAL_PICA_VERSION_4, 0.0);
        eprintln!(
            "face at 60 s: version 5 under the cold flow {five_flow:.0} K, under none {five_none:.0} K; version 4 under the cold flow {four_flow:.0} K, under none {four_none:.0} K; wear {:.3} and {:.3} mm",
            five_wear * 1e3,
            four_wear * 1e3
        );
        assert!(five_none > 500.0 && five_none < 1500.0);
        // The flow is colder than the face, so it cools it.
        assert!(five_flow < five_none - 5.0, "{five_flow} {five_none}");
        // The table as it stands burns the gas in the cold flow and heats the face.
        assert!(four_flow > four_none + 20.0, "{four_flow} {four_none}");
        // With no flow there is no wall gas and the two versions agree.
        assert!(
            (five_none - four_none).abs() < 0.5,
            "{five_none} {four_none}"
        );
    }
}

/// Stage nine: the stream's gas as part of the question.
#[cfg(test)]
mod stream_gas_tests {
    use super::*;

    /// Wall states from the surface work's own Python-made tables for three
    /// streams of the arc-jet tests (air with 27.6 percent argon at 2.3 kPa;
    /// nitrogen with 10 percent oxygen at 18.5 kPa; air with 11.2 percent
    /// argon at 84.4 kPa): oxygen and argon mass fractions, pressure (Pa),
    /// B'g, temperature (K); then B'c, the wall-gas enthalpy (J/kg), the
    /// same two without carbon vapour, and the enthalpy with no carbon
    /// exchanged.
    #[rustfmt::skip]
    const PYTHON_STREAM_STATES: [(f64, f64, f64, f64, f64, f64, f64, f64, f64, f64); 30] = [
        (0.16753359999999998, 0.276, 2300.0, 0.0, 1000.0, 0.12531177045590658, -364459.82646753656, 0.12531177045590725, -364459.82646753656, 646081.5502208854),
        (0.16753359999999998, 0.276, 2300.0, 0.0, 2880.0, 0.1352927853130239, 1954645.2073278776, 0.12970900673428942, 1817746.0805497516, 3853431.2837527497),
        (0.16753359999999998, 0.276, 2300.0, 0.06, 2000.0, 0.1145675598579659, 847929.7263294174, 0.11456755721742523, 847929.6480638103, 86203.25944433127),
        (0.16753359999999998, 0.276, 2300.0, 0.15, 1625.0, 0.09345636149105299, 421250.8606979757, 0.09345636149098971, 421250.8606955576, -680217.0602332472),
        (0.16753359999999998, 0.276, 2300.0, 0.3, 2650.0, 0.09352909255324265, 4103501.340243565, 0.09296437229361826, 4092789.183296643, 3376295.170943061),
        (0.16753359999999998, 0.276, 2300.0, 0.5, 3180.0, 0.5632273404672747, 15682638.081902739, 0.1376947590551243, 11916132.910539174, 11720253.582223069),
        (0.16753359999999998, 0.276, 2300.0, 1.0, 3580.0, f64::INFINITY, 29233967.10578035, 0.417458937946253, 22157630.47542287, 22292571.011843525),
        (0.16753359999999998, 0.276, 2300.0, 3.0, 2980.0, 0.0, 18854727.95942003, 0.0, 18256681.694688387, 18870728.25529012),
        (0.16753359999999998, 0.276, 2300.0, 10.0, 1750.0, 0.0, 1861979.009424148, 0.0, 1861979.009110169, 3712021.711398651),
        (0.16753359999999998, 0.276, 2300.0, 0.1, 3930.0, f64::INFINITY, 28589876.066774167, 0.5022553351177189, 14143997.745782223, 9081894.247411614),
        (0.1, 0.0, 18500.0, 0.0, 1000.0, 0.0738644662548098, 107931.62229805344, 0.07386446625480958, 107931.62229805354, 760558.0196875171),
        (0.1, 0.0, 18500.0, 0.0, 2880.0, 0.07789675891037029, 2596542.5774806873, 0.07718658656938526, 2578711.4739369643, 3573697.929326772),
        (0.1, 0.0, 18500.0, 0.06, 2000.0, 0.0643799375148349, 1470898.8352072493, 0.06437993717819507, 1470898.824973458, 670759.8442051887),
        (0.1, 0.0, 18500.0, 0.15, 1625.0, 0.04289188324641749, 957277.0773782245, 0.042891883246408385, 957277.0773779091, 476718.4476434359),
        (0.1, 0.0, 18500.0, 0.3, 2650.0, 0.05226775517492804, 3860395.981097638, 0.05219947238488981, 3859063.51010476, 3422021.811352932),
        (0.1, 0.0, 18500.0, 0.5, 3180.0, 0.1675777163726606, 9740661.177927999, 0.12507137560698323, 9204456.649155764, 8760847.8009364),
        (0.1, 0.0, 18500.0, 1.0, 3580.0, 4.244163912715225, 25668513.478802435, 0.37360746091894015, 18390693.847012118, 18870663.913662687),
        (0.1, 0.0, 18500.0, 3.0, 2980.0, 0.0, 11678718.601417996, 0.0, 11582084.87029284, 11793411.220388984),
        (0.1, 0.0, 18500.0, 10.0, 1750.0, 0.0, 1925665.7896108604, 0.0, 1925665.7895716457, 3781050.6253078138),
        (0.1, 0.0, 18500.0, 0.1, 3930.0, f64::INFINITY, 28980587.08620349, 0.3553506073051783, 12841450.69765803, 8524882.287995234),
        (0.2054832, 0.112, 84400.0, 0.0, 1000.0, 0.13865969122055644, -715000.344159233, 0.13865969122055644, -715000.344159233, 709709.3399216288),
        (0.2054832, 0.112, 84400.0, 0.0, 2880.0, 0.15545135194743676, 1786824.503568593, 0.15528937868367687, 1782919.702137394, 3281026.6820750097),
        (0.2054832, 0.112, 84400.0, 0.06, 2000.0, 0.14326643281682278, 791520.9699517789, 0.1432664327403071, 791520.9677352274, 205514.1653715803),
        (0.2054832, 0.112, 84400.0, 0.15, 1625.0, 0.1219902193383954, 347154.17663003696, 0.1219902193383943, 347154.176629969, -1105825.5916014174),
        (0.2054832, 0.112, 84400.0, 0.3, 2650.0, 0.1279691890325379, 2831383.004506156, 0.12795406931282344, 2831093.828968895, 1605840.7154046586),
        (0.2054832, 0.112, 84400.0, 0.5, 3180.0, 0.2117379847908607, 6753418.5381360855, 0.20295211041611205, 6633836.133269315, 5507091.023584248),
        (0.2054832, 0.112, 84400.0, 1.0, 3580.0, 0.9425117857668246, 16358676.008623049, 0.4412281954268842, 13488236.76905695, 12896598.924366895),
        (0.2054832, 0.112, 84400.0, 3.0, 2980.0, 0.0, 8742447.614703655, 0.0, 8720123.934372123, 8847315.730372341),
        (0.2054832, 0.112, 84400.0, 10.0, 1750.0, 0.0, 1820290.600914611, 0.0, 1820290.6009060624, 3480431.1725612944),
        (0.2054832, 0.112, 84400.0, 0.1, 3930.0, 23.580181040312326, 28192667.243210196, 0.3161038912402053, 8678773.605592191, 8539679.555942137),
    ];

    fn across(pressure_pa: &[f64]) -> Vec<StreamRow> {
        pressure_pa
            .iter()
            .enumerate()
            .map(|(row, &pressure_pa)| StreamRow {
                time_s: row as f64,
                pressure_pa,
                heat_transfer_coefficient_kg_per_m2_s: 0.0,
                flow_enthalpy_j_per_kg: 0.0,
                radiative_heating_w_per_m2: 0.0,
            })
            .collect()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1.0e-10 * b.abs() + 1.0e-5
    }

    #[test]
    fn the_surface_computed_here_is_the_surface_works_at_thirty_states_of_three_streams() {
        let (_, locked) = frozen_pica();
        let nt = locked.temperature_k.len();
        for &(oxygen, argon, p, bg, t, bc, hw, bc_nv, hw_nv, h_none) in &PYTHON_STREAM_STATES {
            let gas = StreamGas {
                oxygen_mass_fraction: oxygen,
                argon_mass_fraction: argon,
            };
            let made = locked.for_stream(&gas, &across(&[p])).expect("computed");
            assert_eq!(made.pressure_pa, vec![p]);
            let ib = locked.blowing.iter().position(|&b| b == bg).unwrap();
            let it = locked.temperature_k.iter().position(|&x| x == t).unwrap();
            let at = ib * nt + it;
            let share = |rate: f64| {
                if rate.is_finite() {
                    rate / (1.0 + bg + rate)
                } else {
                    1.0
                }
            };
            assert!(
                (made.share[at] - share(bc)).abs() < 1.0e-12,
                "{oxygen} {bg} {t}"
            );
            assert!(
                (made.share_without_vapour[at] - share(bc_nv)).abs() < 1.0e-12,
                "{oxygen} {bg} {t}"
            );
            assert!(close(made.wall_enthalpy[at], hw), "{oxygen} {bg} {t}");
            assert!(
                close(made.wall_enthalpy_without_vapour[at], hw_nv),
                "{oxygen} {bg} {t}"
            );
            assert!(
                close(made.wall_enthalpy_no_exchange[at], h_none),
                "{oxygen} {bg} {t}"
            );
        }
    }

    /// Rows of the locked air table and the same rows computed here for the
    /// gas given, state by state: the largest difference in the char's share
    /// of the wall gas, in the two wall-gas enthalpies (J/kg), and the
    /// states at which the no-exchange enthalpy differs by more than 1 J/kg.
    fn against_locked(gas: &StreamGas, lowest: f64, highest: f64) -> (f64, f64, Vec<(f64, f64)>) {
        let (_, locked) = frozen_pica();
        let made = locked
            .for_stream(gas, &across(&[lowest, highest]))
            .expect("computed");
        let (nb, nt) = (locked.blowing.len(), locked.temperature_k.len());
        let first = locked
            .pressure_pa
            .iter()
            .position(|&p| (p / lowest - 1.0).abs() < 1.0e-9)
            .unwrap();
        let rows = made.pressure_pa.len();
        for (row, &p) in made.pressure_pa.iter().enumerate() {
            assert!((p / locked.pressure_pa[first + row] - 1.0).abs() < 1.0e-9);
        }
        let (mut share, mut enthalpy, mut cold) = (0.0f64, 0.0f64, Vec::new());
        for at in 0..rows * nb * nt {
            let there = first * nb * nt + at;
            share = share
                .max((made.share[at] - locked.share[there]).abs())
                .max((made.share_without_vapour[at] - locked.share_without_vapour[there]).abs());
            enthalpy = enthalpy
                .max((made.wall_enthalpy[at] - locked.wall_enthalpy[there]).abs())
                .max(
                    (made.wall_enthalpy_without_vapour[at]
                        - locked.wall_enthalpy_without_vapour[there])
                        .abs(),
                );
            if (made.wall_enthalpy_no_exchange[at] - locked.wall_enthalpy_no_exchange[there]).abs()
                > 1.0
            {
                cold.push((
                    locked.blowing[(at / nt) % nb],
                    locked.temperature_k[at % nt],
                ));
            }
        }
        for at in 0..rows * nt {
            assert!(
                (made.gas_alone_enthalpy[at] - locked.gas_alone_enthalpy[first * nt + at]).abs()
                    < 1.0e-4
            );
        }
        (share, enthalpy, cold)
    }

    #[test]
    fn air_computed_here_is_the_locked_table_at_its_main_pressures() {
        // 0.07, 0.1 and 0.13 atm: 18,954 states.
        let (share, enthalpy, cold) = against_locked(&StreamGas::AIR, 7092.75, 13172.25);
        eprintln!(
            "air computed here against the locked table at 0.07 to 0.13 atm: largest difference in the char's share {share:e}, in the wall-gas enthalpies {enthalpy:e} J/kg; no-exchange enthalpy differs at {} cold states",
            cold.len()
        );
        assert!(share < 1.0e-12, "{share}");
        assert!(enthalpy < 1.0e-5, "{enthalpy}");
        // The few states the method does not converge at, carried from
        // their neighbours: cold, with about a tenth as much gas as stream.
        assert!(cold.len() < 60, "{}", cold.len());
        assert!(
            cold.iter()
                .all(|&(bg, t)| t <= 500.0 && (0.04..=0.3).contains(&bg)),
            "{cold:?}"
        );
    }

    /// The locked air table was put together from two computations that did
    /// not use quite the same air: its three lowest pressures (1 to 30 Pa)
    /// and its atomic-oxygen table were made for 23.14 percent oxygen by
    /// mass, the other twenty pressures for 23.29. It is kept as it is so
    /// that every recorded air run repeats. This test holds the fact.
    #[test]
    fn the_locked_air_table_has_three_low_pressure_rows_made_for_slightly_less_oxygen() {
        let low = StreamGas::air_with_argon(0.0);
        assert_eq!(low.oxygen_mass_fraction, 0.2314);
        let (share, enthalpy, _) = against_locked(&low, 1.01325, 1.01325);
        assert!(share < 1.0e-12 && enthalpy < 1.0e-5, "{share} {enthalpy}");
        let (share, _, _) = against_locked(&StreamGas::AIR, 1.01325, 1.01325);
        assert!(share > 3.0e-4 && share < 2.0e-3, "{share}");
        // The atomic-oxygen table.
        let (_, locked) = frozen_pica();
        let made = locked.for_stream(&low, &across(&[1.01325])).unwrap();
        let worst = made
            .atom_fraction
            .iter()
            .zip(&locked.atom_fraction)
            .fold(0.0f64, |w, (a, b)| w.max((a - b).abs()));
        assert!(worst < 1.0e-12, "{worst}");
    }

    fn in_stream(gas: StreamGas, pressure_pa: f64) -> SlabQuestion {
        let row = |time_s: f64, on: f64| StreamRow {
            time_s,
            pressure_pa,
            heat_transfer_coefficient_kg_per_m2_s: 0.1 * on,
            flow_enthalpy_j_per_kg: 2.0e7,
            radiative_heating_w_per_m2: 0.0,
        };
        SlabQuestion {
            thickness_m: 0.03,
            start_temperature_k: 300.0,
            surface: SurfaceCondition::GasStream {
                history: vec![row(0.0, 0.0), row(0.5, 1.0), row(4.0, 1.0)],
                surroundings_temperature_k: 300.0,
                face_wears: true,
                stream_gas: gas,
            },
            continue_after_s: 0.0,
            thermocouple_depths_m: vec![0.002],
        }
    }

    #[test]
    fn less_oxygen_in_the_stream_wears_the_face_less() {
        // A study of the physics beyond what was compared: 0.03 m.
        let wear = |gas: StreamGas| {
            let a = answer_beyond_compared(
                MATERIAL_PICA_SERVED,
                &in_stream(gas, 20_000.0),
                &Wanted::default(),
            )
            .expect("answer");
            let s = a.summary;
            assert!(
                s.energy_account_residual_j_per_m2.abs() <= s.energy_account_tolerance_j_per_m2
            );
            assert!(s.mass_account_residual_kg_per_m2.abs() <= s.mass_account_tolerance_kg_per_m2);
            (s.wear_m, s.peak_face_temperature_k)
        };
        let (air, air_face) = wear(StreamGas::AIR);
        let (argon, argon_face) = wear(StreamGas::air_with_argon(0.276));
        let (lean, _) = wear(StreamGas {
            oxygen_mass_fraction: 0.10,
            argon_mass_fraction: 0.0,
        });
        let (rich, _) = wear(StreamGas {
            oxygen_mass_fraction: 0.30,
            argon_mass_fraction: 0.0,
        });
        eprintln!(
            "wear in 4 s at 20 kPa: air {:.4} mm, air with 27.6 % argon {:.4}, nitrogen with 10 % oxygen {:.4}, with 30 % {:.4}; face {air_face:.0} and {argon_face:.0} K",
            air * 1e3,
            argon * 1e3,
            lean * 1e3,
            rich * 1e3
        );
        assert!(lean < argon && argon < air && air < rich);
        // The stream with argon has 0.72 of air's oxygen. Over these first
        // four seconds, with the face still heating, it wears 0.58 as much.
        let ratio = argon / air;
        assert!(ratio > 0.5 && ratio < 0.75, "{ratio}");
        // The same question again gives the same bits (the kept table).
        let again = wear(StreamGas::air_with_argon(0.276));
        assert_eq!(again.0.to_bits(), argon.to_bits());
    }

    #[test]
    fn a_stream_a_hair_from_air_answers_as_air_computed_afresh_and_nearly_as_the_locked_table() {
        let afresh = SurfaceOptions {
            air_table_computed_afresh: true,
            ..crate::pica::SURFACE_OPTIONS_VERSION_5
        };
        let v5 = crate::pica::VERSION_4;
        let q = in_stream(StreamGas::AIR, 20_000.0);
        let locked = solve_pica_study(&v5, &crate::pica::SURFACE_OPTIONS_VERSION_5, None, &q)
            .expect("locked");
        let served = solve_material(MATERIAL_PICA_SERVED, &q).expect("served");
        assert_eq!(
            served.recession_m.last().unwrap().to_bits(),
            locked.recession_m.last().unwrap().to_bits()
        );
        let computed = solve_pica_study(&v5, &afresh, None, &q).expect("computed");
        let hair = StreamGas {
            oxygen_mass_fraction: AIR_OXYGEN_MASS_FRACTION * (1.0 + 1.0e-12),
            argon_mass_fraction: 0.0,
        };
        assert!(!hair.is_air());
        let beside =
            solve_material(MATERIAL_PICA_SERVED, &in_stream(hair, 20_000.0)).expect("beside air");
        let end = |r: &SlabResult| {
            (
                *r.recession_m.last().unwrap(),
                *r.surface_temperature_k.last().unwrap(),
            )
        };
        let (a, b, c) = (end(&locked), end(&computed), end(&beside));
        eprintln!(
            "air at 20 kPa for 4 s: locked table wear {:.6} mm, face {:.3} K; computed afresh {:.6} mm, {:.3} K",
            a.0 * 1e3,
            a.1,
            b.0 * 1e3,
            b.1
        );
        // Computed at the question's own pressure against read between the
        // table's 0.16 and 0.2 atm rows: the difference is the table's
        // straight line in the logarithm of pressure.
        assert!(
            (b.0 / a.0 - 1.0).abs() < 1.0e-5 && (b.1 - a.1).abs() < 0.01,
            "{a:?} {b:?}"
        );
        assert!(
            (c.0 / b.0 - 1.0).abs() < 1.0e-9 && (c.1 - b.1).abs() < 1.0e-6,
            "{b:?} {c:?}"
        );
    }

    #[test]
    fn the_stream_gas_is_checked_before_anything_is_computed() {
        let gas = |oxygen: f64, argon: f64| StreamGas {
            oxygen_mass_fraction: oxygen,
            argon_mass_fraction: argon,
        };
        for (g, name) in [
            (gas(0.049, 0.0), "StreamOxygenOutOfRange"),
            (gas(0.401, 0.0), "StreamOxygenOutOfRange"),
            (gas(f64::NAN, 0.0), "StreamOxygenOutOfRange"),
            (gas(0.2, -1.0e-9), "StreamArgonOutOfRange"),
            (gas(0.2, 0.501), "StreamArgonOutOfRange"),
            (gas(0.2, f64::NAN), "StreamArgonOutOfRange"),
        ] {
            for material in MATERIAL_NAMES {
                let q = in_stream(g, 20_000.0);
                let checked = check_question(material, &q).expect_err("outside");
                assert_eq!(checked.name(), name, "{material} {g:?}");
                let answered = answer(material, &q, &Wanted::default()).expect_err("outside");
                assert_eq!(answered.name(), name, "{material} {g:?}");
            }
            if !g.oxygen_mass_fraction.is_nan() && !g.argon_mass_fraction.is_nan() {
                let cold = stream_enthalpy_at_cold_wall_j_per_kg(&g).expect_err("outside");
                assert_eq!(cold.name(), name);
            }
        }
        // TACOT's published table is for air alone.
        let argon = in_stream(StreamGas::air_with_argon(0.1), 20_000.0);
        let refusal = SlabReject::StreamNotAirForThisMaterial {
            stream_gas: StreamGas::air_with_argon(0.1),
        };
        assert_eq!(check_question(MATERIAL_TACOT, &argon), Err(refusal.clone()));
        assert_eq!(
            answer(MATERIAL_TACOT, &argon, &Wanted::default()).expect_err("not air"),
            refusal
        );
        assert!(refusal.to_string().contains("air alone"));
        assert_eq!(
            check_question(MATERIAL_TACOT, &in_stream(StreamGas::AIR, 20_000.0)),
            Ok(())
        );
        // Every PICA version's physics takes it, at the ends of the range
        // too; the served entry refuses this 0.03 m slab as never compared.
        for material in &MATERIAL_NAMES[1..] {
            for g in [
                gas(STREAM_OXYGEN_MASS_FRACTION_MIN, 0.0),
                gas(
                    STREAM_OXYGEN_MASS_FRACTION_MAX,
                    STREAM_ARGON_MASS_FRACTION_MAX,
                ),
            ] {
                assert_eq!(check_beyond_compared(material, &in_stream(g, 20_000.0)), Ok(()));
                // 0.03 m lies in the thermocouple blocks' thicknesses, but
                // its start, gas and history are not theirs.
                assert_eq!(
                    check_question(material, &in_stream(g, 20_000.0)).unwrap_err().name(),
                    "QuestionNotCompared"
                );
            }
        }
        // A held face has no stream and no gas to state.
        assert!(StreamGas::AIR.is_air() && !StreamGas::air_with_argon(0.0).is_air());
    }

    #[test]
    fn a_history_over_several_pressures_gets_a_table_at_its_own_ends_and_the_grid_between() {
        let (_, locked) = frozen_pica();
        let gas = StreamGas::air_with_argon(0.15);
        let history = across(&[3000.0, 9000.0, 500.0, 2000.0]);
        let made = locked.for_stream(&gas, &history).expect("computed");
        // 0.01, 0.02, 0.03, 0.05 and 0.07 atm lie between 500 and 9,000 Pa.
        let expected = [500.0, 1013.25, 2026.5, 3039.75, 5066.25, 7092.75, 9000.0];
        assert_eq!(made.pressure_pa.len(), expected.len());
        for (p, e) in made.pressure_pa.iter().zip(&expected) {
            assert!((p / e - 1.0).abs() < 1.0e-12, "{p} {e}");
        }
        assert_eq!((made.pressure_pa[0], made.pressure_pa[6]), (500.0, 9000.0));
        // Read at a pressure of the history, the table is the one computed
        // at that pressure alone: nothing is carried in from outside.
        let alone = locked.for_stream(&gas, &across(&[9000.0])).expect("alone");
        let nodes = locked.blowing.len() * locked.temperature_k.len();
        let last = 6 * nodes;
        for at in 0..nodes {
            assert_eq!(made.share[last + at].to_bits(), alone.share[at].to_bits());
            assert_eq!(
                made.wall_enthalpy[last + at].to_bits(),
                alone.wall_enthalpy[at].to_bits()
            );
        }
        let q = SlabQuestion {
            thickness_m: 0.03,
            start_temperature_k: 300.0,
            surface: SurfaceCondition::GasStream {
                history: vec![
                    StreamRow {
                        time_s: 0.0,
                        pressure_pa: 500.0,
                        heat_transfer_coefficient_kg_per_m2_s: 0.0,
                        flow_enthalpy_j_per_kg: 3.0e7,
                        radiative_heating_w_per_m2: 0.0,
                    },
                    StreamRow {
                        time_s: 3.0,
                        pressure_pa: 9000.0,
                        heat_transfer_coefficient_kg_per_m2_s: 0.08,
                        flow_enthalpy_j_per_kg: 2.0e7,
                        radiative_heating_w_per_m2: 0.0,
                    },
                    StreamRow {
                        time_s: 6.0,
                        pressure_pa: 2000.0,
                        heat_transfer_coefficient_kg_per_m2_s: 0.0,
                        flow_enthalpy_j_per_kg: 1.0e7,
                        radiative_heating_w_per_m2: 0.0,
                    },
                ],
                surroundings_temperature_k: 300.0,
                face_wears: true,
                stream_gas: gas,
            },
            continue_after_s: 0.0,
            thermocouple_depths_m: vec![],
        };
        let a = answer_beyond_compared(MATERIAL_PICA_SERVED, &q, &Wanted::default()).expect("answer");
        assert!(a.summary.wear_m > 0.0 && a.summary.peak_face_temperature_k > 1500.0);
    }
}
