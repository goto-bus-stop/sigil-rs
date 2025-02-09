//! A minimal and fast identicon generator.
//!
//! sigil is compatible with [Cupcake Sigil].
//!
//! # Example
//!
//! ```no_run
//! use sigil_rs::Theme;
//! use sigil_rs::Sigil;
//!
//! // The default theme uses 5 rows/columns.
//! let theme = Theme::default();
//! let sigil = Sigil::generate(&theme, "username");
//!
//! // The width value must be a multiple of `(rows + 1) * 2`.
//! let image = sigil.to_image(240);
//! image
//!     .save("username.png")
//!     .expect("writing to disk failed");
//! ```
//!
//! # Image formats
//!
//! The [`Sigil::to_image`] function method returns an [`RgbImage`].
//! sigil enables only the PNG encoder in the [`image`] crate. If you want to use a different
//! format, enable the relevant [`image`] feature in your Cargo.toml:
//! ```toml
//! [dependencies]
//! image = { version = "0.25", default-features = false, features = ["bmp"] }
//! ```
//!
//! # HTTP Caching
//!
//! A useful pattern for caching the identicon HTTP responses is to split the hashing and rendering.
//! You can use the hash as an `ETag` header and not render the image at all if the client's
//! `If-None-Match` header contains the `ETag`.
//!
//! For example, using [axum] and the [md-5] crate for hashing:
//! ```no_run
//! use axum::extract::Path;
//! use axum::http::header;
//! use axum::http::HeaderMap;
//! use axum::http::StatusCode;
//! use axum::response::IntoResponse;
//! use axum::response::Response;
//! use md5::Digest;
//! use sigil_rs::Sigil;
//! use sigil_rs::Theme;
//!
//! async fn endpoint_sigil(headers: HeaderMap, path: Path<String>) -> Response {
//!     let mut hash = md5::Md5::new();
//!     hash.update(path.as_bytes());
//!     let hash = hash.finalize();
//!
//!     // Md5::finalize can be formatted as a hexadecimal string:
//!     let etag = format!("{hash:x}");
//!     let response_headers = [
//!         (header::ETAG, etag.as_str()),
//!     ];
//!     if let Some(if_none_match) = headers
//!         .get(header::IF_NONE_MATCH)
//!         .and_then(|value| value.to_str().ok())
//!     {
//!         // The client has this image cached: return early
//!         if if_none_match.contains(&etag) {
//!             return (StatusCode::NOT_MODIFIED, response_headers).into_response();
//!         }
//!     }
//!
//!     let sigil = Sigil::from_hash(&Theme::default(), hash.into());
//!
//!     todo!()
//! }
//! ```
//!
//! [axum]: https://docs.rs/axum
//! [md-5]: https://docs.rs/md-5
//! [Cupcake Sigil]: https://github.com/tent/sigil

use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Write;

use md5::Digest as _;

pub use image::RgbImage;
/// Colour type for configuring [Theme::foreground] and [Theme::background].
pub type Rgb = image::Rgb<u8>;

// Default colours from https://github.com/tent/sigil, BSD 3-Clause
const DEFAULT_FOREGROUND: [Rgb; 7] = [
    image::Rgb([45, 79, 255]),
    image::Rgb([254, 180, 44]),
    image::Rgb([226, 121, 234]),
    image::Rgb([30, 179, 253]),
    image::Rgb([232, 77, 65]),
    image::Rgb([49, 203, 115]),
    image::Rgb([141, 69, 170]),
];

/// Configure the way a sigil looks.
pub struct Theme {
    /// Supported values: 1-15 inclusive.
    pub rows: u16,
    /// Available foreground colours. Each sigil will use one foreground colour.
    ///
    /// Up to 256 different colours are supported.
    pub foreground: Vec<Rgb>,
    /// Background colour.
    pub background: Rgb,
}
impl Default for Theme {
    fn default() -> Self {
        Self {
            rows: 5,
            foreground: DEFAULT_FOREGROUND.to_vec(),
            background: Rgb::from([224, 224, 224]),
        }
    }
}

impl Theme {
    fn pick_foreground(&self, v: u8) -> Rgb {
        self.foreground[usize::from(v) % self.foreground.len()]
    }
}

/// A bit set of up to 256 cells.
#[derive(Clone)]
struct Cells {
    bits: [u8; 32],
}
impl Cells {
    /// Initialise cells to zero.
    const fn new() -> Self {
        Self { bits: [0; 32] }
    }

    const fn capacity(&self) -> usize {
        self.bits.len() * 8
    }

    fn get(&self, index: usize) -> bool {
        debug_assert!(index < self.capacity());
        let byte = self.bits[index / 8];
        let mask = 1 << (index % 8);
        byte & mask != 0
    }

    fn set(&mut self, index: usize) {
        debug_assert!(index < self.capacity());
        self.bits[index / 8] |= 1 << (index % 8);
    }
}

struct DisplayCells<'a>(&'a Cells, usize);
impl Display for DisplayCells<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for y in 0..self.1 {
            for x in 0..self.1 {
                f.write_char(if self.0.get(y * self.1 + x) { 'X' } else { '-' })?;
            }
            f.write_char('\n')?;
        }
        Ok(())
    }
}
impl Debug for DisplayCells<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

// The cells algorithm comes from https://github.com/tent/sigil/tree/master/gen, BSD 3-Clause
fn should_fill(index: usize, hash: &[u8]) -> bool {
    debug_assert_eq!(hash.len(), 15);
    (hash[index / 8] >> (8 - ((index % 8) + 1))) & 1 == 1
}

fn generate_cells(size: usize, hash: &[u8]) -> Cells {
    debug_assert_eq!(hash.len(), 15);

    let cols = (size / 2) + (size % 2);

    let mut cells = Cells::new();
    for i in (0..cols * size).filter(|i| should_fill(*i, hash)) {
        let x = i / size;
        let y = i % size;

        cells.set(y * size + x);
        // Mirror it.
        cells.set(y * size + size - 1 - x);
    }

    cells
}

fn md5(input: &[u8]) -> [u8; 16] {
    let mut hash = md5::Md5::new();
    hash.update(input);
    hash.finalize().into()
}

/// Represents a Sigil that can be rendered to an image.
///
/// ```
/// use sigil_rs::Sigil;
/// use sigil_rs::Theme;
///
/// let theme = Theme::default();
/// let sigil = Sigil::generate(&theme, "my input value");
/// ```
#[derive(Clone)]
pub struct Sigil {
    foreground: Rgb,
    background: Rgb,
    rows: u16,
    cells: Cells,
}

impl Debug for Sigil {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sigil")
            .field("foreground", &self.foreground)
            .field("background", &self.background)
            .field("rows", &self.rows)
            .field("cells", &DisplayCells(&self.cells, self.rows as usize))
            .finish()
    }
}

impl Sigil {
    /// Create a sigil for a precomputed hash.
    ///
    /// # Panics
    /// Panics if the theme has an invalid [`Theme::rows`] value.
    pub fn from_hash(theme: &Theme, hash: [u8; 16]) -> Self {
        assert!(theme.rows > 0);
        assert!(theme.rows < 16);

        let foreground = theme.pick_foreground(hash[0]);
        let background = theme.background;
        let cells = generate_cells(theme.rows.into(), &hash[1..]);

        Self {
            foreground,
            background,
            rows: theme.rows,
            cells,
        }
    }

    /// Generate a sigil by hashing an input.
    ///
    /// # Panics
    /// Panics if the theme has an invalid `rows` value.
    pub fn generate(theme: &Theme, input: impl AsRef<[u8]>) -> Self {
        let hash = md5(input.as_ref());

        Self::from_hash(theme, hash)
    }

    /// Swap foreground and background colours.
    ///
    /// ```
    /// use sigil_rs::Sigil;
    /// use sigil_rs::Theme;
    ///
    /// let theme = Theme::default();
    /// let sigil = Sigil::generate(&theme, "my input value").invert();
    /// // Now `sigil` will output a grey foreground and coloured background.
    /// ```
    pub fn invert(mut self) -> Self {
        std::mem::swap(&mut self.foreground, &mut self.background);
        self
    }

    /// Create a square image of the given size.
    ///
    /// # Panics
    /// Panics if `size` is not a multiple of `(rows + 1) * 2`.
    pub fn to_image(&self, size: u32) -> RgbImage {
        let rows = u32::from(self.rows);
        assert_eq!(size % ((rows + 1) * 2), 0);

        let cell_size = size / (rows + 1);
        let padding = cell_size / 2;

        RgbImage::from_fn(size, size, |x, y| {
            if x < padding || x >= size - padding || y < padding || y >= size - padding {
                return self.background;
            }

            let x = (x - padding) / cell_size;
            let y = (y - padding) / cell_size;
            let cell_index = y * rows + x;
            if self.cells.get(cell_index as usize) {
                self.foreground
            } else {
                self.background
            }
        })
    }

    #[cfg(test)]
    fn display(&self) -> DisplayCells<'_> {
        DisplayCells(&self.cells, self.rows.into())
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;

    #[test]
    fn same_as_cupcake() {
        assert_eq!(
            Sigil::generate(&Theme::default(), "test")
                .display()
                .to_string(),
            indoc! {"
                XXXXX
                -X-X-
                -XXX-
                -----
                XXXXX
            "}
        );

        assert_eq!(
            Sigil::generate(&Theme::default(), "56fbc0305cea0414184cb72b")
                .display()
                .to_string(),
            indoc! {"
                XX-XX
                -XXX-
                -X-X-
                XXXXX
                XX-XX
            "}
        );
    }

    #[test]
    fn even_rows() {
        let theme = Theme {
            rows: 6,
            ..Default::default()
        };
        assert_eq!(
            Sigil::generate(&theme, "test").display().to_string(),
            indoc! {"
                XXXXXX
                -X--X-
                --XX--
                -XXXX-
                XXXXXX
                X-XX-X
            "}
        );
    }
}
