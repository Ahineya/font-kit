// font-kit/src/loaders/freetype.rs
//
// Copyright © 2018 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A cross-platform loader that uses the FreeType library to load and rasterize fonts.
//!
//! On macOS and Windows, the Cargo feature `loader-freetype-default` can be used to opt into this
//! loader by default.

use byteorder::{BigEndian, ReadBytesExt};

use log::warn;
use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry::rect::{RectF, RectI};
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::vector::{Vector2F, Vector2I};
use pathfinder_simd::default::F32x4;
use std::f32;
use std::ffi::{CStr, CString};
use std::fmt::{self, Debug, Formatter};
use std::io::{Seek, SeekFrom};
use std::iter;
use std::mem;
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::slice;
use std::sync::Arc;

use crate::canvas::{Canvas, Format, RasterizationOptions};
use crate::error::{FontLoadingError, GlyphLoadingError};
use crate::file_type::FileType;
use crate::handle::Handle;
use crate::hinting::HintingOptions;
use crate::loader::{FallbackResult, Loader};
use crate::metrics::Metrics;
use crate::outline::OutlineSink;
use crate::properties::{Properties, Stretch, Style, Weight};
use crate::utils;

#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

const PS_DICT_FULL_NAME: u32 = 38;
const TT_NAME_ID_FULL_NAME: u16 = 4;

const TT_PLATFORM_APPLE_UNICODE: u16 = 0;

const FT_POINT_TAG_ON_CURVE: c_char = 0x01;
const FT_POINT_TAG_CUBIC_CONTROL: c_char = 0x02;

const OS2_FS_SELECTION_OBLIQUE: u16 = 1 << 9;

// Not in our FreeType bindings, so we define these ourselves.
#[allow(dead_code)]
const BDF_PROPERTY_TYPE_NONE: BDF_PropertyType = 0;
#[allow(dead_code)]
const BDF_PROPERTY_TYPE_ATOM: BDF_PropertyType = 1;
#[allow(dead_code)]
const BDF_PROPERTY_TYPE_INTEGER: BDF_PropertyType = 2;
#[allow(dead_code)]
const BDF_PROPERTY_TYPE_CARDINAL: BDF_PropertyType = 3;

// thread_local! {
//     static FREETYPE_LIBRARY: FtLibrary = {
//         unsafe {
//             let mut library = ptr::null_mut();
//             assert_eq!(FT_Init_FreeType(&mut library), 0);
//             FT_Library_SetLcdFilter(library, FT_LCD_FILTER_DEFAULT);
//             FtLibrary(library)
//         }
//     };
// }

// #[repr(transparent)]
// struct FtLibrary(FT_Library);
// 
// impl Drop for FtLibrary {
//     fn drop(&mut self) {
//         unsafe {
//             let mut library = ptr::null_mut();
//             mem::swap(&mut library, &mut self.0);
//             FT_Done_FreeType(library);
//         }
//     }
// }

type FT_Face = Option<String>;

/// The handle that the FreeType API natively uses to represent a font.
/// 
/// 
/// 
pub type NativeFont = FT_Face;

// Not in our FreeType bindings, so we define this ourselves.
#[allow(non_camel_case_types)]
type BDF_PropertyType = i32;

// Not in our FreeType bindings, so we define this ourselves.
#[repr(C)]
struct BDF_PropertyRec {
    property_type: BDF_PropertyType,
    value: *const c_char,
}

/// A cross-platform loader that uses the FreeType library to load and rasterize fonts.
///
/// On macOS and Windows, the Cargo feature `loader-freetype-default` can be used to opt into this
/// loader by default.
pub struct Font {
    freetype_face: FT_Face,
    font_data: Arc<Vec<u8>>,
}

impl Font {
    /// Loads a font from raw font data (the contents of a `.ttf`/`.otf`/etc. file).
    ///
    /// If the data represents a collection (`.ttc`/`.otc`/etc.), `font_index` specifies the index
    /// of the font to load from it. If the data represents a single font, pass 0 for `font_index`.
    pub fn from_bytes(font_data: Arc<Vec<u8>>, font_index: u32) -> Result<Font, FontLoadingError> {
        Err(FontLoadingError::NotImplemented)
    }

    /// Loads a font from a `.ttf`/`.otf`/etc. file.
    ///
    /// If the file is a collection (`.ttc`/`.otc`/etc.), `font_index` specifies the index of the
    /// font to load from it. If the file represents a single font, pass 0 for `font_index`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_file(file: &mut File, font_index: u32) -> Result<Font, FontLoadingError> {
        Err(FontLoadingError::NotImplemented)
    }

    /// Loads a font from the path to a `.ttf`/`.otf`/etc. file.
    ///
    /// If the file is a collection (`.ttc`/`.otc`/etc.), `font_index` specifies the index of the
    /// font to load from it. If the file represents a single font, pass 0 for `font_index`.
    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_path<P>(path: P, font_index: u32) -> Result<Font, FontLoadingError>
    where
        P: AsRef<Path>,
    {
        Err(FontLoadingError::NotImplemented)
    }

    /// Creates a font from a native API handle.
    pub unsafe fn from_native_font(freetype_face: NativeFont) -> Font {
        Font {
            freetype_face,
            font_data: Arc::new(Vec::new()),
        }
    }

    /// Loads the font pointed to by a handle.
    #[inline]
    pub fn from_handle(handle: &Handle) -> Result<Self, FontLoadingError> {
        Err(FontLoadingError::NotImplemented)
    }

    /// Determines whether a blob of raw font data represents a supported font, and, if so, what
    /// type of font it is.
    pub fn analyze_bytes(font_data: Arc<Vec<u8>>) -> Result<FileType, FontLoadingError> {
        Err(FontLoadingError::NotImplemented)
    }

    /// Determines whether a file represents a supported font, and, if so, what type of font it is.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn analyze_file(file: &mut File) -> Result<FileType, FontLoadingError> {
        Err(FontLoadingError::NotImplemented)
    }

    /// Determines whether a path points to a supported font, and, if so, what type of font it is.
    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn analyze_path<P>(path: P) -> Result<FileType, FontLoadingError>
    where
        P: AsRef<Path>,
    {
        <Self as Loader>::analyze_path(path)
    }

    /// Returns the wrapped native font handle.
    ///
    /// This function increments the reference count of the FreeType face before returning it.
    /// Therefore, it is the caller's responsibility to free it with `FT_Done_Face`.
    pub fn native_font(&self) -> NativeFont {
        None
    }

    /// Returns the PostScript name of the font. This should be globally unique.
    pub fn postscript_name(&self) -> Option<String> {
        Some("not implemented".to_string())
    }

    /// Returns the full name of the font (also known as "display name" on macOS).
    pub fn full_name(&self) -> String {
        "not implemented".to_string()
    }

    /// Returns the name of the font family.
    pub fn family_name(&self) -> String {
        "not implemented".to_string()
    }

    /// Returns true if and only if the font is monospace (fixed-width).
    pub fn is_monospace(&self) -> bool {
        false
    }

    /// Returns the values of various font properties, corresponding to those defined in CSS.
    pub fn properties(&self) -> Properties {
        let mut properties = Properties::default();

        warn!("unimplemented");

        properties
    }

    /// Returns the usual glyph ID for a Unicode character.
    ///
    /// Be careful with this function; typographically correct character-to-glyph mapping must be
    /// done using a *shaper* such as HarfBuzz. This function is only useful for best-effort simple
    /// use cases like "what does character X look like on its own".
    #[inline]
    pub fn glyph_for_char(&self, character: char) -> Option<u32> {
        None
    }

    /// Returns the glyph ID for the specified glyph name.
    #[inline]
    pub fn glyph_by_name(&self, name: &str) -> Option<u32> {
        None
    }

    /// Returns the number of glyphs in the font.
    ///
    /// Glyph IDs range from 0 inclusive to this value exclusive.
    #[inline]
    pub fn glyph_count(&self) -> u32 {
        0
    }

    /// Sends the vector path for a glyph to a path builder.
    ///
    /// If `hinting_mode` is not None, this function performs grid-fitting as requested before
    /// sending the hinding outlines to the builder.
    ///
    /// TODO(pcwalton): What should we do for bitmap glyphs?
    pub fn outline<S>(
        &self,
        glyph_id: u32,
        hinting: HintingOptions,
        sink: &mut S,
    ) -> Result<(), GlyphLoadingError>
    where
        S: OutlineSink,
    {
        warn!("unimplemented");
        Ok(())
    }

    /// Returns the boundaries of a glyph in font units.
    pub fn typographic_bounds(&self, glyph_id: u32) -> Result<RectF, GlyphLoadingError> {
        warn!("unimplemented");
        Ok(RectF::default())
    }

    /// Returns the distance from the origin of the glyph with the given ID to the next, in font
    /// units.
    pub fn advance(&self, glyph_id: u32) -> Result<Vector2F, GlyphLoadingError> {
        warn!("unimplemented");
        Ok(Vector2F::default())
    }

    /// Returns the amount that the given glyph should be displaced from the origin.
    ///
    /// FIXME(pcwalton): This always returns zero on FreeType.
    pub fn origin(&self, _: u32) -> Result<Vector2F, GlyphLoadingError> {
        warn!("unimplemented");
        Ok(Vector2F::default())
    }

    /// Retrieves various metrics that apply to the entire font.
    pub fn metrics(&self) -> Metrics {
        warn!("unimplemented");
        Metrics::default()
    }

    /// Returns true if and only if the font loader can perform hinting in the requested way.
    ///
    /// Some APIs support only rasterizing glyphs with hinting, not retrieving hinted outlines. If
    /// `for_rasterization` is false, this function returns true if and only if the loader supports
    /// retrieval of hinted *outlines*. If `for_rasterization` is true, this function returns true
    /// if and only if the loader supports *rasterizing* hinted glyphs.
    #[inline]
    pub fn supports_hinting_options(
        &self,
        hinting_options: HintingOptions,
        for_rasterization: bool,
    ) -> bool {
        false
    }

    fn get_type_1_or_sfnt_name(&self, type_1_id: u32, sfnt_id: u16) -> Option<String> {
        None
    }

    fn get_os2_table(&self) -> Option<String> {
        None
    }

    /// Returns the pixel boundaries that the glyph will take up when rendered using this loader's
    /// rasterizer at the given size and origin.
    #[inline]
    pub fn raster_bounds(
        &self,
        glyph_id: u32,
        point_size: f32,
        transform: Transform2F,
        hinting_options: HintingOptions,
        rasterization_options: RasterizationOptions,
    ) -> Result<RectI, GlyphLoadingError> {
        warn!("unimplemented");
        Ok(RectI::default())
    }

    /// Rasterizes a glyph to a canvas with the given size and origin.
    ///
    /// Format conversion will be performed if the canvas format does not match the rasterization
    /// options. For example, if bilevel (black and white) rendering is requested to an RGBA
    /// surface, this function will automatically convert the 1-bit raster image to the 32-bit
    /// format of the canvas. Note that this may result in a performance penalty, depending on the
    /// loader.
    ///
    /// If `hinting_options` is not None, the requested grid fitting is performed.
    pub fn rasterize_glyph(
        &self,
        canvas: &mut Canvas,
        glyph_id: u32,
        point_size: f32,
        transform: Transform2F,
        hinting_options: HintingOptions,
        rasterization_options: RasterizationOptions,
    ) -> Result<(), GlyphLoadingError> {
        // TODO(pcwalton): This is woefully incomplete. See WebRender's code for a more complete
        // implementation.
        warn!("unimplemented");
        Ok(())
    }

    fn hinting_and_rasterization_options_to_load_flags(
        &self,
        hinting: HintingOptions,
        rasterization: RasterizationOptions,
    ) -> i32 {
        0
    }

    /// Returns a handle to this font, if possible.
    ///
    /// This is useful if you want to open the font with a different loader.
    #[inline]
    pub fn handle(&self) -> Option<Handle> {
        <Self as Loader>::handle(self)
    }

    /// Attempts to return the raw font data (contents of the font file).
    ///
    /// If this font is a member of a collection, this function returns the data for the entire
    /// collection.
    pub fn copy_font_data(&self) -> Option<Arc<Vec<u8>>> {
        Some(self.font_data.clone())
    }

    /// Get font fallback results for the given text and locale.
    ///
    /// Note: this is currently just a stub implementation, a proper implementation
    /// would likely use FontConfig, at least on Linux. It's not clear what a
    /// FreeType loader with a non-FreeType source should do.
    fn get_fallbacks(&self, text: &str, _locale: &str) -> FallbackResult<Font> {
        warn!("unsupported");
        FallbackResult {
            fonts: Vec::new(),
            valid_len: text.len(),
        }
    }

    /// Returns the raw contents of the OpenType table with the given tag.
    ///
    /// Tags are four-character codes. A list of tags can be found in the [OpenType specification].
    ///
    /// [OpenType specification]: https://docs.microsoft.com/en-us/typography/opentype/spec/
    pub fn load_font_table(&self, table_tag: u32) -> Option<Box<[u8]>> {
        None
    }
}

impl Clone for Font {
    fn clone(&self) -> Font {
        unsafe {
            // assert_eq!(FT_Reference_Face(self.freetype_face), 0);
            Font {
                freetype_face: self.freetype_face.clone(),
                font_data: self.font_data.clone(),
            }
        }
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        // The AccessError can be ignored, as it means FREETYPE_LIBRARY has already been
        // destroyed, and it already destroys all FreeType resources.
        // https://freetype.org/freetype2/docs/reference/ft2-module_management.html#ft_done_library
        
    }
}

impl Debug for Font {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        self.family_name().fmt(fmt)
    }
}

impl Loader for Font {
    type NativeFont = NativeFont;

    #[inline]
    fn from_bytes(font_data: Arc<Vec<u8>>, font_index: u32) -> Result<Self, FontLoadingError> {
        Font::from_bytes(font_data, font_index)
    }

    #[inline]
    #[cfg(not(target_arch = "wasm32"))]
    fn from_file(file: &mut File, font_index: u32) -> Result<Font, FontLoadingError> {
        Font::from_file(file, font_index)
    }

    #[inline]
    fn analyze_bytes(font_data: Arc<Vec<u8>>) -> Result<FileType, FontLoadingError> {
        Font::analyze_bytes(font_data)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn analyze_file(file: &mut File) -> Result<FileType, FontLoadingError> {
        Font::analyze_file(file)
    }

    #[inline]
    fn native_font(&self) -> Self::NativeFont {
        self.native_font()
    }

    #[inline]
    unsafe fn from_native_font(native_font: Self::NativeFont) -> Self {
        Font::from_native_font(native_font)
    }

    #[inline]
    fn postscript_name(&self) -> Option<String> {
        self.postscript_name()
    }

    #[inline]
    fn full_name(&self) -> String {
        self.full_name()
    }

    #[inline]
    fn family_name(&self) -> String {
        self.family_name()
    }

    #[inline]
    fn is_monospace(&self) -> bool {
        self.is_monospace()
    }

    #[inline]
    fn properties(&self) -> Properties {
        self.properties()
    }

    #[inline]
    fn glyph_for_char(&self, character: char) -> Option<u32> {
        self.glyph_for_char(character)
    }

    #[inline]
    fn glyph_by_name(&self, name: &str) -> Option<u32> {
        self.glyph_by_name(name)
    }

    #[inline]
    fn glyph_count(&self) -> u32 {
        self.glyph_count()
    }

    #[inline]
    fn outline<S>(
        &self,
        glyph_id: u32,
        hinting_mode: HintingOptions,
        sink: &mut S,
    ) -> Result<(), GlyphLoadingError>
    where
        S: OutlineSink,
    {
        self.outline(glyph_id, hinting_mode, sink)
    }

    #[inline]
    fn typographic_bounds(&self, glyph_id: u32) -> Result<RectF, GlyphLoadingError> {
        self.typographic_bounds(glyph_id)
    }

    #[inline]
    fn advance(&self, glyph_id: u32) -> Result<Vector2F, GlyphLoadingError> {
        self.advance(glyph_id)
    }

    #[inline]
    fn origin(&self, origin: u32) -> Result<Vector2F, GlyphLoadingError> {
        self.origin(origin)
    }

    #[inline]
    fn metrics(&self) -> Metrics {
        self.metrics()
    }

    #[inline]
    fn copy_font_data(&self) -> Option<Arc<Vec<u8>>> {
        self.copy_font_data()
    }

    #[inline]
    fn supports_hinting_options(
        &self,
        hinting_options: HintingOptions,
        for_rasterization: bool,
    ) -> bool {
        self.supports_hinting_options(hinting_options, for_rasterization)
    }

    #[inline]
    fn rasterize_glyph(
        &self,
        canvas: &mut Canvas,
        glyph_id: u32,
        point_size: f32,
        transform: Transform2F,
        hinting_options: HintingOptions,
        rasterization_options: RasterizationOptions,
    ) -> Result<(), GlyphLoadingError> {
        self.rasterize_glyph(
            canvas,
            glyph_id,
            point_size,
            transform,
            hinting_options,
            rasterization_options,
        )
    }

    #[inline]
    fn get_fallbacks(&self, text: &str, locale: &str) -> FallbackResult<Self> {
        self.get_fallbacks(text, locale)
    }

    #[inline]
    fn load_font_table(&self, table_tag: u32) -> Option<Box<[u8]>> {
        self.load_font_table(table_tag)
    }
}

unsafe fn setup_freetype_face(face: FT_Face) {
    reset_freetype_face_char_size(face);
}

unsafe fn reset_freetype_face_char_size(face: FT_Face) {
    // Apple Color Emoji has 0 units per em. Whee!
    // let units_per_em = (*face).units_per_EM as i64;
    // if units_per_em > 0 {
        // assert_eq!(
        //     FT_Set_Char_Size(face, ((*face).units_per_EM as FT_Long) << 6, 0, 0, 0),
        //     0
        // );
    // }
}

trait F32ToFtFixed {
    type Output;
    fn f32_to_ft_fixed_26_6(self) -> Self::Output;
}

trait FtFixedToF32 {
    type Output;
    fn ft_fixed_26_6_to_f32(self) -> Self::Output;
}

impl F32ToFtFixed for Vector2F {
    type Output = Vector2I;
    #[inline]
    fn f32_to_ft_fixed_26_6(self) -> Vector2I {
        (self * 64.0).to_i32()
    }
}

impl F32ToFtFixed for f32 {
    type Output = f32;
    #[inline]
    fn f32_to_ft_fixed_26_6(self) -> f32 {
        (self * 64.0)
    }
}

impl FtFixedToF32 for Vector2I {
    type Output = Vector2F;
    #[inline]
    fn ft_fixed_26_6_to_f32(self) -> Vector2F {
        (self.to_f32() * (1.0 / 64.0)).round()
    }
}

impl FtFixedToF32 for RectI {
    type Output = RectF;
    #[inline]
    fn ft_fixed_26_6_to_f32(self) -> RectF {
        self.to_f32() * (1.0 / 64.0)
    }
}

#[cfg(test)]
mod test {
    use crate::loaders::freetype::Font;

    static PCF_FONT_PATH: &str = "resources/tests/times-roman-pcf/timR12.pcf";
    static PCF_FONT_POSTSCRIPT_NAME: &str = "Times-Roman";

    #[test]
    fn get_pcf_postscript_name() {
        let font = Font::from_path(PCF_FONT_PATH, 0).unwrap();
        assert_eq!(font.postscript_name().unwrap(), PCF_FONT_POSTSCRIPT_NAME);
    }
}
