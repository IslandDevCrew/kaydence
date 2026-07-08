# ADR-0008: Naming (Kaydence) and pricing (one-time, no subscription)

- **Status:** Accepted
- **Date:** 2026-06-19
- **PRD items affected:** PRD §6

## Context
"Cadence" carries a hard software trademark conflict (Cadence Design Systems,
NASDAQ: CDNS) and every viable domain is taken (cadence.com/.app, get/use/try —
all registered). The category's subscription model (Wispr $144/yr, Aqua $96/yr,
Willow $144/yr; Cotypist trial → yearly) is the fatigue the product should reject.

## Decision
**Name:** Kaydence, primary domain `kaydence.io` (confirmed available; back with
`getkaydence.com`). A coined respelling of a given name — far cleaner mark than
Cadence (formal trademark search still required before launch). Brand string is
never hardcoded (`APP_NAME` in `settings/`), so the codename→name change is one
line. **Pricing:** free open-core (MIT) for local dictation + Raw/Light cleanup;
**one-time Pro license, ~$59–79** for Whisper-Ahead, profiles, snippets,
dictionary import, BYOK lanes, context reading, and the analytics dashboard.
No subscription. Optional paid major-version upgrade every couple years is the
only sustainability lever, used sparingly.

## Alternatives considered
- Keep Cadence: trademark + domain walls. Rejected.
- Coined .com names (voqal, dikta, saido, …): all taken. Rejected.
- Subscription pricing: the fatigue we exist to counter; one-time is itself a
  differentiator (Superwhisper/VoiceInk precedent). Rejected.
- Higher one-time price (~$189 considered): kept the lower $59–79 band to
  undercut yearly incumbents decisively (pays for itself vs Wispr in ~6 months).

## Consequences
Repo rebranded to Kaydence (Cadence retained only as historical codename note);
PRD §6 records the model; a formal trademark search is a pre-launch gate.
