//! The connections to the chat servers and the auth service. Nothing here draws anything: the app
//! asks for things, and what the servers send comes back as `Event`s on a channel.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use web_time::Instant;

use common::{is_valid_email, AuthReply, AuthRequest, ClientMsg, ServerMsg};

use crate::discover::FoundServer;
use crate::net::{self, Connection, NetEvent};
use crate::{auth_client, discord_login, keychain};

/// Opens a Discord login page in the browser, or says how to open it by hand.
fn open_discord(url: &str) -> Result<(), CmdError> {
    discord_login::open_browser(url).map_err(|e| {
        CmdError::new(format!(
            "Could not open your browser ({e}). Open this address to log in: {url}"
        ))
    })
}

/// How long to wait before trying a server again after its direct messages could not be reached.
const DM_RETRY: Duration = Duration::from_secs(30);
/// How long to wait before reconnecting to a server whose direct message link dropped.
const DM_DROP_RETRY: Duration = Duration::from_secs(5);
pub const MIN_PASSWORD_LEN: usize = 8;

/// An error the app shows. `auth_down` is true when the auth service could not be reached.
#[derive(Debug, Clone)]
pub struct CmdError {
    pub message: String,
    pub auth_down: bool,
}

impl CmdError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            auth_down: false,
        }
    }

    fn unreachable(message: String) -> Self {
        Self {
            message,
            auth_down: true,
        }
    }
}

/// The outside accounts linked to this account.
#[derive(Debug, Clone)]
pub struct LinkedAccounts {
    /// The Discord name, or None when no Discord account is linked.
    pub discord: Option<String>,
    pub can_unlink_discord: bool,
}

/// The account's name, username and email, as the auth service reports them.
#[derive(Debug, Clone)]
pub struct Profile {
    /// False for an account made with Discord that has no password yet.
    pub has_password: bool,
    pub name: String,
    pub username: String,
    pub email: Option<String>,
    /// True once the email address has been confirmed with a code.
    pub verified: bool,
    /// True when the auth service can send the confirmation code.
    pub can_verify: bool,
    pub bio: String,
    /// The banner colour (`#rrggbb`), or empty for the default.
    pub banner: String,
    /// The profile picture as base64 PNG, or empty for none.
    pub avatar: String,
}

/// Someone's public profile card, as the auth service reports it.
#[derive(Debug, Clone)]
pub struct Card {
    pub name: String,
    pub username: String,
    pub bio: String,
    pub banner: String,
    pub avatar: String,
    /// The status text in the bubble on the card. Empty when none is set.
    pub status: String,
    /// When the account was created, in Unix seconds (0 when unknown).
    pub created: u64,
    /// The servers (by address) that the viewer and this member both are in.
    pub mutual: Vec<String>,
}

/// The outcome of restoring a saved login at startup.
#[derive(Debug, Clone)]
pub struct Restored {
    /// The account's name, or None when nothing was restored.
    pub user: Option<String>,
    /// True when the auth service could not be reached and the saved login was used as it is.
    pub auth_down: bool,
    /// Why the saved login could not be used, if it was there but ended.
    pub error: Option<String>,
    /// A note for the user, such as "Using your saved login".
    pub notice: Option<String>,
}

/// Where the backend's events go: an async channel, read by the app's subscription. Usable from any
/// thread, and in the browser, which has none.
pub type EventSender = iced::futures::channel::mpsc::UnboundedSender<Event>;
pub type EventReceiver = iced::futures::channel::mpsc::UnboundedReceiver<Event>;

/// A new channel for the backend's events.
pub fn event_channel() -> (EventSender, EventReceiver) {
    iced::futures::channel::mpsc::unbounded()
}

/// Something a connection tells the app.
#[derive(Debug, Clone)]
pub enum Event {
    /// A message from the open server (`dm` is false) or from a direct message connection.
    Net {
        dm: bool,
        addr: String,
        id: u64,
        msg: ServerMsg,
    },
    /// A connection ended.
    Closed {
        dm: bool,
        addr: String,
        id: u64,
        reason: String,
    },
    /// A direct message connection came up or went down.
    DmLink { addr: String, id: u64, up: bool },
    /// A server answered a discovery request.
    Discovered(FoundServer),
    /// Something the window shows has arrived, such as an emoji's artwork in the browser: draw again.
    Redraw,
}

/// One open connection and the id the app uses to tell it apart from earlier ones.
struct Link {
    id: u64,
    conn: Connection,
}

#[derive(Default)]
struct Inner {
    token: Option<String>,
    user: String,
    /// "Keep me logged in" was ticked, so the token is in the keychain.
    remember: bool,
    next_id: u64,
    /// The connection to the open server.
    main: Option<Link>,
    /// Background connections for direct messages, one per server, by address.
    dm: HashMap<String, Link>,
    dm_connecting: HashSet<String>,
    dm_retry_at: HashMap<String, Instant>,
}

/// Shared handle to the connections. Cheap to clone; every clone sees the same connections.
#[derive(Clone)]
pub struct Backend {
    inner: Arc<Mutex<Inner>>,
    events: EventSender,
    config_dir: PathBuf,
}

impl Backend {
    pub fn new(events: EventSender, config_dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
            events,
            config_dir,
        }
    }

    /// A sender for events, for background work that reports back to the app.
    pub fn events_sender(&self) -> EventSender {
        self.events.clone()
    }

    pub fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn token(&self) -> Option<String> {
        self.lock().token.clone()
    }

    fn set_session(&self, token: String, user: String, remember: bool) {
        let mut inner = self.lock();
        inner.token = Some(token);
        inner.user = user;
        inner.remember = remember;
    }

    /// What a connection does with everything it receives: passes it to the app, and forgets the
    /// connection once it ends.
    fn forwarder(&self, dm: bool, addr: String, id: u64) -> impl FnMut(NetEvent) + Send + 'static {
        let backend = self.clone();
        move |event| match event {
            NetEvent::Server(msg) => {
                let _ = backend.events.unbounded_send(Event::Net {
                    dm,
                    addr: addr.clone(),
                    id,
                    msg,
                });
            }
            NetEvent::Disconnected(reason) => {
                backend.forget_link(dm, &addr, id);
                let _ = backend.events.unbounded_send(Event::Closed {
                    dm,
                    addr: addr.clone(),
                    id,
                    reason,
                });
            }
        }
    }

    fn next_id(&self) -> u64 {
        let mut inner = self.lock();
        inner.next_id += 1;
        inner.next_id
    }

    /// Removes a connection that has ended, so it is not used again.
    fn forget_link(&self, dm: bool, addr: &str, id: u64) {
        if !dm {
            let mut inner = self.lock();
            if inner.main.as_ref().is_some_and(|l| l.id == id) {
                inner.main = None;
            }
            return;
        }
        let dropped = {
            let mut inner = self.lock();
            if inner.dm.get(addr).is_some_and(|l| l.id == id) {
                inner.dm.remove(addr);
                inner
                    .dm_retry_at
                    .insert(addr.to_string(), Instant::now() + DM_DROP_RETRY);
                true
            } else {
                false
            }
        };
        if dropped {
            let _ = self.events.unbounded_send(Event::DmLink {
                addr: addr.to_string(),
                id,
                up: false,
            });
        }
    }

    /// Connects to the server that is open, logging in with the session token.
    pub fn connect_main(&self, addr: &str) -> Result<u64, CmdError> {
        let token = self.token().ok_or_else(|| CmdError::new("Log in first."))?;
        // Close the old connection first, so the server sees it end.
        let old = self.lock().main.take();
        drop(old);
        let id = self.next_id();
        let conn = net::connect(
            addr,
            ClientMsg::Connect { token },
            self.forwarder(false, addr.to_string(), id),
        )
        .map_err(CmdError::new)?;
        self.lock().main = Some(Link { id, conn });
        Ok(id)
    }

    pub fn disconnect_main(&self) {
        let old = self.lock().main.take();
        drop(old);
    }

    /// Sends the server a logout (so it forgets the session) and closes the connection.
    fn farewell_main(&self) {
        let link = self.lock().main.take();
        if let Some(link) = link {
            link.conn.send_last(ClientMsg::Logout);
        }
    }

    pub fn send_main(&self, msg: ClientMsg) -> Result<(), CmdError> {
        let inner = self.lock();
        let link = inner
            .main
            .as_ref()
            .ok_or_else(|| CmdError::new("Not connected to a server."))?;
        link.conn
            .send(msg)
            .map_err(|_| CmdError::new("Not connected to a server."))
    }

    /// Sets this account's status on every server it is connected to.
    /// Tells every server this client is connected to the status text, so everyone connected sees it.
    pub fn broadcast_status(&self, text: &str) {
        let inner = self.lock();
        let links = inner.main.iter().chain(inner.dm.values());
        for link in links {
            let _ = link.conn.send(ClientMsg::SetProfileStatus { text: text.to_string() });
        }
    }

    pub fn set_status_everywhere(&self, status: common::PresenceStatus) {
        let inner = self.lock();
        let links = inner.main.iter().chain(inner.dm.values());
        for link in links {
            let _ = link.conn.send(ClientMsg::SetStatus { status });
        }
    }

    pub fn send_dm(&self, addr: &str, msg: ClientMsg) -> Result<(), CmdError> {
        let inner = self.lock();
        let link = inner
            .dm
            .get(addr)
            .ok_or_else(|| CmdError::new("Not connected to that server."))?;
        link.conn
            .send(msg)
            .map_err(|_| CmdError::new("Not connected to that server."))
    }

    /// Starts a background connection to each of these servers that has none yet. Each one is made
    /// on its own thread, so a slow or offline server does not hold up the others.
    pub fn dm_connect_all(&self, addrs: Vec<String>) {
        let Some(token) = self.token() else {
            return;
        };
        let now = Instant::now();
        for addr in addrs {
            {
                let mut inner = self.lock();
                if inner.dm.contains_key(&addr) || inner.dm_connecting.contains(&addr) {
                    continue;
                }
                if inner.dm_retry_at.get(&addr).is_some_and(|at| *at > now) {
                    continue;
                }
                inner.dm_connecting.insert(addr.clone());
            }
            let backend = self.clone();
            let token = token.clone();
            #[cfg(not(target_arch = "wasm32"))]
            std::thread::spawn(move || backend.connect_dm(addr, token));
            // A browser connection opens without waiting, so it needs no thread of its own.
            #[cfg(target_arch = "wasm32")]
            backend.connect_dm(addr, token);
        }
    }

    fn connect_dm(&self, addr: String, token: String) {
        let id = self.next_id();
        let result = net::connect(
            &addr,
            ClientMsg::Connect { token },
            self.forwarder(true, addr.clone(), id),
        );
        let mut inner = self.lock();
        inner.dm_connecting.remove(&addr);
        match result {
            Ok(conn) => {
                // The account may have logged out while this was connecting.
                if inner.token.is_none() {
                    return;
                }
                inner.dm_retry_at.remove(&addr);
                inner.dm.insert(addr.clone(), Link { id, conn });
                drop(inner);
                let _ = self.events.unbounded_send(Event::DmLink { addr, id, up: true });
            }
            Err(_) => {
                inner.dm_retry_at.insert(addr, Instant::now() + DM_RETRY);
            }
        }
    }

    /// Ends the connection for direct messages to one server.
    pub fn dm_disconnect(&self, addr: &str) {
        let old = {
            let mut inner = self.lock();
            inner.dm_connecting.remove(addr);
            inner.dm_retry_at.remove(addr);
            inner.dm.remove(addr)
        };
        drop(old);
    }

    /// Ends every background connection for direct messages.
    pub fn dm_disconnect_all(&self) {
        let old = {
            let mut inner = self.lock();
            inner.dm_connecting.clear();
            inner.dm_retry_at.clear();
            std::mem::take(&mut inner.dm)
        };
        drop(old);
    }

    async fn auth(&self, request: AuthRequest) -> Result<AuthReply, CmdError> {
        auth_client::request(request).await.map_err(CmdError::unreachable)
    }

    /// Registers or logs in with the auth service. With `name` set, the account is registered.
    pub async fn login(
        &self,
        email: String,
        password: String,
        name: Option<String>,
        remember: bool,
    ) -> Result<String, CmdError> {
        let email = email.trim().to_string();
        let request = match name {
            Some(name) => {
                let name = name.trim().to_string();
                if name.is_empty() {
                    return Err(CmdError::new("Enter a display name."));
                }
                if !is_valid_email(&email) {
                    return Err(CmdError::new("Enter a valid email address."));
                }
                if password.chars().count() < MIN_PASSWORD_LEN {
                    return Err(CmdError::new(format!(
                        "Password must be at least {MIN_PASSWORD_LEN} characters."
                    )));
                }
                AuthRequest::Register { name, email, password }
            }
            None => {
                if email.is_empty() || password.is_empty() {
                    return Err(CmdError::new("Enter your email and password."));
                }
                AuthRequest::Login { email, password }
            }
        };
        match self.auth(request).await? {
            AuthReply::Session { token, name } => Ok(self.finish_login(token, name, remember)),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    /// Logs in with a Discord account. Opens Discord in the browser and waits until the login finishes.
    pub async fn discord_login(&self, remember: bool) -> Result<String, CmdError> {
        let (url, state) = self.start_discord(String::new()).await?;
        open_discord(&url)?;
        match self.wait_discord(&state).await? {
            AuthReply::Session { token, name } => Ok(self.finish_login(token, name, remember)),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    /// Links a Discord account to the account that is logged in. Returns the Discord name.
    pub async fn link_discord(&self) -> Result<String, CmdError> {
        let token = self.token().ok_or_else(|| CmdError::new("Log in first."))?;
        let (url, state) = self.start_discord(token).await?;
        open_discord(&url)?;
        match self.wait_discord(&state).await? {
            AuthReply::DiscordLinked { discord } => Ok(discord),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    /// Asks the auth service to start a Discord login (or a link, with a session token).
    async fn start_discord(&self, token: String) -> Result<(String, String), CmdError> {
        match self.auth(AuthRequest::DiscordStart { token }).await? {
            AuthReply::DiscordUrl { url, state } => Ok((url, state)),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    /// Polls until the Discord login finishes. The browser may take a while, so this waits up to ten
    /// minutes, which is also how long the auth service keeps the login.
    async fn wait_discord(&self, state: &str) -> Result<AuthReply, CmdError> {
        let deadline = Instant::now() + Duration::from_secs(10 * 60);
        loop {
            match self
                .auth(AuthRequest::DiscordPoll {
                    state: state.to_string(),
                })
                .await?
            {
                AuthReply::DiscordPending => {
                    if Instant::now() >= deadline {
                        return Err(CmdError::new("The Discord login took too long. Try again."));
                    }
                    crate::util::sleep(Duration::from_secs(1)).await;
                }
                other => return Ok(other),
            }
        }
    }

    /// Sets a password on an account that has none.
    pub async fn set_password(&self, password: String) -> Result<(), CmdError> {
        let token = self.token().ok_or_else(|| CmdError::new("Log in first."))?;
        match self.auth(AuthRequest::SetPassword { token, password }).await? {
            AuthReply::PasswordSet => Ok(()),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    /// Permanently deletes the account, after checking the password. The caller logs out afterwards.
    pub async fn delete_account(&self, password: String) -> Result<(), CmdError> {
        let token = self.token().ok_or_else(|| CmdError::new("Log in first."))?;
        match self.auth(AuthRequest::DeleteAccount { token, password }).await? {
            AuthReply::AccountDeleted => Ok(()),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    /// Keeps a new login: saves it to the keychain when asked to, and makes it the session.
    fn finish_login(&self, token: String, name: String, remember: bool) -> String {
        if remember {
            if let Err(e) = keychain::store(&name, &token) {
                eprintln!("could not save your login to the keychain: {e}");
            }
        }
        self.set_session(token, name.clone(), remember);
        name
    }

    /// Checks a login saved from an earlier run. If the auth service cannot be reached, the saved
    /// login is used anyway, so the chat server can still accept it.
    pub async fn restore(&self, saved_name: &str) -> Restored {
        let none = |error: Option<String>| Restored {
            user: None,
            auth_down: false,
            error,
            notice: None,
        };
        let Some(token) = keychain::load(saved_name) else {
            return none(None);
        };
        match self.auth(AuthRequest::Verify { token: token.clone() }).await {
            Ok(AuthReply::Verified { name, .. }) => {
                self.set_session(token, name.clone(), true);
                Restored {
                    user: Some(name),
                    auth_down: false,
                    error: None,
                    notice: None,
                }
            }
            Ok(AuthReply::Error { text }) => none(Some(format!("Your saved login has ended: {text}"))),
            Ok(_) => none(Some("Unexpected reply from the auth service.".into())),
            Err(e) => {
                self.set_session(token, saved_name.to_string(), true);
                Restored {
                    user: Some(saved_name.to_string()),
                    auth_down: true,
                    error: None,
                    notice: Some(format!(
                        "Auth service unreachable ({}). Using your saved login.",
                        e.message
                    )),
                }
            }
        }
    }

    /// Ends the session on the server and the auth service, and forgets the saved login.
    pub async fn logout(&self) {
        self.farewell_main();
        self.dm_disconnect_all();
        let (token, user) = {
            let mut inner = self.lock();
            inner.remember = false;
            (inner.token.take(), std::mem::take(&mut inner.user))
        };
        if let Some(token) = token {
            // Logging out still clears the local session if the auth service cannot be reached.
            let _ = self.auth(AuthRequest::Logout { token }).await;
        }
        keychain::forget(&user);
    }

    pub async fn profile(&self) -> Result<Profile, CmdError> {
        let token = self.token().ok_or_else(|| CmdError::new("Log in first."))?;
        let reply = self.auth(AuthRequest::Profile { token }).await?;
        self.apply_profile(reply)
    }

    /// Saves the bio and banner colour. Returns what was saved.
    pub async fn set_about(&self, bio: String, banner: String) -> Result<(String, String), CmdError> {
        let token = self.token().ok_or_else(|| CmdError::new("Log in first."))?;
        match self.auth(AuthRequest::SetAbout { token, bio, banner }).await? {
            AuthReply::AboutSaved { bio, banner } => Ok((bio, banner)),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    /// Sets the status text on the profile card. Returns the text now saved (empty when cleared).
    pub async fn set_status_text(&self, text: String) -> Result<String, CmdError> {
        let token = self.token().ok_or_else(|| CmdError::new("Log in first."))?;
        match self.auth(AuthRequest::SetStatusText { token, text }).await? {
            AuthReply::StatusTextSaved { text } => Ok(text),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    /// Sets or removes the profile picture. Returns the picture now saved (empty when removed).
    pub async fn set_avatar(&self, png_base64: Option<String>) -> Result<String, CmdError> {
        let token = self.token().ok_or_else(|| CmdError::new("Log in first."))?;
        match self.auth(AuthRequest::SetAvatar { token, png_base64 }).await? {
            AuthReply::AvatarSaved { avatar } => Ok(avatar),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    /// Someone's public profile card, by their account id.
    pub async fn profile_card(&self, id: &str) -> Result<Card, CmdError> {
        // The session lets the card list the servers the two of you share.
        let token = self.token().unwrap_or_default();
        match self
            .auth(AuthRequest::ProfileCard {
                id: id.to_string(),
                token,
            })
            .await?
        {
            AuthReply::ProfileCard {
                name,
                username,
                bio,
                banner,
                avatar,
                created,
                status,
                mutual,
            } => Ok(Card {
                name,
                username,
                bio,
                banner,
                avatar,
                status,
                created,
                mutual,
            }),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    /// Tells the auth service which servers this account is in, so other people's cards can show shared ones.
    pub async fn set_servers(&self, servers: Vec<String>) -> Result<(), CmdError> {
        let token = self.token().ok_or_else(|| CmdError::new("Log in first."))?;
        match self.auth(AuthRequest::SetServers { token, servers }).await? {
            AuthReply::ServersSaved => Ok(()),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    pub async fn update_profile(&self, name: Option<String>, username: Option<String>) -> Result<Profile, CmdError> {
        let token = self.token().ok_or_else(|| CmdError::new("Log in first."))?;
        let reply = self.auth(AuthRequest::UpdateProfile { token, name, username }).await?;
        self.apply_profile(reply)
    }

    /// Stores the name the auth service now reports, and moves the saved login to it.
    fn apply_profile(&self, reply: AuthReply) -> Result<Profile, CmdError> {
        match reply {
            AuthReply::Profile {
                name,
                username,
                email,
                verified,
                can_verify,
                bio,
                banner,
                avatar,
                has_password,
            } => {
                let (old, token, remember) = {
                    let mut inner = self.lock();
                    let old = std::mem::replace(&mut inner.user, name.clone());
                    (old, inner.token.clone(), inner.remember)
                };
                if let (true, Some(token)) = (remember && old != name, token) {
                    if let Err(e) = keychain::store(&name, &token) {
                        eprintln!("could not save your login to the keychain: {e}");
                    }
                    keychain::forget(&old);
                }
                Ok(Profile {
                    name,
                    username,
                    email,
                    verified,
                    can_verify,
                    bio,
                    banner,
                    avatar,
                    has_password,
                })
            }
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    /// Emails a confirmation code to the account's address.
    pub async fn request_email_code(&self) -> Result<(), CmdError> {
        let token = self.token().ok_or_else(|| CmdError::new("Log in first."))?;
        match self.auth(AuthRequest::RequestEmailCode { token }).await? {
            AuthReply::CodeSent => Ok(()),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    /// Confirms the email address with the code that was emailed.
    pub async fn verify_email(&self, code: String) -> Result<(), CmdError> {
        let token = self.token().ok_or_else(|| CmdError::new("Log in first."))?;
        match self.auth(AuthRequest::VerifyEmail { token, code }).await? {
            AuthReply::EmailVerified => Ok(()),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    /// The Discord account linked to this account, and whether the link can be removed.
    pub async fn linked_accounts(&self) -> Result<LinkedAccounts, CmdError> {
        let token = self.token().ok_or_else(|| CmdError::new("Log in first."))?;
        match self.auth(AuthRequest::LinkedAccounts { token }).await? {
            AuthReply::LinkedAccounts {
                discord,
                can_unlink_discord,
            } => Ok(LinkedAccounts {
                discord,
                can_unlink_discord,
            }),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    /// Removes the link to the Discord account.
    pub async fn unlink_discord(&self) -> Result<(), CmdError> {
        let token = self.token().ok_or_else(|| CmdError::new("Log in first."))?;
        match self.auth(AuthRequest::UnlinkDiscord { token }).await? {
            AuthReply::DiscordUnlinked => Ok(()),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }

    pub async fn change_email(&self, password: String, new_email: String) -> Result<String, CmdError> {
        let token = self.token().ok_or_else(|| CmdError::new("Log in first."))?;
        match self
            .auth(AuthRequest::ChangeEmail {
                token,
                password,
                new_email,
            })
            .await?
        {
            AuthReply::EmailChanged { email } => Ok(email),
            AuthReply::Error { text } => Err(CmdError::new(text)),
            _ => Err(CmdError::new("Unexpected reply from the auth service.")),
        }
    }
}
