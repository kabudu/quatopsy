use libm::{cos, sin, sqrt};

pub fn mul(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ]
}

pub fn conj(q: [f64; 4]) -> [f64; 4] {
    [q[0], -q[1], -q[2], -q[3]]
}

pub fn normalize(q: [f64; 4]) -> Result<[f64; 4], &'static str> {
    let n = sqrt(q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]);
    if !n.is_finite() || n < 1.0e-15 {
        return Err("navigator quaternion is near zero");
    }
    Ok([q[0] / n, q[1] / n, q[2] / n, q[3] / n])
}

pub fn exp_so3(phi: [f64; 3]) -> [f64; 4] {
    let n = sqrt(phi[0] * phi[0] + phi[1] * phi[1] + phi[2] * phi[2]);
    if n < 1.0e-16 {
        return [1.0, 0.5 * phi[0], 0.5 * phi[1], 0.5 * phi[2]];
    }
    let half = 0.5 * n;
    let s = sin(half) / n;
    [cos(half), phi[0] * s, phi[1] * s, phi[2] * s]
}

pub fn integrate(q: [f64; 4], omega: [f64; 3], dt: f64) -> Result<[f64; 4], &'static str> {
    normalize(mul(
        q,
        exp_so3([omega[0] * dt, omega[1] * dt, omega[2] * dt]),
    ))
}

pub fn finite(q: [f64; 4]) -> bool {
    q.iter().all(|item| item.is_finite())
}

/// Multiplicative attitude error `2 vec(q_hat^* ⊗ q_meas)`, antipode-safe.
pub fn attitude_error(q_hat: [f64; 4], q_meas: [f64; 4]) -> [f64; 3] {
    let dq = mul(conj(q_hat), q_meas);
    let sign = if dq[0] < 0.0 { -2.0 } else { 2.0 };
    [sign * dq[1], sign * dq[2], sign * dq[3]]
}
