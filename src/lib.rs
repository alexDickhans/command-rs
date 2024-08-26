#![no_std]
extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;
use vexide::core::competition::{CompetitionMode, status};
use crate::command::{AnyCommand, CancelBehavior};
use crate::ScheduleFailure::CantCancelRunning;
use crate::subsystem::AnySubsystem;

pub mod command;
pub mod subsystem;

pub struct CommandScheduler {
    subsystems: BTreeMap<AnySubsystem, AnyCommand>,
    scheduled_commands: Vec<AnyCommand>,
    requirements: BTreeMap<AnySubsystem, AnyCommand>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ScheduleFailure {
    CompetitionDisabled,
    CantCancelRunning
}

impl CommandScheduler {
    pub fn new() -> Self {
        Self {
            subsystems: BTreeMap::new(),
            scheduled_commands: vec![],
            requirements: BTreeMap::new(),
        }
    }

    fn init_command(&mut self, command: AnyCommand) {
        for requirement in command.0.borrow().requirements() {
            self.requirements.insert(requirement.clone(), command.clone());
        }

        command.0.borrow_mut().initialize();

        self.scheduled_commands.push(command);
    }

    pub fn schedule(&mut self, command: AnyCommand) -> Result<(), ScheduleFailure> {
        if self.scheduled(&command) || status().mode() == CompetitionMode::Disabled && !command.0.borrow().runs_when_disabled() {
            return Err(ScheduleFailure::CompetitionDisabled)
        }

        self.require_not_composed(&[command.clone()]);

        let command_borrow = command.0.borrow();

        let requirements = command_borrow.requirements();

        let mut is_disjoint = true;

        for requirement in requirements.iter() {
            if self.requirements.contains_key(requirement) {
                is_disjoint = false;
            }
        }

        if is_disjoint {
            self.init_command(command.clone());
            Ok(())
        } else {
            for requirement in requirements.iter() {
                if let Some(command) = self.requiring(requirement) {
                    if command.0.borrow().cancel_behavior() == CancelBehavior::CancelIncoming {
                        return Err(CantCancelRunning)
                    }
                }
            }

            for requirement in requirements.iter() {
                if let Some(command) = self.requiring(requirement) {
                    self.cancel(command.clone());
                }
            }

            self.init_command(command.clone());

            Ok(())
        }
    }

    pub fn requiring(&self, subsystem: &AnySubsystem) -> Option<&AnyCommand> {
        self.requirements.get(subsystem)
    }

    pub fn register_subsystem(&mut self, subsystem: AnySubsystem, command: AnyCommand) {
        // self.require_not_composed(&command);

        assert!(command.0.borrow().requirements().contains(&subsystem), "Default commands must require their subsystem");
        assert_eq!(command.0.borrow().requirements().len(), 1, "Command must only require one subsystem");

        self.subsystems.insert(subsystem, command);
    }

    pub fn scheduled(&self, command: &AnyCommand) -> bool {
        self.scheduled_commands.contains(command)
    }

    pub fn require_not_composed(&self, _command: &[AnyCommand]) {
        todo!()
    }

    pub fn run(&mut self) {
        for (subsystem, _) in self.subsystems.iter() {
            subsystem.0.borrow_mut().periodic();
        }

        // TODO: POLL BUTTONS

        let mut commands_to_cancel = vec![];

        for command in self.scheduled_commands.iter() {

            if status().mode() == CompetitionMode::Disabled && !command.0.borrow().runs_when_disabled() {
                commands_to_cancel.push(command.clone());
            }

            command.0.borrow_mut().execute();

            if command.0.borrow().finished() {
                command.0.borrow_mut().end(false);

                for requirement in command.0.borrow().requirements() {
                    self.requirements.remove(requirement);
                }
            }
        }

        for cancel_command in commands_to_cancel {
            self.cancel(cancel_command);
        }

        let mut to_schedule = vec![];

        for (subsystem, command) in self.subsystems.iter() {
            if !self.requirements.contains_key(subsystem) {
                to_schedule.push(command.clone());
            }
        }

        for schedule_command in to_schedule {
            self.schedule(schedule_command).unwrap()
        }
    }

    fn cancel(&mut self, command: AnyCommand) {
        if self.scheduled(&command) {
            command.0.borrow_mut().end(true);
            self.scheduled_commands.retain(|x| *x != command);
            for requirements in command.0.borrow().requirements() {
                self.requirements.remove(requirements);
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;
    use core::cell::RefCell;
    use crate::command::Command;
    use crate::subsystem::Subsystem;

    #[derive(Clone, Debug)]
    pub struct MockSubsystem(pub SubsystemState);

    impl Subsystem for MockSubsystem {
        fn periodic(&mut self) {
            self.0.periodic_called = true;
        }
    }

    #[derive(Clone, Debug)]
    pub struct SubsystemState {
        pub name: &'static str,
        pub periodic_called: bool,
    }

    impl MockSubsystem {
        pub fn new(name: &'static str) -> Self {
            MockSubsystem(SubsystemState {
                name,
                periodic_called: false,
            })
        }
    }

    #[derive(Debug)]
    pub struct MockCommand {
        pub runs_when_disabled: bool,
        pub requirements: Vec<AnySubsystem>,
        pub initialized: bool,
        pub executed: bool,
        pub finished: bool,
        pub cancel_behavior: CancelBehavior,
    }

    impl MockCommand {
        pub fn new(requirements: Vec<AnySubsystem>, runs_when_disabled: bool) -> Self {
            MockCommand {
                runs_when_disabled,
                requirements,
                initialized: false,
                executed: false,
                finished: false,
                cancel_behavior: CancelBehavior::CancelIncoming,
            }
        }
    }
    impl Command for MockCommand {

        fn initialize(&mut self) {
            self.initialized = true;
        }

        fn execute(&mut self) {
            self.executed = true;
        }

        fn end(&mut self, _interrupted: bool) {
            self.finished = true;
        }

        fn finished(&self) -> bool {
            self.finished
        }

        fn requirements(&self) -> &[AnySubsystem] {
            self.requirements.as_slice()
        }

        fn runs_when_disabled(&self) -> bool {
            self.runs_when_disabled
        }

        fn cancel_behavior(&self) -> CancelBehavior {
            self.cancel_behavior.clone()
        }
    }

    impl PartialEq for MockCommand {
        fn eq(&self, other: &Self) -> bool {
            self.requirements == other.requirements
        }
    }

    #[test]
    fn test_schedule_command_successfully() {
        let mut scheduler = CommandScheduler::new();

        let subsystem = Arc::new(RefCell::new(MockSubsystem::new("DriveSubsystem")));
        let command = Arc::new(RefCell::new(MockCommand::new(vec![subsystem.into()], true)));

        assert_eq!(scheduler.schedule(command.clone().into()), Ok(()));
        assert_eq!(scheduler.scheduled(&command.clone().into()), true);
        assert_eq!(command.borrow().initialized, true);
    }

    #[test]
    fn test_cannot_schedule_when_competition_disabled() {
        let mut scheduler = CommandScheduler::new();

        let subsystem = Arc::new(RefCell::new(MockSubsystem::new("DriveSubsystem")));
        let command = Arc::new(RefCell::new(MockCommand::new(vec![subsystem.into()], false)));

        // Mock competition status to be disabled
        let competition_status = CompetitionMode::Disabled;

        if competition_status == CompetitionMode::Disabled {
            assert_eq!(scheduler.schedule(command.into()), Err(ScheduleFailure::CompetitionDisabled));
        }
    }

    #[test]
    fn test_cancel_running_command() {
        let mut scheduler = CommandScheduler::new();

        let subsystem = Arc::new(RefCell::new(MockSubsystem::new("DriveSubsystem")));
        let command = Arc::new(RefCell::new(MockCommand::new(vec![subsystem.into()], true)));

        // Schedule the command
        scheduler.schedule(command.clone().into()).unwrap();

        // Cancel the running command
        scheduler.cancel(command.clone().into());

        assert_eq!(scheduler.scheduled(&command.clone().into()), false);
        assert_eq!(command.borrow().finished(), true);
    }

    #[test]
    fn test_run_scheduler() {
        let mut scheduler = CommandScheduler::new();

        let subsystem = Arc::new(RefCell::new(MockSubsystem::new("DriveSubsystem")));
        let command = Arc::new(RefCell::new(MockCommand::new(vec![subsystem.into()], true)));

        // Schedule the command
        scheduler.schedule(command.clone().into()).unwrap();

        // Run the scheduler
        scheduler.run();

        assert_eq!(command.clone().borrow().executed, true);
    }
}
