// font-kit/src/font.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt::{self, Debug, Formatter};

pub use loader::Font;

impl Debug for Font {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        self.descriptor().fmt(fmt)
    }
}

/// Various metrics that apply to the entire font.
///
/// For OpenType fonts, these mostly come from the `OS/2` table.
#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    /// The number of font units per em.
    ///
    /// Font sizes are usually expressed in pixels per em; e.g. `12px` means 12 pixels per em.
    pub units_per_em: u32,
    /// The maximum amount the font rises above the baseline, in font units.
    pub ascent: f32,
    /// The maximum amount the font descends below the baseline, in font units.
    ///
    /// NB: This is typically a negative value to match the definition of `sTypoDescender` in the
    /// `OS/2` table in the OpenType specification. If you are used to using Windows or Mac APIs,
    /// beware, as the sign is reversed.
    pub descent: f32,
    /// Distance between baselines, in font units.
    pub line_gap: f32,
    pub underline_position: f32,
    pub underline_thickness: f32,
    /// The approximate amount that uppercase letters rise above the baseline, in font units.
    pub cap_height: f32,
    /// The approximate amount that non-ascending lowercase letters rise above the baseline, in
    /// font units.
    pub x_height: f32,
}
