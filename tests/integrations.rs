use deciders_rs::deciders::{
    AdaptedDecider, ComposedDeciders, Decider, ManyDecider, MappedDecider,
};
use deciders_rs::processes::{AdaptedProcess, CombinedProcessDecider, Process};
use deciders_rs::utilities::{Either, FallibleConverter, InMemoryRunner, InfallibleConverter};
use std::collections::HashMap;

struct NeutralDecider;

impl Decider<(), (), (), ()> for NeutralDecider {
    fn decide(_command: &(), _state: &()) -> Vec<()> {
        vec![]
    }

    fn evolve(_state: &(), _event: &()) {}

    fn initial_state() {}

    fn is_terminal(_state: &()) -> bool {
        true
    }
}

mod bulb {
    use super::Decider;

    #[derive(Copy, Clone, Debug)]
    pub enum Command {
        Fit { max_uses: u64 },
        SwitchOn,
        SwitchOff,
    }

    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum Event {
        Fitted { max_uses: u64 },
        SwitchedOn,
        SwitchedOff,
        Blew,
    }

    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum Status {
        On,
        Off,
    }

    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum State {
        NotFitted,
        Working { status: Status, remaining_uses: u64 },
        Blown,
    }

    #[derive(Copy, Clone)]
    pub struct Bulb;

    impl Decider<Command, Event, State, State> for Bulb {
        fn decide(command: &Command, state: &State) -> Vec<Event> {
            match (command, state) {
                (Command::Fit { max_uses }, State::NotFitted) => {
                    vec![Event::Fitted {
                        max_uses: *max_uses,
                    }]
                }
                (Command::Fit { .. }, ..) => panic!("Bulb has already been fitted!"),
                (
                    Command::SwitchOn,
                    State::Working {
                        status: Status::Off,
                        remaining_uses,
                    },
                ) if *remaining_uses > 0 => vec![Event::SwitchedOn],
                (
                    Command::SwitchOn,
                    State::Working {
                        status: Status::Off,
                        remaining_uses,
                    },
                ) if *remaining_uses == 0 => vec![Event::Blew],
                (
                    Command::SwitchOff,
                    State::Working {
                        status: Status::On, ..
                    },
                ) => vec![Event::SwitchedOff],
                _ => Vec::new(),
            }
        }

        fn evolve(state: &State, event: &Event) -> State {
            match (state, event) {
                (State::NotFitted, Event::Fitted { max_uses }) => State::Working {
                    status: Status::Off,
                    remaining_uses: *max_uses,
                },
                (State::Working { remaining_uses, .. }, Event::SwitchedOn) => State::Working {
                    status: Status::On,
                    remaining_uses: *remaining_uses - 1,
                },
                (State::Working { remaining_uses, .. }, Event::SwitchedOff) => State::Working {
                    status: Status::Off,
                    remaining_uses: *remaining_uses,
                },
                (State::Working { .. }, Event::Blew) => State::Blown,
                _ => *state,
            }
        }

        fn initial_state() -> State {
            State::NotFitted
        }

        fn is_terminal(state: &State) -> bool {
            matches!(*state, State::Blown)
        }
    }
}

mod cat {
    use super::Decider;

    #[derive(Copy, Clone, Debug)]
    pub enum Command {
        WakeUp,
        GetToSleep,
    }

    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum Event {
        WokeUp,
        GotToSleep,
    }

    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum State {
        Awake,
        Asleep,
    }

    #[derive(Copy, Clone)]
    pub struct Cat;

    impl Decider<Command, Event, State, State> for Cat {
        fn decide(command: &Command, state: &State) -> Vec<Event> {
            match (command, state) {
                (Command::WakeUp, State::Asleep) => vec![Event::WokeUp],
                (Command::GetToSleep, State::Awake) => vec![Event::GotToSleep],
                _ => vec![],
            }
        }

        fn evolve(state: &State, event: &Event) -> State {
            match (state, event) {
                (State::Awake, Event::GotToSleep) => State::Asleep,
                (State::Asleep, Event::WokeUp) => State::Awake,
                _ => *state,
            }
        }

        fn initial_state() -> State {
            State::Awake
        }

        fn is_terminal(_state: &State) -> bool {
            false
        }
    }
}

mod cat_light {
    use super::Process;

    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum Event {
        SwitchedOn,
        WokeUp,
    }

    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum Command {
        WakeUp,
    }

    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum State {
        Idle,
        WakingUp,
    }

    pub struct CatLight;

    impl Process<Event, Command, State> for CatLight {
        fn evolve(_state: &State, event: &Event) -> State {
            match event {
                Event::SwitchedOn => State::WakingUp,
                Event::WokeUp => State::Idle,
            }
        }

        fn resume(state: &State) -> Vec<Command> {
            match state {
                State::Idle => vec![],
                State::WakingUp => vec![Command::WakeUp],
            }
        }

        fn react(state: &State, event: &Event) -> Vec<Command> {
            match (state, event) {
                (State::WakingUp, Event::SwitchedOn) => {
                    vec![Command::WakeUp]
                }
                _ => vec![],
            }
        }

        fn initial_state() -> State {
            State::Idle
        }

        fn is_terminal(state: &State) -> bool {
            matches!(state, State::Idle)
        }
    }
}

fn run_decider<C, E, S, D>(events: &[E], command: &C) -> Vec<E>
where
    D: Decider<C, E, S, S>,
{
    let new_state = events
        .iter()
        .fold(D::initial_state(), |s, e| D::evolve(&s, e));
    D::decide(command, &new_state)
}

#[test]
#[allow(clippy::unit_cmp)]
fn neutral_test() {
    assert_eq!(NeutralDecider::initial_state(), ());
    assert!(NeutralDecider::is_terminal(&()));
    assert_eq!(NeutralDecider::decide(&(), &()), vec![]);
    assert_eq!(NeutralDecider::evolve(&(), &()), ());
}

#[test]
fn bulb_test_1() {
    assert_eq!(
        run_decider::<bulb::Command, bulb::Event, bulb::State, bulb::Bulb>(
            &[],
            &bulb::Command::Fit { max_uses: 5 }
        ),
        [bulb::Event::Fitted { max_uses: 5 }]
    )
}

#[test]
fn bulb_test_2() {
    assert_eq!(
        run_decider::<bulb::Command, bulb::Event, bulb::State, bulb::Bulb>(
            &[bulb::Event::Fitted { max_uses: 5 }],
            &bulb::Command::SwitchOn
        ),
        [bulb::Event::SwitchedOn]
    )
}

#[test]
fn bulb_test_3() {
    assert_eq!(
        run_decider::<bulb::Command, bulb::Event, bulb::State, bulb::Bulb>(
            &[bulb::Event::Fitted { max_uses: 5 }, bulb::Event::SwitchedOn],
            &bulb::Command::SwitchOn
        ),
        []
    )
}

#[test]
fn bulb_test_4() {
    assert_eq!(
        run_decider::<bulb::Command, bulb::Event, bulb::State, bulb::Bulb>(
            &[bulb::Event::Fitted { max_uses: 5 }, bulb::Event::SwitchedOn],
            &bulb::Command::SwitchOff
        ),
        [bulb::Event::SwitchedOff]
    )
}

#[test]
fn bulb_test_5() {
    assert_eq!(
        run_decider::<bulb::Command, bulb::Event, bulb::State, bulb::Bulb>(
            &[
                bulb::Event::Fitted { max_uses: 5 },
                bulb::Event::SwitchedOn,
                bulb::Event::SwitchedOff
            ],
            &bulb::Command::SwitchOff
        ),
        []
    )
}

#[test]
fn bulb_test_6() {
    assert_eq!(
        run_decider::<bulb::Command, bulb::Event, bulb::State, bulb::Bulb>(
            &[
                bulb::Event::Fitted { max_uses: 1 },
                bulb::Event::SwitchedOn,
                bulb::Event::SwitchedOff
            ],
            &bulb::Command::SwitchOn
        ),
        [bulb::Event::Blew]
    )
}

#[test]
fn cat_test_1() {
    assert_eq!(
        run_decider::<cat::Command, cat::Event, cat::State, cat::Cat>(
            &[],
            &cat::Command::GetToSleep
        ),
        [cat::Event::GotToSleep]
    )
}

#[test]
fn cat_test_2() {
    assert_eq!(
        run_decider::<cat::Command, cat::Event, cat::State, cat::Cat>(
            &[cat::Event::GotToSleep],
            &cat::Command::WakeUp
        ),
        [cat::Event::WokeUp]
    )
}

#[test]
fn cat_test_3() {
    assert_eq!(
        run_decider::<cat::Command, cat::Event, cat::State, cat::Cat>(
            &[cat::Event::GotToSleep],
            &cat::Command::GetToSleep
        ),
        []
    )
}

#[test]
fn cat_test_4() {
    assert_eq!(
        run_decider::<cat::Command, cat::Event, cat::State, cat::Cat>(
            &[cat::Event::GotToSleep, cat::Event::WokeUp],
            &cat::Command::WakeUp
        ),
        []
    )
}

#[test]
fn compose_test_1() {
    type CatAndBulb = ComposedDeciders<
        cat::Cat,
        cat::Command,
        cat::Event,
        cat::State,
        bulb::Bulb,
        bulb::Command,
        bulb::Event,
        bulb::State,
    >;
    use Either::*;
    assert_eq!(
        run_decider::<
            Either<cat::Command, bulb::Command>,
            Either<cat::Event, bulb::Event>,
            (cat::State, bulb::State),
            CatAndBulb,
        >(&[], &Left(cat::Command::GetToSleep)),
        [Left(cat::Event::GotToSleep)]
    );
    assert_eq!(
        run_decider::<
            Either<cat::Command, bulb::Command>,
            Either<cat::Event, bulb::Event>,
            (cat::State, bulb::State),
            CatAndBulb,
        >(&[], &Right(bulb::Command::Fit { max_uses: 5 })),
        [Right(bulb::Event::Fitted { max_uses: 5 })]
    );
}

#[test]
fn many_test_1() {
    type ManyCats = ManyDecider<cat::Cat, cat::Command, cat::Event, cat::State>;
    assert_eq!(
        run_decider::<
            (String, cat::Command),
            (String, cat::Event),
            HashMap<String, cat::State>,
            ManyCats,
        >(&[], &("Floof".to_string(), cat::Command::GetToSleep)),
        [("Floof".to_string(), cat::Event::GotToSleep)]
    );
    assert_eq!(
        run_decider::<
            (String, cat::Command),
            (String, cat::Event),
            HashMap<String, cat::State>,
            ManyCats,
        >(
            &[
                ("Floof".to_string(), cat::Event::GotToSleep),
                ("Floof".to_string(), cat::Event::WokeUp)
            ],
            &("Shadow".to_string(), cat::Command::GetToSleep)
        ),
        [("Shadow".to_string(), cat::Event::GotToSleep)]
    );
}

#[test]
fn adapted_mapped_test_1() {
    enum AdaptedCommand {
        Sleep,
        Wake,
    }
    #[derive(Copy, Clone, Debug, PartialEq)]
    enum AdaptedEvent {
        Slept,
        Woke,
    }
    enum AdaptedState {
        Slep,
        Wake,
    }
    struct CatCommandConverter;

    impl FallibleConverter<AdaptedCommand, cat::Command> for CatCommandConverter {
        fn convert(input: &AdaptedCommand) -> Option<cat::Command> {
            match input {
                AdaptedCommand::Sleep => Some(cat::Command::GetToSleep),
                AdaptedCommand::Wake => Some(cat::Command::WakeUp),
            }
        }
    }

    struct CatEventInConverter;
    impl FallibleConverter<AdaptedEvent, cat::Event> for CatEventInConverter {
        fn convert(input: &AdaptedEvent) -> Option<cat::Event> {
            match input {
                AdaptedEvent::Slept => Some(cat::Event::GotToSleep),
                AdaptedEvent::Woke => Some(cat::Event::WokeUp),
            }
        }
    }

    struct CatEventOutConverter;
    impl InfallibleConverter<cat::Event, AdaptedEvent> for CatEventOutConverter {
        fn convert(input: &cat::Event) -> AdaptedEvent {
            match input {
                cat::Event::GotToSleep => AdaptedEvent::Slept,
                cat::Event::WokeUp => AdaptedEvent::Woke,
            }
        }
    }

    struct CatStateInConverter;
    impl InfallibleConverter<AdaptedState, cat::State> for CatStateInConverter {
        fn convert(input: &AdaptedState) -> cat::State {
            match input {
                AdaptedState::Slep => cat::State::Asleep,
                AdaptedState::Wake => cat::State::Awake,
            }
        }
    }

    struct CatStateOutConverter;
    impl InfallibleConverter<cat::State, AdaptedState> for CatStateOutConverter {
        fn convert(input: &cat::State) -> AdaptedState {
            match input {
                cat::State::Asleep => AdaptedState::Slep,
                cat::State::Awake => AdaptedState::Wake,
            }
        }
    }
    type AdaptedCat = AdaptedDecider<
        cat::Cat,
        CatCommandConverter,
        CatEventInConverter,
        CatEventOutConverter,
        CatStateInConverter,
        AdaptedEvent,
        cat::Event,
        AdaptedCommand,
        cat::Command,
        AdaptedState,
        cat::State,
    >;

    type AdaptedMappedCat = MappedDecider<
        AdaptedCat,
        CatStateOutConverter,
        AdaptedCommand,
        AdaptedEvent,
        AdaptedState,
        cat::State,
        AdaptedState,
    >;

    assert_eq!(
        run_decider::<AdaptedCommand, AdaptedEvent, AdaptedState, AdaptedMappedCat>(
            &[],
            &AdaptedCommand::Sleep
        ),
        &[AdaptedEvent::Slept]
    );

    assert_eq!(
        run_decider::<AdaptedCommand, AdaptedEvent, AdaptedState, AdaptedMappedCat>(
            &[AdaptedEvent::Slept,],
            &AdaptedCommand::Wake
        ),
        &[AdaptedEvent::Woke]
    );

    assert_eq!(
        run_decider::<AdaptedCommand, AdaptedEvent, AdaptedState, AdaptedMappedCat>(
            &[AdaptedEvent::Slept, AdaptedEvent::Woke],
            &AdaptedCommand::Sleep
        ),
        &[AdaptedEvent::Slept]
    );
}

#[test]
fn process_test() {
    use cat_light::*;
    assert_eq!(
        CatLight::evolve(&State::Idle, &Event::SwitchedOn),
        State::WakingUp
    );
    assert_eq!(CatLight::evolve(&State::Idle, &Event::WokeUp), State::Idle);
    assert_eq!(
        CatLight::evolve(&State::WakingUp, &Event::SwitchedOn),
        State::WakingUp
    );
    assert_eq!(
        CatLight::evolve(&State::WakingUp, &Event::WokeUp),
        State::Idle
    );
    assert_eq!(CatLight::resume(&State::Idle), vec![]);
    assert_eq!(CatLight::resume(&State::WakingUp), vec![Command::WakeUp]);
    assert_eq!(CatLight::react(&State::Idle, &Event::SwitchedOn), vec![]);
    assert_eq!(CatLight::react(&State::Idle, &Event::WokeUp), vec![]);
    assert_eq!(
        CatLight::react(&State::WakingUp, &Event::SwitchedOn),
        vec![Command::WakeUp]
    );
    assert_eq!(CatLight::react(&State::WakingUp, &Event::WokeUp), vec![]);
    assert_eq!(CatLight::initial_state(), State::Idle);
    assert!(CatLight::is_terminal(&State::Idle));
    assert!(!CatLight::is_terminal(&State::WakingUp));
}

#[test]
fn compose_process() {
    type CatAndBulb = ComposedDeciders<
        cat::Cat,
        cat::Command,
        cat::Event,
        cat::State,
        bulb::Bulb,
        bulb::Command,
        bulb::Event,
        bulb::State,
    >;
    use Either::*;
    struct CatLightEventConverter;
    struct CatLightCommandConverter;

    impl FallibleConverter<Either<cat::Event, bulb::Event>, cat_light::Event>
        for CatLightEventConverter
    {
        fn convert(event: &Either<cat::Event, bulb::Event>) -> Option<cat_light::Event> {
            match event {
                Left(cat::Event::WokeUp) => Some(cat_light::Event::WokeUp),
                Right(bulb::Event::SwitchedOn) => Some(cat_light::Event::SwitchedOn),
                _ => None,
            }
        }
    }

    impl InfallibleConverter<cat_light::Command, Either<cat::Command, bulb::Command>>
        for CatLightCommandConverter
    {
        fn convert(command: &cat_light::Command) -> Either<cat::Command, bulb::Command> {
            match command {
                cat_light::Command::WakeUp => Left(cat::Command::WakeUp),
            }
        }
    }
    type CatLightProcess = AdaptedProcess<
        cat_light::CatLight,
        Either<cat::Event, bulb::Event>,
        cat_light::Event,
        cat_light::Command,
        Either<cat::Command, bulb::Command>,
        cat_light::State,
        CatLightEventConverter,
        CatLightCommandConverter,
    >;

    type CatBulb = CombinedProcessDecider<
        CatLightProcess,
        CatAndBulb,
        Either<cat::Event, bulb::Event>,
        Either<cat::Command, bulb::Command>,
        cat_light::State,
        (cat::State, bulb::State),
    >;

    assert_eq!(
        run_decider::<
            Either<cat::Command, bulb::Command>,
            Either<cat::Event, bulb::Event>,
            (cat_light::State, (cat::State, bulb::State)),
            CatBulb,
        >(&[], &Right(bulb::Command::Fit { max_uses: 5 })),
        vec![Right(bulb::Event::Fitted { max_uses: 5 })]
    );

    let mut in_mem_runner = InMemoryRunner::<_, _, _, CatBulb>::new();

    in_mem_runner.command(&Right(bulb::Command::Fit { max_uses: 5 }));
    in_mem_runner.command(&Left(cat::Command::GetToSleep));
    in_mem_runner.command(&Left(cat::Command::WakeUp));
    in_mem_runner.command(&Right(bulb::Command::SwitchOn));
    in_mem_runner.command(&Right(bulb::Command::SwitchOff));
    assert_eq!(
        *in_mem_runner.get_state(),
        (
            cat_light::State::WakingUp,
            (
                cat::State::Awake,
                bulb::State::Working {
                    status: bulb::Status::Off,
                    remaining_uses: 4
                }
            )
        )
    );
}
