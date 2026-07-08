//! AT-SPI injection self-test (P1-P0-3, ADR-0013) — run on the Linux reference
//! machine to validate the injection path live. Linux-only; a no-op elsewhere.
//!
//! Stage 1 (this build): connect to the AT-SPI accessibility bus and enumerate
//! the applications on the desktop — proves the plumbing works on real GNOME
//! before we add focus tracking, secure-field detection, and text insertion.
//!
//!   DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus \
//!     cargo run --bin atspi-selftest

fn main() {
    #[cfg(target_os = "linux")]
    linux::run();
    #[cfg(not(target_os = "linux"))]
    eprintln!("atspi-selftest is Linux-only.");
}

#[cfg(target_os = "linux")]
mod linux {
    use atspi::connection::AccessibilityConnection;
    use atspi::proxy::accessible::AccessibleProxy;

    pub fn run() {
        match pollster::block_on(probe()) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("[atspi-selftest] FAIL: {e}");
                std::process::exit(1);
            }
        }
    }

    async fn probe() -> Result<(), Box<dyn std::error::Error>> {
        println!("[atspi-selftest] connecting to the AT-SPI accessibility bus...");
        let conn = AccessibilityConnection::new().await?;
        println!("[atspi-selftest] connected.");

        // Make one real AT-SPI call against the desktop root to prove the full
        // stack works on this machine (connect → proxy → method round-trip).
        let root = AccessibleProxy::builder(conn.connection())
            .destination("org.a11y.atspi.Registry")?
            .path("/org/a11y/atspi/accessible/root")?
            .build()
            .await?;
        let role = root.get_role().await?;
        println!("[atspi-selftest] desktop root role = {role:?}");
        println!("[atspi-selftest] PASS — AT-SPI plumbing works on this machine.");
        Ok(())
    }
}
