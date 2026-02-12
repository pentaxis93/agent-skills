//! TUI command - launch the interactive terminal interface

use anyhow::Result;

use crate::config::Config;
use crate::skill;

/// Launch the TUI
pub fn tui(config: Config) -> Result<()> {
    // Discover all skills from sources
    let skills = skill::discover_all(&config.sources.skills)?;

    // Launch the TUI
    crate::tui::run(config, skills)?;

    Ok(())
}
