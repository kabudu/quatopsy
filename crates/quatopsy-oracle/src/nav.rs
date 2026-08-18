//! Independent navigator and allocator oracles. Must not share filter gains.

#![allow(clippy::needless_range_loop)]

use crate::{RefQuat, geodesic_angle};

pub fn innovation_nis(z: [f64; 3], s: [[f64; 3]; 3]) -> Result<f64, &'static str> {
    if z.iter().any(|item| !item.is_finite()) {
        return Err("oracle NIS innovation is not finite");
    }
    let inv = invert3(s)?;
    Ok(
        z[0] * (inv[0][0] * z[0] + inv[0][1] * z[1] + inv[0][2] * z[2])
            + z[1] * (inv[1][0] * z[0] + inv[1][1] * z[1] + inv[1][2] * z[2])
            + z[2] * (inv[2][0] * z[0] + inv[2][1] * z[1] + inv[2][2] * z[2]),
    )
}

pub fn error_nees(e: [f64; 6], p: [[f64; 6]; 6]) -> Result<f64, &'static str> {
    if e.iter().any(|item| !item.is_finite()) {
        return Err("oracle NEES error is not finite");
    }
    let inv = invert6(p)?;
    let mut acc = 0.0;
    for i in 0..6 {
        let mut row = 0.0;
        for j in 0..6 {
            row += inv[i][j] * e[j];
        }
        acc += e[i] * row;
    }
    Ok(acc)
}

pub fn allocation_residual(
    requested: [f64; 3],
    axes: &[[f64; 3]],
    wheel_torque: &[f64],
) -> Result<[f64; 3], &'static str> {
    if axes.len() != wheel_torque.len() || axes.is_empty() || axes.len() > 8 {
        return Err("oracle allocation dimensions are invalid");
    }
    if requested.iter().any(|item| !item.is_finite())
        || wheel_torque.iter().any(|item| !item.is_finite())
        || axes
            .iter()
            .any(|axis| axis.iter().any(|item| !item.is_finite()))
    {
        return Err("oracle allocation values are not finite");
    }
    let mut body = [0.0; 3];
    for (axis, torque) in axes.iter().zip(wheel_torque.iter()) {
        body[0] += axis[0] * *torque;
        body[1] += axis[1] * *torque;
        body[2] += axis[2] * *torque;
    }
    Ok([
        requested[0] - body[0],
        requested[1] - body[1],
        requested[2] - body[2],
    ])
}

pub fn reference_rate_residual(
    q0: RefQuat,
    q1: RefQuat,
    omega: [f64; 3],
    dt: f64,
) -> Result<f64, &'static str> {
    if dt <= 0.0 || !dt.is_finite() {
        return Err("oracle reference dt is invalid");
    }
    if omega.iter().any(|item| !item.is_finite()) {
        return Err("oracle reference rate is not finite");
    }
    let geo = geodesic_angle(q0, q1) / dt;
    let n = (omega[0] * omega[0] + omega[1] * omega[1] + omega[2] * omega[2]).sqrt();
    Ok((geo - n).abs())
}

fn invert3(m: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3], &'static str> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if !det.is_finite() || det.abs() < 1.0e-18 {
        return Err("oracle 3x3 is singular");
    }
    let inv = 1.0 / det;
    Ok([
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv,
        ],
    ])
}

fn invert6(a: [[f64; 6]; 6]) -> Result<[[f64; 6]; 6], &'static str> {
    let mut m = a;
    let mut inv = [[0.0; 6]; 6];
    for i in 0..6 {
        inv[i][i] = 1.0;
    }
    for col in 0..6 {
        let mut pivot = col;
        let mut best = m[col][col].abs();
        for row in col + 1..6 {
            if m[row][col].abs() > best {
                best = m[row][col].abs();
                pivot = row;
            }
        }
        if best < 1.0e-18 {
            return Err("oracle 6x6 is singular");
        }
        if pivot != col {
            m.swap(col, pivot);
            inv.swap(col, pivot);
        }
        let diag = m[col][col];
        for j in 0..6 {
            m[col][j] /= diag;
            inv[col][j] /= diag;
        }
        for i in 0..6 {
            if i == col {
                continue;
            }
            let factor = m[i][col];
            for j in 0..6 {
                m[i][j] -= factor * m[col][j];
                inv[i][j] -= factor * inv[col][j];
            }
        }
    }
    Ok(inv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nis_is_one_for_unit_innovation_and_identity_s() {
        let nis = innovation_nis(
            [1.0, 0.0, 0.0],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        )
        .unwrap();
        assert!((nis - 1.0).abs() < 1e-15);
    }

    #[test]
    fn allocation_residual_vanishes_for_an_aligned_triad() {
        let axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let residual = allocation_residual([1.0, -2.0, 0.5], &axes, &[1.0, -2.0, 0.5]).unwrap();
        assert!(residual.iter().all(|item| item.abs() < 1e-15));
    }

    #[test]
    fn allocation_residual_is_nonzero_if_a_wheel_is_dropped() {
        let axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let residual = allocation_residual([1.0, 0.0, 0.0], &axes, &[0.0, 0.0, 0.0]).unwrap();
        assert!((residual[0] - 1.0).abs() < 1e-15);
    }
}
