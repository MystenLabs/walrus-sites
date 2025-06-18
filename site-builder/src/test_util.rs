use std::path::PathBuf;

use crate::args::{Commands, GeneralArgs};

pub struct ArgsForTesting {
    /// The path to the configuration file for the site builder.
    pub config: Option<PathBuf>,
    /// The context with which to load the configuration.
    ///
    /// If specified, the context will be taken from the config file. Otherwise, the default
    /// context, which is also specified in the config file, will be used.
    pub context: Option<String>,
    pub general: GeneralArgs,
    pub command: Commands,
}
