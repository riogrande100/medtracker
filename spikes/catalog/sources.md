# Catalog spike sources (fetched 2026-11-19 UTC unless noted)

| # | URL | Used for |
|---|-----|----------|
| 1 | https://www.nlm.nih.gov/research/umls/rxnorm/docs/termsofservice.html (page says last reviewed 2022-01-31) | RxNorm ToS: public-domain NLM content, attribution text, UMLS license for full release, update obligation |
| 2 | https://www.nlm.nih.gov/research/umls/rxnorm/docs/rxnormfiles.html | Release files: Full (license) vs Current Prescribable Content (no license), Nov 2 2026 release names, monthly + weekly |
| 3 | https://www.nlm.nih.gov/research/umls/rxnorm/docs/prescribe.html | Current Prescribable Content: SAB=RXNORM + MTHSPL only, "no license required; public domain", draft status |
| 4 | https://www.nlm.nih.gov/research/umls/rxnorm/docs/techdoc.html | TTY definitions, RRF columns, SRL 1-4/9 restrictions, supplements out of scope |
| 5 | https://lhncbc.nlm.nih.gov/MOR/RxTerms | RxTerms: current release RxTerms202611.zip, monthly, fields |
| 6 | https://data.lhncbc.nlm.nih.gov/public/rxterms/release/RxTerms202611.zip | Actually downloaded to scratchpad (not repo); 2,812,707 bytes; sha256 fc1392663e6c4e5c15e49069ef6bfc8cbd73a2e7c574de0eac65fecefd787b47 |
| 7 | https://lhncbc.nlm.nih.gov/RxNav/TermsofService.html | RxNav/RxNorm/RxTerms API terms: attribution statement, no NLM logo, rate limits, SNOMED in RxClass |
| 8 | https://www.nlm.nih.gov/web_policies.html | NLM copyright/data reuse: US gov works not copyrighted; third-party material; logo restrictions |
| 9 | https://open.fda.gov/terms/ | openFDA CC0, attribution requested, GMDN exception, as-is |
| 10 | https://www.fda.gov/drugs/drug-approvals-and-databases/national-drug-code-directory | NDC Directory scope, daily update, caveats, exclusions |
| 11 | https://dailymed.nlm.nih.gov/dailymed/app-support-download-data.cfm and .../app-support-web-services.cfm | DailyMed downloads and web services; page itself states no reuse license (links to nlm.nih.gov/copyright.html, not read) |
| 12 | https://ods.od.nih.gov/Research/Dietary_Supplement_Label_Database/ (HTTP 403 on fetch) and https://dsld.od.nih.gov/downloads (empty on fetch); web search snippets only | DSLD: UNVERIFIED terms |
| 13 | HEAD https://download.nlm.nih.gov/rxnorm/RxNorm_full_prescribe_11022026.zip | Content-Length 75,032,750 bytes (2026-11-19) |
| 14 | RxTerms202611 release notes (inside zip #6) | Per-TTY counts; contains no licence/attribution text (grep) |
