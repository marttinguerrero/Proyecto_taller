use std::path::PathBuf;

use crate::repo_paths::RepoPaths;
pub enum UiEvent {
    GiIinit(RepoPaths),
    AddCommand(Vec<String>, RepoPaths),
    CommitCommand(Vec<String>, RepoPaths),
    ConfigCommand(RepoPaths, Vec<String>),
    CheckoutCommand(RepoPaths, Vec<String>),
    BranchCommand(RepoPaths, Vec<String>),
    MergeCommand(RepoPaths, Vec<String>),
    RemoteCommand(Vec<String>, PathBuf),
    CloneCommand(RepoPaths, Vec<String>),
    PushCommand(RepoPaths, Vec<String>),
    PullCommand(RepoPaths, Vec<String>),
}
