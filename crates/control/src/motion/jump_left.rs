use color_eyre::{eyre::Context, Result};
use context_attribute::context;
use framework::MainOutput;
use motionfile::MotionFile;
use types::{
    CycleTime, Joints, JointsCommand, MotionSafeExits, MotionSelection, MotionType,
    SensorData,
};

use motionfile::SplineInterpolator;

pub struct JumpLeft {
    interpolator: SplineInterpolator<Joints<f32>>,
}

#[context]
pub struct CreationContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,

    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub jump_left_joints_command: MainOutput<JointsCommand<f32>>,
}

impl JumpLeft {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            interpolator: MotionFile::from_path("etc/motions/jump_left.json")?.try_into()?,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        if context.motion_selection.current_motion == MotionType::JumpLeft {
            self.interpolator.advance_by(last_cycle_duration);
        } else {
            self.interpolator.reset();
        }

        context.motion_safe_exits[MotionType::JumpLeft] = self.interpolator.is_finished();

        Ok(MainOutputs {
            jump_left_joints_command: JointsCommand {
                positions: self
                    .interpolator
                    .value()
                    .wrap_err("error computing interpolation in jump_left")?,
                stiffnesses: Joints::fill(if self.interpolator.is_finished() {
                    0.0
                } else {
                    0.9
                }),
            }
            .into(),
        })
    }
}
