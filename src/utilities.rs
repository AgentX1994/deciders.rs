use std::{fmt::Debug, marker::PhantomData};

use crate::deciders::Decider;

/// A simple enum representing one of two types.
///
/// This is a reimplementation of the `Either` type as seen in Haskell or F#.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Either<L, R> {
    /// A value of the left type, `L`.
    Left(L),
    /// A value of the right type, `R`.
    Right(R),
}

/// A simple trait used for converting one type to another, with the option for failure by
/// returning `None`
///
/// This is not just using [`std::convert::TryInto`]/[`std::convert::TryFrom`], since that
/// restricts users to one implementation per type pair.
pub trait FallibleConverter<I, O> {
    /// Converts the input of type `I` into an optional output of type `O`
    fn convert(input: &I) -> Option<O>;
}

/// A simple trait used for converting one type to another that must succeed.
///
/// This is not just using [`std::convert::Into`]/[`std::convert::From`], since that
/// restricts users to one implementation per type pair.
pub trait InfallibleConverter<I, O> {
    /// Converts the input of type `I` into an output of type `O`
    fn convert(input: &I) -> O;
}

/// A type that wraps a decider type and stores that decider's state type internally and exposing a
/// simpler interface.
///
/// Since this type stores the decider's state internally, it can just take in commands and return
/// the corresponding vector of events, freeing the user from having to keep track of the decider's
/// state.
pub struct InMemoryRunner<C, E, S, D>
where
    D: Decider<C, E, S, S>,
{
    state: S,
    command: PhantomData<C>,
    event: PhantomData<E>,
    decider: PhantomData<D>,
}

impl<C, E, S, D> InMemoryRunner<C, E, S, D>
where
    D: Decider<C, E, S, S>,
{
    /// Constructs a new `InMemoryRunner`, initializing the state to the default initial state of
    /// the decider.
    pub fn new() -> Self {
        Self {
            state: D::initial_state(),
            command: PhantomData,
            event: PhantomData,
            decider: PhantomData,
        }
    }

    /// Constructs a new `InMemoryRunner`, initializing the state to the given state
    pub fn with_state(state: S) -> Self {
        Self {
            state,
            command: PhantomData,
            event: PhantomData,
            decider: PhantomData,
        }
    }

    /// Feeds the given command `command` through the decider and returns the generated list of
    /// events.
    ///
    /// Also evolves the internal state of the decider according to the generated events.
    pub fn command(&mut self, command: &C) -> Vec<E> {
        let events = D::decide(command, &self.state);
        for e in events.iter() {
            self.state = D::evolve(&self.state, e);
        }
        events
    }

    /// Returns a reference to the current state of the decider.
    pub fn get_state(&self) -> &S {
        &self.state
    }
}

impl<C, E, S, D> Default for InMemoryRunner<C, E, S, D>
where
    D: Decider<C, E, S, S>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<C, E, S, D> Debug for InMemoryRunner<C, E, S, D>
where
    D: Decider<C, E, S, S>,
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryRunner")
            .field("state", &self.state)
            .finish()
    }
}
