//! Pure geometry/state policy. All sizes are logical content sizes; offsets are
//! relative to one screen's work area, never mixed-DPI desktop coordinates.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DEFAULT: Size = Size {
    width: 1180.0,
    height: 780.0,
};
pub const MINIMUM: Size = Size {
    width: 760.0,
    height: 620.0,
};
const MARGIN: f64 = 12.0;
const EPSILON: f64 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}
impl Size {
    pub fn valid(self) -> bool {
        [self.width, self.height]
            .iter()
            .all(|v| v.is_finite() && *v > 0.0 && *v <= 65536.0)
    }
    pub fn near(self, other: Self) -> bool {
        (self.width - other.width).abs() <= EPSILON && (self.height - other.height).abs() <= EPSILON
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    fn valid_anchor(self) -> bool {
        [self.x, self.y]
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
    }
    fn near(self, other: Self) -> bool {
        (self.x - other.x).abs() <= EPSILON && (self.y - other.y).abs() <= EPSILON
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Screen {
    /// Session identity, including screens for which a persistent ID is unavailable.
    pub token: String,
    /// Only a unique, platform-provided, hashed identity is persisted.
    pub key: Option<String>,
    pub work: Size,
    pub scale: f64,
    /// Native origin used only to detect topology changes, not for policy arithmetic.
    pub origin: Point,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Normal,
    Maximized,
    Fullscreen,
    Minimized,
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    Arranged,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub screens: Vec<Screen>,
    pub current: String,
    pub content: Size,
    pub offset: Point,
    pub decoration: Size,
    pub mode: Mode,
    pub interacting: bool,
    pub user_resizing: bool,
    pub display_epoch: u64,
}
impl Snapshot {
    pub fn screen(&self) -> Option<&Screen> {
        self.screens.iter().find(|s| s.token == self.current)
    }
    fn same_environment(&self, other: &Self) -> bool {
        self.current == other.current
            && self.screens == other.screens
            && self.display_epoch == other.display_epoch
    }
    fn stable(&self, other: &Self) -> bool {
        self.same_environment(other)
            && self.content.near(other.content)
            && self.offset.near(other.offset)
            && self.mode == other.mode
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub preferred: Size,
    pub effective: Size,
    pub anchor: Point,
    pub maximized: bool,
    pub updated: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Saved {
    pub schema_version: u8,
    pub last_monitor: Option<String>,
    pub last_normal: Size,
    pub last_maximized: bool,
    pub profiles: BTreeMap<String, Profile>,
}
impl Default for Saved {
    fn default() -> Self {
        Self {
            schema_version: 1,
            last_monitor: None,
            last_normal: DEFAULT,
            last_maximized: false,
            profiles: BTreeMap::new(),
        }
    }
}
impl Saved {
    pub fn valid(&self) -> bool {
        let key_ok = |key: &str| key.len() == 64 && key.bytes().all(|c| c.is_ascii_hexdigit());
        self.schema_version == 1
            && self.last_normal.valid()
            && self.profiles.len() <= 16
            && self.last_monitor.as_deref().is_none_or(key_ok)
            && self.profiles.iter().all(|(key, p)| {
                key_ok(key)
                    && p.preferred.valid()
                    && p.effective.valid()
                    && p.anchor.valid_anchor()
                    && p.updated < u64::MAX / 2
            })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub screen: String,
    pub content: Size,
    pub offset: Point,
    pub minimum: Size,
    pub maximize: bool,
}

pub fn fit(screen: &Screen, desired: Size, decoration: Size, anchor: Point) -> Placement {
    // Keep the window operable even if the work area is below the usual minimum.
    let margin_x = MARGIN.min((screen.work.width / 8.0).max(0.0));
    let margin_y = MARGIN.min((screen.work.height / 8.0).max(0.0));
    let max = Size {
        width: (screen.work.width - 2.0 * margin_x - decoration.width).max(1.0),
        height: (screen.work.height - 2.0 * margin_y - decoration.height).max(1.0),
    };
    let minimum = Size {
        width: MINIMUM.width.min(max.width),
        height: MINIMUM.height.min(max.height),
    };
    let desired = if desired.valid() { desired } else { DEFAULT };
    let content = Size {
        width: desired.width.clamp(minimum.width, max.width),
        height: desired.height.clamp(minimum.height, max.height),
    };
    let anchor = if anchor.valid_anchor() {
        anchor
    } else {
        Point { x: 0.5, y: 0.5 }
    };
    Placement {
        screen: screen.token.clone(),
        content,
        minimum,
        maximize: false,
        offset: Point {
            x: margin_x + (max.width - content.width) * anchor.x,
            y: margin_y + (max.height - content.height) * anchor.y,
        },
    }
}

fn anchor(snapshot: &Snapshot, screen: &Screen) -> Point {
    let fraction = |position: f64, available: f64| {
        if available <= EPSILON {
            0.5
        } else {
            ((position - MARGIN) / available).clamp(0.0, 1.0)
        }
    };
    Point {
        x: fraction(
            snapshot.offset.x,
            screen.work.width - snapshot.content.width - snapshot.decoration.width - 2.0 * MARGIN,
        ),
        y: fraction(
            snapshot.offset.y,
            screen.work.height
                - snapshot.content.height
                - snapshot.decoration.height
                - 2.0 * MARGIN,
        ),
    }
}

pub struct Engine {
    pub saved: Saved,
    pub revision: u64,
    pub restored: bool,
    pub shown: bool,
    observed: Option<Snapshot>,
    accepted: Option<Snapshot>,
    stable_since: u64,
    applying_until: u64,
    user_edit: bool,
    correction_attempts: u8,
}
impl Engine {
    pub fn ready_to_show(&self, now: u64) -> bool {
        self.restored && now >= self.applying_until
    }
    pub fn new(saved: Saved) -> Self {
        let revision = saved
            .profiles
            .values()
            .map(|p| p.updated)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        Self {
            saved,
            revision,
            restored: false,
            shown: false,
            observed: None,
            accepted: None,
            stable_since: 0,
            applying_until: 0,
            user_edit: false,
            correction_attempts: 0,
        }
    }

    pub fn step(
        &mut self,
        snapshot: Snapshot,
        now: u64,
        user_resize_ended: bool,
        closing: bool,
    ) -> Option<Placement> {
        let screen = snapshot.screen()?.clone();
        if !screen.work.valid() || !snapshot.content.valid() {
            return None;
        }
        let stable = self
            .observed
            .as_ref()
            .is_some_and(|old| old.stable(&snapshot));
        if !stable {
            self.stable_since = now;
        }
        self.observed = Some(snapshot.clone());

        // Do not interpret startup/setFrame callbacks as user intent.
        if now < self.applying_until {
            return None;
        }
        let environment_changed = self
            .accepted
            .as_ref()
            .is_some_and(|old| !old.same_environment(&snapshot));
        if environment_changed {
            self.user_edit = false;
        }
        if self.restored
            && !environment_changed
            && snapshot.mode == Mode::Normal
            && (snapshot.user_resizing || user_resize_ended)
        {
            self.user_edit = true;
        }
        if !closing && (snapshot.interacting || now.saturating_sub(self.stable_since) < 300) {
            return None;
        }

        if !self.restored {
            if closing {
                return None;
            }
            let target = self
                .saved
                .last_monitor
                .as_ref()
                .and_then(|key| {
                    snapshot
                        .screens
                        .iter()
                        .find(|s| s.key.as_ref() == Some(key))
                })
                .unwrap_or(&screen);
            let profile = target
                .key
                .as_ref()
                .and_then(|key| self.saved.profiles.get(key));
            let mut placement = fit(
                target,
                profile.map_or(self.saved.last_normal, |p| p.preferred),
                snapshot.decoration,
                profile.map_or(Point { x: 0.5, y: 0.5 }, |p| p.anchor),
            );
            placement.maximize = profile.map_or(self.saved.last_maximized, |p| p.maximized);
            self.restored = true;
            // Seed the expected environment so a planned startup move isn't mistaken for a cross-screen drag.
            let target_token = target.token.clone();
            let mut accepted = snapshot;
            accepted.current = target_token;
            self.accepted = Some(accepted);
            self.applying_until = now + 800;
            return Some(placement);
        }

        if matches!(
            snapshot.mode,
            Mode::Fullscreen | Mode::Minimized | Mode::Arranged
        ) {
            self.user_edit = false;
            // Keep the previous normal environment, so unminimizing/unfullscreening on a new screen is checked.
            return None;
        }

        if environment_changed && snapshot.mode == Mode::Normal && !closing {
            let mut placement = fit(
                &screen,
                snapshot.content,
                snapshot.decoration,
                anchor(&snapshot, &screen),
            );
            // A window shrunk below the usual minimum on a tiny display must not
            // grow by itself when it returns to a larger display.
            placement.content.width = placement.content.width.min(snapshot.content.width);
            placement.content.height = placement.content.height.min(snapshot.content.height);
            placement.minimum.width = placement.minimum.width.min(placement.content.width);
            placement.minimum.height = placement.minimum.height.min(placement.content.height);
            let margin_x = MARGIN.min(screen.work.width / 8.0);
            let margin_y = MARGIN.min(screen.work.height / 8.0);
            placement.offset.x = snapshot.offset.x.clamp(
                margin_x,
                (screen.work.width
                    - placement.content.width
                    - snapshot.decoration.width
                    - margin_x)
                    .max(margin_x),
            );
            placement.offset.y = snapshot.offset.y.clamp(
                margin_y,
                (screen.work.height
                    - placement.content.height
                    - snapshot.decoration.height
                    - margin_y)
                    .max(margin_y),
            );
            if !placement.content.near(snapshot.content) || !placement.offset.near(snapshot.offset)
            {
                // At most two corrective applications per environment, then accept the OS outcome.
                if self.correction_attempts < 2 {
                    self.correction_attempts += 1;
                    self.applying_until = now + 800;
                    return Some(placement);
                }
            }
        }

        let before = self.saved.clone();
        let previous_profile = screen
            .key
            .as_ref()
            .and_then(|key| self.saved.profiles.get(key));
        let preferred = if self.user_edit && snapshot.mode == Mode::Normal {
            snapshot.content
        } else {
            previous_profile.map_or(self.saved.last_normal, |p| p.preferred)
        };
        if self.user_edit && snapshot.mode == Mode::Normal {
            self.saved.last_normal = snapshot.content;
        }
        self.saved.last_monitor = screen.key.clone();
        self.saved.last_maximized = snapshot.mode == Mode::Maximized;
        if let Some(key) = &screen.key {
            let old = self.saved.profiles.get(key);
            let effective = if snapshot.mode == Mode::Normal {
                snapshot.content
            } else {
                old.map_or(preferred, |p| p.effective)
            };
            let anchor = if snapshot.mode == Mode::Normal {
                anchor(&snapshot, &screen)
            } else {
                old.map_or(Point { x: 0.5, y: 0.5 }, |p| p.anchor)
            };
            let updated = old.map_or(0, |p| p.updated);
            let mut profile = Profile {
                preferred,
                effective,
                anchor,
                maximized: self.saved.last_maximized,
                updated,
            };
            if old != Some(&profile) {
                profile.updated = self.revision;
            }
            self.saved.profiles.insert(key.clone(), profile);
            while self.saved.profiles.len() > 16 {
                let oldest = self
                    .saved
                    .profiles
                    .iter()
                    .filter(|(k, _)| *k != key)
                    .min_by_key(|(_, p)| p.updated)
                    .map(|(k, _)| k.clone());
                if let Some(oldest) = oldest {
                    self.saved.profiles.remove(&oldest);
                } else {
                    break;
                }
            }
        }
        if self.saved != before {
            self.revision += 1;
        }
        self.user_edit = false;
        self.correction_attempts = 0;
        self.accepted = Some(snapshot);
        None
    }
}
