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

/// White lines on transparency, coloured by the UI, and the parts the
/// render shows lit.
pub struct Art {
    pub outline: SharedPixelBuffer<Rgba8Pixel>,
    pub details: SharedPixelBuffer<Rgba8Pixel>,
    pub lights: Lights,
}

/// A part lit in the render, as white light on transparency for the UI to
/// colour: the render's own light, stroke for stroke, halo and all.
pub struct Light {
    pub mask: SharedPixelBuffer<Rgba8Pixel>,
    /// A little more than the part, to hide the line art under it while lit.
    pub cover: SharedPixelBuffer<Rgba8Pixel>,
    /// Its centre, as fractions of the drawing's width and height.
    pub at: (f32, f32),
    /// Its right-hand edge, as a fraction of the drawing's width.
    pub right: f32,
}

#[derive(Default)]
pub struct Lights {
    pub logo: Option<Light>,
    pub wheel: Option<Light>,
    pub sides: Option<Light>,
    /// Light around the rim, as underglow.
    pub under: Option<Light>,
    /// Where the mouse's body starts and ends, as fractions of the drawing's
    /// height, leaving out its cable.
    pub body: (f32, f32),
}

fn cache_path(pid: u16) -> PathBuf {
    crate::storage::data_dir().join("images").join(format!("{pid:04x}.png"))
}

/// Whether a picture of the model is cached, or was put there by hand.
pub fn cached(pid: u16) -> bool {
    cache_path(pid).exists()
}

/// Bumped whenever `draw` changes, so line art from an older version is redrawn.
const ART_VERSION: u32 = 2;

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
    let (outline, details, lit) = draw(w, h, &rgba).ok_or_else(|| io::Error::other("nothing opaque to draw"))?;
    let packed = pack(&outline, &details, &lit)?;
    if let Err(e) = std::fs::write(art_path(pid), &packed) {
        log::warn!("could not keep the line art: {e}");
    }
    // Read back from the packed layers, so a picture looks the same drawn
    // now as loaded later.
    unpack(&packed)
}

/// The layers in one image: the outline in red, the details in green and
/// the lit parts in blue.
fn pack(outline: &Plane, details: &Plane, lit: &Plane) -> io::Result<Vec<u8>> {
    let byte = |v: f32| (v * 255.).round() as u8;
    let rgba = (0..outline.v.len())
        .flat_map(|i| [byte(outline.v[i]), byte(details.v[i]), byte(lit.v[i]), 255])
        .collect();
    encode(&(outline.w, outline.h, rgba))
}

fn unpack(bytes: &[u8]) -> io::Result<Art> {
    let (w, h, rgba) = decode(bytes)?;
    let plane = |channel: usize| Plane {
        w,
        h,
        v: rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| f32::from(p[channel]) / 255.)
            .collect(),
    };
    Ok(art(&plane(0), &plane(1), &plane(2)))
}

fn art(outline: &Plane, details: &Plane, lit: &Plane) -> Art {
    Art {
        outline: pixels(outline),
        details: pixels(details),
        lights: lights(lit, outline),
    }
}

fn pixels(plane: &Plane) -> SharedPixelBuffer<Rgba8Pixel> {
    white(plane.w, plane.h, plane.v.iter().map(|&a| (a * 255.).round() as u8))
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
/// Returns the outline, the details and the parts lit in the render.
fn draw(w: usize, h: usize, rgba: &[u8]) -> Option<(Plane, Plane, Plane)> {
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
    // Razer shows its mice with their lights on: strong colour, unlike the
    // greys and blacks of the shell. Around each such patch, the lightness
    // shapes the part, as pale strokes stand out from their coloured halo.
    let colours: Vec<[f32; 3]> = px
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| {
            if p[3] < 0.9 {
                [0.; 3]
            } else {
                [p[0] / p[3], p[1] / p[3], p[2] / p[3]]
            }
        })
        .collect();
    let colourful: Vec<bool> = colours
        .iter()
        .map(|&[r, g, b]| {
            let (hi, lo) = (r.max(g).max(b), r.min(g).min(b));
            let saturation = if hi > 0. { (hi - lo) / hi } else { 0. };
            smoothstep(0.35, 0.55, saturation) * smoothstep(0.25, 0.45, hi) > 0.5
        })
        .collect();
    let shell = body.v.iter().filter(|&&v| v > 0.5).count();
    let lit_share = colourful.iter().filter(|&&c| c).count() as f32 / shell.max(1) as f32;
    let near = if lit_share > MAX_LIT_SHARE {
        vec![false; w * h]
    } else {
        dilate(w, h, &colourful, (h / 80).max(2))
    };
    let lit = Plane {
        w,
        h,
        v: colours
            .iter()
            .zip(near)
            .map(|(&[r, g, b], near)| {
                // Kept above zero, so the patch stays whole in the cache.
                if near {
                    ((r.max(g).max(b) + r.min(g).min(b)) / 2.).max(1. / 255.)
                } else {
                    0.
                }
            })
            .collect(),
    };
    Some((outline, details, lit))
}

/// Light over more of the mouse than this is the colour of its shell, as on
/// a green or pink edition, rather than its LEDs.
const MAX_LIT_SHARE: f32 = 0.15;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Part {
    Logo,
    Wheel,
    Sides,
    Under,
}

/// Where a lit patch sits and how it is shaped, measured on the body.
struct Patch {
    /// Its light, summed.
    area: f32,
    /// Across the body at its height, 0 to 1 from left to right.
    across: f32,
    /// Down the body, 0 to 1 from its top, cable aside.
    down: f32,
    /// How much of its light lies along the rim.
    rim: f32,
    /// Its width against the body's widest.
    width: f32,
}

/// Which part a lit patch is. Wide light low on the body is underglow, and
/// narrow light along the rim a side strip. Inside, the wheel sits high and
/// the logo low near the middle, reaching further off it on mice whose thumb
/// side makes them lopsided; what lies well to either side is the sides.
fn part(p: &Patch) -> Option<Part> {
    if !(-0.02..=1.02).contains(&p.down) {
        return None;
    }
    if p.width >= 0.4 {
        return (p.down >= 0.5).then_some(Part::Under);
    }
    if p.rim > 0.5 {
        return Some(if p.across < 0.3 || p.across > 0.7 {
            Part::Sides
        } else {
            Part::Under
        });
    }
    let off = (p.across - 0.5).abs();
    if p.down < 0.45 && (off < 0.2 || (p.down < 0.3 && off < 0.25)) {
        Some(Part::Wheel)
    } else if p.down >= 0.45 && (off < 0.2 || (p.down >= 0.72 && off < 0.28)) {
        Some(Part::Logo)
    } else if off >= 0.2 {
        Some(Part::Sides)
    } else {
        None
    }
}

/// A patch no LED makes: wide light high on the body, or a narrower one far
/// larger than any logo, as on a shell printed or moulded in colour.
fn shell(p: &Patch, h: usize) -> bool {
    p.rim <= 0.5
        && if p.width >= 0.4 {
            p.down < 0.5
        } else {
            p.area > (h * h) as f32 / 120.
        }
}

/// The logo, wheel, sides and underglow as the render shows them lit.
fn lights(lit: &Plane, outline: &Plane) -> Lights {
    let (w, h) = (lit.w, lit.h);
    // Each row's span of the mouse, between the first and last of its outline.
    let span: Vec<Option<(usize, usize)>> = (0..h)
        .map(|y| {
            let row = &outline.v[y * w..][..w];
            let first = row.iter().position(|&v| v > 0.5)?;
            let last = row.iter().rposition(|&v| v > 0.5)?;
            (last > first).then_some((first, last))
        })
        .collect();
    // The body is the rows at least a third as wide as the widest, which
    // leaves out a cable trailing from it.
    let widest = span.iter().flatten().map(|(l, r)| r - l).max().unwrap_or(0);
    let wide = |y: usize| span[y].is_some_and(|(l, r)| (r - l) * 3 >= widest);
    let top = (0..h).find(|&y| wide(y)).unwrap_or(0);
    let bottom = (0..h).rfind(|&y| wide(y)).unwrap_or(h - 1).max(top + 1);
    let margin = (h / 50).max(3);
    let on_rim = |x: usize, y: usize| {
        span[y].is_none_or(|(l, r)| x < l + margin || x + margin > r) || y < top + margin || y + margin > bottom
    };

    let (labels, n) = label(w, h, &lit.v.iter().map(|&v| v > 0.).collect::<Vec<_>>());
    let mut peak = vec![0f32; n];
    for (i, &l) in labels.iter().enumerate().filter(|(_, l)| **l > 0) {
        peak[l - 1] = peak[l - 1].max(lit.v[i]);
    }
    // How brightly each pixel shows, against the brightest of its patch and
    // above the shell around it; squared, so the strokes stand clear.
    let core = lit.map(|x, y| {
        let i = y as usize * w + x as usize;
        match labels[i] {
            0 => 0.,
            l => ((lit.v[i] / peak[l - 1] - 0.25) / 0.75).clamp(0., 1.).powi(2),
        }
    });
    // Light, its centre, light along the rim, and leftmost and rightmost.
    let mut sums = vec![(0f32, 0f32, 0f32, 0f32, w, 0usize); n];
    for (i, &l) in labels.iter().enumerate().filter(|(_, l)| **l > 0) {
        let (x, y, k) = (i % w, i / w, core.v[i]);
        let s = &mut sums[l - 1];
        s.0 += k;
        s.1 += k * x as f32;
        s.2 += k * y as f32;
        if on_rim(x, y) {
            s.3 += k;
        }
        if k > 0.5 {
            s.4 = s.4.min(x);
            s.5 = s.5.max(x);
        }
    }
    let min_area = (h * h / 80_000).max(3) as f32;
    let patches: Vec<Option<Patch>> = sums
        .iter()
        .map(|&(area, sx, sy, rim, left, right)| {
            if area < min_area {
                return None;
            }
            let (cx, cy) = (sx / area, sy / area);
            let (l, r) = span[cy as usize]?;
            Some(Patch {
                area,
                across: (cx - l as f32) / (r - l) as f32,
                down: (cy - top as f32) / (bottom - top) as f32,
                rim: rim / area,
                width: right.saturating_sub(left) as f32 / widest.max(1) as f32,
            })
        })
        .collect();
    let body = (top as f32 / h as f32, bottom as f32 / h as f32);
    // A patterned shell's colour cannot be told from its lights.
    if patches.iter().flatten().any(|p| shell(p, h)) {
        return Lights {
            body,
            ..Lights::default()
        };
    }
    let parts: Vec<Option<Part>> = patches.iter().map(|p| p.as_ref().and_then(part)).collect();

    let light = |which: Part| {
        let of = |i: usize| labels[i] > 0 && parts[labels[i] - 1] == Some(which);
        let mine = core.map(|x, y| {
            let i = y as usize * w + x as usize;
            if of(i) { core.v[i] } else { 0. }
        });
        let (area, sx, sy) = sums
            .iter()
            .zip(&parts)
            .filter(|(_, p)| **p == Some(which))
            .fold((0., 0., 0.), |a, (s, _)| (a.0 + s.0, a.1 + s.1, a.2 + s.2));
        if area == 0. {
            return None;
        }
        let mask = mine.map(|x, y| (mine.at(x, y) * 1.6).min(1.));
        let shown: Vec<bool> = mask.v.iter().map(|&v| v > 0.05).collect();
        let cover = Plane {
            w,
            h,
            v: dilate(w, h, &shown, (h / 320).max(1))
                .into_iter()
                .map(|c| if c { 1. } else { 0. })
                .collect(),
        }
        .blur();
        let right = (0..w * h).filter(|&i| mine.v[i] > 0.5).map(|i| i % w).max()?;
        Some(Light {
            mask: pixels(&mask),
            cover: pixels(&cover),
            at: (sx / area / w as f32, sy / area / h as f32),
            right: right as f32 / w as f32,
        })
    };
    Lights {
        logo: light(Part::Logo),
        wheel: light(Part::Wheel),
        sides: light(Part::Sides),
        under: light(Part::Under),
        body,
    }
}

/// `on` grown by `r` pixels each way.
fn dilate(w: usize, h: usize, on: &[bool], r: usize) -> Vec<bool> {
    let mut rows = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            rows[y * w + x] = (x.saturating_sub(r)..(x + r + 1).min(w)).any(|x| on[y * w + x]);
        }
    }
    let mut out = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            out[y * w + x] = (y.saturating_sub(r)..(y + r + 1).min(h)).any(|y| rows[y * w + x]);
        }
    }
    out
}

/// Numbers each connected patch of `on` from 1, 0 elsewhere; returns the
/// numbers and how many patches there are.
fn label(w: usize, h: usize, on: &[bool]) -> (Vec<usize>, usize) {
    let mut labels = vec![0; w * h];
    let mut n = 0;
    let mut stack = Vec::new();
    for start in 0..w * h {
        if !on[start] || labels[start] != 0 {
            continue;
        }
        n += 1;
        labels[start] = n;
        stack.push(start);
        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            let neighbours = [
                (x > 0).then(|| i - 1),
                (x + 1 < w).then(|| i + 1),
                (y > 0).then(|| i - w),
                (y + 1 < h).then(|| i + w),
            ];
            for j in neighbours.into_iter().flatten() {
                if on[j] && labels[j] == 0 {
                    labels[j] = n;
                    stack.push(j);
                }
            }
        }
    }
    (labels, n)
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

    /// `canvas` with flat patches of colour on top. A logo's strokes light
    /// far less than its square, so a patch standing for one is kept small.
    fn paint(mut rgba: Vec<u8>, w: usize, patches: &[([usize; 4], [u8; 3])]) -> Vec<u8> {
        for &([x0, y0, x1, y1], [r, g, b]) in patches {
            for y in y0..y1 {
                for x in x0..x1 {
                    rgba[(y * w + x) * 4..][..4].copy_from_slice(&[r, g, b, 255]);
                }
            }
        }
        rgba
    }

    fn drawn(w: usize, h: usize, rgba: &[u8]) -> Art {
        let (outline, details, lit) = draw(w, h, rgba).unwrap();
        art(&outline, &details, &lit)
    }

    #[test]
    fn traces_the_shape_at_display_size() {
        let art = drawn(100, 100, &canvas(100, 100, &[[30, 20, 70, 80]]));
        assert_eq!(art.outline.height() as usize, HEIGHT);
        let ratio = art.outline.width() as f32 / art.outline.height() as f32;
        assert!((0.6..0.75).contains(&ratio), "keeps the block's proportions: {ratio}");
        let strong = |b: &SharedPixelBuffer<Rgba8Pixel>| b.as_slice().iter().filter(|p| p.a > 128).count();
        assert!(strong(&art.outline) > 1000, "the edge is traced");
        assert!(strong(&art.details) > 100, "the seam inside is found");
        assert!(
            art.lights.logo.is_none() && art.lights.wheel.is_none(),
            "a grey mouse has no lights"
        );
    }

    #[test]
    fn finds_the_lit_logo_wheel_and_sides_where_they_are() {
        const GREEN: [u8; 3] = [0x44, 0xD6, 0x2C];
        let body = canvas(100, 100, &[[30, 10, 70, 90]]);
        let rgba = paint(
            body,
            100,
            &[
                ([48, 18, 52, 30], GREEN),
                ([48, 64, 52, 67], [0xD0, 0x30, 0xF0]),
                ([31, 45, 33, 55], GREEN),
            ],
        );
        let lights = drawn(100, 100, &rgba).lights;
        let at = |l: &Option<Light>| l.as_ref().expect("lit").at;
        let (wx, wy) = at(&lights.wheel);
        assert!(
            (wx - 0.5).abs() < 0.05 && (0.15..0.35).contains(&wy),
            "wheel at {wx}, {wy}"
        );
        let (lx, ly) = at(&lights.logo);
        assert!(
            (lx - 0.5).abs() < 0.05 && (0.6..0.8).contains(&ly),
            "logo at {lx}, {ly}"
        );
        let (sx, _) = at(&lights.sides);
        assert!(sx < 0.2, "side at {sx}");
        // Lit at its centre, softening at its edge, dark below it.
        let logo = lights.logo.unwrap().mask;
        let w = logo.width() as usize;
        let below: Vec<u8> = ((ly * HEIGHT as f32) as usize..HEIGHT)
            .map(|y| logo.as_slice()[y * w + (lx * w as f32) as usize].a)
            .collect();
        assert_eq!(below[0], 255);
        assert!(below.iter().any(|a| (1..200).contains(a)), "a soft edge");
        assert_eq!(below.last(), Some(&0));
    }

    #[test]
    fn wide_light_low_on_the_rim_is_underglow() {
        let body = canvas(100, 100, &[[30, 10, 70, 90]]);
        let rgba = paint(
            body,
            100,
            &[
                ([30, 82, 70, 90], [0x30, 0x80, 0xFF]),
                ([47, 62, 53, 67], [0x44, 0xD6, 0x2C]),
            ],
        );
        let lights = drawn(100, 100, &rgba).lights;
        assert!(
            lights.under.as_ref().is_some_and(|l| l.at.1 > 0.8),
            "the band is underglow"
        );
        assert!(
            lights.logo.as_ref().is_some_and(|l| l.at.1 < 0.75),
            "and the logo stays the logo"
        );
    }

    #[test]
    fn a_printed_pattern_lights_nothing() {
        // Wide colour high on the body, as on a patterned edition.
        let rgba = paint(
            canvas(100, 100, &[[30, 10, 70, 90]]),
            100,
            &[([33, 20, 67, 40], [0x20, 0xC0, 0x40])],
        );
        let lights = drawn(100, 100, &rgba).lights;
        assert!(lights.logo.is_none() && lights.wheel.is_none() && lights.under.is_none());
    }

    #[test]
    fn a_cable_does_not_move_the_wheel_down() {
        // The body fills the lower half; a thin cable runs up from it.
        let rgba = paint(
            canvas(100, 100, &[[48, 0, 52, 50], [30, 50, 70, 100]]),
            100,
            &[([47, 54, 53, 64], [0x44, 0xD6, 0x2C])],
        );
        let lights = drawn(100, 100, &rgba).lights;
        assert!(
            lights.wheel.is_some() && lights.logo.is_none(),
            "high on the body is the wheel"
        );
        let (top, bottom) = lights.body;
        assert!(
            top > 0.45 && bottom > 0.95,
            "the body starts below the cable: {top}, {bottom}"
        );
    }

    #[test]
    fn a_coloured_shell_is_not_lit() {
        let rgba = paint(
            canvas(100, 100, &[[30, 10, 70, 90]]),
            100,
            &[([30, 10, 70, 90], [0x30, 0xC0, 0x40])],
        );
        let lights = drawn(100, 100, &rgba).lights;
        assert!(lights.logo.is_none() && lights.wheel.is_none() && lights.sides.is_none());
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
        let rgba = paint(
            canvas(100, 100, &[[30, 20, 70, 80]]),
            100,
            &[([48, 63, 52, 66], [0, 200, 0])],
        );
        let (outline, details, lit) = draw(100, 100, &rgba).unwrap();
        let art = art(&outline, &details, &lit);
        let back = unpack(&pack(&outline, &details, &lit).unwrap()).unwrap();
        assert_eq!(back.outline.as_slice(), art.outline.as_slice());
        assert_eq!(back.details.as_slice(), art.details.as_slice());
        // The lit layer is kept to a byte, so its light may differ by a hair.
        let at = |a: &Art| a.lights.logo.as_ref().map(|l| l.at);
        let ((x0, y0), (x1, y1)) = (at(&art).unwrap(), at(&back).unwrap());
        assert!((x0 - x1).abs() < 0.01 && (y0 - y1).abs() < 0.01);
    }

    #[test]
    fn keeps_the_biggest_shape() {
        let rgba = canvas(200, 100, &[[10, 10, 90, 90], [120, 40, 140, 60]]);
        let (_, bounds) = main_shape(200, 100, &rgba).unwrap();
        assert_eq!(bounds, [10, 10, 90, 90]);
        assert!(draw(10, 10, &[0; 400]).is_none(), "nothing opaque, nothing drawn");
    }
}
