# Catalog licensing: questions for legal review

Status: DRAFT for the owner to send. Prepared 2026-11-18. **Not legal advice, and nothing here passes gate GT-LIC.** GT-LIC passes only when a named human records written, release-specific approval in [registers.md](registers.md). Background and sources: [spikes/catalog/FINDINGS.md](../spikes/catalog/FINDINGS.md), [spikes/catalog/sources.md](../spikes/catalog/sources.md).

## Context to give the reviewer

MedTracker is a standalone, offline medication log for iOS and Android, distributed in the U.S. only (D-31). It has no accounts, no ads and no analytics, and it makes no clinical claims: it never advises on doses or interactions. We want to bundle a searchable medication-name list so users can find a medication by typing a few letters. Manual entry always remains available.

What we plan to ship: a compact SQLite file built offline from NLM's **RxTerms** monthly release (brand and generic names, strength, dose form, route), with NLM's **RxNorm Current Prescribable Content** as the provenance baseline. We exclude the full RxNorm release, RxClass and SNOMED content, NDC package rows, DailyMed label text and the NLM logo. The app would show the source release in an About screen, along with NLM's acknowledgment text and a no-endorsement statement.

## Questions

1. **Repackaging in a store app.** NLM's RxNorm terms say its created names and codes are public domain and may be distributed with acknowledgment to NLM. No NLM page we could fetch addresses app stores. May we redistribute a derived, searchable SQLite file built from RxTerms and Current Prescribable Content inside an app distributed through the Apple App Store and Google Play, whether free or paid? (Sources: RxNorm terms of service; RxNorm files page; Current Prescribable Content page.)
2. **RxTerms terms.** The current RxTerms download page says "free to use" but states no license, no attribution wording and no registration. An archived 2009 page mentioned registration. What terms govern RxTerms today? Is registration required? Which attribution wording applies: the RxNorm terms of service text or the RxNav/API terms text? (Sources: RxTerms page; RxNav terms of service.)
3. **Third-party rights in MTHSPL-derived names.** Current Prescribable Content includes MTHSPL source data. Do names derived from it carry any third-party rights or restrictions?
4. **Form of acknowledgment.** Is an About screen, plus the store listing text, enough to satisfy "acknowledgment to NLM"? NLM asks that we not use its name or logo to imply endorsement. Does the sentence "This product uses publicly available data courtesy of the U.S. National Library of Medicine (NLM)" conflict with that, and does it need the no-endorsement sentence beside it?
5. **Keeping the data current.** NLM's terms ask redistributors to keep data current or disclose that it may not reflect the latest release. Are an in-app release label ("Catalog release 2026-11") and a "may be incomplete or out of date" notice enough? What update cadence, if any, do we commit to?
6. **Trademarks and wording.** The list includes brand names such as Repatha, which are third-party trademarks. Is showing them to identify products acceptable? Is our proposed copy acceptable: "Names come from a public U.S. reference list and may be incomplete or out of date. Listing a product does not mean it is recommended or suitable for you. This app does not give medical advice."?

## Also ask, if the reviewer is willing

- Whether store health-app disclosures (Apple privacy labels, Google Play data-safety form) are affected by a bundled medication list. The app collects no data and sends nothing off the device.
- Whether the NIH Dietary Supplement Label Database may be bundled the same way. We could not read its terms (the fetch returned 403 or empty), so this is post-MVP and unverified.

## What we will hand over

The 10-item verification checklist in [FINDINGS.md](../spikes/catalog/FINDINGS.md) (section 5), the source list with fetch dates, and the build manifest (source release, URL, checksum, row counts, filter rules) once the pipeline exists.

## Where to send NLM-specific questions

Questions 2 and 3 are NLM's to answer. The RxNorm terms and RxTerms pages point to NLM's support channels; use the contact route listed on the current NLM page rather than the one noted here, since we have not verified the current address.
