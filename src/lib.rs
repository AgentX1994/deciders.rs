// These link references will override the link references at the bottom of README.md,
// allowing for the links to point the same place both in GitHub and on docs.rs.
// See https://linebender.org/blog/doc-include/
//! [`Decider<C, E, So, Si>`]: crate::deciders::Decider
//! [`decide(command: &C, state: &Si) -> Vec<E>`]: crate::deciders::Decider::decide
//! [`evolve(state: &Si, event: &E) -> So`]: crate::deciders::Decider::evolve
//! [`initial_state() -> So`]: crate::deciders::Decider::initial_state
//! [`is_terminal(state: &Si) -> bool`]: crate::deciders::Decider::is_terminal
//! [`Process<E, C, S>`]: crate::processes::Process
//! [`evolve(state: &S, event: &E) -> S`]: crate::processes::Process::evolve
//! [`resume(state: &S) -> Vec<C>`]: crate::processes::Process::resume
//! [`react(state: &S, event: &E) -> Vec<C>`]: crate::processes::Process::react
//! [`initial_state() -> S`]: crate::processes::Process::initial_state
//! [`is_terminal(state: &S) -> bool`]: crate::processes::Process::is_terminal
//! [`ComposedDeciders`]: crate::deciders::ComposedDeciders
//! [`Either`]: crate::utilities::Either
//! [`ManyDecider`]: crate::deciders::ManyDecider
//! [`AdaptedDecider`]: crate::deciders::AdaptedDecider
//! [`FallibleConverter`]: crate::utilities::FallibleConverter
//! [`InfallibleConverter`]: crate::utilities::InfallibleConverter
//! [`MappedDecider`]: crate::deciders::MappedDecider
//! [`Map2Deciders`]: crate::deciders::Map2Deciders
//! [`AppliedDecider`]: crate::deciders::AppliedDecider
//! [`AdaptedProcess`]: crate::processes::AdaptedProcess
//! [`EventConverter`]: crate::processes::EventConverter
//! [`CommandConverter`]: crate::processes::CommandConverter
//! [`collect_fold`]: crate::processes::collect_fold
//! [`CombinedProcessDecider`]: crate::processes::CombinedProcessDecider
//! [`InMemoryRunner`]: crate::utilities::InMemoryRunner
//! [examples]: https://github.com/AgentX1994/deciders.rs/blob/main/examples
//! [integration tests]: https://github.com/AgentX1994/deciders.rs/blob/main/tests/integrations.rs
#![doc = include_str!("../README.md")]
pub mod deciders;
pub mod processes;
pub mod utilities;
