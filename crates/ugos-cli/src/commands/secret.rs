//! Reading a secret that must not be a command-line argument.

use std::io::IsTerminal;

use anyhow::{Result, bail};

/// A secret: from stdin when asked to, else from the environment variable,
/// else typed on the terminal without echo.
///
/// `stdin_flag` names the flag that reads stdin, for the messages.
pub fn read(prompt: &str, env_name: &str, stdin_flag: &str, from_stdin: bool) -> Result<String> {
    if from_stdin {
        return crate::password_from_reader(&mut std::io::stdin().lock(), stdin_flag);
    }
    if let Some(value) = std::env::var(env_name).ok().filter(|v| !v.is_empty()) {
        return Ok(value);
    }
    if !std::io::stdin().is_terminal() {
        bail!("no {prompt}: set {env_name} or pass {stdin_flag}");
    }
    let secret = rpassword::prompt_password(format!("{prompt}: "))?;
    if secret.is_empty() {
        bail!("the {prompt} is empty");
    }
    Ok(secret)
}
