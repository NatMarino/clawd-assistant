// Doorways for home apps that can send a webhook but can't write Claw'd's
// item format: Sonarr, Radarr, Overseerr and Plex post their own JSON to
// /hooks/<app> on the item door, and this turns each into an item (or
// resolves one). No big brain involved: instant, and free.
//
// They can't all send a Bearer header, so /hooks/* also takes the token as
// ?token=… or as the password of Basic auth (Sonarr and Radarr's webhook
// "Password" field).

use serde_json::{json, Value};

/// The items for one webhook from `app`, as a JSON array for parse_batch,
/// or None when it's something he ignores (a pause, a grab…).
pub fn translate(app: &str, body: &str, content_type: &str) -> Result<Option<Value>, String> {
    let v: Value = if app == "plex" && content_type.to_ascii_lowercase().starts_with("multipart/") {
        serde_json::from_str(&plex_payload(body).ok_or("no payload part in Plex's webhook")?).map_err(|e| e.to_string())?
    } else {
        serde_json::from_str(body.trim_start_matches('\u{feff}')).map_err(|e| format!("not JSON: {e}"))?
    };
    Ok(match app {
        "sonarr" => arr(&v, "sonarr"),
        "radarr" => arr(&v, "radarr"),
        "overseerr" | "jellyseerr" => overseerr(&v),
        "plex" => plex(&v),
        _ => return Err(format!("no doorway for {app}")),
    })
}

fn s<'a>(v: &'a Value, path: &str) -> &'a str {
    v.pointer(path).and_then(|x| x.as_str()).unwrap_or("")
}
fn n(v: &Value, path: &str) -> i64 {
    v.pointer(path).and_then(|x| x.as_i64()).unwrap_or(0)
}
fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}
fn cap(t: &str) -> String {
    let mut c = t.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

/// Sonarr and Radarr share their event shapes.
fn arr(v: &Value, app: &str) -> Option<Value> {
    let name = cap(app);
    match s(v, "/eventType") {
        "Download" => {
            if v.get("isUpgrade").and_then(|x| x.as_bool()) == Some(true) {
                return None; // a better copy of something already there
            }
            if app == "sonarr" {
                let show = s(v, "/series/title");
                let eps = v.get("episodes").and_then(|e| e.as_array()).cloned().unwrap_or_default();
                let first = eps.first().cloned().unwrap_or(Value::Null);
                let (se, ep) = (n(&first, "/seasonNumber"), n(&first, "/episodeNumber"));
                let code = if eps.len() > 1 { format!("S{se:02}E{ep:02} (+{})", eps.len() - 1) } else { format!("S{se:02}E{ep:02}") };
                Some(json!([{
                    "id": format!("sonarr:{}:{code}", n(v, "/series/id")),
                    "kind": "message", "source": "sonarr", "rule": "media-ready",
                    "title": format!("{show} {code} is ready"),
                    "spoken": if eps.len() > 1 { format!("New episodes of {show} are ready to watch!") } else { format!("The new {show} episode is ready to watch!") },
                }]))
            } else {
                let movie = s(v, "/movie/title");
                let year = n(v, "/movie/year");
                Some(json!([{
                    "id": format!("radarr:{}", n(v, "/movie/id")),
                    "kind": "message", "source": "radarr", "rule": "media-ready",
                    "title": if year > 0 { format!("{movie} ({year}) is ready") } else { format!("{movie} is ready") },
                    "spoken": format!("{movie} is ready to watch!"),
                }]))
            }
        }
        "Health" => Some(json!([{
            "id": format!("down:{app}:{}", s(v, "/type")),
            "kind": if s(v, "/level") == "error" { "waiting" } else { "note" },
            "source": app, "rule": "server-down",
            "title": format!("{name}: {}", s(v, "/message")),
            "spoken": format!("{name} says something's wrong: {}", s(v, "/message")),
        }])),
        "HealthRestored" => Some(json!([{ "id": format!("down:{app}:{}", s(v, "/type")), "resolved": true }])),
        "ManualInteractionRequired" => Some(json!([{
            "id": format!("{app}:manual:{}", today()),
            "kind": "waiting", "source": app, "rule": "server-down",
            "title": format!("{name} needs you to pick a download"),
        }])),
        "Test" => Some(json!([{ "id": format!("{app}:test"), "kind": "note", "source": app, "title": format!("{name} can reach me!"), "spoken": format!("Hi {name}! I can hear you.") }])),
        _ => None,
    }
}

fn overseerr(v: &Value) -> Option<Value> {
    let what = s(v, "/subject");
    let who = s(v, "/request/requestedBy_username");
    let rid = match v.pointer("/request/request_id") {
        Some(Value::String(x)) => x.clone(),
        Some(x) if !x.is_null() => x.to_string(),
        _ => String::new(),
    };
    let id = format!("request:{rid}");
    match s(v, "/notification_type") {
        "MEDIA_PENDING" => Some(json!([{
            "id": id, "kind": "waiting", "source": "overseerr", "who": who, "rule": "media-requests",
            "title": format!("{what} (needs your OK)"),
            "spoken": if who.is_empty() { format!("Someone requested {what}. It needs your OK!") } else { format!("{who} requested {what}. It needs your OK!") },
        }])),
        "MEDIA_APPROVED" | "MEDIA_AUTO_APPROVED" | "MEDIA_DECLINED" => Some(json!([{ "id": id, "resolved": true }])),
        "MEDIA_AVAILABLE" => Some(json!([
            { "id": id, "resolved": true },
            { "id": format!("overseerr:available:{rid}"), "kind": "message", "source": "overseerr", "who": who, "rule": "media-ready",
              "title": format!("{what} is available"), "spoken": format!("{what} is ready to watch!") }
        ])),
        "MEDIA_FAILED" => Some(json!([{
            "id": format!("overseerr:failed:{rid}"), "kind": "waiting", "source": "overseerr", "rule": "server-down",
            "title": format!("{what} failed to download"),
        }])),
        "TEST_NOTIFICATION" => Some(json!([{ "id": "overseerr:test", "kind": "note", "source": "overseerr", "title": "Overseerr can reach me!", "spoken": "Hi Overseerr! I can hear you." }])),
        _ => None,
    }
}

/// Plex (Plex Pass webhooks): who's watching what, and new things added.
/// Your own playback isn't news to you, so only other people's.
fn plex(v: &Value) -> Option<Value> {
    let kind = s(v, "/Metadata/type");
    let title = if kind == "episode" {
        format!("{} S{:02}E{:02}", s(v, "/Metadata/grandparentTitle"), n(v, "/Metadata/parentIndex"), n(v, "/Metadata/index"))
    } else {
        s(v, "/Metadata/title").to_string()
    };
    let who = s(v, "/Account/title");
    let session = format!("stream:{}:{}", who, s(v, "/Metadata/ratingKey"));
    let own = v.get("owner").and_then(|x| x.as_bool()) == Some(true) && v.get("user").and_then(|x| x.as_bool()) == Some(true);
    match s(v, "/event") {
        "media.play" | "media.resume" if !own => Some(json!([{
            "id": session, "kind": "note", "source": "plex", "who": who, "rule": "now-streaming",
            "title": format!("Watching {title}"),
            "spoken": format!("{who} is watching {title}."),
            "expires": (chrono::Local::now() + chrono::Duration::hours(4)).to_rfc3339(),
        }])),
        "media.stop" => Some(json!([{ "id": session, "resolved": true }])),
        "library.new" => Some(json!([{
            "id": format!("plex:{}", s(v, "/Metadata/ratingKey")), "kind": "message", "source": "plex", "rule": "media-ready",
            "title": format!("{title} was added"), "spoken": format!("{title} just landed on Plex!"),
        }])),
        _ => None,
    }
}

/// The JSON part of Plex's multipart/form-data webhook.
fn plex_payload(body: &str) -> Option<String> {
    let at = body.find("name=\"payload\"")?;
    let rest = &body[at..];
    let start = rest.find("\r\n\r\n").map(|i| i + 4).or_else(|| rest.find("\n\n").map(|i| i + 2))?;
    let json = &rest[start..];
    let end = json.find("\r\n--").or_else(|| json.find("\n--")).unwrap_or(json.len());
    Some(json[..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(app: &str, body: &str) -> Value {
        translate(app, body, "application/json").unwrap().unwrap()[0].clone()
    }

    #[test]
    fn sonarr_and_radarr() {
        let e = one("sonarr", r#"{"eventType":"Download","isUpgrade":false,"series":{"id":7,"title":"Severance"},"episodes":[{"seasonNumber":2,"episodeNumber":4}]}"#);
        assert_eq!(e["id"], "sonarr:7:S02E04");
        assert_eq!(e["title"], "Severance S02E04 is ready");
        assert!(translate("sonarr", r#"{"eventType":"Download","isUpgrade":true}"#, "").unwrap().is_none());
        let m = one("radarr", r#"{"eventType":"Download","movie":{"id":3,"title":"Dune","year":2021}}"#);
        assert_eq!(m["title"], "Dune (2021) is ready");
        let h = one("radarr", r#"{"eventType":"Health","level":"error","type":"IndexerStatusCheck","message":"All indexers are unavailable"}"#);
        assert_eq!((h["kind"].as_str(), h["id"].as_str()), (Some("waiting"), Some("down:radarr:IndexerStatusCheck")));
        let r = one("radarr", r#"{"eventType":"HealthRestored","type":"IndexerStatusCheck"}"#);
        assert_eq!(r["resolved"], true);
        assert!(translate("sonarr", r#"{"eventType":"Grab"}"#, "").unwrap().is_none());
    }

    #[test]
    fn overseerr_requests() {
        let p = one("overseerr", r#"{"notification_type":"MEDIA_PENDING","subject":"Dune (2021)","request":{"request_id":"12","requestedBy_username":"Sam"}}"#);
        assert_eq!((p["id"].as_str(), p["kind"].as_str()), (Some("request:12"), Some("waiting")));
        assert!(p["spoken"].as_str().unwrap().starts_with("Sam requested Dune"));
        let a = translate("overseerr", r#"{"notification_type":"MEDIA_AVAILABLE","subject":"Dune (2021)","request":{"request_id":12}}"#, "").unwrap().unwrap();
        assert_eq!(a[0]["resolved"], true);
        assert_eq!(a[1]["title"], "Dune (2021) is available");
    }

    #[test]
    fn plex_webhooks_in_multipart() {
        let payload = r#"{"event":"media.play","user":false,"owner":true,"Account":{"title":"Sam"},"Metadata":{"type":"episode","grandparentTitle":"Severance","parentIndex":2,"index":4,"ratingKey":"55"}}"#;
        let body = format!("--XYZ\r\nContent-Disposition: form-data; name=\"payload\"\r\nContent-Type: application/json\r\n\r\n{payload}\r\n--XYZ--\r\n");
        let v = translate("plex", &body, "multipart/form-data; boundary=XYZ").unwrap().unwrap();
        assert_eq!(v[0]["spoken"], "Sam is watching Severance S02E04.");
        // your own playback isn't news
        let mine = payload.replace(r#""user":false"#, r#""user":true"#);
        assert!(translate("plex", &mine, "application/json").unwrap().is_none());
        let stop = payload.replace("media.play", "media.stop");
        assert_eq!(translate("plex", &stop, "application/json").unwrap().unwrap()[0]["resolved"], true);
    }
}
