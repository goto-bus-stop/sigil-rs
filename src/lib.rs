//! A small and fast identicon generator.
//!
//! sigil is compatible with [Cupcake Sigil].
//!
//! [Cupcake Sigil]: https://github.com/tent/sigil

use std::fmt::{Debug, Write};

use md5::Digest as _;

pub use image::RgbImage;
pub type Rgb = image::Rgb<u8>;

const DEFAULT_FOREGROUND: [Rgb; 7] = [
    image::Rgb([45, 79, 255]),
    image::Rgb([254, 180, 44]),
    image::Rgb([226, 121, 234]),
    image::Rgb([30, 179, 253]),
    image::Rgb([232, 77, 65]),
    image::Rgb([49, 203, 115]),
    image::Rgb([141, 69, 170]),
];

pub struct Theme {
    /// Supported values: 1-15 inclusive.
    pub rows: u16,
    pub foreground: Vec<Rgb>,
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

fn md5(input: &[u8]) -> [u8; 16] {
    let mut hash = md5::Md5::new();
    hash.update(input);
    hash.finalize().into()
}

// The cells algorithm comes from https://github.com/tent/sigil/tree/master/gen, BSD 3-Clause
fn should_fill(index: usize, hash: &[u8]) -> bool {
    debug_assert_eq!(hash.len(), 15);
    (hash[index / 8] >> (8 - ((index % 8) + 1))) & 1 == 1
}

fn generate_cells(size: usize, hash: &[u8]) -> Vec<bool> {
    debug_assert_eq!(hash.len(), 15);

    let cols = (size / 2) + (size % 2);

    let mut cells = vec![false; size * size];
    for i in (0..cols * size).filter(|i| should_fill(*i, hash)) {
        let x = i / size;
        let y = i % size;

        cells[y * size + x] = true;
        // Mirror it.
        cells[y * size + size - 1 - x] = true;
    }

    cells
}

/// Represents a Sigil that can be rendered to an image. Use [`Sigil::generate`].
#[derive(Clone)]
pub struct Sigil {
    foreground: Rgb,
    background: Rgb,
    rows: u32,
    cells: Vec<bool>, // TODO: 256 bits?
}

struct DebugCells<'a>(&'a [bool], usize);
impl Debug for DebugCells<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for y in 0..self.1 {
            for x in 0..self.1 {
                f.write_char(if self.0[y * self.1 + x] {
                    'X'
                } else {
                    '-'
                })?;
            }
            f.write_char('\n')?;
        }
        Ok(())
    }
}

impl Debug for Sigil {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sigil")
            .field("foreground", &self.foreground)
            .field("background", &self.background)
            .field("rows", &self.rows)
            .field("cells", &DebugCells(&self.cells, self.rows as usize))
            .finish()
    }
}

impl Sigil {
    /// Create a sigil for a precomputed hash.
    ///
    /// # Panics
    /// Panics if the theme has an invalid `rows` value.
    pub fn from_hash(theme: &Theme, hash: [u8; 16]) -> Self {
        assert!(theme.rows > 0);
        assert!(theme.rows < 16);

        let foreground = theme.pick_foreground(hash[0]);
        let background = theme.background;
        let cells = generate_cells(theme.rows.into(), &hash[1..]);

        Self {
            foreground,
            background,
            rows: theme.rows.into(),
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
    pub fn invert(&mut self) {
        std::mem::swap(&mut self.foreground, &mut self.background);
    }

    /// Create a square image of the given size.
    ///
    /// # Panics
    /// Panics if `size` is not a multiple of `(rows + 1) * 2`.
    pub fn to_image(&self, size: u32) -> RgbImage {
        assert_eq!(size % ((self.rows + 1) * 2), 0);

        let mut image = RgbImage::new(size, size);
        let cell_size = size / (self.rows + 1);
        let padding = cell_size / 2;

        for (x, y, px) in image.enumerate_pixels_mut() {
            if x < padding || x >= size - padding || y < padding || y >= size - padding {
                *px = self.background;
                continue;
            }

            let x = (x - padding) / cell_size; 
            let y = (y - padding) / cell_size;
            let cell_index = y * self.rows + x;
            if self.cells[cell_index as usize] {
                *px = self.foreground;
            } else {
                *px = self.background;
            }
        }

        image
    }
}

pub fn sigil(theme: &Theme, width: u32, input: impl AsRef<[u8]>) -> RgbImage {
    Sigil::generate(theme, input).to_image(width)
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;

    fn format_cells(rows: usize, cells: &[bool]) -> String {
        format!("{:?}", DebugCells(cells, rows))
    }

    #[test]
    fn same_as_sigil() {
        const ROWS: usize = 5;

        let hash = md5(b"test");
        assert_eq!(format_cells(ROWS, &generate_cells(ROWS, &hash[1..])), indoc! {"
            XXXXX
            -X-X-
            -XXX-
            -----
            XXXXX
        "});

        let hash = md5(b"56fbc0305cea0414184cb72b");
        assert_eq!(format_cells(ROWS, &generate_cells(ROWS, &hash[1..])), indoc! {"
            XX-XX
            -XXX-
            -X-X-
            XXXXX
            XX-XX
        "});
    }
}
