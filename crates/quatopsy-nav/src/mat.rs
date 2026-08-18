#![allow(clippy::needless_range_loop)]

use crate::{Matrix6, Vec6};

pub fn identity6() -> Matrix6 {
    let mut m = [[0.0; 6]; 6];
    for i in 0..6 {
        m[i][i] = 1.0;
    }
    m
}

pub fn zeros6() -> Matrix6 {
    [[0.0; 6]; 6]
}

pub fn transpose(a: Matrix6) -> Matrix6 {
    let mut out = [[0.0; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            out[i][j] = a[j][i];
        }
    }
    out
}

pub fn mul(a: Matrix6, b: Matrix6) -> Matrix6 {
    let mut out = [[0.0; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            let mut s = 0.0;
            for k in 0..6 {
                s += a[i][k] * b[k][j];
            }
            out[i][j] = s;
        }
    }
    out
}

pub fn add(a: Matrix6, b: Matrix6) -> Matrix6 {
    let mut out = [[0.0; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            out[i][j] = a[i][j] + b[i][j];
        }
    }
    out
}

pub fn scale(a: Matrix6, s: f64) -> Matrix6 {
    let mut out = [[0.0; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            out[i][j] = a[i][j] * s;
        }
    }
    out
}

#[allow(dead_code)]
pub fn mul_vec(a: Matrix6, v: Vec6) -> Vec6 {
    let mut out = [0.0; 6];
    for i in 0..6 {
        let mut s = 0.0;
        for j in 0..6 {
            s += a[i][j] * v[j];
        }
        out[i] = s;
    }
    out
}

pub fn trace(a: Matrix6) -> f64 {
    a[0][0] + a[1][1] + a[2][2] + a[3][3] + a[4][4] + a[5][5]
}

pub fn floor_diag(mut a: Matrix6, min: f64) -> Matrix6 {
    for i in 0..6 {
        a[i][i] = a[i][i].max(min);
    }
    a
}

pub fn invert3(m: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3], &'static str> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if !det.is_finite() || det.abs() < 1.0e-36 {
        return Err("3x3 matrix is singular");
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

#[allow(dead_code)]
pub fn invert6(a: Matrix6) -> Result<Matrix6, &'static str> {
    let mut m = a;
    let mut inv = identity6();
    for col in 0..6 {
        let mut pivot = col;
        let mut best = m[col][col].abs();
        for row in col + 1..6 {
            let mag = m[row][col].abs();
            if mag > best {
                best = mag;
                pivot = row;
            }
        }
        if best < 1.0e-18 {
            return Err("6x6 matrix is singular");
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

pub fn cholesky(a: Matrix6) -> Result<Matrix6, &'static str> {
    let mut l = zeros6();
    for i in 0..6 {
        for j in 0..=i {
            let mut sum = a[i][j];
            for k in 0..j {
                sum -= l[i][k] * l[j][k];
            }
            if i == j {
                if sum <= 1.0e-18 {
                    return Err("covariance is not positive definite");
                }
                l[i][j] = sum.sqrt();
            } else {
                l[i][j] = sum / l[j][j];
            }
        }
    }
    Ok(l)
}

pub fn skew(w: [f64; 3]) -> [[f64; 3]; 3] {
    [[0.0, -w[2], w[1]], [w[2], 0.0, -w[0]], [-w[1], w[0], 0.0]]
}
