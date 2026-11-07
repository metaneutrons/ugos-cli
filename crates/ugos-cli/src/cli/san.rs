//! Arguments of the `san` commands: iSCSI targets, LUNs, groups and the
//! global setting.

use clap::{Args, Subcommand, ValueEnum};

use super::OnOff;

/// How CHAP is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ChapMode {
    /// No CHAP.
    Off,
    /// The default CHAP of the global setting (targets only).
    Default,
    /// One-way: the initiator proves itself.
    OneWay,
    /// Mutual: the target proves itself as well.
    Mutual,
}

/// CHAP credentials. The passwords are not arguments: the first is read from
/// stdin with `--chap-password-stdin`, else `UGOS_CHAP_PASSWORD`, else asked;
/// the mutual one from `UGOS_CHAP_MUTUAL_PASSWORD` or asked.
#[derive(Debug, Args)]
pub struct ChapArgs {
    /// CHAP mode.
    #[arg(long, value_enum)]
    pub chap: Option<ChapMode>,
    /// The CHAP user, letters and digits, up to 12.
    #[arg(long)]
    pub chap_user: Option<String>,
    /// The target's CHAP user, for mutual CHAP.
    #[arg(long)]
    pub chap_mutual_user: Option<String>,
    /// Read the CHAP password from stdin.
    #[arg(long)]
    pub chap_password_stdin: bool,
}

/// `san setting set`.
#[derive(Debug, Args)]
pub struct SettingSetArgs {
    /// The iSCSI port, 1001 to 65535. A new port restarts the service.
    #[arg(long)]
    pub iscsi_port: Option<i64>,
    /// The start of every IQN.
    #[arg(long)]
    pub iqn_prefix: Option<String>,
    #[command(flatten)]
    pub chap: ChapArgs,
    /// Go on although this restarts the service or changes credentials.
    #[arg(long)]
    pub force: bool,
}

/// `san setting` subcommands.
#[derive(Debug, Subcommand)]
pub enum SettingAction {
    /// Show port, IQN prefix and default CHAP.
    Show,
    /// Change them.
    Set(SettingSetArgs),
}

/// `san target create`.
#[derive(Debug, Args)]
pub struct TargetCreateArgs {
    /// Name, letters, digits, `_` and `-`, up to 32.
    pub name: String,
    /// The LUN to map to it, by name or id. It must not be mapped yet.
    #[arg(long)]
    pub lun: String,
    /// The IQN. The next free one when not given.
    #[arg(long)]
    pub iqn: Option<String>,
    #[command(flatten)]
    pub chap: ChapArgs,
}

/// `san target edit`.
#[derive(Debug, Args)]
pub struct TargetEditArgs {
    /// The target, by name or id.
    pub name: String,
    /// A new name.
    #[arg(long)]
    pub new_name: Option<String>,
    /// A new IQN.
    #[arg(long)]
    pub iqn: Option<String>,
    /// Several connections per initiator.
    #[arg(long, value_enum)]
    pub multi_conn: Option<OnOff>,
    /// Header digest.
    #[arg(long, value_enum)]
    pub head_crc: Option<OnOff>,
    /// Data digest.
    #[arg(long, value_enum)]
    pub data_crc: Option<OnOff>,
    #[command(flatten)]
    pub chap: ChapArgs,
    /// Apply CHAP and digest changes to connected initiators now, which drops
    /// their sessions.
    #[arg(long)]
    pub apply_now: bool,
    /// Go on although initiators are connected.
    #[arg(long)]
    pub force: bool,
}

/// `san target` subcommands.
#[derive(Debug, Subcommand)]
pub enum TargetAction {
    /// List the targets.
    List,
    /// Show one target.
    Show {
        /// Name or id.
        name: String,
    },
    /// The next free name and IQN.
    Predict,
    /// Initiators that are connected.
    Connections {
        /// A target, or all of them.
        name: Option<String>,
    },
    /// Make a target and map a LUN to it.
    Create(TargetCreateArgs),
    /// Change a target.
    Edit(TargetEditArgs),
    /// Switch a target on.
    Enable {
        /// Name or id.
        name: String,
    },
    /// Switch a target off. Connected initiators lose it.
    Disable {
        /// Name or id.
        name: String,
        /// Go on although initiators are connected.
        #[arg(long)]
        force: bool,
    },
    /// Delete a target. The LUN stays, unless `--with-lun`.
    Delete {
        /// Name or id.
        name: String,
        /// Delete the mapped LUN and its data as well.
        #[arg(long)]
        with_lun: bool,
        /// Go on although initiators are connected or the LUN goes with it.
        #[arg(long)]
        force: bool,
    },
}

/// Who may use a LUN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AccessMode {
    /// Every group.
    All,
    /// The groups named with `--group`.
    Groups,
}

/// `san lun create`.
#[derive(Debug, Args)]
pub struct LunCreateArgs {
    /// Name, letters, digits, `_` and `-`, up to 32.
    pub name: String,
    /// Size in whole GB or TB, like `100G` or `2T`.
    #[arg(long)]
    pub size: String,
    /// The volume, by name or label.
    #[arg(long)]
    pub volume: String,
    /// A thin LUN, which takes space as it is written. Else thick.
    #[arg(long)]
    pub thin: bool,
    /// A description, up to 32 characters.
    #[arg(long, default_value = "")]
    pub desc: String,
    /// Go on although a thin LUN is larger than the free space.
    #[arg(long)]
    pub force: bool,
}

/// `san lun edit`.
#[derive(Debug, Args)]
pub struct LunEditArgs {
    /// The LUN, by name or id.
    pub name: String,
    /// A new name.
    #[arg(long)]
    pub new_name: Option<String>,
    /// A new description.
    #[arg(long)]
    pub desc: Option<String>,
    /// A larger size, like `200G`. It cannot be made smaller again.
    #[arg(long)]
    pub size: Option<String>,
    /// Force unit access.
    #[arg(long, value_enum)]
    pub fua: Option<OnOff>,
    /// Who may use the LUN.
    #[arg(long, value_enum)]
    pub access: Option<AccessMode>,
    /// A group with access, as `name` (read-write) or `name=ro`.
    #[arg(long = "group")]
    pub groups: Vec<String>,
    /// Go on although a size or access change cannot be undone.
    #[arg(long)]
    pub force: bool,
}

/// `san lun` subcommands.
#[derive(Debug, Subcommand)]
pub enum LunAction {
    /// List the LUNs.
    List,
    /// Show one LUN.
    Show {
        /// Name or id.
        name: String,
    },
    /// Make a LUN.
    Create(LunCreateArgs),
    /// Change or enlarge a LUN.
    Edit(LunEditArgs),
    /// Delete a LUN and its data.
    Delete {
        /// Name or id.
        name: String,
        /// Go on: the data is destroyed.
        #[arg(long)]
        force: bool,
    },
}

/// `san group` subcommands.
#[derive(Debug, Subcommand)]
pub enum SanGroupAction {
    /// List the initiator groups.
    List,
    /// Make a group.
    Create {
        /// Name, letters, digits, `_` and `-`, up to 32.
        name: String,
        /// Initiator IQNs, at least one.
        #[arg(long = "iqn", required = true)]
        iqns: Vec<String>,
        /// A description, up to 32 characters.
        #[arg(long, default_value = "")]
        desc: String,
    },
    /// Change a group.
    Edit {
        /// Name or id.
        name: String,
        /// A new name.
        #[arg(long)]
        new_name: Option<String>,
        /// A new description.
        #[arg(long)]
        desc: Option<String>,
        /// Initiator IQNs to add.
        #[arg(long = "add")]
        add: Vec<String>,
        /// Initiator IQNs to remove.
        #[arg(long = "remove")]
        remove: Vec<String>,
        /// Go on although LUNs are bound to the group.
        #[arg(long)]
        force: bool,
    },
    /// Delete a group.
    Delete {
        /// Name or id.
        name: String,
        /// Go on although LUNs are bound to the group.
        #[arg(long)]
        force: bool,
    },
}

/// `san` subcommands: iSCSI.
#[derive(Debug, Subcommand)]
pub enum SanAction {
    /// Port, IQN prefix and default CHAP.
    Setting {
        #[command(subcommand)]
        action: SettingAction,
    },
    /// iSCSI targets.
    Target {
        #[command(subcommand)]
        action: TargetAction,
    },
    /// LUNs.
    Lun {
        #[command(subcommand)]
        action: LunAction,
    },
    /// Initiator groups.
    Group {
        #[command(subcommand)]
        action: SanGroupAction,
    },
}
