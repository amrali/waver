use super::*;
use crate::{error::Error, Modulation, Wave, WaveFunc};
extern crate std;
use std::vec::Vec;

pub fn calculate_variance(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let mean = samples.iter().sum::<f32>() / samples.len() as f32;
    let variance = samples.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / samples.len() as f32;
    variance
}

mod basic;
mod edge_cases;
mod recorded;
