//! A picture of each model: Razer's render from above, downloaded once and
//! redrawn as line art in the app's style.

use std::{io, path::PathBuf};

use slint::{Rgba8Pixel, SharedPixelBuffer};

/// Renders run from a few hundred kilobytes to about 14 MB; anything far
/// larger is not one.
const MAX_DOWNLOAD: usize = 32 * 1024 * 1024;
/// Tall enough for the drawing on a high-DPI screen, and no more.
const HEIGHT: usize = 640;
/// The largest side kept while decoding, well above `HEIGHT`.
const MAX_DECODE: usize = 1600;

/// White lines on transparency, coloured by the UI.
pub struct Art {
    pub outline: SharedPixelBuffer<Rgba8Pixel>,
    pub details: SharedPixelBuffer<Rgba8Pixel>,
}

fn cache_path(pid: u16) -> PathBuf {
    crate::storage::data_dir().join("images").join(format!("{pid:04x}.png"))
}

/// Whether a picture of the model is cached, or was put there by hand.
pub fn cached(pid: u16) -> bool {
    cache_path(pid).exists()
}

/// Bumped whenever `draw` changes, so line art from an older version is redrawn.
const ART_VERSION: u32 = 1;

fn art_path(pid: u16) -> PathBuf {
    cache_path(pid).with_extension(format!("lines-v{ART_VERSION}.png"))
}

/// The line art drawn last time, unless the picture it came from is newer,
/// as when one was put there by hand.
fn drawn(pid: u16) -> Option<Art> {
    let modified = |p: &PathBuf| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let art = art_path(pid);
    if modified(&cache_path(pid))? > modified(&art)? {
        return None;
    }
    unpack(&std::fs::read(art).ok()?).ok()
}

/// The model's line art: drawn before, or drawn now from the cached
/// picture, or else from the one at `url`.
pub fn load(pid: u16, url: Option<&str>) -> io::Result<Art> {
    if let Some(art) = drawn(pid) {
        return Ok(art);
    }
    let path = cache_path(pid);
    let (w, h, rgba) = match std::fs::read(&path) {
        Ok(bytes) => decode(&bytes)?,
        Err(_) => {
            let url = url.ok_or_else(|| io::Error::other("no picture of this model"))?;
            let decoded = decode(&crate::win::http::get(url, MAX_DOWNLOAD)?)?;
            // Kept at the decoded size, so the full render is fetched and
            // decoded only once.
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&path, encode(&decoded)?)?;
            decoded
        }
    };
    let art = draw(w, h, &rgba).ok_or_else(|| io::Error::other("nothing opaque to draw"))?;
    if let Err(e) = std::fs::write(art_path(pid), pack(&art)?) {
        log::warn!("could not keep the line art: {e}");
    }
    Ok(art)
}

/// Both layers in one image: the outline in red, the details in green.
fn pack(art: &Art) -> io::Result<Vec<u8>> {
    let (w, h) = (art.outline.width() as usize, art.outline.height() as usize);
    let rgba = art
        .outline
        .as_slice()
        .iter()
        .zip(art.details.as_slice())
        .flat_map(|(o, d)| [o.a, d.a, 0, 255])
        .collect();
    encode(&(w, h, rgba))
}

fn unpack(bytes: &[u8]) -> io::Result<Art> {
    let (w, h, rgba) = decode(bytes)?;
    let layer = |channel: usize| white(w, h, rgba.as_chunks::<4>().0.iter().map(|p| p[channel]));
    Ok(Art {
        outline: layer(0),
        details: layer(1),
    })
}

/// White pixels at the given alphas, for the UI to colour.
fn white(w: usize, h: usize, alpha: impl Iterator<Item = u8>) -> SharedPixelBuffer<Rgba8Pixel> {
    let mut out = SharedPixelBuffer::<Rgba8Pixel>::new(w as u32, h as u32);
    for (px, a) in out.make_mut_slice().iter_mut().zip(alpha) {
        *px = Rgba8Pixel {
            r: 255,
            g: 255,
            b: 255,
            a,
        };
    }
    out
}

fn encode((w, h, rgba): &(usize, usize, Vec<u8>)) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, *w as u32, *h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(rgba))
        .map_err(io::Error::other)?;
    Ok(out)
}

/// Decodes to RGBA, averaging blocks of pixels as rows arrive so a render
/// thousands of pixels across never sits in memory whole.
fn decode(bytes: &[u8]) -> io::Result<(usize, usize, Vec<u8>)> {
    let mut decoder = png::Decoder::new(io::Cursor::new(bytes));
    decoder.set_transformations(
        png::Transformations::EXPAND | png::Transformations::ALPHA | png::Transformations::STRIP_16,
    );
    let mut reader = decoder.read_info().map_err(io::Error::other)?;
    let (w, h) = reader.info().size();
    let (w, h) = (w as usize, h as usize);
    let channels = match reader.output_color_type().0 {
        png::ColorType::Rgba => 4,
        png::ColorType::GrayscaleAlpha => 2,
        _ => return Err(io::Error::other("no transparency to find the mouse by")),
    };
    let rgba = |p: &[u8]| {
        if channels == 4 {
            [p[0], p[1], p[2], p[3]]
        } else {
            [p[0], p[0], p[0], p[1]]
        }
    };

    // Interlaced rows arrive out of order; those images are small anyway.
    let f = if reader.info().interlaced {
        1
    } else {
        w.max(h).div_ceil(MAX_DECODE).max(1)
    };
    if f == 1 {
        let mut buf = vec![
            0;
            reader
                .output_buffer_size()
                .ok_or_else(|| io::Error::other("image too large"))?
        ];
        reader.next_frame(&mut buf).map_err(io::Error::other)?;
        return Ok((w, h, buf.chunks_exact(channels).flat_map(rgba).collect()));
    }

    let (ow, oh) = (w.div_ceil(f), h.div_ceil(f));
    let mut out = vec![0u8; ow * oh * 4];
    let mut sums = vec![0u32; ow * 4];
    for y in 0..h {
        let row = reader
            .next_row()
            .map_err(io::Error::other)?
            .ok_or_else(|| io::Error::other("image ends early"))?;
        for (x, p) in row.data().chunks_exact(channels).take(w).enumerate() {
            for (sum, v) in sums[x / f * 4..][..4].iter_mut().zip(rgba(p)) {
                *sum += u32::from(v);
            }
        }
        let rows = y % f + 1;
        if rows == f || y + 1 == h {
            for (ox, block) in sums.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                let n = ((w - ox * f).min(f) * rows) as u32;
                for (c, sum) in block.iter_mut().enumerate() {
                    out[((y / f) * ow + ox) * 4 + c] = (*sum / n) as u8;
                    *sum = 0;
                }
            }
        }
    }
    Ok((ow, oh, out))
}

/// A single-channel image with clamped reads.
struct Plane {
    w: usize,
    h: usize,
    v: Vec<f32>,
}

impl Plane {
    fn at(&self, x: isize, y: isize) -> f32 {
        let x = x.clamp(0, self.w as isize - 1) as usize;
        let y = y.clamp(0, self.h as isize - 1) as usize;
        self.v[y * self.w + x]
    }

    fn map(&self, f: impl Fn(isize, isize) -> f32) -> Plane {
        let f = &f;
        let v = (0..self.h as isize)
            .flat_map(|y| (0..self.w as isize).map(move |x| f(x, y)))
            .collect();
        Plane {
            w: self.w,
            h: self.h,
            v,
        }
    }

    fn blur(&self) -> Plane {
        const K: [(isize, isize, f32); 9] = [
            (-1, -1, 1.),
            (0, -1, 2.),
            (1, -1, 1.),
            (-1, 0, 2.),
            (0, 0, 4.),
            (1, 0, 2.),
            (-1, 1, 1.),
            (0, 1, 2.),
            (1, 1, 1.),
        ];
        self.map(|x, y| K.iter().map(|&(dx, dy, k)| self.at(x + dx, y + dy) * k).sum::<f32>() / 16.)
    }

    fn sobel(&self, x: isize, y: isize) -> f32 {
        let a = |dx, dy| self.at(x + dx, y + dy);
        let gx = a(1, -1) + 2. * a(1, 0) + a(1, 1) - a(-1, -1) - 2. * a(-1, 0) - a(-1, 1);
        let gy = a(-1, 1) + 2. * a(0, 1) + a(1, 1) - a(-1, -1) - 2. * a(0, -1) - a(1, -1);
        (gx * gx + gy * gy).sqrt()
    }
}

fn smoothstep(lo: f32, hi: f32, v: f32) -> f32 {
    let t = ((v - lo) / (hi - lo)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}

/// The biggest opaque shape: a render may show the mouse from more than
/// one side, or with its cable or dongle. Returns its pixels and bounds.
fn main_shape(w: usize, h: usize, rgba: &[u8]) -> Option<(Vec<bool>, [usize; 4])> {
    let solid = |i: usize| rgba[i * 4 + 3] > 127;
    // Marks the shape containing `start` in `seen` and returns its size.
    let fill = |seen: &mut [bool], stack: &mut Vec<usize>, start: usize| {
        let mut size = 0;
        seen[start] = true;
        stack.push(start);
        while let Some(i) = stack.pop() {
            size += 1;
            let (x, y) = (i % w, i / w);
            let neighbours = [
                (x > 0).then(|| i - 1),
                (x + 1 < w).then(|| i + 1),
                (y > 0).then(|| i - w),
                (y + 1 < h).then(|| i + w),
            ];
            for n in neighbours.into_iter().flatten() {
                if !seen[n] && solid(n) {
                    seen[n] = true;
                    stack.push(n);
                }
            }
        }
        size
    };
    // Every shape once to find the biggest, then that one again on its own.
    let mut seen = vec![false; w * h];
    let mut stack = Vec::new();
    let mut best: Option<(usize, usize)> = None;
    for start in 0..w * h {
        if solid(start) && !seen[start] {
            let size = fill(&mut seen, &mut stack, start);
            if best.is_none_or(|(_, largest)| size > largest) {
                best = Some((start, size));
            }
        }
    }
    let (start, _) = best?;
    seen.fill(false);
    fill(&mut seen, &mut stack, start);
    let mut bounds = [w, h, 0, 0];
    for (i, _) in seen.iter().enumerate().filter(|(_, m)| **m) {
        let (x, y) = (i % w, i / w);
        bounds = [
            bounds[0].min(x),
            bounds[1].min(y),
            bounds[2].max(x + 1),
            bounds[3].max(y + 1),
        ];
    }
    Some((seen, bounds))
}

/// The main shape cropped and scaled to `height`, transparent elsewhere:
/// bilinear when enlarging, averaged when shrinking.
fn resample(
    w: usize,
    rgba: &[u8],
    mine: &[bool],
    [left, top, right, bottom]: [usize; 4],
    height: usize,
) -> (usize, usize, Vec<f32>) {
    let pad = ((bottom - top) / 50).max(2) as f32;
    let (x0, y0) = (left as f32 - pad, top as f32 - pad);
    let (cw, ch) = ((right - left) as f32 + 2. * pad, (bottom - top) as f32 + 2. * pad);
    let scale = ch / height as f32;
    let (ow, oh) = (((cw / scale).round() as usize).max(1), height);
    let h = rgba.len() / 4 / w;
    let texel = |x: isize, y: isize| -> [f32; 4] {
        if x < 0 || y < 0 || x as usize >= w || y as usize >= h || !mine[y as usize * w + x as usize] {
            return [0.; 4];
        }
        let p = &rgba[(y as usize * w + x as usize) * 4..][..4];
        let a = f32::from(p[3]) / 255.;
        // Colour weighted by coverage, so edges do not darken.
        [
            f32::from(p[0]) / 255. * a,
            f32::from(p[1]) / 255. * a,
            f32::from(p[2]) / 255. * a,
            a,
        ]
    };
    let mut out = vec![0f32; ow * oh * 4];
    for oy in 0..oh {
        for ox in 0..ow {
            let mut acc = [0f32; 4];
            if scale <= 1. {
                let (sx, sy) = (
                    x0 + (ox as f32 + 0.5) * scale - 0.5,
                    y0 + (oy as f32 + 0.5) * scale - 0.5,
                );
                let (fx, fy) = (sx.floor(), sy.floor());
                let (tx, ty) = (sx - fx, sy - fy);
                for (dx, dy, k) in [
                    (0, 0, (1. - tx) * (1. - ty)),
                    (1, 0, tx * (1. - ty)),
                    (0, 1, (1. - tx) * ty),
                    (1, 1, tx * ty),
                ] {
                    let t = texel(fx as isize + dx, fy as isize + dy);
                    (0..4).for_each(|c| acc[c] += t[c] * k);
                }
            } else {
                let (sx0, sy0) = ((x0 + ox as f32 * scale) as isize, (y0 + oy as f32 * scale) as isize);
                let (sx1, sy1) = (
                    (x0 + (ox + 1) as f32 * scale) as isize,
                    (y0 + (oy + 1) as f32 * scale) as isize,
                );
                let mut n = 0.;
                for sy in sy0..sy1.max(sy0 + 1) {
                    for sx in sx0..sx1.max(sx0 + 1) {
                        let t = texel(sx, sy);
                        (0..4).for_each(|c| acc[c] += t[c]);
                        n += 1.;
                    }
                }
                acc.iter_mut().for_each(|v| *v /= n);
            }
            out[(oy * ow + ox) * 4..][..4].copy_from_slice(&acc);
        }
    }
    (ow, oh, out)
}

/// Traces the main shape at the size it will be shown: its edge from the
/// transparency, cut hard so soft drop shadows leave no halo, and the seams,
/// wheel and buttons from the shading inside it, kept clear of the rim.
fn draw(w: usize, h: usize, rgba: &[u8]) -> Option<Art> {
    let (mine, bounds) = main_shape(w, h, rgba)?;
    let (w, h, px) = resample(w, rgba, &mine, bounds, HEIGHT);
    let body = Plane {
        w,
        h,
        v: px
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| smoothstep(0.4, 0.6, p[3]))
            .collect(),
    };
    let lum = Plane {
        w,
        h,
        v: px
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| {
                if p[3] > 0. {
                    (0.299 * p[0] + 0.587 * p[1] + 0.114 * p[2]) / p[3]
                } else {
                    0.
                }
            })
            .collect(),
    }
    .blur();
    let margin = (h / 120).max(3) as isize;
    let outline = body.map(|x, y| (body.sobel(x, y) * 0.7).min(1.));
    let details = body.map(|x, y| {
        let inside = [(margin, 0), (-margin, 0), (0, margin), (0, -margin), (0, 0)]
            .iter()
            .all(|&(dx, dy)| body.at(x + dx, y + dy) > 0.95);
        if inside {
            ((lum.sobel(x, y) - 0.09) * 4.).clamp(0., 1.)
        } else {
            0.
        }
    });
    let pixels = |plane: &Plane| white(w, h, plane.v.iter().map(|&a| (a * 255.) as u8));
    Some(Art {
        outline: pixels(&outline),
        details: pixels(&details),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Opaque blocks, darker in their lower halves, on a transparent canvas.
    fn canvas(w: usize, h: usize, blocks: &[[usize; 4]]) -> Vec<u8> {
        let mut rgba = vec![0u8; w * h * 4];
        for &[x0, y0, x1, y1] in blocks {
            for y in y0..y1 {
                for x in x0..x1 {
                    let shade = if y < (y0 + y1) / 2 { 200 } else { 60 };
                    rgba[(y * w + x) * 4..][..4].copy_from_slice(&[shade, shade, shade, 255]);
                }
            }
        }
        rgba
    }

    #[test]
    fn traces_the_shape_at_display_size() {
        let art = draw(100, 100, &canvas(100, 100, &[[30, 20, 70, 80]])).unwrap();
        assert_eq!(art.outline.height() as usize, HEIGHT);
        let ratio = art.outline.width() as f32 / art.outline.height() as f32;
        assert!((0.6..0.75).contains(&ratio), "keeps the block's proportions: {ratio}");
        let strong = |b: &SharedPixelBuffer<Rgba8Pixel>| b.as_slice().iter().filter(|p| p.a > 128).count();
        assert!(strong(&art.outline) > 1000, "the edge is traced");
        assert!(strong(&art.details) > 100, "the seam inside is found");
    }

    #[test]
    fn shrinks_big_renders_while_decoding() {
        // 3300 pixels across shrinks by 3: columns 0..3 average 0, 30 and 60.
        let (w, h) = (3300u32, 4u32);
        let pixels: Vec<u8> = (0..w * h).flat_map(|i| [((i % w) % 3 * 30) as u8, 0, 0, 255]).collect();
        let mut png_bytes = Vec::new();
        let mut enc = png::Encoder::new(&mut png_bytes, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&pixels).unwrap();
        let (ow, oh, rgba) = decode(&png_bytes).unwrap();
        assert_eq!((ow, oh), (1100, 2));
        assert_eq!(&rgba[..4], &[30, 0, 0, 255]);
    }

    #[test]
    fn line_art_survives_the_cache() {
        let art = draw(100, 100, &canvas(100, 100, &[[30, 20, 70, 80]])).unwrap();
        let back = unpack(&pack(&art).unwrap()).unwrap();
        assert_eq!(back.outline.as_slice(), art.outline.as_slice());
        assert_eq!(back.details.as_slice(), art.details.as_slice());
    }

    #[test]
    fn keeps_the_biggest_shape() {
        let rgba = canvas(200, 100, &[[10, 10, 90, 90], [120, 40, 140, 60]]);
        let (_, bounds) = main_shape(200, 100, &rgba).unwrap();
        assert_eq!(bounds, [10, 10, 90, 90]);
        assert!(draw(10, 10, &[0; 400]).is_none(), "nothing opaque, nothing drawn");
    }
}
