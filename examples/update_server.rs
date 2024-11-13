use std::io::Write;

use deciders_rs::utilities::InMemoryRunner;

const VERSIONS: [&str; 6] = ["1.0.0", "1.0.1", "1.1.0", "1.1.1", "1.2.0", "2.0.0"];
const UPDATE_DATA: [&str; 6] = [
    "Download data for v1.0.0",
    "Download data for v1.0.1",
    "Download data for v1.1.0",
    "Download data for v1.1.1",
    "Download data for v1.2.0",
    "Download data for v2.0.0",
];

fn split_version_string(version: &str) -> (u64, u64, u64) {
    let chunks: Vec<&str> = version.split('.').map(str::trim).collect();
    assert_eq!(chunks.len(), 3);
    let major: u64 = chunks[0].parse().expect("Invalid version string");
    let minor: u64 = chunks[1].parse().expect("Invalid version string");
    let patch: u64 = chunks[2].parse().expect("Invalid version string");
    (major, minor, patch)
}

fn compare_versions(current: &str, to_check: &str) -> bool {
    let (current_major, current_minor, current_patch) = split_version_string(current);
    let (to_check_major, to_check_minor, to_check_patch) = split_version_string(to_check);
    match to_check_major.cmp(&current_major) {
        std::cmp::Ordering::Less => return false,
        std::cmp::Ordering::Equal => (),
        std::cmp::Ordering::Greater => return true,
    }
    match to_check_minor.cmp(&current_minor) {
        std::cmp::Ordering::Less => return false,
        std::cmp::Ordering::Equal => (),
        std::cmp::Ordering::Greater => return true,
    }
    to_check_patch > current_patch
}

fn get_available_updates(current_version: &str) -> &[&'static str] {
    let mut newer_versions = &VERSIONS[..];
    while !newer_versions.is_empty() && !compare_versions(current_version, newer_versions[0]) {
        newer_versions = &newer_versions[1..];
    }
    newer_versions
}

mod update_decider {
    use deciders_rs::deciders::Decider;

    use crate::{get_available_updates, UPDATE_DATA, VERSIONS};

    pub enum Command {
        QueryForUpdate { current_version: String },
        DownloadUpdate { desired_version: String },
    }

    #[derive(Clone)]
    pub enum State {
        NewConnection,
        UpdateAvailable { new_versions: Vec<&'static str> },
        UnknownVersion { version: String },
        NoNewUpdate,
        DownloadReady { update_data: String },
        DownloadUnavailable,
    }

    pub enum Event {
        UpdateAvailable { new_versions: Vec<&'static str> },
        UnknownVersionQueried { version: String },
        AlreadyUpToDate,
        GotUpdateData { update_data: String },
        InvalidVersion,
    }

    pub struct UpdateServer;

    impl Decider<Command, Event, State, State> for UpdateServer {
        fn decide(command: &Command, state: &State) -> Vec<Event> {
            match (state, command) {
                (State::NewConnection, Command::QueryForUpdate { current_version }) => {
                    if !VERSIONS.contains(&&current_version[..]) {
                        return vec![Event::UnknownVersionQueried {
                            version: current_version.clone(),
                        }];
                    }
                    let newer_versions = get_available_updates(current_version);
                    if newer_versions.is_empty() {
                        vec![Event::AlreadyUpToDate]
                    } else {
                        vec![Event::UpdateAvailable {
                            new_versions: newer_versions.to_vec(),
                        }]
                    }
                }
                (
                    State::UpdateAvailable { new_versions },
                    Command::DownloadUpdate { desired_version },
                ) => {
                    if !new_versions.contains(&&desired_version[..]) {
                        return vec![Event::InvalidVersion];
                    }
                    let update = VERSIONS
                        .iter()
                        .enumerate()
                        .find(|v| *v.1 == desired_version)
                        .map(|v| UPDATE_DATA[v.0]);
                    match update {
                        Some(data) => vec![Event::GotUpdateData {
                            update_data: data.to_string(),
                        }],
                        None => vec![Event::InvalidVersion],
                    }
                }
                _ => vec![],
            }
        }

        fn evolve(state: &State, event: &Event) -> State {
            match (state, event) {
                (State::NewConnection, Event::UpdateAvailable { new_versions }) => {
                    State::UpdateAvailable {
                        new_versions: new_versions.clone(),
                    }
                }
                (State::NewConnection, Event::UnknownVersionQueried { version }) => {
                    State::UnknownVersion {
                        version: version.clone(),
                    }
                }
                (State::NewConnection, Event::AlreadyUpToDate) => State::NoNewUpdate,
                (State::UpdateAvailable { .. }, Event::GotUpdateData { update_data }) => {
                    State::DownloadReady {
                        update_data: update_data.clone(),
                    }
                }
                (State::UpdateAvailable { .. }, Event::InvalidVersion) => {
                    State::DownloadUnavailable
                }
                _ => state.clone(),
            }
        }

        fn initial_state() -> State {
            State::NewConnection
        }

        fn is_terminal(state: &State) -> bool {
            matches!(
                state,
                State::NoNewUpdate | State::DownloadReady { .. } | State::DownloadUnavailable
            )
        }
    }
}

fn main() {
    use std::io;
    use update_decider::{Command, Event, UpdateServer};
    let mut runner = InMemoryRunner::<_, _, _, UpdateServer>::new();
    let mut buffer = String::new();
    print!("What is your current software version? > ");
    io::stdout().flush().expect("Could not flush stdout!");
    io::stdin()
        .read_line(&mut buffer)
        .expect("Could not read current version");
    let events = runner.command(&Command::QueryForUpdate {
        current_version: buffer.trim().to_string(),
    });
    let event = &events[0];
    match event {
        Event::UpdateAvailable { new_versions } => {
            println!("Updates available! Newer versions:");
            for v in new_versions {
                println!("\t- {v}");
            }
        }
        Event::UnknownVersionQueried { version } => {
            println!("Nonexistent version \"{version}\"!");
            return;
        }
        Event::AlreadyUpToDate => {
            println!("Up to date!");
            return;
        }
        _ => unreachable!(),
    }
    print!("What version do you want to upgrade to? > ");
    buffer.clear();
    io::stdout().flush().expect("Could not flush stdout!");
    io::stdin()
        .read_line(&mut buffer)
        .expect("Could not read current version");
    let events = runner.command(&Command::DownloadUpdate {
        desired_version: buffer.trim().to_string(),
    });
    let event = &events[0];
    match event {
        Event::GotUpdateData { update_data } => println!("Update downloaded: {update_data}"),
        Event::InvalidVersion => println!("Invalid version!"),
        _ => unreachable!(),
    }
}
