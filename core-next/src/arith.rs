/// clamp `x` between +/- `max_unsigned`
pub fn clamp_symetric(x: f32, max_unsigned: f32) -> f32 {
    let max_unsigned = max_unsigned.abs();
    x.clamp(-max_unsigned, max_unsigned)
}

/// The function f(x) =
///   k > 0  => exp(k * (x - a)) - b
///   k == 0 => x
///   k < 0  => -(ln(x + b) / k) + a
/// ...where a and b are chosen so that f(0) = 0 and f(1) = 1.
/// The k parameter controls how sharp the curve is.
/// The functions when k != 0 are inverses of each other and zip approach linearity as k
/// approaches 0.
pub fn exp_01(x: f32, k: f32) -> f32 {
    let b = 1.0 / k.abs().exp_m1();
    let a = -b.ln() / k.abs();
    if k > 0.0 {
        (k * (x - a)).exp() - b
    } else if k < 0.0 {
        -((x + b).ln() / k) + a
    } else {
        x
    }
}
