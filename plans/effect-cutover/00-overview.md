---
plan: effect-cutover
title: "Effect Cutover — Migrate Legacy Event Consumers to Effect"
status: not-started
---

# Effect Cutover

## Context

Section 03 of the spec-conformance plan introduced the `Effect`/`EffectSink` system and migrated all VTE handler emission sites to emit `Effect` instead of `Event`. The `LegacyEventSink` adapter bridges `Effect` → `Event` for backward compatibility. The deprecated `Event::ClipboardLoad` and `Event::ColorRequest` variants carry closures created by the adapter.

This plan completes the migration by moving each legacy consumer from `Event` subscriptions to direct `Effect` subscriptions, then deleting the adapter and deprecated variants.

## Goal

- Migrate `oriterm_mux` IO thread from `LegacyEventSink` → `QueueingEffectSink`
- Migrate `oriterm` application from `Event`-based dispatch → `Effect`-based dispatch
- Delete `LegacyEventSink`, `Event::ClipboardLoad`, `Event::ColorRequest`
- Remove the `drain_notifications()` thin shim
- All consumers process effects via `drain_into()` — no separate notification drain

## Dependency

Requires spec-conformance Section 03 complete (Effect types, EffectSink, LegacyEventSink exist and are stable).
