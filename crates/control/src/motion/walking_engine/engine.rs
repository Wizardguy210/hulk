use nalgebra::{Isometry3, Vector3};
use types::{RobotDimensions, Side};

use super::foot_offsets::FootOffsets;

pub fn calculate_foot_to_robot(
    side: Side,
    foot: FootOffsets,
    turn_left_right: f32,
    foot_lift: f32,
    torso_offset: f32,
    walk_hip_height: f32,
) -> Isometry3<f32> {
    let hip_to_robot = match side {
        Side::Left => Isometry3::from(RobotDimensions::ROBOT_TO_LEFT_PELVIS),
        Side::Right => Isometry3::from(RobotDimensions::ROBOT_TO_RIGHT_PELVIS),
    };
    let foot_rotation = match side {
        Side::Left => turn_left_right,
        Side::Right => -turn_left_right,
    };
    hip_to_robot
        * Isometry3::translation(
            foot.forward - torso_offset,
            foot.left,
            -walk_hip_height + foot_lift,
        )
        * Isometry3::rotation(Vector3::z() * foot_rotation)
}

pub fn parabolic_return(x: f32, midpoint: f32) -> f32 {
    if x < midpoint {
        -2.0 / midpoint.powi(3) * x.powi(3) + 3.0 / midpoint.powi(2) * x.powi(2)
    } else {
        -1.0 / (midpoint - 1.0).powi(3)
            * (2.0 * x.powi(3) - 3.0 * (midpoint + 1.0) * x.powi(2) + 6.0 * midpoint * x
                - 3.0 * midpoint
                + 1.0)
    }
}

pub fn parabolic_step(x: f32) -> f32 {
    if x < 0.5 {
        2.0 * x * x
    } else {
        4.0 * x - 2.0 * x * x - 1.0
    }
}

pub fn non_continuous_quadratic_return(x: f32) -> f32 {
    -(x * x) + 1.0
}
