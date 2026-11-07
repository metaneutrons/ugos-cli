//! User accounts: finding, creating, changing and deleting them.
//!
//! Listing and reading an account are plain v1 calls. Creating and editing
//! take the password sealed with the login key, as the login does, and
//! deleting wants the administrator's own password in a header. See
//! `docs/api-users.md`.

use serde_json::{Value, json};

use crate::api::syslog::SysLogApi;
use crate::client::UgosClient;
use crate::error::{Result, UgosError};
use crate::types::syslog::User;
use crate::types::users::{DeleteCheck, Group, GroupName, NewUser, UserEdit};

/// `Api` code for a login name that is taken.
const NAME_TAKEN: i32 = 1007;

/// User account operations.
#[allow(clippy::module_name_repetitions)]
pub trait UsersApi {
    /// Find an account by login name, or by numeric uid.
    ///
    /// A name wins over a uid, since an account may be called `1001`.
    ///
    /// # Errors
    ///
    /// Returns [`UgosError::NotFound`] when no account matches.
    fn user_find(&self, name_or_uid: &str) -> impl Future<Output = Result<User>> + Send;

    /// Whether UGOS would accept this login name.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure other than a taken name.
    fn user_name_free(&self, name: &str) -> impl Future<Output = Result<bool>> + Send;

    /// Create an account and return its uid, when UGOS says which.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn user_create(&self, new: &NewUser) -> impl Future<Output = Result<Option<i64>>> + Send;

    /// Write an account back as `edit` describes it.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn user_edit(&self, edit: &UserEdit) -> impl Future<Output = Result<()>> + Send;

    /// What would be lost by deleting these accounts.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn user_check_delete(&self, uids: &[i64]) -> impl Future<Output = Result<DeleteCheck>> + Send;

    /// Delete accounts. UGOS wants the password of the account this client
    /// logged in with, and the client sends it.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn user_delete(&self, uids: &[i64]) -> impl Future<Output = Result<()>> + Send;

    /// The groups an administrator made. The built-in ones are not listed.
    ///
    /// # Errors
    ///
    /// Returns the appropriate [`UgosError`] on failure.
    fn group_list(&self) -> impl Future<Output = Result<Vec<Group>>> + Send;
}

/// Wrapper for the group listing, which nests under `list`.
#[derive(serde::Deserialize)]
struct GroupList {
    #[serde(default, deserialize_with = "crate::types::common::null_vec_as_empty")]
    list: Vec<Group>,
}

fn group_names(groups: &[String]) -> Vec<GroupName> {
    groups
        .iter()
        .map(|name| GroupName { name: name.clone() })
        .collect()
}

/// The body of `user/create`, with the password already sealed.
#[must_use]
pub fn create_body(new: &NewUser, sealed_password: &str) -> Value {
    json!({
        "username": new.username,
        "password": sealed_password,
        "email": new.email,
        "description": new.description,
        "role": new.role,
        "groups": group_names(&new.groups),
        "enable_home_dir": new.enable_home_dir,
        "deny_change_pwd": new.deny_change_pwd,
        "dir_quota": new.dir_quota,
        "unit": new.unit,
    })
}

/// The body of `user/edit`. The password is left out unless there is a new
/// one; the email must always be there, or UGOS blanks it.
#[must_use]
pub fn edit_body(edit: &UserEdit, sealed_password: Option<&str>) -> Value {
    let mut body = json!({
        "uid": edit.uid,
        "username": edit.username,
        "email": edit.email,
        "description": edit.description,
        "role": edit.role,
        "groups": group_names(&edit.groups),
        "enable_home_dir": edit.enable_home_dir,
        "deny_change_pwd": edit.deny_change_pwd,
        "dir_quota": edit.dir_quota,
        "unit": edit.unit,
        "expire_time": edit.expire_time,
        "expire_date": edit.expire_date,
    });
    if let Some(password) = sealed_password {
        body["password"] = json!(password);
    }
    body
}

/// Pick an account by name, then by numeric uid.
///
/// # Errors
///
/// Returns [`UgosError::NotFound`] when neither matches.
pub fn pick_user(users: &[User], name_or_uid: &str) -> Result<User> {
    if let Some(found) = users.iter().find(|user| user.username == name_or_uid) {
        return Ok(found.clone());
    }
    if let Ok(uid) = name_or_uid.parse::<i64>()
        && let Some(found) = users.iter().find(|user| user.uid == uid)
    {
        return Ok(found.clone());
    }
    Err(UgosError::NotFound {
        kind: "user",
        name: name_or_uid.to_owned(),
    })
}

impl UsersApi for UgosClient {
    async fn user_find(&self, name_or_uid: &str) -> Result<User> {
        pick_user(&self.users().await?, name_or_uid)
    }

    async fn user_name_free(&self, name: &str) -> Result<bool> {
        let checked: Result<Value> = self.post("user/name/check", &json!({ "name": name })).await;
        match checked {
            Ok(_) => Ok(true),
            Err(UgosError::Api { code, .. }) if code == NAME_TAKEN => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn user_create(&self, new: &NewUser) -> Result<Option<i64>> {
        let sealed = self.seal_password(&new.password).await?;
        let reply: Value = self.post("user/create", &create_body(new, &sealed)).await?;
        Ok(reply.get("result").and_then(Value::as_i64))
    }

    async fn user_edit(&self, edit: &UserEdit) -> Result<()> {
        let sealed = match &edit.password {
            Some(password) => Some(self.seal_password(password).await?),
            None => None,
        };
        let _: Value = self
            .post("user/edit", &edit_body(edit, sealed.as_deref()))
            .await?;
        Ok(())
    }

    async fn user_check_delete(&self, uids: &[i64]) -> Result<DeleteCheck> {
        self.post("user/check/delete", &uids).await
    }

    async fn user_delete(&self, uids: &[i64]) -> Result<()> {
        let _: Value = self
            .delete_with_body_confirmed("user/delete", &uids)
            .await?;
        Ok(())
    }

    async fn group_list(&self) -> Result<Vec<Group>> {
        let reply: GroupList = self.get("group/list").await?;
        Ok(reply.list)
    }
}
