//! Smooth noise for organic motion: 2D/3D gradient noise and fBm.

fn hash3(x: i32, y: i32, z: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ (z as u32).wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^ (h >> 15)
}

fn grad3(h: u32, x: f32, y: f32, z: f32) -> f32 {
    match h % 12 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 => -y + z,
        10 => y - z,
        _ => -y - z,
    }
}

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// 3D gradient (Perlin-style) noise, output roughly in `-1..1`.
pub fn noise3(x: f32, y: f32, z: f32) -> f32 {
    let (xi, yi, zi) = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
    let (xf, yf, zf) = (x - xi as f32, y - yi as f32, z - zi as f32);
    let (u, v, w) = (fade(xf), fade(yf), fade(zf));
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let g = |dx: i32, dy: i32, dz: i32| {
        grad3(hash3(xi + dx, yi + dy, zi + dz), xf - dx as f32, yf - dy as f32, zf - dz as f32)
    };
    let x00 = l(g(0, 0, 0), g(1, 0, 0), u);
    let x10 = l(g(0, 1, 0), g(1, 1, 0), u);
    let x01 = l(g(0, 0, 1), g(1, 0, 1), u);
    let x11 = l(g(0, 1, 1), g(1, 1, 1), u);
    (l(l(x00, x10, v), l(x01, x11, v), w) * 0.9).clamp(-1.0, 1.0)
}

/// 2D gradient noise, output roughly in `-1..1`.
pub fn noise2(x: f32, y: f32) -> f32 {
    noise3(x, y, 0.5)
}

/// Fractal Brownian motion: `octaves` layers of noise, output roughly in `-1..1`.
pub fn fbm3(x: f32, y: f32, z: f32, octaves: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;
    let mut norm = 0.0;
    for _ in 0..octaves.max(1) {
        sum += amp * noise3(x * freq, y * freq, z * freq);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Noise remapped to `0..1`.
pub fn noise01(x: f32, y: f32, z: f32) -> f32 {
    noise3(x, y, z) * 0.5 + 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_and_continuous() {
        let mut prev = noise3(0.0, 0.0, 0.0);
        for i in 1..2000 {
            let x = i as f32 * 0.01;
            let n = noise3(x, x * 0.7, x * 0.3);
            assert!((-1.0..=1.0).contains(&n));
            assert!((n - prev).abs() < 0.2);
            prev = n;
        }
    }
}
