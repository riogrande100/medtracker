# Sigil: Product Spec (draft v0.2)

**Status:** draft. Owner decisions recorded 2026-11-20 (section 17). Working name **Sigil** (formerly MedTracker). Not yet reconciled with `product-spec.md`, `backlog.md` or `registers.md`; those describe the medication app and are historical until this direction is formally adopted.
**Scope decision recorded here:** no MCP server or any other AI-chat access in this version. Data leaves the phone only by explicit user export.

## 1. Purpose

A private, local-first mobile journal for tracking anything. The user defines what to track, logs entries as text, photo, audio (with transcript) or a quick structured value, and later sees trends. It is free, with no payments, ads, accounts or analytics, but it is **planned for public distribution** on the App Store and Google Play, so the store, privacy and legal requirements in section 10 apply from the start.

It is deliberately **not** a medical, health, or medication product. That framing removes the FDA/health-data and drug-catalog licensing work that blocked the medication tracker.

## 2. Non-goals

- No medical claims, diagnosis, dosing, interactions, or advice. No drug catalog.
- No HealthKit / Health Connect integration.
- No MCP server, no remote API, no AI-chat connector.
- No user accounts, no server, no cloud sync run by us. (Optional OS-level backup is allowed; see section 9.)
- No ads, in-app purchases, subscriptions, or third-party analytics/SDKs.
- No social features or sharing feed.
- No localization in v1 (U.S. English), per existing decision D-31.

## 3. Principles

1. **Local-first.** Everything works offline. Data lives in an on-device SQLite database plus a media folder.
2. **You own your data.** Full export at any time in open formats.
3. **Fast capture.** Logging an entry takes at most two taps from launch for a saved tracker.
4. **Descriptive, not interpretive.** Reports state what was logged ("12 entries this week"), never what it means.
5. **Private by default.** Optional app lock; no network access in v1 except where section 6.3 says otherwise.

## 4. Core concepts

| Concept | Meaning |
|---|---|
| **Tracker** | A named thing the user tracks (e.g. "Sleep", "Garden", "Mood", "Gym"). Has a type (section 5), optional color/icon, and archived state. |
| **Entry** | One timestamped record under a tracker. May carry a value, text, and any number of attachments. |
| **Attachment** | A photo or audio clip belonging to an entry. Stored as a file; metadata in SQLite. |
| **Transcript** | Text produced from an audio attachment. Editable by the user; the original machine text is kept. |
| **Tag** | Free-form label on an entry, shared across trackers. |
| **Note-only tracker** | A tracker with no structured value: a plain journal. |

## 5. Tracker types

Each tracker has exactly one value type. Every entry may additionally have free text and attachments.

| Type | Value | Trend views |
|---|---|---|
| Event | none (timestamp only) | count per day/week/month, streaks, time-of-day distribution |
| Count | integer | sum/average per period |
| Rating | 1-5 or 1-10 (configured) | average over time, distribution |
| Yes/No | boolean | rate per period, streaks |
| Duration | seconds | total/average per period |
| Number | decimal plus optional unit label | line chart, min/max/average |
| Choice | one of user-defined options | frequency by option |
| Text only | none | keyword frequency, entry count |

Changing a tracker's type after entries exist is not allowed. The user may archive it and create a new one.

## 6. Capture

### 6.1 Entry creation
- Home screen lists trackers; tapping one opens quick-log (value + optional note). Long-press or "+" opens the full editor (text, photo, audio, tags, time).
- Default timestamp is now; the user can backdate. Entries store UTC instant plus the original local time and UTC offset, reusing the existing clock model. Zone handling uses the bundled tz database (`jiff`), fixing the known DST finding (B1-04/B1-05) rather than carrying it over.
- Editing an entry creates a revision (before/after), reusing the existing revision pattern. "Delete" is a privacy action that erases content and media and leaves only a tombstone; "undo" restores a recently deleted entry within a short grace window.

### 6.2 Photos and audio
- Photos: camera or photo library via a system picker (limited-library access is enough; request the minimum permission). Stored in the app container, downscaled to a configurable max edge (default 2048 px), EXIF location stripped by default.
- Audio: in-app recorder with pause/resume, saved as AAC/M4A. Maximum length per clip is configurable (default 10 minutes).
- Each entry may have several attachments. Deleting an entry deletes its files.

### 6.3 Transcription
- **Decided: use the platform speech recognizers, in on-device mode only.** iOS: the Speech framework with on-device recognition required. Android: the platform's on-device recognizer API. The app must never use a mode that sends audio to Apple or Google servers; that is what keeps the "Data Not Collected" label accurate. A bundled model is not used.
- **Availability varies** by OS version, device and language (U.S. English first). The Phase 0 spike must confirm, at the minimum supported OS on each platform: (a) the recognizer reports on-device support, (b) recognition works with the network disabled, and (c) whether it can transcribe a saved audio file or only live microphone input. If a platform is live-only, transcription runs during recording and the audio is saved alongside it.
- Transcription runs after recording, in the background; entries show state: pending, done, failed, unavailable for this language.
- The user can edit the transcript; the raw machine output is retained so edits are reversible.
- Cloud transcription is **out of scope for v1**. If ever added it must be opt-in per recording, disclose the provider, and ship with an updated privacy policy and App Store privacy labels.
- If on-device recognition is unavailable for a device/language, the audio is still saved and the UI says so; nothing falls back to a network service silently. Platform recognizers may also impose duration limits per request, so long recordings are transcribed in chunks.

## 7. Trends and reports

All computation is local and descriptive.

- **Per-tracker view:** chart appropriate to type (section 5), selectable range (7d, 30d, 90d, 1y, custom), and period grouping (day/week/month) using the user's local day boundaries.
- **Calendar heatmap** of entry counts per day, tapping a day opens its entries.
- **Streaks and gaps:** current/longest streak, days since last entry, labeled as counts, with no judgment copy.
- **Cross-tracker view (v1.1):** overlay two trackers on one timeline to eyeball patterns. No correlation claims or causal language.
- **Search:** full-text across notes and transcripts, filterable by tracker, tag, date, and attachment type.
- **Text trends:** top keywords and entry frequency from notes/transcripts, computed locally, with an editable stop-word list. Optional on-device summarization is a stretch goal, not v1.
- **Export from any report** as CSV or image, via the OS share sheet (user-initiated).
- Every report states its definitions (what counts as an entry, how days are grouped, time zone used), following the existing report "definitions" convention.

## 8. Reminders

- Optional per-tracker reminders: daily at a set local time, or specific weekdays. Local notifications only.
- Notification text is generic ("Time to log: <tracker name>"). Tracker names may be sensitive, so provide a setting to show "Time to log an entry" instead.
- Not required for the first usable build; the current notification module is a stub and needs real device work (iOS elapsed-time triggers and Android exact-alarm limits noted in `MOBILE.md` still apply).

## 9. Data, backup, privacy

- **Storage:** SQLite (migrations as in the existing `storage/` module) plus a media directory excluded from device-wide caches.
- **App lock:** optional biometric/passcode gate on launch and resume.
- **Backup/restore:** reuse the existing encrypted backup format (Argon2 KDF + ChaCha20-Poly1305), extended to include media files in a single archive. Restore validates integrity and keeps a pre-restore copy, as today.
- **Export:** JSON (full fidelity: trackers, entries, revisions, tag links, transcript metadata) plus CSV per tracker plus a media folder. Round-trip import of the app's own JSON export is in scope.
- **OS backup (decided: allowed):** the app's data may be included in iCloud device backup (iOS) and Google Auto Backup (Android). This is the platform's backup of the device, not an app-run sync service, so the app still collects nothing. Provide a user setting to exclude media from OS backup. Android Auto Backup has a per-app size limit (documented at about 25 MB; verify), so the database is always included and media may need to be excluded or left to the encrypted export. Restore from OS backup must be tested on both platforms, including a database with its media files missing.
- **Cloud storage of exports:** the share sheet can save an export or encrypted backup to iCloud Drive or Google Drive, user-initiated.
- **Network:** the runtime allowlist stays empty (existing PR-23/D-27 model). No SDKs that phone home.
- **Privacy policy:** short and truthful: data stays on device; no collection; transcription is on-device; exports are user-initiated. App Store privacy labels: "Data Not Collected."

## 10. Legal and store posture

- **Positioning:** a general-purpose personal tracker/journal. Store listing, screenshots, in-app copy and default tracker templates must not mention medications, symptoms, conditions, diagnosis or treatment, and must not present health outcomes.
- **Templates:** ship neutral examples only (habits, hobbies, chores, mood as a plain rating). Avoid health-flavored defaults (blood pressure, glucose, doses).
- **Users may log anything**, including health items, but the app never interprets it. Keep that stated in the terms.
- **Permissions strings:** camera, microphone, photos, speech recognition, notifications, biometrics, each with a plain-language purpose.
- **Other people in media:** terms note the user is responsible for consent when recording or photographing others.
- **Open-source/licensing:** audit third-party model and library licenses (speech model, chart library, fonts) before release.
- **Public distribution checklist (decided 2026-11-20: public release is planned):**
  - Short legal review of the privacy policy, terms, and store copy before the first public release.
  - Privacy policy hosted at a public URL (both stores require one), matching the App Store privacy labels and the Google Play Data safety form. Both should say no data is collected, which holds only while transcription stays on-device and the network allowlist stays empty.
  - Apple export-compliance declaration: the backup feature uses encryption (Argon2 and ChaCha20-Poly1305), so answer the encryption questions accurately.
  - Developer accounts and signing for both stores (GT-SIGN, GT-STORE). Google Play may require closed testing before production for new personal developer accounts; verify current policy.
  - Age rating questionnaires, support contact, and open-source license notices for bundled libraries and fonts.
  - Trademark and store-name clearance for the app name (section 17).
  - Territories: U.S.-only unless decided otherwise (D-31 and section 17).
- This section is a posture, not legal advice.

## 11. Platform and architecture

**Decided (2026-11-20): Tauri 2 with a web view (React/TypeScript) on iOS and Android.** Domain logic, storage, backup and trend queries stay in the Rust core; the UI stays in the web view, as in the current codebase.

Native capabilities the web view cannot provide come from Tauri mobile plugins (Swift on iOS, Kotlin on Android), exposed to the UI only through narrow Rust commands. The current capability file grants the web view no plugin permissions; keep it that way.

| Capability | Planned source | Notes |
|---|---|---|
| Audio recording | Custom plugin (platform recorder APIs) | Microphone permission; AAC/M4A output |
| Photos and camera | Evaluate the dialog plugin's picker first; otherwise a custom plugin | Minimum permission; downscale and EXIF strip in Rust |
| Speech recognition | Custom plugin over the platform recognizers | On-device only (section 6.3) |
| Biometric app lock | Official Tauri biometric plugin (check it is maintained) | |
| Local notifications | Official notification plugin | OS limits noted in `MOBILE.md` |
| Export/backup save | Existing `transfer.rs` (dialog and fs, used only from Rust) | |

Charts are drawn in the web view with a small client-side library (audit license and size) and always offer a data-table alternative.

Risks specific to this choice: Tauri mobile support is young; nothing has run on a simulator or device yet; the Android toolchain is not installed (B0-10); and the audio, photo and speech plugins are custom native code with no existing reference in the repo. Phase 0 exists to retire these before feature work.

Invariants: domain and storage code free of UI concerns; schema in versioned SQL migrations; the empty network allowlist and strict CSP (no remote script, font or image) remain tested invariants.

## 12. Data model (v1 sketch)

```
tracker(id, name, type, config_json, color, icon, status[active|archived], created_at_utc, updated_at_utc)
entry(id, tracker_id, occurred_at_utc, entered_at_utc, original_local, original_offset_seconds,
      value_num, value_text, note, current_revision, deleted)
entry_revision(id, entry_id, revision_number, changed_at_utc, reason[correction|undo|delete], before_value, after_value)
attachment(id, entry_id, kind[photo|audio], rel_path, bytes, duration_ms, width, height, created_at_utc)
transcript(id, attachment_id, language, engine, raw_text, edited_text, status[pending|done|failed|unavailable], created_at_utc)
tag(id, name)  entry_tag(entry_id, tag_id)
reminder(id, tracker_id, local_time, weekdays_mask, enabled)
```

Full-text index over `entry.note` and transcript text (SQLite FTS5). Entries are never hard-deleted except by the privacy-delete action, which nulls content, removes files, and keeps a tombstone row.

## 13. Reuse from the medication codebase

| Keep | Adapt | Drop |
|---|---|---|
| UI primitives, tabs, dialogs, Tailwind/Radix setup, migration runner, encrypted backup, time/zone engine, file transfer, export plumbing | `dose_event` + revisions into `entry` + revisions; ledger undo/delete into entry undo/privacy delete; reports module into new trend queries | medication, schedule and occurrence tables and code, Today view, catalog and legal-gate docs, HealthKit spike, `spikes/` |

## 14. Phases

- **Phase 0 - rename and device spikes:** rename the app to Sigil (done in config, see section 17 for the bundle id). Install the Android toolchain (B0-10) and get iOS and Android builds running. On a real device per platform: record audio, transcribe on-device with the network off, pick a photo, fire a local notification, and restore an OS backup that includes media. Exit: one evidence artifact per platform and capability.
- **Phase 1 - core journal:** trackers (all types), entries (text + value), history list, edit/undo/privacy-delete, search, JSON/CSV export, optional app lock.
- **Phase 2 - media:** photo and audio capture, attachments in entries, transcription, backup including media.
- **Phase 3 - trends:** per-tracker charts, calendar heatmap, streaks, text keyword trends.
- **Phase 4 - polish:** reminders, cross-tracker overlay, accessibility pass.
- **Phase 5 - public release:** legal review, privacy policy URL, labels and Data safety form, export compliance, signing and store accounts, closed testing, store listing, name clearance.

## 15. Acceptance criteria (v1)

1. Create a tracker of each type; log entries offline; data persists across app restarts.
2. On a real iOS device and a real Android device with airplane mode on, record audio and get a transcript with no network use (verified by network capture and the allowlist test).
3. Photos and audio survive encrypted backup, restore on a clean install, and export; OS backup restore (iCloud and Android Auto Backup) works, including when media is excluded.
4. Trend charts match a hand-computed fixture dataset, including across a DST change and a travel/zone change.
5. Privacy-delete removes the entry's text, transcript, and media files from disk, verified by test.
6. Accessibility: screen-reader labels and Dynamic Type on all primary screens; charts have a table alternative.
7. No network requests at runtime; no third-party SDKs; "Data Not Collected" label is accurate.
8. Release readiness: privacy policy URL live, store privacy disclosures complete, export-compliance answered, legal review done, name cleared.

## 16. Risks

| Risk | Mitigation |
|---|---|
| On-device transcription quality or language gaps | Spike first; always keep the audio; show state clearly |
| Media bloats backups and storage | Downscale photos, cap audio length, show storage usage, allow media-free export |
| Tauri native-plugin effort higher than expected | Phase 0 spike; fall back to native iOS |
| Users log health data despite positioning | Neutral copy, no interpretation, terms; revisit posture if it changes |
| Generic scope creep ("track anything") | Fixed tracker types in v1; no custom formulas or scripting |

## 17. Decisions and remaining questions

### Decisions (owner, 2026-11-20)

| # | Decision |
|---|---|
| T-1 | Platforms: iOS and Android. |
| T-2 | Stack: Tauri 2 with a web view; custom native plugins for audio, photos and speech. |
| T-3 | Transcription: platform speech recognition, on-device only, no cloud fallback. |
| T-4 | Backup: iCloud and Google (OS-level) backup allowed, plus encrypted export. |
| T-5 | Distribution: public release planned, with a legal review before the first release. |
| T-6 | Name: **Sigil** (replaces MedTracker). |

### Remaining questions

1. **Bundle identifier.** Config currently uses a placeholder (`app.sigil.journal`). Replace it with a reverse-domain you control before any signing; it cannot change after a store release.
2. **Name clearance.** "Sigil" is a common word and is already used by existing software (for example an EPUB editor). Search the App Store, Google Play, and trademark databases before committing to branding or a store listing.
3. **Territories.** U.S.-only (D-31) or worldwide? Worldwide adds localization and regional privacy-law considerations even without data collection.
4. **Minimum OS versions.** On-device recognition support depends on OS version; set minimums after the Phase 0 spike.
5. **Who does the legal review** and when (before Phase 5 at the latest, ideally before Phase 2 since it covers audio and photos).
