use std::time::SystemTime;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::{point, Isometry2, Point2, UnitComplex, Vector2};
use ordered_float::NotNan;
use types::{
    configuration::LookAction as LookActionConfiguration, BallState, CycleTime, FieldDimensions,
    Obstacle, ObstacleKind,
};

pub struct ActiveVision {
    field_mark_positions: Vec<Point2<f32>>,
    last_position_of_interest_switch: Option<SystemTime>,
    position_of_interest_index: usize,
}

#[context]
pub struct CreationContext {
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
}

#[context]
pub struct CycleContext {
    pub ball: Input<Option<BallState>, "ball_state?">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub obstacles: Input<Vec<Obstacle>, "obstacles">,
    pub parameters: Parameter<LookActionConfiguration, "behavior.look_action">,
    pub robot_to_field: Input<Option<Isometry2<f32>>, "robot_to_field?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub position_of_interest: MainOutput<Point2<f32>>,
}

impl ActiveVision {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            field_mark_positions: generate_field_mark_positions(context.field_dimensions),
            last_position_of_interest_switch: None,
            position_of_interest_index: 0,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let cycle_start_time = context.cycle_time.start_time;
        let mut positions_of_interest = vec![point![1.0, 0.0]];

        if let Some(ball_state) = context.ball {
            positions_of_interest.push(ball_state.position);
        }

        let closest_interesting_obstacle_position = context
            .obstacles
            .iter()
            .filter(|obstacle| matches!(obstacle.kind, ObstacleKind::Robot | ObstacleKind::Unknown))
            .map(|obstacle| obstacle.position)
            .filter(|obstacle_position| is_position_visible(*obstacle_position, context.parameters))
            .min_by_key(|position| NotNan::new(position.coords.norm()).unwrap());
        if let Some(interesting_obstacle_position) = closest_interesting_obstacle_position {
            positions_of_interest.push(interesting_obstacle_position);
        }

        if let Some(robot_to_field) = context.robot_to_field {
            let field_mark_of_interest = closest_field_mark_visible(
                &self.field_mark_positions,
                context.parameters,
                robot_to_field,
            );

            if let Some(field_mark_position) = field_mark_of_interest {
                positions_of_interest.push(field_mark_position);
            }
        }

        if self.last_position_of_interest_switch.is_none()
            || cycle_start_time.duration_since(self.last_position_of_interest_switch.unwrap())?
                > context.parameters.position_of_interest_switch_interval
        {
            self.position_of_interest_index += 1;
            self.last_position_of_interest_switch = Some(cycle_start_time);
        }

        self.position_of_interest_index %= positions_of_interest.len();

        let position_of_interest = positions_of_interest[self.position_of_interest_index];

        Ok(MainOutputs {
            position_of_interest: position_of_interest.into(),
        })
    }
}

fn is_position_visible(position: Point2<f32>, parameters: &LookActionConfiguration) -> bool {
    UnitComplex::rotation_between(&Vector2::x(), &position.coords)
        .angle()
        .abs()
        < parameters.angle_threshold
        && position.coords.norm() < parameters.distance_threshold
}

fn closest_field_mark_visible(
    field_mark_positions: &[Point2<f32>],
    parameters: &LookActionConfiguration,
    robot_to_field: &Isometry2<f32>,
) -> Option<Point2<f32>> {
    field_mark_positions
        .iter()
        .map(|position| robot_to_field.inverse() * position)
        .filter(|position| is_position_visible(*position, parameters))
        .min_by_key(|position| NotNan::new(position.coords.norm()).unwrap())
}

fn generate_field_mark_positions(field_dimensions: &FieldDimensions) -> Vec<Point2<f32>> {
    let left_center_circle_junction = point![0.0, field_dimensions.center_circle_diameter / 2.0];
    let right_center_circle_junction = point![0.0, -field_dimensions.center_circle_diameter / 2.0];
    let left_center_t_junction = point![0.0, field_dimensions.width / 2.0];
    let right_center_t_junction = point![0.0, -field_dimensions.width / 2.0];
    let left_opponent_penalty_box_corner = point![
        field_dimensions.length / 2.0 - field_dimensions.penalty_area_length,
        field_dimensions.penalty_area_width / 2.0
    ];
    let right_opponent_penalty_box_corner = point![
        field_dimensions.length / 2.0 - field_dimensions.penalty_area_length,
        -field_dimensions.penalty_area_width / 2.0
    ];
    let left_own_penalty_box_corner = point![
        -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length,
        field_dimensions.penalty_area_width / 2.0
    ];
    let right_own_penalty_box_corner = point![
        -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length,
        -field_dimensions.penalty_area_width / 2.0
    ];
    vec![
        left_center_circle_junction,
        right_center_circle_junction,
        left_center_t_junction,
        right_center_t_junction,
        left_opponent_penalty_box_corner,
        right_opponent_penalty_box_corner,
        left_own_penalty_box_corner,
        right_own_penalty_box_corner,
    ]
}