# ADR-0009: Kaydence Relay — pairing and end-to-end crypto design

- **Status:** Proposed  <!-- CRITICAL DECISION PATH: requires operator design review before the P4-1 (Relay) build begins. Do not mark Accepted without that gate. -->
- **Date:** 2026-07-07
- **PRD items affected:** P4-6, P4-7 (Relay epic); refines non-negotiable #1 (privacy) for the multi-device case

## Context
Relay (Flagship 01) lets a user pair their Mac, Windows, and Linux machines and
have dictated text land where their focus is on another machine, plus sync their
dictionary, per-app profiles, Voiceprint artifact, and analytics — **without any
of it touching a third-party cloud**. That promise is only credible if the crypto
is real: the headline is "your text never touches anyone's cloud," so a weak or
hand-rolled scheme would be a privacy-line violation (non-negotiable #1), not a
feature gap. This is the single most security-sensitive surface in the product,
which is why the charter marks it a critical decision path (JUDGE-AUDITOR §3/§4)
and the plan halts for operator review before build.

Two transport situations exist:
1. **On-LAN** — both devices on the same local network. Primary path; target
   ≤500 ms added latency (P4-2 gate).
2. **Off-network** — devices on different networks. Optional, via a **rendezvous
   relay** that only forwards opaque ciphertext and is **self-hostable for free**
   (Harbor hosts one for convenience; ADR-0010). The relay never holds keys and
   never sees plaintext.

## Decision
Adopt a **pairing-based, device-to-device end-to-end encrypted** design. Stated
in one sentence: *devices form a trust group by out-of-band short-code pairing,
each holds a long-lived identity keypair, and all Relay traffic is E2E encrypted
with per-session keys via an authenticated Noise handshake — the rendezvous relay,
when used, is a blind ciphertext forwarder.* Detail:

**Identity.** Each device generates an Ed25519 identity keypair on first run,
private key stored in the OS keychain/secure enclave (Keychain / DPAPI-backed
store / kernel keyring), never on disk in plaintext, never transmitted.

**Discovery.** On-LAN via mDNS/DNS-SD (`_kaydence._tcp`), advertising only the
device's public identity fingerprint and a port — no user data in the broadcast.

**Pairing (trust establishment).** Initiator shows a **6-word / short numeric
code** derived from a hash of both public keys plus a session nonce (SAS —
short authentication string). The user confirms the same code on the second
device. This authenticates the key exchange against MITM. Confirmed peers are
added to a local **trust group**: a signed roster of device public keys, itself
gossiped and CRDT-merged so every device converges on the same membership.

**Session crypto.** Transport uses the **Noise Protocol Framework** (Noise_XX
pattern: mutual authentication, forward secrecy, identity hiding) over TCP/QUIC.
XX gives each side proof of the other's static key (checked against the trust
roster) and a fresh ephemeral session key per connection. AEAD = ChaCha20-Poly1305.

**Sync payloads.** Dictionary / profiles / Voiceprint / analytics sync as
**CRDTs** (per plan) so concurrent edits on multiple devices merge without
conflict; payloads are encrypted end-to-end and only decrypted on member devices.
The Voiceprint model artifact travels as an opaque encrypted blob.

**Rendezvous (off-network).** A minimal relay matches two devices by an ephemeral
rendezvous token and forwards Noise ciphertext frames. It is authenticated only
enough to prevent abuse; it **cannot decrypt** (no keys) and stores nothing.
Self-hostable; a Harbor-hosted instance is a paid convenience, never a capability
gate (ADR-0010).

**Key rotation & revocation.** Identity keys rotate on user command or schedule;
rotation is a signed roster update. **Device revocation** removes a device's key
from the roster (signed by a remaining trusted device) and forces re-pairing;
revoked devices can no longer complete the Noise handshake against the group.

**Safety invariants (bind the build):**
- No Relay traffic is ever sent unencrypted, on-LAN or off.
- The rendezvous relay never receives a key or plaintext; a self-hosted relay is
  documented and buildable from the repo.
- Pairing requires explicit human confirmation of the SAS on both devices.
- `scripts/audit-network.sh` must show Relay egress goes only to paired peers or
  the user-configured rendezvous endpoint — nothing else.

## Alternatives considered
- **Account + server-brokered keys (the incumbent model).** Simplest to ship,
  but reintroduces the cloud the product exists to reject; a server that brokers
  keys can be compelled. Rejected — it breaks the headline promise.
- **Hand-rolled crypto / raw TLS with a shared PSK.** A single shared secret has
  no per-device revocation and no forward secrecy; hand-rolled AEAD framing is a
  liability. Rejected in favor of vetted Noise + libsodium primitives.
- **QR-code-only pairing.** Good UX on phones, poor across three desktops without
  cameras. SAS short-code works everywhere; QR can be an additive convenience.
- **Gossip trust with TOFU, no SAS.** Trust-on-first-use is MITM-able on a hostile
  LAN. The SAS confirmation closes that hole for a small UX cost.

## Consequences
- New `relay/` module (transport, pairing, roster, rendezvous client) and a
  vetted crypto dependency set (a Noise implementation + libsodium/`ring`-class
  primitives) — each a critical-path dependency add (JUDGE-AUDITOR §2).
- New network surface → `audit-network.sh` allowlist entries + tests asserting no
  plaintext egress and no traffic to unpaired peers.
- CRDT sync engine shared with Voiceprint/profiles; conflict semantics need their
  own tests.
- A self-hostable rendezvous reference implementation must ship (proof of the
  free self-host guarantee).
- **This ADR stays Proposed until the operator reviews the pairing/crypto design;
  the P4-1 build must not start before that gate.**
