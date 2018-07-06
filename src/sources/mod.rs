// font-kit/src/sources/mod.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[cfg(target_os = "macos")]
pub use sources::core_text as default;
#[cfg(target_family = "windows")]
pub use sources::directwrite as default;
#[cfg(not(any(target_os = "macos", target_family = "windows")))]
pub use sources::fontconfig as default;

#[cfg(target_os = "macos")]
pub mod core_text;
#[cfg(target_family = "windows")]
pub mod directwrite;
#[cfg(any(not(any(target_os = "macos", target_family = "windows")),
          feature = "source-fontconfig"))]
pub mod fontconfig;
