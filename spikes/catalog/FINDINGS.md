# Catalog source spike (B0-03 input) - FINDINGS

Date: 2026-11-18. Status: research only. **GT-LIC is NOT PASSED; this document does not pass it.** Nothing here is legal advice; items marked LAWYER need counsel.
Labels: DOCUMENTED = stated on a fetched page (see sources.md, # refs); UNVERIFIED = inferred, not found, or fetch failed; MEASURED = observed locally in RxTerms202611.

## 1. Candidate sources

| Source | Brand + generic | Strength / form / route | OTC / supplements | License to download |
|---|---|---|---|---|
| RxNorm Full (monthly/weekly) | Yes (SBD/SCD, BN, IN) | Strength+form in normalized names; route via dose-form groups | Some OTC; **dietary supplements out of scope** ("do not receive RxNorm normalized names") DOCUMENTED #4 | **UMLS license required** DOCUMENTED #1,#2. Contains source vocabularies with SRL 1-4 restrictions and SNOMED (SRL 9) #4 |
| RxNorm Current Prescribable Content (CPC) | Yes | Same | Prescription approximation "also includes many OTC"; no supplements | **No license; "public domain"**; only SAB=RXNORM and MTHSPL (no FDB/Micromedex/VA) DOCUMENTED #2,#3. Page calls it "draft" and subject to change. Zip is 75,032,750 bytes (#13) |
| RxTerms | Yes (DISPLAY_NAME generic or brand + route) | STRENGTH, NEW_DOSE_FORM, ROUTE fields; Ingredients file | Derived only from non-proprietary RxNorm content (search snippet; UNVERIFIED on primary page); same OTC/supplement scope as RxNorm | Page says "Free to use"; states no licence/attribution/registration text (#5). An archived 2009 NLM page asked for registration on first download (UNVERIFIED current). Release notes contain no licence text (#14). Zip 2.8 MB, monthly |
| FDA NDC Directory / openFDA | Brand + generic name, labeler | Strength, dosage form, route per product | Prescription and OTC finished drugs; excludes kits, animal, blood products; **not complete**; labeler-reported, no FDA verification DOCUMENTED #10 | openFDA content CC0 unless marked; attribution requested not required #9. Package-level (NDC) rows are far larger than needed |
| DailyMed (SPL labels) | Yes | In label XML | RRx + OTC labels; not complete listing | Page states no reuse licence; copyright page not read - UNVERIFIED #11. Label text is large and noisy; not needed for a name index |
| NIH DSLD | Product/brand names | Ingredient amounts per serving; form varies | **Supplements** (the only candidate for vitamins/supplements) | UNVERIFIED: DSLD pages unreadable by the fetch tool; NIH-wide policy (#8) says US gov works not copyrighted but label images/text may be third-party. Needs direct review |

## 2. Redistribution (DOCUMENTED unless noted)

- NLM-created names and codes (incl. RXCUI) are public domain and may be distributed "with acknowledgment to NLM"; source-vocabulary proprietary data has separate restrictions (#1).
- CPC is offered with no licensing restrictions and excludes non-RXNORM/MTHSPL sources (#3). This is the cleanest path. RxTerms is derived from the non-proprietary portion (UNVERIFIED on its own page).
- Attribution requested: "This product uses publicly available data courtesy of the U.S. National Library of Medicine (NLM)" plus NLM does not endorse (#1); API-terms variant: "NLM is not responsible for the product and does not endorse or recommend this or any other product." (#7). Cite Nelson et al., JAMIA 2011 (#1).
- Prohibited: using NLM name/logo with the app; implying NLM endorsement (#1,#7,#8). No medical-advice claim (#1).
- Update obligation: redistributor must keep data current **or clearly disclose it may not reflect the latest NLM data** (#1). A bundled catalog therefore needs a visible "catalog release YYYY-MM" notice.
- No click-through for CPC (#3). Full release needs a UMLS licence (login, click-through, and per-vocab Section 12 terms) - avoid.
- MTHSPL atoms originate from FDA SPL (US gov); status as pure public domain is UNVERIFIED beyond NLM's "no licensing restrictions" statement.
- Store distribution (Apple/Google) of a derived SQLite file: no NLM page addresses app-store distribution specifically. UNVERIFIED. LAWYER.
- Trademark: brand names (Repatha etc.) are third-party marks; used nominatively to identify products. LAWYER (not a source-license matter).

## 3. Practical build

**Counts (MEASURED, RxTerms202611, post-filter of suppressed/retired per publisher advice):** 21,270 rows: SCD 11,839, SBD 8,091, GPCK 623, BPCK 717 (release notes quote SCD 10,854 / SBD 8,082 excl. suppressed - filter on SUPPRESS_FOR/IS_RETIRED before counting). Route groups: Oral Pill 7,665, Injectable 5,728, Topical 2,176, Pack 1,340, Oral Liquid 1,029. Spot checks: "Repatha" and "evolocumab" present (6 rows), metformin present, "vitamin D" only as combination Rx/OTC products with cholecalciferol (151 rows with "vitamin d" text), standalone supplements largely absent.

**Size estimate (UNVERIFIED, extrapolated):** RxTerms txt 6.4 MB raw for 21k rows. A compact index (rxcui, tty, display, brand, ingredient, strength, form, route; plus FTS5 external-content index with prefix tokens) is about 5-9 MB on disk, ~2-3 MB compressed inside an IPA/AAB. Adding CPC synonyms/NDC would grow it several-fold: not recommended. Measure once built.

**Schema sketch** (matches spec section 281 naming; read-only `catalog.sqlite`):
```
catalog_meta(source, source_release, source_file, source_sha256, built_at_utc, schema_version, attribution_text, manifest_hash)
catalog_drug(rxcui INTEGER PK, tty, display_name, full_name, brand_name, ingredient, strength, dose_form, route, source_release)
catalog_synonym(rxcui, term, term_type, source)   -- e.g. RxTerms DISPLAY_NAME_SYNONYM
catalog_fts USING fts5(display_name, brand_name, ingredient, synonyms, tokenize='unicode61 remove_diacritics 2', prefix='2 3 4', content='catalog_drug')
```
Query: prefix match on each typed token (`repat*`), rank by bm25 with brand/ingredient boost and tty preference; limit 25.

**Cadence:** RxNorm/CPC and RxTerms monthly (first Monday-ish; RxTerms202611 dated Oct 27); weekly updates exist for CPC. App releases ship a fixed catalog; for a standalone offline app, rebuild with each app release, record release ID, show it in Settings.

**Offline reproducible pipeline** (build step is the only networked step; the repo never contains raw downloads):
1. Download pinned `RxTerms<YYYYMM>.zip` (and optionally CPC zip + published MD5) into an untracked work dir.
2. Verify MD5/SHA-256; write the checksum to `source.lock` (URL, release, bytes, sha256, fetch date).
3. Filter: drop SUPPRESS_FOR non-null, IS_RETIRED non-null; keep TTY in SCD, SBD (decide on GPCK/BPCK); derive ingredient from Ingredients file; normalize strength text (display only).
4. Build SQLite + FTS5 deterministically (sorted inserts, fixed `user_version`, VACUUM); write catalog_meta incl. source sha256.
5. Checksum the output `catalog.sqlite`; assert row counts within +/-5% of prior release and run golden searches (repatha, metformin, vitamin d, tylenol, lisinopril).
6. Version `catalog-YYYYMM-bN`; emit manifest (JSON) of source + output checksums for the legal evidence pack.

## 4. Risks / UX

- Duplicates/noise: SCD (generic) vs SBD (brand) share strength/form; show brand first when matched on a brand token, group by ingredient; hide packs by default; many strengths per drug - choose strength as a second step.
- Long RxNorm names; use DISPLAY_NAME (e.g. "Repatha (Injectable)") plus strength, not FULL_NAME, for list rows.
- Combination products match on every component; rank exact-name matches first.
- Typos/unknown meds: always show a persistent "Add '<typed text>' manually" row at the top or bottom; manual entry stays complete (spec PR-25).
- Coverage gap: supplements and many OTC items (vitamin D alone) are not in RxNorm; do not imply the list is complete. Copy: "Names come from a public U.S. reference list and may be incomplete or out of date. Listing a product does not mean it is recommended or suitable for you. This app does not give medical advice." Also the NLM disclaimer text (#1/#7). Never use NLM logo.
- Strength shown as product strength, never used to compute doses.

## 5. Recommendation (MVP, US only)

- **Use RxTerms (monthly) as the primary display/search source, with CPC (SAB RXNORM + MTHSPL, no licence) as the provenance baseline / RXCUI cross-check.** Both are NLM-published and licence-free per NLM pages; RxTerms is a smaller, search-ready derivative. Avoid the full RxNorm and UMLS entirely.
- **Exclude:** full RxNorm / any SRL 1-4 or SNOMED content, NDC/package rows, DailyMed label text, classes (RxClass includes SNOMED), DSLD until reviewed, NLM logo.
- OTC/supplements: ship what RxNorm covers; label as non-exhaustive; DSLD is a post-MVP candidate pending terms review; manual entry covers the rest.

### GT-LIC verification checklist (owner: legal review; mark PASSED only by named human)
1. Counsel confirms CPC (and RxTerms if used) may be redistributed inside a derived SQLite file in a free or commercial iOS/Android app; record the NLM pages and fetch dates (sources.md) in the evidence pack.
2. Confirm RxTerms redistribution terms from a primary NLM statement (current download page lacks one; check NLM Support Center; confirm no registration needed).
3. Confirm that MTHSPL-derived content has no third-party restrictions.
4. Attribution/disclaimer text approved and placed in-app (Settings > About/Catalog) and in store metadata; no NLM name/logo; no endorsement implied.
5. Update-obligation approach approved (release shown in-app and "may not reflect latest" notice).
6. Build manifest present: source release, URL, sha256, output sha256, row counts, filter rules; reproduced by a second person.
7. A scan proves no SAB other than RXNORM/MTHSPL and no SNOMED/NDC fields entered the catalog.
8. Brand-name/trademark nominative-use position approved.
9. No clinical claims in catalog UI copy (reviewed); manual-entry fallback verified when catalog absent.
10. Written release-specific approval by named human recorded in docs/registers.md (this spike does not do that).

## Open legal questions
- Does any NLM term restrict repackaging into a "derived, searchable" proprietary file or an app-store paid app?
- Is "acknowledgment to NLM" satisfied by an About screen alone?
- RxTerms: governing terms and possible registration; is attribution wording #1 or #7?
- Third-party rights in MTHSPL/SPL-derived names; GMDN-style carve-outs do not apply here but confirm.
- Brand trademark use; foreign-store/regional availability (US-only distribution).
