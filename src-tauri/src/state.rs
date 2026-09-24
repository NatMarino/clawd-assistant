// The inbox: everything Claude has told Claw'd about, kept as a small store
// of items keyed by id. Both feeds (the watched folder and the loopback HTTP
// door, feed.rs) send batches into one channel; this thread upserts them,
// expires the stale ones, persists the lot, and publishes a snapshot to the
// webview as "pet-state".
//
// Deliberately NOT here: which clip he plays, when he speaks, reminders
// coming due, nudges. Those are presentation and live in index.html, which
// derives them from this snapshot plus the clock. Rust keeps the record.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::PoisonTolerant;

pub enum PetEvent {
    /// a batch from one of the feeds; `via` is "folder" or "http"
    Items(Vec<Item>, &'static str),
    /// a feed hit something it could not use (bad file, bad JSON)
    FeedError(String),
    /// the user ticked an item off in the popover
    Dismiss(String),
    /// the user pressed Do it on an item: it now belongs to Claude
    HandOff(String),
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Default, Debug)]
pub struct Proposal {
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub draft_link: String,
}

/// One thing Claude wants Claw'd to show. Everything but `id` is optional,
/// because the writer is a language model following a skill, not a schema
/// validator: a missing field gets a sensible default rather than a rejected
/// batch. Unknown fields are ignored.
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Item {
    pub id: String,
    /// waiting | reminder | message | delivery | note | heartbeat
    #[serde(default = "default_kind")]
    pub kind: String,
    /// slack | gcal | gmail | asana | drive | claude | ...
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub who: String,
    #[serde(default)]
    pub title: String,
    /// what he says out loud; falls back to "who: title"
    #[serde(default)]
    pub spoken: String,
    #[serde(default)]
    pub link: String,
    #[serde(default)]
    pub at: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub expires: Option<String>,
    /// 0 = low .. 3 = drop everything
    #[serde(default = "default_urgency")]
    pub urgency: u8,
    #[serde(default)]
    pub resolved: bool,
    /// which "waiting on you" rule produced it (for the popover tooltip)
    #[serde(default)]
    pub rule: String,
    #[serde(default)]
    pub proposal: Option<Proposal>,
    /// heartbeat only: the sweep interval, so staleness is judged against
    /// what the user actually scheduled
    #[serde(default)]
    pub interval_min: Option<u32>,
    /// heartbeat only: no sweeps are coming before this time (evenings,
    /// weekends), so silence until then is expected rather than a fault
    #[serde(default)]
    pub quiet_until: Option<String>,
}

fn default_kind() -> String {
    "note".into()
}
fn default_urgency() -> u8 {
    1
}

/// An item as the pet holds it: Claude's fields plus the pet's own
/// bookkeeping, with the times pre-parsed to epoch ms so the frontend never
/// parses a date.
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Stored {
    #[serde(flatten)]
    pub item: Item,
    pub first_seen_ms: u64,
    pub updated_ms: u64,
    #[serde(default)]
    pub handed_off: bool,
    #[serde(default)]
    pub at_ms: Option<u64>,
    #[serde(default)]
    pub due_ms: Option<u64>,
    #[serde(default)]
    pub expires_ms: Option<u64>,
}

#[derive(Clone, Serialize, PartialEq, Default)]
pub struct PetStatePayload {
    /// sorted: urgency desc, then oldest first
    pub items: Vec<Stored>,
    /// epoch ms of the last batch or heartbeat from Claude; None = never
    pub last_heard_ms: Option<u64>,
    /// from the last heartbeat; see Item::quiet_until
    pub quiet_until_ms: Option<u64>,
    pub sweep_minutes: u32,
    pub nudge_minutes: u32,
    /// the most recent feed problem, for the popover; empty = fine
    pub feed_error: String,
    pub inbox_dir: String,
    pub http_ok: bool,
}

pub struct PetStateStore(pub Arc<Mutex<PetStatePayload>>);

#[tauri::command]
pub fn get_pet_state(store: tauri::State<PetStateStore>) -> PetStatePayload {
    store.0.lock_or_recover().clone()
}

/// Commands from the popover go through the same channel as the feeds, so
/// the state thread stays the only writer.
pub struct EventSender(pub Mutex<Sender<PetEvent>>);

#[tauri::command]
pub fn dismiss_item(tx: tauri::State<EventSender>, id: String) {
    let _ = tx.0.lock_or_recover().send(PetEvent::Dismiss(id));
}

#[tauri::command]
pub fn hand_off_item(tx: tauri::State<EventSender>, id: String) {
    let _ = tx.0.lock_or_recover().send(PetEvent::HandOff(id));
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// RFC 3339 first; then a bare local time ("2026-09-24T14:05", with or
/// without seconds, "T" or space), because that is what a person — or a
/// model reading a calendar in local time — naturally writes.
pub fn parse_time(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(t) = DateTime::parse_from_rfc3339(s) {
        return u64::try_from(t.timestamp_millis()).ok();
    }
    for fmt in ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"] {
        if let Ok(n) = NaiveDateTime::parse_from_str(s, fmt) {
            if let Some(t) = Local.from_local_datetime(&n).earliest() {
                return u64::try_from(t.timestamp_millis()).ok();
            }
        }
    }
    None
}

const HOUR: u64 = 3_600_000;
const DAY: u64 = 24 * HOUR;
/// a ticked-off id stays ticked off this long, so the next sweep re-sending
/// the same Slack message doesn't resurrect it
const DISMISS_MEMORY: u64 = 3 * DAY;

/// When an item leaves on its own if Claude never resolves it. An explicit
/// `expires` always wins; otherwise each kind gets a lifetime that matches
/// what it is for, so a sweep that stops running can't leave the popover
/// full of week-old noise.
fn effective_expiry(s: &Stored) -> u64 {
    if let Some(e) = s.expires_ms {
        return e;
    }
    let base = s.at_ms.unwrap_or(s.first_seen_ms);
    match s.item.kind.as_str() {
        "reminder" => s.due_ms.map(|d| d + 2 * HOUR).unwrap_or(s.updated_ms + 12 * HOUR),
        "waiting" => s.updated_ms + 3 * DAY,
        "delivery" => base + DAY,
        "message" => s.updated_ms + 12 * HOUR,
        _ => s.updated_ms + DAY,
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Persisted {
    items: HashMap<String, Stored>,
    dismissed: HashMap<String, u64>,
    last_heard_ms: Option<u64>,
    #[serde(default)]
    quiet_until_ms: Option<u64>,
}

fn persist_path() -> Option<PathBuf> {
    crate::app_data_dir().map(|d| d.join("inbox.json"))
}

fn load_persisted() -> Persisted {
    persist_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_persisted(p: &Persisted) {
    let Some(path) = persist_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // write-then-rename: a crash mid-write must never cost the whole inbox
    let tmp = path.with_extension("json.tmp");
    if let Ok(json) = serde_json::to_string_pretty(p) {
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

pub struct Inbox {
    p: Persisted,
    pub feed_error: String,
    dirty: bool,
}

impl Inbox {
    pub fn load() -> Self {
        Inbox { p: load_persisted(), feed_error: String::new(), dirty: false }
    }

    #[cfg(test)]
    fn empty() -> Self {
        Inbox { p: Persisted::default(), feed_error: String::new(), dirty: false }
    }

    pub fn apply(&mut self, ev: PetEvent, now: u64) -> Option<u32> {
        let mut interval = None;
        match ev {
            PetEvent::Items(items, _via) => {
                self.p.last_heard_ms = Some(now);
                self.feed_error.clear();
                for it in items {
                    if it.kind == "heartbeat" {
                        interval = it.interval_min.or(interval);
                        // every heartbeat restates it: absent means "not quiet"
                        self.p.quiet_until_ms = it.quiet_until.as_deref().and_then(parse_time);
                        continue;
                    }
                    self.upsert(it, now);
                }
            }
            PetEvent::FeedError(e) => self.feed_error = e,
            PetEvent::Dismiss(id) => {
                self.p.items.remove(&id);
                self.p.dismissed.insert(id, now);
            }
            PetEvent::HandOff(id) => {
                if let Some(s) = self.p.items.get_mut(&id) {
                    s.handed_off = true;
                }
            }
        }
        self.dirty = true;
        interval
    }

    fn upsert(&mut self, it: Item, now: u64) {
        let id = it.id.trim().to_string();
        if id.is_empty() {
            return;
        }
        if it.resolved {
            // Claude saw it handled (replied, joined, closed): gone, and no
            // longer remembered as dismissed either
            self.p.items.remove(&id);
            self.p.dismissed.remove(&id);
            return;
        }
        if self.p.dismissed.contains_key(&id) {
            return;
        }
        let at_ms = it.at.as_deref().and_then(parse_time);
        let due_ms = it.due.as_deref().and_then(parse_time);
        let expires_ms = it.expires.as_deref().and_then(parse_time);
        match self.p.items.get_mut(&id) {
            Some(s) => {
                s.item = Item { id: id.clone(), ..it };
                s.updated_ms = now;
                s.at_ms = at_ms;
                s.due_ms = due_ms;
                s.expires_ms = expires_ms;
            }
            None => {
                self.p.items.insert(
                    id.clone(),
                    Stored {
                        item: Item { id, ..it },
                        first_seen_ms: now,
                        updated_ms: now,
                        handed_off: false,
                        at_ms,
                        due_ms,
                        expires_ms,
                    },
                );
            }
        }
    }

    /// Drop expired items and forget old dismissals. Returns whether
    /// anything changed.
    pub fn expire(&mut self, now: u64) -> bool {
        let before = (self.p.items.len(), self.p.dismissed.len());
        self.p.items.retain(|_, s| effective_expiry(s) > now);
        self.p.dismissed.retain(|_, at| now.saturating_sub(*at) < DISMISS_MEMORY);
        let changed = before != (self.p.items.len(), self.p.dismissed.len());
        self.dirty |= changed;
        changed
    }

    pub fn save_if_dirty(&mut self) {
        if self.dirty {
            self.dirty = false;
            save_persisted(&self.p);
        }
    }

    pub fn payload(&self, base: &PetStatePayload) -> PetStatePayload {
        let mut items: Vec<Stored> = self.p.items.values().cloned().collect();
        items.sort_by(|a, b| {
            b.item
                .urgency
                .cmp(&a.item.urgency)
                .then(a.first_seen_ms.cmp(&b.first_seen_ms))
                .then(a.item.id.cmp(&b.item.id))
        });
        PetStatePayload {
            items,
            last_heard_ms: self.p.last_heard_ms,
            quiet_until_ms: self.p.quiet_until_ms,
            feed_error: self.feed_error.clone(),
            ..base.clone()
        }
    }
}

pub fn spawn_state_thread(
    rx: Receiver<PetEvent>,
    handle: AppHandle,
    base: PetStatePayload,
) {
    std::thread::spawn(move || {
        let mut inbox = Inbox::load();
        let mut base = base;
        inbox.expire(now_ms());
        publish(&inbox, &base, &handle);
        loop {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(ev) => {
                    if let Some(min) = inbox.apply(ev, now_ms()) {
                        base.sweep_minutes = min.clamp(1, 24 * 60);
                    }
                    // drain whatever else is queued before publishing once
                    while let Ok(ev) = rx.try_recv() {
                        if let Some(min) = inbox.apply(ev, now_ms()) {
                            base.sweep_minutes = min.clamp(1, 24 * 60);
                        }
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            inbox.expire(now_ms());
            inbox.save_if_dirty();
            publish(&inbox, &base, &handle);
        }
    });
}

fn publish(inbox: &Inbox, base: &PetStatePayload, handle: &AppHandle) {
    let payload = inbox.payload(base);
    let store = handle.state::<PetStateStore>();
    let mut cur = store.0.lock_or_recover();
    if *cur == payload {
        return;
    }
    *cur = payload.clone();
    drop(cur);
    let _ = handle.emit_to("pet", "pet-state", payload);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(json: &str) -> Item {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn minimal_item_gets_defaults() {
        let it = item(r#"{"id":"x"}"#);
        assert_eq!(it.kind, "note");
        assert_eq!(it.urgency, 1);
        assert!(it.proposal.is_none());
    }

    #[test]
    fn upsert_keeps_first_seen_and_hand_off() {
        let mut ib = Inbox::empty();
        ib.apply(PetEvent::Items(vec![item(r#"{"id":"a","kind":"waiting","title":"one"}"#)], "http"), 1000);
        ib.apply(PetEvent::HandOff("a".into()), 1500);
        ib.apply(PetEvent::Items(vec![item(r#"{"id":"a","kind":"waiting","title":"two"}"#)], "http"), 2000);
        let s = &ib.p.items["a"];
        assert_eq!(s.first_seen_ms, 1000);
        assert_eq!(s.updated_ms, 2000);
        assert_eq!(s.item.title, "two");
        assert!(s.handed_off);
    }

    #[test]
    fn dismissed_stays_dismissed_until_resolved() {
        let mut ib = Inbox::empty();
        let a = || item(r#"{"id":"a","kind":"waiting"}"#);
        ib.apply(PetEvent::Items(vec![a()], "folder"), 1000);
        ib.apply(PetEvent::Dismiss("a".into()), 1100);
        ib.apply(PetEvent::Items(vec![a()], "folder"), 1200);
        assert!(ib.p.items.is_empty());
        ib.apply(PetEvent::Items(vec![item(r#"{"id":"a","resolved":true}"#)], "folder"), 1300);
        ib.apply(PetEvent::Items(vec![a()], "folder"), 1400);
        assert_eq!(ib.p.items.len(), 1);
    }

    #[test]
    fn heartbeat_is_not_stored_but_counts_as_heard() {
        let mut ib = Inbox::empty();
        let min = ib.apply(PetEvent::Items(vec![item(r#"{"id":"hb","kind":"heartbeat","interval_min":30}"#)], "folder"), 5000);
        assert_eq!(min, Some(30));
        assert!(ib.p.items.is_empty());
        assert_eq!(ib.p.last_heard_ms, Some(5000));
        ib.apply(PetEvent::Items(vec![item(r#"{"id":"hb","kind":"heartbeat","quiet_until":"1970-01-01T00:00:09Z"}"#)], "folder"), 6000);
        assert_eq!(ib.p.quiet_until_ms, Some(9000));
        ib.apply(PetEvent::Items(vec![item(r#"{"id":"hb","kind":"heartbeat"}"#)], "folder"), 7000);
        assert_eq!(ib.p.quiet_until_ms, None);
    }

    #[test]
    fn explicit_expiry_wins_and_expires() {
        let mut ib = Inbox::empty();
        ib.apply(PetEvent::Items(vec![item(r#"{"id":"a","expires":"2000-01-01T00:00:00Z"}"#)], "http"), now_ms());
        assert!(ib.expire(now_ms()));
        assert!(ib.p.items.is_empty());
    }

    #[test]
    fn parses_offset_and_local_times() {
        assert_eq!(parse_time("1970-01-01T00:00:01Z"), Some(1000));
        assert!(parse_time("2026-09-24T14:05").is_some());
        assert!(parse_time("2026-09-24 14:05:30").is_some());
        assert_eq!(parse_time("next tuesday"), None);
    }

    #[test]
    fn sort_is_urgency_then_age() {
        let mut ib = Inbox::empty();
        ib.apply(PetEvent::Items(vec![item(r#"{"id":"old","urgency":1}"#)], "http"), 1000);
        ib.apply(PetEvent::Items(vec![item(r#"{"id":"hot","urgency":3}"#)], "http"), 2000);
        ib.apply(PetEvent::Items(vec![item(r#"{"id":"new","urgency":1}"#)], "http"), 3000);
        let ids: Vec<_> = ib.payload(&PetStatePayload::default()).items.into_iter().map(|s| s.item.id).collect();
        assert_eq!(ids, ["hot", "old", "new"]);
    }
}
