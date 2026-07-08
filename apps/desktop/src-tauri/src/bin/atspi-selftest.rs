//! AT-SPI injection self-test (P1-P0-3, ADR-0013) — run on the Linux reference
//! machine to validate the injection path live. Linux-only; a no-op elsewhere.
//!
//! Tracks keyboard focus on the AT-SPI bus and, for each newly-focused field:
//!   - reads its role/state and classifies it (secure-field detection, #8),
//!   - inserts a marker into editable, non-secure fields (native text insert),
//!   - REFUSES password/secure fields (the load-bearing safety behaviour).
//!
//!   DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus \
//!     cargo run --bin atspi-selftest
//!
//! Then focus GNOME Text Editor (should get the marker) and a password entry
//! (should be refused). Ctrl-C or the shell timeout stops it.

fn main() {
    #[cfg(target_os = "linux")]
    {
        // `--direct <app-name-substring>` walks the tree to that app's editable
        // text field and inserts there (no keyboard focus needed — proves native
        // insertion). Default mode tracks focus and injects on focus-gain.
        let args: Vec<String> = std::env::args().collect();
        match args.get(1).map(String::as_str) {
            Some("--direct") => {
                let want = args.get(2).cloned().unwrap_or_else(|| "Text Editor".into());
                linux::run_direct(want);
            }
            Some("--read") => {
                let want = args.get(2).cloned().unwrap_or_default();
                linux::run_read(want);
            }
            _ => linux::run(),
        }
    }
    #[cfg(not(target_os = "linux"))]
    eprintln!("atspi-selftest is Linux-only.");
}

#[cfg(target_os = "linux")]
mod linux {
    use atspi::connection::AccessibilityConnection;
    use atspi::events::object::StateChangedEvent;
    use atspi::proxy::accessible::AccessibleProxy;
    use atspi::proxy::editable_text::EditableTextProxy;
    use atspi::proxy::text::TextProxy;
    use atspi::State;
    use futures_lite::stream::StreamExt;
    use kaydence_lib::inject::linux::role_to_field_kind;
    use kaydence_lib::inject::FieldKind;

    const MARKER: &str = "[Kaydence\u{2713}] ";

    pub fn run() {
        if let Err(e) = pollster::block_on(track()) {
            eprintln!("[atspi-selftest] FAIL: {e}");
            std::process::exit(1);
        }
    }

    pub fn run_direct(app_substr: String) {
        if let Err(e) = pollster::block_on(direct(&app_substr)) {
            eprintln!("[atspi-selftest] FAIL: {e}");
            std::process::exit(1);
        }
    }

    pub fn run_read(app_substr: String) {
        if let Err(e) = pollster::block_on(read_field(&app_substr)) {
            eprintln!("[atspi-selftest] FAIL: {e}");
            std::process::exit(1);
        }
    }

    /// Read the current text of an app's editable field via AT-SPI (the read
    /// path works even where InsertText no-ops — used to verify keystroke
    /// injection landed).
    async fn read_field(app_substr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = AccessibilityConnection::new().await?;
        let c = conn.connection();
        let root = AccessibleProxy::builder(c)
            .destination("org.a11y.atspi.Registry")?
            .path("/org/a11y/atspi/accessible/root")?
            .build()
            .await?;
        for app in root.get_children().await? {
            let Some(name) = app.name() else { continue };
            let proxy = AccessibleProxy::builder(c)
                .destination(name.to_owned())?
                .path(app.path().to_owned())?
                .build()
                .await?;
            if !proxy.name().await.unwrap_or_default().contains(app_substr) {
                continue;
            }
            let Some((n, p, _role)) = find_editable(c, &proxy, 0).await else {
                return Err("no editable field found".into());
            };
            let text = TextProxy::builder(c)
                .destination(n)?
                .path(p)?
                .build()
                .await?;
            let count = text.character_count().await.unwrap_or(0);
            let body = text.get_text(0, count).await.unwrap_or_default();
            println!("[read] {app_substr}: {count} chars: {body:?}");
            return Ok(());
        }
        Err(format!("no app matching {app_substr:?}").into())
    }

    /// Find the first editable-text descendant of `acc` (bounded DFS).
    async fn find_editable(
        conn: &zbus::Connection,
        acc: &AccessibleProxy<'_>,
        depth: u32,
    ) -> Option<(String, zbus::zvariant::OwnedObjectPath, atspi::Role)> {
        if depth > 12 {
            return None;
        }
        let children = acc.get_children().await.ok()?;
        for child in children {
            let name = child.name()?.to_owned();
            let path = child.path().to_owned();
            let Ok(proxy) = AccessibleProxy::builder(conn)
                .destination(name.clone())
                .ok()?
                .path(path.clone())
                .ok()?
                .build()
                .await
            else {
                continue;
            };
            let editable = matches!(proxy.get_state().await, Ok(s) if s.contains(State::Editable));
            if editable {
                let role = proxy.get_role().await.unwrap_or(atspi::Role::Text);
                return Some((name.to_string(), path.into(), role));
            }
            if let Some(found) = Box::pin(find_editable(conn, &proxy, depth + 1)).await {
                return Some(found);
            }
        }
        None
    }

    async fn direct(app_substr: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("[direct] connecting to the AT-SPI accessibility bus...");
        let conn = AccessibilityConnection::new().await?;
        let c = conn.connection();

        let root = AccessibleProxy::builder(c)
            .destination("org.a11y.atspi.Registry")?
            .path("/org/a11y/atspi/accessible/root")?
            .build()
            .await?;

        // Find the target application by accessible name.
        let apps = root.get_children().await?;
        let mut target = None;
        for app in apps {
            let Some(name) = app.name() else { continue };
            let proxy = AccessibleProxy::builder(c)
                .destination(name.to_owned())?
                .path(app.path().to_owned())?
                .build()
                .await?;
            let title = proxy.name().await.unwrap_or_default();
            if std::env::var("KAYD_DEBUG").is_ok() {
                println!("[direct]   app on bus: {title:?}");
            }
            if title.contains(app_substr) {
                println!("[direct] found application: {title:?}");
                target = Some(proxy);
                break;
            }
        }
        let Some(app) = target else {
            return Err(format!("no application matching {app_substr:?} on the a11y bus").into());
        };

        let Some((name, path, role)) = find_editable(c, &app, 0).await else {
            return Err("no editable text field found in that application".into());
        };
        let kind = role_to_field_kind(role, true);
        println!("[direct] editable field: role={role:?} -> {kind:?}");
        if kind == FieldKind::Secure {
            println!("[direct] field is secure — refusing (non-negotiable #8).");
            return Ok(());
        }

        let text = TextProxy::builder(c)
            .destination(name.clone())?
            .path(path.clone())?
            .build()
            .await?;
        let before = text.character_count().await.unwrap_or(0);
        println!("[direct] field currently has {before} character(s).");

        let editable = EditableTextProxy::builder(c)
            .destination(name)?
            .path(path)?
            .build()
            .await?;
        let ok = editable
            .insert_text(before.max(0), MARKER, MARKER.chars().count() as i32)
            .await?;
        println!("[direct] insert_text returned {ok}");

        let after = text.character_count().await.unwrap_or(0);
        let readback = text.get_text(0, after).await.unwrap_or_default();
        if readback.contains(MARKER.trim()) {
            println!("[direct] readback: {readback:?}");
            println!("[direct] PASS — Kaydence inserted text into a live GNOME app via AT-SPI.");
            Ok(())
        } else {
            Err(format!("marker not found after insert; readback={readback:?}").into())
        }
    }

    async fn track() -> Result<(), Box<dyn std::error::Error>> {
        println!("[atspi-selftest] connecting to the AT-SPI accessibility bus...");
        let conn = AccessibilityConnection::new().await?;
        conn.register_event::<StateChangedEvent>().await?;
        println!("[atspi-selftest] connected. Focus a text field to inject; focus a");
        println!("[atspi-selftest] password field to see it refused. (Ctrl-C to stop.)");

        let events = conn.event_stream();
        futures_lite::pin!(events);
        let mut inserted = 0u32;

        while let Some(ev) = events.next().await {
            let Ok(atspi::Event::Object(atspi::events::ObjectEvents::StateChanged(sc))) = ev else {
                continue;
            };
            if std::env::var("KAYD_DEBUG").is_ok() {
                eprintln!("[event] state={:?} enabled={}", sc.state, sc.enabled);
            }
            // Only care about a field GAINING focus.
            if sc.state != State::Focused || !sc.enabled {
                continue;
            }
            let Some(name) = sc.item.name() else { continue };
            let path = sc.item.path();

            let acc = AccessibleProxy::builder(conn.connection())
                .destination(name.to_owned())?
                .path(path.to_owned())?
                .build()
                .await?;
            let Ok(role) = acc.get_role().await else {
                continue;
            };
            let editable = matches!(acc.get_state().await, Ok(s) if s.contains(State::Editable));
            let kind = role_to_field_kind(role, editable);
            println!("[focus] role={role:?} editable={editable} -> {kind:?}");

            match kind {
                FieldKind::Secure => {
                    println!(
                        "  REFUSED: secure/password field — not injecting (non-negotiable #8)."
                    );
                }
                FieldKind::Editable => {
                    let text = TextProxy::builder(conn.connection())
                        .destination(name.to_owned())?
                        .path(path.to_owned())?
                        .build()
                        .await?;
                    let caret = text.caret_offset().await.unwrap_or(0);
                    let editable_proxy = EditableTextProxy::builder(conn.connection())
                        .destination(name.to_owned())?
                        .path(path.to_owned())?
                        .build()
                        .await?;
                    match editable_proxy
                        .insert_text(caret, MARKER, MARKER.chars().count() as i32)
                        .await
                    {
                        Ok(_) => {
                            inserted += 1;
                            println!("  INSERTED {MARKER:?} via AT-SPI EditableText ({inserted}).");
                        }
                        Err(e) => println!("  insert failed: {e}"),
                    }
                }
                FieldKind::NoTarget | FieldKind::Unknown => {
                    println!("  no injectable target.");
                }
            }

            if inserted >= 5 {
                println!("[atspi-selftest] PASS — inserted into {inserted} field(s); stopping.");
                break;
            }
        }
        Ok(())
    }
}
