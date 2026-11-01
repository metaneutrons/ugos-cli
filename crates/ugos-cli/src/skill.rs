//! The agent skill `ugos-cli` ships, so an AI tool knows when and how to use it.
//!
//! The guidance in `skill/body.md` is written by hand. The command reference is
//! generated from this build's own command tree, so a command the binary has is
//! in the skill and one it lacks is not.
//!
//! Nothing here ever asks a question that could block a script. The only place
//! that reads an answer is `ugos-cli skill --install`, and only on a terminal;
//! everywhere else the user is pointed at that command once and left alone.

use std::fmt::Write as _;
use std::io::{IsTerminal as _, Write};
use std::path::{Path, PathBuf};

use agent_skill_installer::{
    Action, AgentSkill, Assessment, Catalog, Done, Found, Outcome, Problem, Program, Skill, Step,
    Why,
};
use anyhow::{Context, Result, bail};

/// The skill's name, which is also its directory in every skills folder.
pub const NAME: &str = "ugos-cli";

/// What the skill is for, which an AI tool reads to decide whether to load it.
const DESCRIPTION: &str = "Manage a UGREEN NAS running UGOS through ugos-cli: virtual \
machines, Docker containers, files, snapshots, downloads and hardware readings. Use it \
whenever a task involves a UGOS device, instead of virsh, qemu, docker over SSH or the \
web UI.";

/// The hand-written guidance.
const BODY: &str = include_str!("../skill/body.md");

/// The MCP tool reference. A test in `ugos-mcp` fails when it drifts from the
/// server, so it is checked in rather than generated here.
const MCP_REFERENCE: &str = include_str!("../skill/references/mcp.md");

/// The skill as this build would install it.
fn skill() -> Result<Skill> {
    Ok(Skill::new(NAME, DESCRIPTION, BODY)?
        .with_field("license", "GPL-3.0-or-later")?
        .with_field(
            "compatibility",
            "Needs the ugos-cli binary on the PATH and a UGREEN NAS running UGOS.",
        )?
        .with_file("references/cli.md", cli_reference())?
        .with_file("references/mcp.md", MCP_REFERENCE)?)
}

/// Where the user's answers are kept.
fn state_dir() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .context("cannot determine config directory")?
        .join("ugos-cli"))
}

/// The skill on this machine, for the user running it.
fn agent() -> Result<AgentSkill> {
    let dir = state_dir()?;
    std::fs::create_dir_all(&dir).context("creating config directory")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
            .context("restricting config directory")?;
    }
    let program = Program::new(NAME, env!("CARGO_PKG_VERSION"))?;
    Ok(AgentSkill::new(
        skill()?,
        program,
        dir.join("skill-consent"),
    )?)
}

/// Bring copies this program installed earlier up to this version.
///
/// Asks nobody, touches only copies that were installed with consent and have
/// not been edited, and never fails the command the user ran: a problem goes to
/// the debug log.
pub fn update_quietly() {
    match agent().and_then(|agent| Ok(agent.update()?)) {
        Ok(outcomes) => {
            for done in outcomes {
                tracing::debug!(place = %done.place.display(), done = %done.done, "agent skill");
            }
        }
        Err(e) => tracing::debug!(error = %e, "agent skill not updated"),
    }
}

/// One line on stderr, once, when an AI tool is here that has no skill yet.
///
/// A pointer, not a question: it reads nothing, so it cannot block. It stays
/// silent in scripts and CI, in JSON mode, and whenever stdin, stdout or
/// stderr is not a terminal.
pub fn hint_once(json: bool) {
    if json
        || std::env::var_os("CI").is_some()
        || std::env::var_os("UGOS_NO_SKILL_HINT").is_some()
        || !std::io::stdin().is_terminal()
        || !std::io::stdout().is_terminal()
        || !std::io::stderr().is_terminal()
    {
        return;
    }
    let Ok(dir) = state_dir() else { return };
    let marker = dir.join("skill-hint");
    if marker.exists() {
        return;
    }
    let offered = agent().and_then(|agent| Ok(agent.offer()?.is_some()));
    if !matches!(offered, Ok(true)) {
        return;
    }
    let mut err = std::io::stderr();
    let _ = writeln!(
        err,
        "\nAI tools on this computer can learn when and how to use {NAME}. \
         `{NAME} skill --install` sets that up, `{NAME} skill --decline` stops this hint."
    );
    let _ = std::fs::write(marker, "");
}

/// What `ugos-cli skill` was asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    /// Show where the skill is and what would happen.
    Status,
    /// Install where it is missing; `ask` unless `--yes` was given.
    Install {
        /// Whether to ask before installing.
        ask: bool,
    },
    /// Stop offering it where it is missing.
    Decline,
    /// Remove the copies this program installed and nobody changed.
    Remove,
}

/// `ugos-cli skill`: the state of the skill, or install, decline or remove it.
///
/// # Errors
/// Fails when the machine cannot be assessed, or an install is asked for
/// without a terminal to ask on and without `--yes`.
pub fn control(request: Request, w: &mut impl Write) -> Result<()> {
    let agent = agent()?;
    match request {
        Request::Status => status(&agent, w),
        Request::Install { ask } => install(&agent, ask, w),
        Request::Decline => {
            let places = match agent.offer()? {
                Some(offer) => {
                    let places = offer.steps().len();
                    offer.decline()?;
                    places
                }
                None => 0,
            };
            writeln!(
                w,
                "Not offered again for {places} place(s). `{NAME} skill --install` still installs there."
            )?;
            Ok(())
        }
        Request::Remove => {
            let outcomes = agent.remove()?;
            if outcomes.is_empty() {
                writeln!(w, "No copy of the skill to remove.")?;
            }
            for done in &outcomes {
                writeln!(w, "{}", outcome(done))?;
            }
            Ok(())
        }
    }
}

fn status(agent: &AgentSkill, w: &mut impl Write) -> Result<()> {
    let assessment = agent.status()?;
    writeln!(w, "Agent skill {NAME} {}", env!("CARGO_PKG_VERSION"))?;
    if assessment.steps.is_empty() {
        writeln!(w, "No AI tool with a skills folder was found.")?;
    }
    for step in &assessment.steps {
        writeln!(w, "{}", describe(step, agent.catalog()))?;
    }
    for unserved in &assessment.plan.unserved {
        writeln!(
            w,
            "{} is installed but cannot be reached: {}",
            unserved.tool, unserved.reason
        )?;
    }
    if assessment.awaiting_approval().next().is_some() {
        writeln!(
            w,
            "\n`{NAME} skill --install` installs it, `--decline` stops offering it."
        )?;
    }
    Ok(())
}

fn install(agent: &AgentSkill, ask: bool, w: &mut impl Write) -> Result<()> {
    let assessment = agent.status()?;
    let steps = installable(&assessment);
    if !steps.is_empty() {
        writeln!(w, "The {NAME} skill would be installed into:")?;
        for step in steps {
            writeln!(w, "  {}", describe(step, agent.catalog()))?;
        }
        if ask {
            if !std::io::stdin().is_terminal() {
                bail!("not installed: add --yes to install without being asked");
            }
            write!(w, "Install it there? [y/N] ")?;
            w.flush()?;
            let mut answer = String::new();
            let _ = std::io::stdin().read_line(&mut answer)?;
            if !matches!(answer.trim(), "y" | "Y" | "yes") {
                writeln!(w, "Not installed.")?;
                return Ok(());
            }
        }
    }
    // Also brings copies installed earlier up to date.
    let outcomes = agent.install_everywhere()?;
    if outcomes.is_empty() {
        writeln!(w, "Nothing to install: it is everywhere it can go.")?;
    }
    for done in &outcomes {
        writeln!(w, "{}", outcome(done))?;
    }
    Ok(())
}

/// The places an explicit install goes to: the undecided ones and those the
/// user declined earlier, since asking for it outright overrides a no.
fn installable(assessment: &Assessment) -> Vec<&Step> {
    assessment
        .steps
        .iter()
        .filter(|step| {
            matches!(
                step.action,
                Action::Install | Action::Nothing(Why::Declined)
            )
        })
        .collect()
}

/// A place for people to read, with the home directory as `~`.
fn shown(place: &Path) -> String {
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        return place.display().to_string();
    };
    let home = PathBuf::from(home);
    // Places come with every symlink resolved, so the home has to be too.
    let resolved = std::fs::canonicalize(&home).unwrap_or_else(|_| home.clone());
    [home, resolved]
        .iter()
        .find_map(|home| place.strip_prefix(home).ok())
        .map_or_else(
            || place.display().to_string(),
            |rest| format!("~/{}", rest.display()),
        )
}

/// The names people know a step's tools by.
fn tool_names<'a>(step: &'a Step, catalog: &'a Catalog) -> Vec<&'a str> {
    step.tools
        .iter()
        .map(|id| {
            catalog
                .tools()
                .iter()
                .find(|tool| &tool.id == id)
                .map_or(id.as_str(), |tool| tool.name.as_str())
        })
        .collect()
}

/// One place, as a line or two: where, for which tools, and what is there or
/// would happen.
fn describe(step: &Step, catalog: &Catalog) -> String {
    let mut line = shown(&step.place);
    if !step.tools.is_empty() {
        let _ = write!(line, " (read by {})", tool_names(step, catalog).join(", "));
    }
    let state = match &step.action {
        Action::Install => "not installed, waiting for your consent".to_string(),
        Action::Update { from } => format!("version {from}, will be updated"),
        Action::Nothing(Why::Declined) => "declined".to_string(),
        Action::Nothing(Why::Current) => "installed and up to date".to_string(),
        Action::Nothing(Why::Newer(version)) => {
            format!("version {version}, newer than this {NAME}; left alone")
        }
        Action::Nothing(Why::NotPlanned) => match &step.found {
            Found::Absent => "not installed".to_string(),
            _ => "installed".to_string(),
        },
        Action::Report(Problem::Modified) => "changed by hand; left alone".to_string(),
        Action::Report(Problem::Foreign(why)) => {
            format!("something else named {NAME} is there ({why}); left alone")
        }
        // A case a newer agent-skill-installer adds; its own words, for now.
        other => format!("{other:?}"),
    };
    let _ = write!(line, "\n    {state}");
    if let Some(root) = &step.git_root {
        let _ = write!(line, "\n    inside the git working tree {}", shown(root));
    }
    line
}

/// What happened in one place, for people.
fn outcome(outcome: &Outcome) -> String {
    let what = match &outcome.done {
        Done::Installed => "installed".to_string(),
        Done::Updated { from } => format!("updated from {from}"),
        Done::Overwritten => "replaced".to_string(),
        Done::Removed => "removed".to_string(),
        Done::Changed => format!("left alone: it changed while {NAME} was looking"),
        Done::Failed(why) => format!("failed: {why}"),
        other => other.to_string(),
    };
    format!("{}: {what}", shown(&outcome.place))
}

/// Every command and option, from the command tree clap parses.
fn cli_reference() -> String {
    use clap::CommandFactory as _;
    let mut root = crate::cli::Cli::command();
    // Fills in what the derive leaves implicit, such as which options take a
    // value.
    root.build();
    let mut out = format!("# {NAME} command line\n\nGenerated from the build.\n\n");
    let globals: Vec<_> = root
        .get_arguments()
        .filter(|arg| arg.is_global_set())
        .collect();
    if !globals.is_empty() {
        out.push_str("## Options of every command\n\n");
        for arg in globals {
            option_line(&mut out, arg);
        }
        out.push('\n');
    }
    for sub in root.get_subcommands() {
        command_section(&mut out, sub, NAME);
    }
    out
}

fn command_section(out: &mut String, cmd: &clap::Command, parent: &str) {
    if cmd.is_hide_set() || cmd.get_name() == "help" {
        return;
    }
    let path = format!("{parent} {}", cmd.get_name());
    let _ = writeln!(out, "## {path}\n");
    if let Some(about) = cmd.get_long_about().or_else(|| cmd.get_about()) {
        let _ = writeln!(out, "{about}\n");
    }
    let positionals: Vec<_> = cmd
        .get_positionals()
        .filter(|arg| !arg.is_hide_set())
        .collect();
    let usage: String = positionals
        .iter()
        .map(|arg| {
            let name = arg.get_id().as_str().to_uppercase();
            if arg.is_required_set() {
                format!(" <{name}>")
            } else {
                format!(" [{name}]")
            }
        })
        .collect();
    let _ = writeln!(out, "`{path}{usage}`\n");
    for arg in &positionals {
        if let Some(help) = arg.get_help() {
            let _ = writeln!(out, "- `{}`: {help}", arg.get_id().as_str().to_uppercase());
        }
    }
    for arg in cmd.get_arguments() {
        if arg.is_positional() || arg.is_global_set() || arg.is_hide_set() {
            continue;
        }
        if matches!(arg.get_id().as_str(), "help" | "version") {
            continue;
        }
        option_line(out, arg);
    }
    out.push('\n');
    for sub in cmd.get_subcommands() {
        command_section(out, sub, &path);
    }
}

fn option_line(out: &mut String, arg: &clap::Arg) {
    let mut names = Vec::new();
    if let Some(short) = arg.get_short() {
        names.push(format!("-{short}"));
    }
    if let Some(long) = arg.get_long() {
        names.push(format!("--{long}"));
    }
    if names.is_empty() {
        return;
    }
    let takes_value = arg.get_num_args().is_some_and(|n| n.takes_values())
        && !matches!(
            arg.get_action(),
            clap::ArgAction::SetTrue | clap::ArgAction::SetFalse | clap::ArgAction::Count
        );
    let value = if takes_value {
        let name = arg
            .get_value_names()
            .and_then(|names| names.first())
            .map_or_else(|| arg.get_id().as_str().to_uppercase(), ToString::to_string);
        format!(" <{name}>")
    } else {
        String::new()
    };
    let _ = write!(out, "- `{}{value}`", names.join(", "));
    if let Some(help) = arg.get_help() {
        let _ = write!(out, ": {}", sentence(&help.to_string()));
    }
    let possible: Vec<String> = arg
        .get_possible_values()
        .iter()
        .filter(|value| !value.is_hide_set())
        .map(|value| format!("`{}`", value.get_name()))
        .collect();
    if takes_value && !possible.is_empty() {
        let _ = write!(out, " One of {}.", possible.join(", "));
    }
    let defaults: Vec<String> = arg
        .get_default_values()
        .iter()
        .map(|value| value.to_string_lossy().into_owned())
        .collect();
    if takes_value && !defaults.is_empty() {
        let _ = write!(out, " Default `{}`.", defaults.join(","));
    }
    out.push('\n');
}

/// A help text as a sentence, so what follows it reads as the next one.
fn sentence(text: &str) -> String {
    let text = text.trim_end();
    if text.ends_with(['.', '!', '?', ')']) {
        text.to_string()
    } else {
        format!("{text}.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_skill_meets_the_specification() {
        assert!(
            skill().is_ok(),
            "the skill's own content breaks the specification"
        );
    }

    #[test]
    fn the_reference_names_every_command_the_binary_has() {
        use clap::CommandFactory as _;
        let reference = cli_reference();
        for sub in crate::cli::Cli::command().get_subcommands() {
            if sub.get_name() == "help" {
                continue;
            }
            assert!(
                reference.contains(&format!("## {NAME} {}", sub.get_name())),
                "the reference lacks `{}`",
                sub.get_name()
            );
        }
    }

    #[test]
    fn the_reference_covers_the_global_options() {
        let reference = cli_reference();
        for option in ["--host", "--user", "--tls-trust-new", "--output"] {
            assert!(reference.contains(option), "the reference lacks {option}");
        }
    }

    #[test]
    fn the_body_tells_an_agent_to_ask_before_changing_anything() {
        for word in ["vm delete", "fs rm", "snapshot rm", "docker rm"] {
            assert!(BODY.contains(word), "the body does not name `{word}`");
        }
    }
}
