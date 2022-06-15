use std::{
    convert::TryFrom,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context};
use mlua::Lua;
use nalgebra::{Point2, Vector2};
use serde::Serialize;
use spl_network::{GameState, SplMessage};

use crate::control::Database;

use super::{
    configuration::{Action, Configuration as SimulationConfiguration},
    robot::Robot,
};

#[derive(Clone, Serialize)]
pub struct State {
    pub configuration: SimulationConfiguration,
    pub now: SystemTime,
    pub game_state: GameState,
    pub ball_position: Point2<f32>,
    pub ball_velocity: Vector2<f32>,
    pub broadcasted_spl_messages: Vec<SplMessage>,
}

impl TryFrom<SimulationConfiguration> for State {
    type Error = anyhow::Error;

    fn try_from(configuration: SimulationConfiguration) -> anyhow::Result<Self> {
        Ok(Self {
            configuration,
            now: UNIX_EPOCH,
            game_state: GameState::Initial,
            ball_position: Point2::origin(),
            ball_velocity: Vector2::zeros(),
            broadcasted_spl_messages: vec![],
        })
    }
}

impl State {
    pub fn step(
        &mut self,
        lua: &Lua,
        robots: &mut [Robot],
    ) -> anyhow::Result<Option<Vec<Database>>> {
        let should_terminate = self
            .apply_rules(lua, robots)
            .context("Failed to apply rules")?;
        if should_terminate {
            return Ok(None);
        }

        let databases = self
            .step_robots_and_apply_outputs(robots)
            .context("Failed to step robots and apply outputs")?;

        self.now += self.configuration.time_step;

        Ok(Some(databases))
    }

    fn apply_rules(&mut self, lua: &Lua, robots: &mut [Robot]) -> anyhow::Result<bool> {
        for rule in self.configuration.rules.iter().rev() {
            if !rule
                .is_triggered(lua)
                .context("Failed to check if rule is triggered")?
            {
                continue;
            }

            match rule.action {
                Action::StopSimulation => return Ok(true),
                Action::SetGameState { game_state } => {
                    self.game_state = game_state;
                }
                Action::SetPenalized {
                    robot_index,
                    is_penalized,
                } => match robots.get_mut(robot_index) {
                    Some(robot) => {
                        robot.is_penalized = is_penalized;
                    }
                    None => bail!("Robot index {} out of range", robot_index),
                },
                Action::SetRobotToField {
                    robot_index,
                    robot_to_field,
                } => match robots.get_mut(robot_index) {
                    Some(robot) => {
                        robot.robot_to_field = robot_to_field;
                    }
                    None => bail!("Robot index {} out of range", robot_index),
                },
            }
        }

        Ok(false)
    }

    fn step_robots_and_apply_outputs(
        &mut self,
        robots: &mut [Robot],
    ) -> anyhow::Result<Vec<Database>> {
        let mut new_ball_velocity = Vector2::zeros();
        let mut new_broadcasted_spl_messages = vec![];
        let mut databases = vec![];
        for robot in robots.iter_mut() {
            let (database, mut spl_messages, ball_bounce_direction) =
                robot.step(self).with_context(|| {
                    format!(
                        "Failed to step robot with player number {}",
                        robot.configuration.player_number
                    )
                })?;

            databases.push(database);

            new_broadcasted_spl_messages.append(&mut spl_messages);

            if let Some(ball_bounce_direction) = ball_bounce_direction {
                new_ball_velocity += ball_bounce_direction;
            }
        }

        self.broadcasted_spl_messages = new_broadcasted_spl_messages;

        if new_ball_velocity != Vector2::zeros() {
            self.ball_velocity = new_ball_velocity;
        }
        self.ball_position += self.ball_velocity * self.configuration.time_step.as_secs_f32();
        self.ball_velocity *= self.configuration.ball_velocity_decay_factor;

        Ok(databases)
    }
}