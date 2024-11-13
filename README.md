# Deciders.rs

This is a simple implementation of Aggregate Composition in Rust, as presented in Jérémie Chassaing's talk at DDDEu 2023.

See the talk here: <https://youtu.be/72TOhMpEVlA>

See the F# code that this library is based on here: <https://github.com/thinkbeforecoding/dddeu-2023-deciders>

## What is in this library?

This library contains two main traits:

### [`Decider<C, E, So, Si>`]

This is an aggregate that takes in commands of type `C`, events of type `E`, and states of type `Si`, and returns states of type `So`. For many deciders, `So` will be the same type as `Si`, but they are different here in order to facilitate more powerful composition.

The [`Decider<C, E, So, Si>`] trait contains 4 trait methods:

- [`decide(command: &C, state: &Si) -> Vec<E>`]
  - This function looks at the incoming command `command`, and the current state `state`, and returns a vector of events. Notably, this does not mutate the state at all, just "decides" how to translate the incoming command into a list of events, based on what the current state is.
- [`evolve(state: &Si, event: &E) -> So`]
  - Updates the current state `state` based on the event `event`.
- [`initial_state() -> So`]
  - Returns what state this decider should start in.
- [`is_terminal(state: &Si) -> bool`]
  - Returns whether the passed in state `state` is "terminal", i.e. this decider has reached an end state

Note: All trait methods of deciders do not take a `self` parameter, so they cannot store any state.

Because deciders only change state based on events, not on commands, the entire history of the decider can be saved only by saving the events that it produces. This allows for easy save/resume using something like an append-only log.

### [`Process<E, C, S>`]

A process is a type of aggregate that can react to events (of type `E`) in order to produce new commands (of type `C`), based on its own state (of type `S`). This can be combined with a decider in order to allow a sort of feedback loop, where a command can be passed to the decider, which produces events, which then cause the process to produce more commands to be fed back into the decider (See [`CombinedProcessDecider`] for a reusable implementation of this).

The [`Process<E, C, S>`] trait defines the following 5 trait methods:

- [`evolve(state: &S, event: &E) -> S`]
  - This is analagous to the `evolve` function on deciders, given an event `event` and the current state `state`, returns a new state.
- [`resume(state: &S) -> Vec<C>`]
  - Given a state `state`, returns a list of commands to return to that state.
- [`react(state: &S, event: &E) -> Vec<C>`]
  - Given the current state `state` and an incoming event `event`, returns a new list of commands in reaction to that event.
- [`initial_state() -> S`]
  - Returns the initial state that this process should start in.
- [`is_terminal(state: &S) -> bool`]
  - Returns whether the passed in state `state` is "terminal", i.e. this process has reached an end state

Note: All trait methods of processes do not take a `self` parameter, so they cannot store any state.

Just like deciders, processes only change state based on events, not on commands, the entire history of the decider can be saved only by saving the events that it produces. This allows for easy save/resume using something like an append-only log.

### Other Utilities

In addition to the above two traits, this crate contains various utilities that were contained in the talk's companion code, including:

- [`ComposedDeciders`]
  - Takes two decider types and combines them into one decider, like the `zip` method on iterators. Uses a Rust based implementation of the [`Either`] type from languages like F# and Haskell, in order to allow passing a command to either decider, depending on which variant of [`Either`] is given. The state becomes a tuple of both deciders' states.
- [`ManyDecider`]
  - Allows for using `N` of the same decider type, using strings as names for each decider. Commands must be bundled together with the string name of the decider they will be used with, and the state is a simple `HashMap<String, S>`.
- [`AdaptedDecider`]
      - Adapts a decider to use different command, event, and state types. To do this, it requires four different converters, which are implemented as types that implement a certain trait:
    - `CC`, the command converter, which must implement the [`FallibleConverter`] trait. Takes in a command of the new type, and returns an optional command of the decider's native command type.
    - `ENC`, the input event converter, which must implement the [`FallibleConverter`] trait. Takes in a event of the new type, and returns an optional event of the decider's native event type.
    - `EDC`, the output event converter, which must implement the [`InfallibleConverter`] trait. Takes in a event of the decider's native type, and returns an event of the new event type.
    - `SC`, the state converter, which must implement the [`InfallibleConverter`] trait. Takes in a state of the new state type, and returns a state of the decider's native state type.
- [`MappedDecider`]
  - This is a simple type which translates the output state type of the given decider into a new output state type. To do this, it takes in a type which implements [`InfallibleConverter`], mapping the deciders native output state type to a new output state type.
- [`Map2Deciders`]
  - This type takes two deciders and a state converter type, which must convert a tuple of both deciders output states, and runs them in sequence, followed by calling the conversion function on both states to get the final output state.
- [`AppliedDecider`]
  - This takes two deciders which take the same input state type, where the first decider's output state is a function that takes in the second deciders output state, and combines them into one decider, where the final output state is the result of running the first decider's output state on the second deciders output state.
  In essence, the evolve method is:
```rust,ignore
let function_to_call = D1::evolve(state, event);
let input_to_function = D2::evolve(state, event);
return function_to_call(&input_to_function);
```
- [`AdaptedProcess`]
  - This is the [`Process<E, C, S>`] equivalent to [`AdaptedDecider`], and adapts a process to use different types for incoming commands and events. It does this by taking in two additional type parameters that implement the [`FallibleConverter`] and the [`InfallibleConverter`] traits, which are used to convert the events and the commands, respectively.
- [`collect_fold`]
  - A helper method that takes in a starting state `state` and a vector of events, and calls `P::evolve` and `P::react` on each event, updating the state accordingly, and returning the final list of all commands that the process generated.
- [`CombinedProcessDecider`]
  - A type that takes in a process type and a decider type, and combines them together into a new type that implements [`Decider<C, E, So, Si>`]. The main implementation is in the `decide` function, which loops over `D::decide` and calling collect_fold on the process until the input command and all commands generated by the process are exhausted.
- [`InMemoryRunner`]
  - A simple helper type which takes in a type that implements [`Decider<C, E, So, Si>`] and stores the state internally, allowing users to simply input commands and receive the list of events that the decider outputs without needing to manually manage the state.

## How to use this library

The first step to use this library is to define a type to implement either the [`Decider<C, E, So, Si>`] trait or [`Process<E, C, S>`] trait on. Since these traits also require at least a command, an event, and a state type, those will be needed as well. A simple example using a decider is the bulb example, as seen in the tests:

```rust
// This defines a new module to store the types corresponding to the bulb decider. This can make referring to the various types cleaner. Another option is to just prefix all types with the name of the decider, like `BulbCommand`, etc.
mod bulb {
    use deciders_rs::deciders::Decider;


    // An enum to represent a command for the bulb decider.
    #[derive(Copy, Clone, Debug)]
    pub enum Command {
        Fit { max_uses: u64 },
        SwitchOn,
        SwitchOff,
    }

    // An enum to represent a event for the bulb decider.
    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum Event {
        Fitted { max_uses: u64 },
        SwitchedOn,
        SwitchedOff,
        Blew,
    }

    // A helper enum to store the status of the bulb
    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum Status {
        On,
        Off,
    }

    // An enum to represent the current state of the bulb
    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum State {
        NotFitted,
        Working { status: Status, remaining_uses: u64 },
        Blown,
    }

    // The type of the bulb decider itself
    #[derive(Copy, Clone)]
    pub struct Bulb;

    // Implements the decider trait for the Bulb type, setting `C` to `bulb::Command`, `E` to `bulb::Event`, and both `So` and `Si` to `bulb::State`.
    impl Decider<Command, Event, State, State> for Bulb {
        fn decide(command: &Command, state: &State) -> Vec<Event> {
            // The events we generate depend on both the command we receive, and the current state.
            match (command, state) {
                // If we receive a Fit command and the bulb is not already fitted, then generate a Fitted event with the `max_uses` from the command.
                (Command::Fit { max_uses }, State::NotFitted) => {
                    vec![Event::Fitted {
                        max_uses: *max_uses,
                    }]
                }
                // Cannot fit a bulb if it has already been fitted.
                (Command::Fit { .. }, ..) => panic!("Bulb has already been fitted!"),
                // If the bulb is currently off and we receive a SwitchOn command, and we have some uses remaining according to our current state, then generate a SwitchedOn event.
                (
                    Command::SwitchOn,
                    State::Working {
                        status: Status::Off,
                        remaining_uses,
                    },
                ) if *remaining_uses > 0 => vec![Event::SwitchedOn],
                // If the bulb is currently off and we receive a SwitchOn command, but our current state says we have no uses remaining, then generate a Blew event.
                (
                    Command::SwitchOn,
                    State::Working {
                        status: Status::Off,
                        remaining_uses,
                    },
                ) if *remaining_uses == 0 => vec![Event::Blew],
                // If the bulb is currently on and we receive a SwitchOff command, then we generate a SwitchedOff event
                (
                    Command::SwitchOff,
                    State::Working {
                        status: Status::On, ..
                    },
                ) => vec![Event::SwitchedOff],
                // Any other combination of input command and current state results in no events.
                _ => Vec::new(),
            }
        }

        fn evolve(state: &State, event: &Event) -> State {
            // The new state depends on both our current state, and the event that is coming in.
            match (state, event) {
                // If the bulb is not fitted, and we receive a Fitted event, then the bulb transitions to a working state where the bulb is not on, and with `max_uses` remaining uses.
                (State::NotFitted, Event::Fitted { max_uses }) => State::Working {
                    status: Status::Off,
                    remaining_uses: *max_uses,
                },
                // If the bulb is in a working state and we get a SwitchedOn event, then set the bulb to be on and subtract one from the remaining uses.
                (State::Working { remaining_uses, .. }, Event::SwitchedOn) => State::Working {
                    status: Status::On,
                    remaining_uses: *remaining_uses - 1,
                },
                // If the bulb is in a working state and we get a SwitchedOff event, then set the bulb to be off.
                (State::Working { remaining_uses, .. }, Event::SwitchedOff) => State::Working {
                    status: Status::Off,
                    remaining_uses: *remaining_uses,
                },
                // If we receive a Blew event, then unconditionally evolve to the Blown state.
                (State::Working { .. }, Event::Blew) => State::Blown,
                // Any other combination just returns the same state
                _ => *state,
            }
        }

        fn initial_state() -> State {
            // The bulb starts off as not fitted
            State::NotFitted
        }

        fn is_terminal(state: &State) -> bool {
            // The Blown state is the only terminal state
            matches!(*state, State::Blown)
        }
    }
}
```

For other examples, see the [examples] and the [integration tests].

[`Decider<C, E, So, Si>`]: https://docs.rs/deciders-rs/latest/deciders-rs/deciders/trait.Decider.html
[`decide(command: &C, state: &Si) -> Vec<E>`]: https://docs.rs/deciders-rs/latest/deciders-rs/deciders/trait.Decider.html#tymethod.decide
[`evolve(state: &Si, event: &E) -> So`]: https://docs.rs/deciders-rs/latest/deciders-rs/deciders/trait.Decider.html#tymethod.evolve
[`initial_state() -> So`]: https://docs.rs/deciders-rs/latest/deciders-rs/deciders/trait.Decider.html#tymethod.initial_state
[`is_terminal(state: &Si) -> So`]: https://docs.rs/deciders-rs/latest/deciders-rs/deciders/trait.Decider.html#tymethod.is_terminal
[`Process<E, C, S>`]: https://docs.rs/deciders-rs/latest/deciders-rs/processes/trait.Process.html
[`evolve(state: &S, event: &E) -> S`]: https://docs.rs/deciders-rs/latest/deciders-rs/processes/trait.Process.html#tymethod.evolve
[`resume(state: &S) -> Vec<C>`]: https://docs.rs/deciders-rs/latest/deciders-rs/processes/trait.Process.html#tymethod.resume
[`react(state: &S, event: &E) -> Vec<C>`]: https://docs.rs/deciders-rs/latest/deciders-rs/processes/trait.Process.html#tymethod.react
[`initial_state() -> S`]: https://docs.rs/deciders-rs/latest/deciders-rs/processes/trait.Process.html#tymethod.initial_state
[`is_terminal(state: &S) -> bool`]: https://docs.rs/deciders-rs/latest/deciders-rs/processes/trait.Process.html#tymethod.is_terminal
[`ComposedDeciders`]: https://docs.rs/deciders-rs/latest/deciders-rs/deciders/struct.ComposedDeciders.html
[`Either`]: https://docs.rs/deciders-rs/latest/deciders-rs/utilities/enum.Either.html
[`ManyDecider`]: https://docs.rs/deciders-rs/latest/deciders-rs/deciders/struct.ManyDecider.html
[`AdaptedDecider`]: https://docs.rs/deciders-rs/latest/deciders-rs/deciders/struct.AdaptedDecider.html
[`FallibleConverter`]: https://docs.rs/deciders-rs/latest/deciders-rs/utilities/trait.FallibleConverter.html
[`InfallibleConverter`]: https://docs.rs/deciders-rs/latest/deciders-rs/utilities/trait.InfallibleConverter.html
[`MappedDecider`]: https://docs.rs/deciders-rs/latest/deciders-rs/deciders/struct.MappedDecider.html
[`Map2Deciders`]: https://docs.rs/deciders-rs/latest/deciders-rs/deciders/struct.Map2Decider.html
[`AppliedDecider`]: https://docs.rs/deciders-rs/latest/deciders-rs/deciders/struct.AppliedDecider.html
[`AdaptedProcess`]: https://docs.rs/deciders-rs/latest/deciders-rs/processes/struct.AdaptedProcess.html
[`collect_fold`]: https://docs.rs/deciders-rs/latest/deciders-rs/processes/fn.collect_fold.html
[`CombinedProcessDecider`]: https://docs.rs/deciders-rs/latest/deciders-rs/processes/struct.CombinedProcessDecider.html
[`InMemoryRunner`]: https://docs.rs/deciders-rs/latest/deciders-rs/utilities/struct.InMemoryRunner.html
[examples]: /examples
[integration tests]: /tests/integrations.rs
