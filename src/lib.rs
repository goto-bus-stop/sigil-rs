//! A small and fast identicon generator.
//!
//! sigil is compatible with [Cupcake Sigil].
//!
//! [Cupcake Sigil]: https://github.com/tent/sigil

use md5::Digest as _;

pub use image::RgbImage;
pub type Rgb = image::Rgb<u8>;

pub struct Config {
    pub rows: u16,
    pub foreground: Vec<Rgb>,
    pub background: Rgb,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            rows: 5,
            foreground: vec![
		Rgb::from([45, 79, 255]),
		Rgb::from([254, 180, 44]),
		Rgb::from([226, 121, 234]),
		Rgb::from([30, 179, 253]),
		Rgb::from([232, 77, 65]),
		Rgb::from([49, 203, 115]),
		Rgb::from([141, 69, 170]),
            ],
            background: Rgb::from([224, 224, 224]),
        }
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

fn cells(size: usize, hash: &[u8]) -> Vec<bool> {
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

fn render(config: &Config, inverted: bool, width: u16, hash: &[u8; 16]) -> RgbImage {
    let foreground = config.foreground[usize::from(hash[0]) % config.foreground.len()];
    let (foreground, background) = if inverted {
        (config.background, foreground)
    } else {
        (foreground, config.background)
    };

    let cells = cells(config.rows.into(), &hash[1..]);

    let mut image = RgbImage::new(width.into(), width.into());
    let cell_size = u32::from(width / (config.rows + 1));
    let padding = cell_size / 2;

    for (x, y, px) in image.enumerate_pixels_mut() {
        if x < padding || x >= u32::from(width) - padding {
            *px = background;
            continue;
        }
        if y < padding || y >= u32::from(width) - padding {
            *px = background;
            continue;
        }

        let x = (x - padding) / cell_size; 
        let y = (y - padding) / cell_size;
        let cell_index = y * u32::from(config.rows) + x;
        if cells[cell_index as usize] {
            *px = foreground;
        } else {
            *px = background;
        }
    }

    image
}

pub fn generate_identicon(config: &Config, width: u16, input: impl AsRef<[u8]>) -> RgbImage {
    assert!(config.rows > 0);
    assert!(config.rows < 16);
    assert!(width % ((config.rows + 1) * 2) == 0);

    let hash = md5(input.as_ref());
    render(config, false, width, &hash)
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::*;

    fn format_cells(rows: usize, cells: &[bool]) -> String {
        let s = cells.len();

        let mut s = String::with_capacity(s + rows);
        for y in 0..rows {
            for x in 0..rows {
                s.push(if cells[y * rows + x] {
                    'X'
                } else {
                    '-'
                });
            }
            s.push('\n');
        }

        s
    }

    #[test]
    fn same_as_sigil() {
        const ROWS: usize = 5;

        let hash = md5(b"test");
        assert_eq!(format_cells(ROWS, &cells(ROWS, &hash[1..])), indoc! {"
            XXXXX
            -X-X-
            -XXX-
            -----
            XXXXX
        "});

        let hash = md5(b"56fbc0305cea0414184cb72b");
        assert_eq!(format_cells(ROWS, &cells(ROWS, &hash[1..])), indoc! {"
            XX-XX
            -XXX-
            -X-X-
            XXXXX
            XX-XX
        "});
    }
}
