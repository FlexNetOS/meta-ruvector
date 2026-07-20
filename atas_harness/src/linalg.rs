//! Small deterministic linear-algebra routines for ridge readouts.

pub fn ridge_solve(rows: &[Vec<f64>], targets: &[f64], lambda: f64) -> Result<Vec<f64>, String> {
    if rows.is_empty() || rows.len() != targets.len() {
        return Err("ridge solve requires matching, non-empty rows and targets".to_owned());
    }
    let width = rows[0].len();
    if width == 0 || rows.iter().any(|row| row.len() != width) {
        return Err("ridge rows must have a consistent non-zero width".to_owned());
    }
    let mut normal = vec![vec![0.0; width + 1]; width];
    for (row, target) in rows.iter().zip(targets) {
        for i in 0..width {
            normal[i][width] += row[i] * target;
            for j in 0..width {
                normal[i][j] += row[i] * row[j];
            }
        }
    }
    for i in 0..width {
        normal[i][i] += lambda.max(0.0);
    }

    for column in 0..width {
        let pivot = (column..width)
            .max_by(|left, right| {
                normal[*left][column]
                    .abs()
                    .total_cmp(&normal[*right][column].abs())
            })
            .expect("non-empty pivot range");
        if normal[pivot][column].abs() < 1e-15 {
            return Err("ridge normal matrix is singular".to_owned());
        }
        normal.swap(column, pivot);
        let divisor = normal[column][column];
        for value in &mut normal[column][column..] {
            *value /= divisor;
        }
        for row in 0..width {
            if row == column {
                continue;
            }
            let factor = normal[row][column];
            for index in column..=width {
                normal[row][index] -= factor * normal[column][index];
            }
        }
    }
    Ok(normal.into_iter().map(|row| row[width]).collect())
}
