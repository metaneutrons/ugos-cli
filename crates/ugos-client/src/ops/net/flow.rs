//! Traffic control rules.
//!
//! UGOS keeps the rules in priority order and takes the whole list back on
//! every change, grouped by interface. The functions here read the list,
//! change it as a flat list of rules, and send it.

use super::validate;
use crate::api::net::{NetApi, NetFlowApi};
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::net::{Flow, FlowGroup, FlowRule};

/// Sections whose rates must be at least 1024 KB/s, as the web UI insists.
const GUARDED_SECTIONS: [&str; 2] = ["com.ugreen.pro.ugos.web", "com.ugreen.pro.system.ssh"];

/// A rule to add.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewRule {
    /// Interface name, or `all`.
    pub interface: String,
    /// `custom` (ports), `all` (every port) or `sys` (built-in services).
    pub kind: String,
    /// Ports for `custom`: a comma list of up to 15, or a range `a-b`.
    pub ports: String,
    /// `src` or `dest` for `custom`.
    pub target: String,
    /// `tcp`, `udp` or `all` for `custom`.
    pub protocol: String,
    /// Guaranteed rate in KB/s.
    pub min_rate: i64,
    /// Limit in KB/s.
    pub max_rate: i64,
    /// Section ids of the built-in services, for `sys`.
    pub sections: Vec<String>,
}

fn refused(message: impl Into<String>) -> UgosError {
    UgosError::Refused(message.into())
}

/// The rules as one list, in priority order.
///
/// UGOS keeps `interface` and `label` on the group, so each rule takes them
/// from its group. Fields that do not belong to its kind (the empty lists
/// UGOS reads back for a port rule, ports for a service rule) are dropped, as
/// the web UI drops them before it sends the list again.
#[must_use]
pub fn flatten(flow: &Flow) -> Vec<FlowRule> {
    flow.list
        .iter()
        .flat_map(|g| {
            g.list.iter().map(|rule| {
                let mut rule = rule.clone();
                rule.interface.clone_from(&g.interface);
                rule.label.clone_from(&g.label);
                if rule.kind == "sys" {
                    rule.ports = None;
                    rule.target = None;
                    rule.protocol = None;
                } else {
                    rule.internal_servers = None;
                    rule.sections = None;
                }
                rule
            })
        })
        .collect()
}

/// Group a flat list by interface, in the order each first appears, as UGOS
/// takes it. A group is labelled after its first rule.
#[must_use]
pub fn group(rules: Vec<FlowRule>) -> Flow {
    let mut list: Vec<FlowGroup> = Vec::new();
    for rule in rules {
        match list.iter_mut().find(|g| g.interface == rule.interface) {
            Some(existing) => existing.list.push(rule),
            None => list.push(FlowGroup {
                interface: rule.interface.clone(),
                label: rule.label.clone(),
                list: vec![rule],
            }),
        }
    }
    Flow { list, ver: 0 }
}

/// Check ports: a comma list of up to 15 ports, or a range.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for ports the web UI would refuse.
pub fn check_ports(text: &str) -> Result<()> {
    if let Some((from, to)) = text.split_once('-') {
        let (from, to) = (parse_port(from)?, parse_port(to)?);
        return if from < to {
            Ok(())
        } else {
            Err(refused(
                "in a port range the first port must be below the second",
            ))
        };
    }
    let ports: Vec<&str> = text.split(',').collect();
    if ports.len() > 15 {
        return Err(refused("give at most 15 ports"));
    }
    ports.iter().try_for_each(|p| parse_port(p).map(|_| ()))
}

fn parse_port(text: &str) -> Result<i64> {
    let value: i64 = text
        .trim()
        .parse()
        .map_err(|_| refused(format!("'{text}' is not a port")))?;
    validate::port(value)
}

fn check_rates(rule: &NewRule) -> Result<()> {
    for rate in [rule.min_rate, rule.max_rate] {
        if !(0..=999_999).contains(&rate) {
            return Err(refused("a rate is 0 to 999999 KB/s"));
        }
    }
    if rule.min_rate > rule.max_rate {
        return Err(refused("the guaranteed rate must not exceed the limit"));
    }
    let guarded = rule
        .sections
        .iter()
        .any(|s| GUARDED_SECTIONS.contains(&s.as_str()));
    if guarded && (rule.min_rate < 1024 || rule.max_rate < 1024) {
        return Err(refused(
            "rates for the web interface and SSH must be at least 1024 KB/s",
        ));
    }
    Ok(())
}

/// Build the rule that the web UI would send for a request.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for input the web UI would refuse.
pub fn build(
    rule: &NewRule,
    label: &str,
    services: &[crate::types::net::FlowService],
) -> Result<FlowRule> {
    check_rates(rule)?;
    let base = FlowRule {
        kind: rule.kind.clone(),
        enable: 1,
        interface: rule.interface.clone(),
        label: label.into(),
        min_rate: rule.min_rate,
        max_rate: rule.max_rate,
        ..FlowRule::default()
    };
    match rule.kind.as_str() {
        "all" => Ok(FlowRule {
            ports: Some("all".into()),
            target: Some("src".into()),
            ..base
        }),
        "custom" => {
            check_ports(&rule.ports)?;
            if !["src", "dest"].contains(&rule.target.as_str()) {
                return Err(refused("the target is src or dest"));
            }
            if !["tcp", "udp", "all"].contains(&rule.protocol.as_str()) {
                return Err(refused("the protocol is tcp, udp or all"));
            }
            Ok(FlowRule {
                ports: Some(rule.ports.clone()),
                target: Some(rule.target.clone()),
                protocol: Some(rule.protocol.clone()),
                ..base
            })
        }
        "sys" => sys_rule(rule, base, services),
        other => Err(refused(format!("the kind '{other}' is custom, all or sys"))),
    }
}

fn sys_rule(
    rule: &NewRule,
    base: FlowRule,
    services: &[crate::types::net::FlowService],
) -> Result<FlowRule> {
    if rule.sections.is_empty() {
        return Err(refused("name at least one built-in service"));
    }
    let mut names = Vec::new();
    for section in &rule.sections {
        let found = services
            .iter()
            .find(|s| s.section == *section || s.name.eq_ignore_ascii_case(section))
            .ok_or_else(|| refused(format!("no built-in service '{section}'")))?;
        names.push(found.name.clone());
    }
    Ok(FlowRule {
        internal_servers: Some(names),
        sections: Some(rule.sections.clone()),
        ..base
    })
}

async fn send(client: &UgosClient, rules: Vec<FlowRule>) -> Result<Flow> {
    client.net_flow_set(&group(rules)).await?;
    client.net_flow().await
}

/// Add a rule at the end of the list.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for input the web UI would refuse or a
/// rule UGOS did not keep.
pub async fn add_rule(client: &UgosClient, rule: &NewRule) -> Result<Flow> {
    let label = if rule.interface == "all" {
        "ALL".to_owned()
    } else {
        client.net_iface(&rule.interface).await?.label
    };
    let services = if rule.kind == "sys" {
        client.net_flow_services(&[]).await?
    } else {
        Vec::new()
    };
    let built = build(rule, &label, &services)?;
    let mut rules = flatten(&client.net_flow().await?);
    let before = rules.len();
    rules.push(built);
    let now = send(client, rules).await?;
    if flatten(&now).len() == before + 1 {
        Ok(now)
    } else {
        Err(refused(
            "UGOS accepted the request but the rule is not in the list",
        ))
    }
}

/// Remove the rule at this position (counting from 1) in the flat list.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a position with no rule.
pub async fn remove_rule(client: &UgosClient, position: usize) -> Result<Flow> {
    let mut rules = flatten(&client.net_flow().await?);
    if position == 0 || position > rules.len() {
        return Err(refused(format!(
            "there is no rule {position}; there are {}",
            rules.len()
        )));
    }
    let _ = rules.remove(position - 1);
    let before = rules.len();
    let now = send(client, rules).await?;
    if flatten(&now).len() == before {
        Ok(now)
    } else {
        Err(refused(
            "UGOS accepted the request but the rule is still in the list",
        ))
    }
}

/// Switch the rule at this position on or off.
///
/// # Errors
///
/// Returns [`UgosError::Refused`] for a position with no rule, or a change
/// UGOS did not keep.
pub async fn enable_rule(client: &UgosClient, position: usize, on: bool) -> Result<Flow> {
    let mut rules = flatten(&client.net_flow().await?);
    let total = rules.len();
    let rule = position
        .checked_sub(1)
        .and_then(|i| rules.get_mut(i))
        .ok_or_else(|| refused(format!("there is no rule {position}; there are {total}")))?;
    rule.enable = i64::from(on);
    let now = send(client, rules).await?;
    let kept = flatten(&now)
        .get(position - 1)
        .is_some_and(|r| r.enable == i64::from(on));
    if kept {
        Ok(now)
    } else {
        Err(refused(
            "UGOS accepted the request but the rule did not change",
        ))
    }
}
