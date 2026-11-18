//! Evidence schema and verdict rules for physical-device acceptance.
//!
//! The scenario catalog (`spikes/scenarios/scenarios.json`) is the single source of truth for pass
//! criteria. A scenario is `PENDING` until a *non-synthetic, physical-device* evidence file
//! exists for it. Documentation, source reading, emulators, and synthetic fixtures never produce
//! a `PASS`.

use std::collections::{BTreeMap, BTreeSet};

use chrono::DateTime;
use serde::Deserialize;

pub const SCENARIO_SCHEMA: &str = "medtracker.phase0.scenarios/1";
pub const EVIDENCE_SCHEMA: &str = "medtracker.phase0.evidence/1";

#[derive(Debug, Clone, Deserialize)]
pub struct Catalog {
    pub schema: String,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    TimedDelivery,
    Checklist,
    Measurement,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Timed {
    pub min_trials: usize,
    pub min_delivered: usize,
    pub window_s: i64,
    pub early_tolerance_s: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Relation {
    pub lhs: String,
    pub op: String,
    pub rhs: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub platform: String,
    pub area: String,
    pub title: String,
    pub kind: Kind,
    #[serde(default)]
    pub required_for_go: bool,
    #[serde(default = "one")]
    pub min_distinct_devices: usize,
    #[serde(default = "default_builds")]
    pub allowed_build_modes: Vec<String>,
    pub timed: Option<Timed>,
    #[serde(default)]
    pub checks: Vec<String>,
    #[serde(default)]
    pub numbers_eq: BTreeMap<String, f64>,
    #[serde(default)]
    pub measurements: Vec<String>,
    #[serde(default)]
    pub relations: Vec<Relation>,
    pub pass: String,
}

fn one() -> usize {
    1
}

fn default_builds() -> Vec<String> {
    vec!["release-dev-signed".into(), "release-distribution-signed".into()]
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Device {
    pub id: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub os_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct App {
    pub build_mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Trial {
    pub target_utc: Option<String>,
    pub observed_utc: Option<String>,
    #[serde(default)]
    pub duplicate: bool,
    #[serde(default)]
    pub stale: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EvidenceFile {
    pub schema: String,
    pub scenario_id: String,
    #[serde(default)]
    pub synthetic: bool,
    pub physical_device: Option<bool>,
    pub device: Option<Device>,
    pub app: Option<App>,
    pub started_at_utc: Option<String>,
    pub operator: Option<String>,
    #[serde(default)]
    pub trials: Vec<Trial>,
    #[serde(default)]
    pub checks: BTreeMap<String, Option<bool>>,
    #[serde(default)]
    pub numbers: BTreeMap<String, Option<f64>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Only tests may set this; the CLI never does.
    pub allow_synthetic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileVerdict {
    Pass,
    Fail(Vec<String>),
    /// The file cannot be judged (or must not count): wrong shape, unfilled, synthetic, emulator.
    Invalid(Vec<String>),
}

pub fn load_catalog(json: &str) -> Result<Catalog, String> {
    let c: Catalog = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if c.schema != SCENARIO_SCHEMA {
        return Err(format!("unexpected scenario schema {}", c.schema));
    }
    let mut seen = BTreeSet::new();
    for s in &c.scenarios {
        if !seen.insert(s.id.clone()) {
            return Err(format!("duplicate scenario id {}", s.id));
        }
        let ok = match s.kind {
            Kind::TimedDelivery => s.timed.is_some(),
            Kind::Checklist => !s.checks.is_empty() || !s.numbers_eq.is_empty(),
            Kind::Measurement => !s.measurements.is_empty(),
        };
        if !ok {
            return Err(format!("scenario {} lacks criteria for its kind", s.id));
        }
    }
    Ok(c)
}

fn parse_ts(s: &Option<String>) -> Option<i64> {
    s.as_deref().and_then(|v| DateTime::parse_from_rfc3339(v).ok()).map(|d| d.timestamp())
}

fn validity(sc: &Scenario, f: &EvidenceFile, opts: Options) -> Vec<String> {
    let mut bad = Vec::new();
    if f.schema != EVIDENCE_SCHEMA {
        bad.push(format!("schema must be {EVIDENCE_SCHEMA}"));
    }
    if f.synthetic && !opts.allow_synthetic {
        bad.push("synthetic evidence never counts".into());
    }
    if f.physical_device != Some(true) {
        bad.push("physical_device must be true (emulators/simulators/hosts do not count)".into());
    }
    match &f.device {
        Some(d) if d.id.is_some() && d.model.is_some() && d.os_version.is_some() => {}
        _ => bad.push("device.id, device.model and device.os_version are required".into()),
    }
    match f.app.as_ref().and_then(|a| a.build_mode.as_deref()) {
        Some(m) if sc.allowed_build_modes.iter().any(|a| a == m) => {}
        Some(m) => bad.push(format!("build_mode {m} not allowed for {} (allowed: {:?})", sc.id, sc.allowed_build_modes)),
        None => bad.push("app.build_mode is required".into()),
    }
    if parse_ts(&f.started_at_utc).is_none() {
        bad.push("started_at_utc must be RFC 3339".into());
    }
    if f.operator.as_deref().map(str::trim).unwrap_or("").is_empty() {
        bad.push("operator is required".into());
    }
    bad
}

pub fn judge_file(sc: &Scenario, f: &EvidenceFile, opts: Options) -> FileVerdict {
    if f.scenario_id != sc.id {
        return FileVerdict::Invalid(vec![format!("file is for {}, not {}", f.scenario_id, sc.id)]);
    }
    let mut invalid = validity(sc, f, opts);
    let mut fail = Vec::new();

    match sc.kind {
        Kind::TimedDelivery => {
            let t = sc.timed.as_ref().expect("validated at load");
            if f.trials.len() < t.min_trials {
                invalid.push(format!("{} trials recorded, {} required", f.trials.len(), t.min_trials));
            }
            let mut delivered = 0usize;
            for (i, tr) in f.trials.iter().enumerate() {
                let (Some(target), observed) = (parse_ts(&tr.target_utc), parse_ts(&tr.observed_utc)) else {
                    invalid.push(format!("trial {i}: target_utc missing"));
                    continue;
                };
                if let Some(obs) = observed {
                    let delta = obs - target;
                    if delta >= -t.early_tolerance_s && delta <= t.window_s {
                        delivered += 1;
                    }
                }
                if tr.duplicate {
                    fail.push(format!("trial {i}: duplicate delivery"));
                }
                if tr.stale {
                    fail.push(format!("trial {i}: stale delivery"));
                }
            }
            if invalid.is_empty() && delivered < t.min_delivered {
                fail.push(format!("{delivered}/{} delivered within window, {} required", f.trials.len(), t.min_delivered));
            }
        }
        Kind::Checklist => {
            for name in &sc.checks {
                match f.checks.get(name) {
                    Some(Some(true)) => {}
                    Some(Some(false)) => fail.push(format!("check {name} is false")),
                    _ => invalid.push(format!("check {name} not filled")),
                }
            }
            for (name, want) in &sc.numbers_eq {
                match f.numbers.get(name) {
                    Some(Some(v)) if (v - want).abs() < f64::EPSILON => {}
                    Some(Some(v)) => fail.push(format!("{name} = {v}, expected {want}")),
                    _ => invalid.push(format!("number {name} not filled")),
                }
            }
        }
        Kind::Measurement => {
            for name in &sc.measurements {
                if !matches!(f.numbers.get(name), Some(Some(_))) {
                    invalid.push(format!("measurement {name} not filled"));
                }
            }
            if invalid.is_empty() {
                for r in &sc.relations {
                    let (l, rr) = (f.numbers[&r.lhs].unwrap(), f.numbers[&r.rhs].unwrap());
                    let ok = match r.op.as_str() {
                        "<=" => l <= rr,
                        ">=" => l >= rr,
                        "==" => (l - rr).abs() < f64::EPSILON,
                        other => {
                            invalid.push(format!("unknown relation operator {other}"));
                            true
                        }
                    };
                    if !ok {
                        fail.push(format!("{} {} {} violated ({l} vs {rr})", r.lhs, r.op, r.rhs));
                    }
                }
            }
        }
    }

    if !invalid.is_empty() {
        FileVerdict::Invalid(invalid)
    } else if !fail.is_empty() {
        FileVerdict::Fail(fail)
    } else {
        FileVerdict::Pass
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// No valid physical-device evidence exists.
    Pending,
    /// Some passing evidence, but fewer distinct devices than the scenario requires.
    Incomplete { passed_devices: usize, needed: usize },
    Pass,
    Fail,
}

#[derive(Debug, Clone)]
pub struct ScenarioReport {
    pub id: String,
    pub platform: String,
    pub area: String,
    pub required_for_go: bool,
    pub status: Status,
    pub files: usize,
    pub invalid_files: usize,
    pub notes: Vec<String>,
}

pub fn summarize(catalog: &Catalog, files: &[(String, EvidenceFile)], opts: Options) -> Vec<ScenarioReport> {
    catalog
        .scenarios
        .iter()
        .map(|sc| {
            let mut passed: BTreeSet<String> = BTreeSet::new();
            let (mut failed, mut invalid, mut n) = (false, 0usize, 0usize);
            let mut notes = Vec::new();
            for (name, f) in files.iter().filter(|(_, f)| f.scenario_id == sc.id) {
                n += 1;
                match judge_file(sc, f, opts) {
                    FileVerdict::Pass => {
                        passed.insert(f.device.as_ref().and_then(|d| d.id.clone()).unwrap_or_default());
                    }
                    FileVerdict::Fail(why) => {
                        failed = true;
                        notes.extend(why.into_iter().map(|w| format!("{name}: FAIL {w}")));
                    }
                    FileVerdict::Invalid(why) => {
                        invalid += 1;
                        notes.extend(why.into_iter().map(|w| format!("{name}: INVALID {w}")));
                    }
                }
            }
            let status = if failed {
                Status::Fail
            } else if passed.len() >= sc.min_distinct_devices {
                Status::Pass
            } else if !passed.is_empty() {
                Status::Incomplete { passed_devices: passed.len(), needed: sc.min_distinct_devices }
            } else {
                Status::Pending
            };
            ScenarioReport {
                id: sc.id.clone(),
                platform: sc.platform.clone(),
                area: sc.area.clone(),
                required_for_go: sc.required_for_go,
                status,
                files: n,
                invalid_files: invalid,
                notes,
            }
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gate {
    /// Required scenarios have no valid evidence yet. This is the starting state.
    Pending,
    /// At least one required scenario failed.
    NoGo,
    /// Some required scenarios passed; the rest are pending or incomplete.
    Incomplete,
    /// Every required scenario passed on physical devices. A human still makes the decision.
    CriteriaMet,
}

pub fn gate(reports: &[ScenarioReport], platform: &str, area: &str) -> Gate {
    let req: Vec<_> = reports.iter().filter(|r| r.required_for_go && r.platform == platform && r.area == area).collect();
    if req.is_empty() || req.iter().any(|r| r.status == Status::Fail) {
        return if req.is_empty() { Gate::Pending } else { Gate::NoGo };
    }
    if req.iter().all(|r| r.status == Status::Pass) {
        Gate::CriteriaMet
    } else if req.iter().all(|r| r.status == Status::Pending) {
        Gate::Pending
    } else {
        Gate::Incomplete
    }
}
