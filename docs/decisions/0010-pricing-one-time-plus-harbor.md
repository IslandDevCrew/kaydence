# ADR-0010: Pricing — one-time tiers + optional Harbor services

- **Status:** Accepted
- **Date:** 2026-07-07
- **PRD items affected:** PRD §6 (supersedes the pricing half of ADR-0008); enables Harbor references in P4 (Relay rendezvous, managed inference, hosted backup)

## Context
ADR-0008 set "free open-core + one-time Pro ~$59–79, no subscription." The v3
plan keeps that spine but the product now has real recurring *server* costs it
did not before: the Relay off-network **rendezvous** service, optional **managed
cloud inference** (BYOK-skip convenience), and **hosted encrypted backup**. A
pure one-time model would leave those features either unbuilt or dishonestly
bundled. Meanwhile the category's wound is subscription fatigue ($96–144/yr
incumbents), so *feature* subscriptions remain off the table. The rule that
resolves the tension: **features are one-time; only services with recurring
marginal cost may subscribe — and every service is self-hostable free.**

## Decision
Adopt **one-time feature tiers + an optional Harbor services subscription**, with
the invariant that Harbor funds servers and never gates capability. Four tiers:

| Tier | Price | Contains | Job |
|---|---|---|---|
| **Core** | **$0** · MIT | Local dictation (Raw/Light), global hotkey, history, single device | Trust + distribution engine (open-source credibility) |
| **Pro** | **$79 one-time** (launch $59) · per major version, free minors | Whisper-Ahead, per-app profiles, snippets, dictionary import, context reading, analytics, **Voiceprint** | Mass-market offer; pays for itself vs Wispr in ~6 months, then free forever |
| **Captain** | **$189 one-time** · lifetime-everything | Pro for **all** future majors + **Relay** + **Conductor** + priority support | Superfan ceiling; anchors $79 as the easy choice |
| **Harbor** | optional **~$6/mo or $60/yr** | Hosted services only: rendezvous relay (off-network sync), managed cloud inference, hosted encrypted backup | Honest recurring price for honest recurring cost; every service documented self-hostable free |

**The generating rule (binds future pricing decisions):** a capability the user's
own machine can perform is one-time; a service with ongoing marginal cost to *us*
(a server we run) may be a subscription, and must ship with free self-host
instructions so the subscription is convenience, never a lock.

## Alternatives considered
- **Feature subscription (industry default).** It *is* the fatigue we exist to
  counter; corrodes the privacy-posture story (recurring billing invites recurring
  data services). Rejected permanently.
- **One-time only, no services.** Clean, but leaves Relay-off-LAN, managed
  inference, and hosted backup unbuilt (no way to fund the servers). Rejected —
  it forfeits real capability.
- **Single $59–79 tier only (ADR-0008 as written).** Undershoots the superfan
  willingness-to-pay and gives Relay/Conductor no home. Superseded by the
  Core/Pro/Captain ladder here, which keeps the $79 mass entry *and* adds a
  ceiling without raising it.

## Consequences
- ADR-0008's pricing section is superseded by this ADR (naming decision in 0008
  stands). PRD §6 updated to the four-tier table.
- Feature gating is by one-time license tier (Core/Pro/Captain); **no feature ever
  sits behind Harbor.** Voiceprint ships in Pro; Relay + Conductor in Captain.
- Harbor requires billing + hosted-service infrastructure — but nothing in Harbor
  blocks the desktop app, and each Harbor service must have a working self-host
  path in-repo before it can be sold.
- Gemma 3 1B (prediction, Windows-mid default) carries **Gemma Terms**, not
  Apache — a **license review is required before any *paid* build bundles it**
  (ADR-0007). Download-on-demand only until cleared.
- License-key verification stays local/offline (no phone-home) to preserve the
  zero-account posture (non-negotiable #1).
