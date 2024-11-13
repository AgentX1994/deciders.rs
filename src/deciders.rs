use std::{collections::HashMap, marker::PhantomData};

use crate::utilities::{Either, FallibleConverter, InfallibleConverter};

/// A trait representing a Decider
///
/// A decider is a type that takes in commands of type `C` and a state of type `Si`, and returns
/// events of type `E`. Those events are then used to update (or "evolve") the state into a new
/// state of type `So`.
///
/// In many deciders, the state types `So` and `Si` are the same type, but having different types
/// is useful in order to facilitate more powerful composition.
///
/// Note that all methods defined on this trait do not take self, so it is useless to make a
/// concrete instance of an object with this trait.
pub trait Decider<C, E, So, Si> {
    /// Given an incoming command `command` and the current state of this decider `state`, output a
    /// vector of events that the command and state combination generates.
    fn decide(command: &C, state: &Si) -> Vec<E>;

    /// Given the current state `state` and an event `event`, return the new state of this decider.
    fn evolve(state: &Si, event: &E) -> So;

    /// Returns the initial state of this decider.
    fn initial_state() -> So;

    /// Given the current state of this decider `state`, return whether or not this decider has
    /// reached an end state.
    fn is_terminal(state: &Si) -> bool;
}

/// A type that combines two deciders, `D1` and `D2`, into a single decider.
///
/// The commands and events become instances of the Either type, allowing for passing commands and
/// events, and receiving events from either decider. The states become tuples of both deciders'
/// states, in order to keep the states of both at the same time.
///
/// Each decider must have a singular state type, i.e. `Si` == `So` for both deciders, but those
/// types can be different between both deciders.
pub struct ComposedDeciders<D1, C1, E1, S1, D2, C2, E2, S2> {
    decider1: PhantomData<D1>,
    command1: PhantomData<C1>,
    event1: PhantomData<E1>,
    state1: PhantomData<S1>,
    decider2: PhantomData<D2>,
    command2: PhantomData<C2>,
    event2: PhantomData<E2>,
    state2: PhantomData<S2>,
}

impl<D1, C1, E1, S1, D2, C2, E2, S2> Decider<Either<C1, C2>, Either<E1, E2>, (S1, S2), (S1, S2)>
    for ComposedDeciders<D1, C1, E1, S1, D2, C2, E2, S2>
where
    S1: Copy + Clone,
    S2: Copy + Clone,
    D1: Decider<C1, E1, S1, S1>,
    D2: Decider<C2, E2, S2, S2>,
{
    fn decide(command: &Either<C1, C2>, state: &(S1, S2)) -> Vec<Either<E1, E2>> {
        match command {
            Either::Left(l) => D1::decide(l, &state.0)
                .into_iter()
                .map(Either::Left)
                .collect(),
            Either::Right(r) => D2::decide(r, &state.1)
                .into_iter()
                .map(Either::Right)
                .collect(),
        }
    }

    fn evolve(state: &(S1, S2), event: &Either<E1, E2>) -> (S1, S2) {
        match event {
            Either::Left(e) => (D1::evolve(&state.0, e), state.1),
            Either::Right(e) => (state.0, D2::evolve(&state.1, e)),
        }
    }

    fn initial_state() -> (S1, S2) {
        (D1::initial_state(), D2::initial_state())
    }

    fn is_terminal(state: &(S1, S2)) -> bool {
        D1::is_terminal(&state.0) && D2::is_terminal(&state.1)
    }
}

/// A type for using `N` instances of the same decider type, giving each a name.
///
/// This allows for running an indeterminate number of the same decider in parallel, where each has
/// a simple string name to refer to it. The commands and events become tuples of the name and
/// command or event, while the state becomes a HashMap mapping each name to the current state of
/// the decider with that name.
pub struct ManyDecider<D, C, E, S>
where
    S: Copy + Clone,
    D: Decider<C, E, S, S>,
{
    decider: PhantomData<D>,
    command: PhantomData<C>,
    event: PhantomData<E>,
    state: PhantomData<S>,
}

impl<D, C, E, S> Decider<(String, C), (String, E), HashMap<String, S>, HashMap<String, S>>
    for ManyDecider<D, C, E, S>
where
    S: Copy + Clone,
    D: Decider<C, E, S, S>,
{
    fn decide((id, command): &(String, C), states: &HashMap<String, S>) -> Vec<(String, E)> {
        let state = match states.get(id) {
            Some(s) => *s,
            None => D::initial_state(),
        };
        D::decide(command, &state)
            .into_iter()
            .map(|e| (id.clone(), e))
            .collect()
    }

    fn evolve(states: &HashMap<String, S>, (id, event): &(String, E)) -> HashMap<String, S> {
        let state = match states.get(id) {
            Some(s) => *s,
            None => D::initial_state(),
        };
        let new_state = D::evolve(&state, event);
        let mut states = states.clone();
        states.insert(id.clone(), new_state);
        states
    }

    fn initial_state() -> HashMap<String, S> {
        HashMap::new()
    }

    fn is_terminal(states: &HashMap<String, S>) -> bool {
        states.values().all(|s| D::is_terminal(s))
    }
}

/// Adapts a decider to use different input and output types.
///
/// This type allows for converting a decider that implements `Decider<Cd, Ed, Sd, Sd>` into a
/// decider that implements `Decider<Cn, En, Sd, Sn>`.
///
/// It does this by taking 4 additional "converter" types, that can convert to and from the
/// decider's native types into the new types:
///
/// - `CC` - The command converter that implements [`FallibleConverter`], which converts the *new* command
///   type into the decider's *native* command type, with the option of failure.
/// - `ENC` - The event converter that implements [`FallibleConverter`], which converts the *new* event
///   type into the decider's *native* event type, with the option of failure.
/// - `EDC` - The event converter that implements [`InfallibleConverter`], which converts the decider's
///   *native* event type into the *new* event type.
/// - `SC` - The state converter that implements [`InfallibleConverter`], which converts the *new*
///   state type into the decider's *native* state type.
///
/// > *NOTE*: The output states are still in the decider's native state type. If this is not desired,
/// > then this can be combines with [`MappedDecider`] to modify the output type to the desired type.
pub struct AdaptedDecider<D, CC, ENC, EDC, SC, En, Ed, Cn, Cd, Sn, Sd>
where
    D: Decider<Cd, Ed, Sd, Sd>,
    CC: FallibleConverter<Cn, Cd>,
    ENC: FallibleConverter<En, Ed>,
    EDC: InfallibleConverter<Ed, En>,
    SC: InfallibleConverter<Sn, Sd>,
{
    decider: PhantomData<D>,
    command_converter: PhantomData<CC>,
    event_new_converter: PhantomData<ENC>,
    event_decider_converter: PhantomData<EDC>,
    state_converter: PhantomData<SC>,
    event_new: PhantomData<En>,
    event_decider: PhantomData<Ed>,
    command_new: PhantomData<Cn>,
    command_decider: PhantomData<Cd>,
    state_new: PhantomData<Sn>,
    state_decider: PhantomData<Sd>,
}

impl<D, CC, ENC, EDC, SC, En, Ed, Cn, Cd, Sn, Sd> Decider<Cn, En, Sd, Sn>
    for AdaptedDecider<D, CC, ENC, EDC, SC, En, Ed, Cn, Cd, Sn, Sd>
where
    D: Decider<Cd, Ed, Sd, Sd>,
    CC: FallibleConverter<Cn, Cd>,
    ENC: FallibleConverter<En, Ed>,
    EDC: InfallibleConverter<Ed, En>,
    SC: InfallibleConverter<Sn, Sd>,
{
    fn decide(command: &Cn, state: &Sn) -> Vec<En> {
        match CC::convert(command) {
            Some(c) => D::decide(&c, &SC::convert(state))
                .into_iter()
                .map(|e| EDC::convert(&e))
                .collect(),
            None => vec![],
        }
    }

    fn evolve(state: &Sn, event: &En) -> Sd {
        match ENC::convert(event) {
            Some(e) => D::evolve(&SC::convert(state), &e),
            None => SC::convert(state),
        }
    }

    fn initial_state() -> Sd {
        D::initial_state()
    }

    fn is_terminal(state: &Sn) -> bool {
        D::is_terminal(&SC::convert(state))
    }
}

/// A type to modify the output state type of a decider
///
/// This type takes a Decider and an additional `SC` type, which implements the
/// [`InfallibleConverter`] trait to convert the decider's native output state type into a new
/// output state type.
pub struct MappedDecider<D, SC, C, E, Sn, Sdo, Sdi>
where
    D: Decider<C, E, Sdo, Sdi>,
    SC: InfallibleConverter<Sdo, Sn>,
{
    decider: PhantomData<D>,
    state_converter: PhantomData<SC>,
    command: PhantomData<C>,
    event: PhantomData<E>,
    state_new: PhantomData<Sn>,
    state_decider_out: PhantomData<Sdo>,
    state_decider_in: PhantomData<Sdi>,
}

impl<D, SC, C, E, Sn, Sdo, Sdi> Decider<C, E, Sn, Sdi> for MappedDecider<D, SC, C, E, Sn, Sdo, Sdi>
where
    D: Decider<C, E, Sdo, Sdi>,
    SC: InfallibleConverter<Sdo, Sn>,
{
    fn decide(command: &C, state: &Sdi) -> Vec<E> {
        D::decide(command, state)
    }

    fn evolve(state: &Sdi, event: &E) -> Sn {
        SC::convert(&D::evolve(state, event))
    }

    fn initial_state() -> Sn {
        SC::convert(&D::initial_state())
    }

    fn is_terminal(state: &Sdi) -> bool {
        D::is_terminal(state)
    }
}

/// A type to combine two deciders that take the same input types, and transform their output types
/// into a common output type.
///
/// This type combines two decider types, simply concatenating the outputs of their decide method.
///
/// It also takes an `SC` type, which must implement [`InfallibleConverter`], which takes as input
/// the tuple of both decider's output state types, and return a new state type, which is what the
/// output state type of this decider will be.
pub struct Map2Deciders<D1, D2, SC, C, E, Si, S1, S2, So>
where
    D1: Decider<C, E, S1, Si>,
    D2: Decider<C, E, S2, Si>,
    SC: InfallibleConverter<(S1, S2), So>,
{
    decider1: PhantomData<D1>,
    decider2: PhantomData<D2>,
    state_combiner: PhantomData<SC>,
    command: PhantomData<C>,
    event: PhantomData<E>,
    state_initial: PhantomData<Si>,
    state_decider_1: PhantomData<S1>,
    state_decider_2: PhantomData<S2>,
    state_output: PhantomData<So>,
}

impl<D1, D2, SC, C, E, Si, S1, S2, So> Decider<C, E, So, Si>
    for Map2Deciders<D1, D2, SC, C, E, Si, S1, S2, So>
where
    D1: Decider<C, E, S1, Si>,
    D2: Decider<C, E, S2, Si>,
    SC: InfallibleConverter<(S1, S2), So>,
{
    fn decide(command: &C, state: &Si) -> Vec<E> {
        D1::decide(command, state)
            .into_iter()
            .chain(D2::decide(command, state))
            .collect()
    }

    fn evolve(state: &Si, event: &E) -> So {
        let s1 = D1::evolve(state, event);
        let s2 = D2::evolve(state, event);
        SC::convert(&(s1, s2))
    }

    fn initial_state() -> So {
        SC::convert(&(D1::initial_state(), D2::initial_state()))
    }

    fn is_terminal(state: &Si) -> bool {
        D1::is_terminal(state) && D2::is_terminal(state)
    }
}

/// This type takes a decider, whose output state is a function, and applies that function to the
/// output state of a second decider.
pub struct AppliedDecider<FD, D, C, E, Si, Sd, So>
where
    FD: Decider<C, E, fn(Sd) -> So, Si>,
    D: Decider<C, E, Sd, Si>,
{
    function_decider: PhantomData<FD>,
    decider: PhantomData<D>,
    command: PhantomData<C>,
    event: PhantomData<E>,
    state_initial: PhantomData<Si>,
    state_decider: PhantomData<Sd>,
    state_output: PhantomData<So>,
}

impl<FD, D, C, E, Si, Sd, So> Decider<C, E, So, Si> for AppliedDecider<FD, D, C, E, Si, Sd, So>
where
    FD: Decider<C, E, fn(Sd) -> So, Si>,
    D: Decider<C, E, Sd, Si>,
{
    fn decide(command: &C, state: &Si) -> Vec<E> {
        FD::decide(command, state)
            .into_iter()
            .chain(D::decide(command, state))
            .collect()
    }

    fn evolve(state: &Si, event: &E) -> So {
        let s1 = FD::evolve(state, event);
        let s2 = D::evolve(state, event);
        s1(s2)
    }

    fn initial_state() -> So {
        FD::initial_state()(D::initial_state())
    }

    fn is_terminal(state: &Si) -> bool {
        FD::is_terminal(state) && D::is_terminal(state)
    }
}
