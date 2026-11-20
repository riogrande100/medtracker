//! Evidence-judging tests with synthetic fixtures. Synthetic evidence must never produce PASS
//! through the CLI path (`allow_synthetic = false`).
use std::fs;
use std::path::PathBuf;

use medtracker_phase0_probes::evidence::*;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}
fn catalog() -> Catalog {
    load_catalog(&fs::read_to_string(root().join("scenarios/scenarios.json")).unwrap()).unwrap()
}
fn scenario(id: &str) -> Scenario {
    catalog().scenarios.into_iter().find(|s| s.id == id).unwrap()
}
fn file(json: serde_json::Value) -> EvidenceFile {
    serde_json::from_value(json).unwrap()
}
fn base(id: &str, synthetic: bool, physical: bool) -> serde_json::Value {
    serde_json::json!({
        "schema": EVIDENCE_SCHEMA, "scenario_id": id, "synthetic": synthetic, "physical_device": physical,
        "device": {"id": "dev1", "manufacturer": "X", "model": "Y", "os_version": "1"},
        "app": {"build_mode": "release-dev-signed"},
        "started_at_utc": "2026-11-18T00:00:00Z", "operator": "tester",
        "checks": {"tauri_ios_build_succeeds": true, "installs_on_physical_device": true,
                   "launches_in_airplane_mode": true, "notification_plugin_version_recorded": true}
    })
}

#[test]
fn catalog_loads_with_expected_shape() {
    let c = catalog();
    assert_eq!(c.scenarios.len(), 58);
}

#[test]
fn synthetic_never_passes() {
    let sc = scenario("REM-IOS-00");
    let f = file(base("REM-IOS-00", true, true));
    assert!(matches!(judge_file(&sc, &f, Options::default()), FileVerdict::Invalid(_)));
    // The test-only escape hatch shows the fixture is otherwise complete.
    assert_eq!(judge_file(&sc, &f, Options { allow_synthetic: true }), FileVerdict::Pass);
}

#[test]
fn emulator_or_host_never_passes() {
    let sc = scenario("REM-IOS-00");
    let f = file(base("REM-IOS-00", false, false));
    assert!(matches!(judge_file(&sc, &f, Options::default()), FileVerdict::Invalid(_)));
}

#[test]
fn debug_build_is_rejected() {
    let sc = scenario("REM-IOS-00");
    let mut j = base("REM-IOS-00", false, true);
    j["app"]["build_mode"] = "debug".into();
    assert!(matches!(judge_file(&sc, &file(j), Options::default()), FileVerdict::Invalid(_)));
}

#[test]
fn false_check_fails_and_unfilled_is_invalid() {
    let sc = scenario("REM-IOS-00");
    let mut j = base("REM-IOS-00", false, true);
    j["checks"]["launches_in_airplane_mode"] = false.into();
    assert!(matches!(judge_file(&sc, &file(j), Options::default()), FileVerdict::Fail(_)));
    let mut j = base("REM-IOS-00", false, true);
    j["checks"]["launches_in_airplane_mode"] = serde_json::Value::Null;
    assert!(matches!(judge_file(&sc, &file(j), Options::default()), FileVerdict::Invalid(_)));
}

#[test]
fn wrong_scenario_id_is_invalid() {
    let sc = scenario("REM-IOS-01");
    let f = file(base("REM-IOS-00", false, true));
    assert!(matches!(judge_file(&sc, &f, Options::default()), FileVerdict::Invalid(_)));
}

#[test]
fn timed_delivery_requires_enough_trials() {
    let sc = scenario("REM-IOS-02");
    let mut j = base("REM-IOS-02", true, true);
    j["trials"] = serde_json::json!([{"target_utc": "2026-11-18T01:00:00Z", "observed_utc": "2026-11-18T01:00:03Z"}]);
    match judge_file(&sc, &file(j), Options { allow_synthetic: true }) {
        FileVerdict::Invalid(w) => assert!(w.iter().any(|m| m.contains("trials recorded"))),
        o => panic!("{o:?}"),
    }
}

#[test]
fn timed_delivery_counts_late_early_duplicate() {
    let sc = scenario("REM-IOS-02");
    let mut trials = vec![];
    for i in 0..20 {
        trials.push(serde_json::json!({"target_utc": format!("2026-11-18T01:{:02}:00Z", i),
            "observed_utc": format!("2026-11-18T01:{:02}:10Z", i)}));
    }
    let mut j = base("REM-IOS-02", true, true);
    j["trials"] = trials.clone().into();
    assert_eq!(judge_file(&sc, &file(j), Options { allow_synthetic: true }), FileVerdict::Pass);
    // 2 never delivered => 18 < 19.
    trials[0]["observed_utc"] = serde_json::Value::Null;
    trials[1]["observed_utc"] = serde_json::Value::Null;
    let mut j = base("REM-IOS-02", true, true);
    j["trials"] = trials.clone().into();
    assert!(matches!(judge_file(&sc, &file(j), Options { allow_synthetic: true }), FileVerdict::Fail(_)));
    // A duplicate fails even if delivery counts are fine.
    trials[0]["observed_utc"] = "2026-11-18T01:00:10Z".into();
    trials[1]["observed_utc"] = "2026-11-18T01:01:10Z".into();
    trials[2]["duplicate"] = true.into();
    let mut j = base("REM-IOS-02", true, true);
    j["trials"] = trials.into();
    assert!(matches!(judge_file(&sc, &file(j), Options { allow_synthetic: true }), FileVerdict::Fail(_)));
}

#[test]
fn measurement_relation_checked() {
    let sc = scenario("REM-IOS-09");
    let mut j = base("REM-IOS-09", true, true);
    j["numbers"] = serde_json::json!({"max_pending_observed": 64, "app_horizon_cap": 100});
    assert!(matches!(judge_file(&sc, &file(j.clone()), Options { allow_synthetic: true }), FileVerdict::Fail(_)));
    j["numbers"]["app_horizon_cap"] = 60.into();
    assert_eq!(judge_file(&sc, &file(j), Options { allow_synthetic: true }), FileVerdict::Pass);
}

#[test]
fn two_device_scenarios_need_two_devices() {
    let c = catalog();
    let sc = c.scenarios.iter().find(|s| s.id == "REM-AND-00").unwrap();
    assert_eq!(sc.min_distinct_devices, 2);
    let checks: Vec<_> = sc.checks.clone();
    let mk = |dev: &str| {
        let mut j = base("REM-AND-00", true, true);
        j["device"]["id"] = dev.into();
        j["checks"] = serde_json::Value::Object(checks.iter().map(|k| (k.clone(), true.into())).collect());
        file(j)
    };
    let opts = Options { allow_synthetic: true };
    let one = [("a.json".to_string(), mk("d1"))];
    let r = summarize(&c, &one, opts);
    assert_eq!(r.iter().find(|r| r.id == "REM-AND-00").unwrap().status, Status::Incomplete { passed_devices: 1, needed: 2 });
    let two = [("a.json".to_string(), mk("d1")), ("b.json".to_string(), mk("d2"))];
    let r = summarize(&c, &two, opts);
    assert_eq!(r.iter().find(|r| r.id == "REM-AND-00").unwrap().status, Status::Pass);
    // Same device twice is still one device.
    let dup = [("a.json".to_string(), mk("d1")), ("b.json".to_string(), mk("d1"))];
    assert_eq!(summarize(&c, &dup, opts).iter().find(|r| r.id == "REM-AND-00").unwrap().status, Status::Incomplete { passed_devices: 1, needed: 2 });
}

#[test]
fn with_no_evidence_everything_is_pending_and_gates_pending() {
    let c = catalog();
    let r = summarize(&c, &[], Options::default());
    assert!(r.iter().all(|x| x.status == Status::Pending));
    for (p, a) in [("ios", "reminders"), ("android", "reminders"), ("windows", "timezone")] {
        assert_eq!(gate(&r, p, a), Gate::Pending);
    }
}

#[test]
fn every_scenario_has_unique_id_and_referenced_readme_exists() {
    let c = catalog();
    let ids: std::collections::BTreeSet<_> = c.scenarios.iter().map(|s| &s.id).collect();
    assert_eq!(ids.len(), c.scenarios.len());
    assert!(root().join("timezone/README.md").exists(), "scenarios.json references spikes/timezone/README.md");
}

#[test]
fn templates_exist_for_every_scenario_and_never_judge_as_pass() {
    let c = catalog();
    let dir = root().join("evidence/templates");
    let mut covered = std::collections::BTreeSet::new();
    let mut files = vec![];
    for e in fs::read_dir(&dir).expect("templates dir").flatten() {
        let txt = fs::read_to_string(e.path()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&txt).unwrap();
        for id in v["_covers_scenarios"].as_array().expect("_covers_scenarios") {
            covered.insert(id.as_str().unwrap().to_string());
        }
        // Every template parses as an evidence file and, left unfilled, is Invalid for its scenario.
        let f: EvidenceFile = serde_json::from_value(v.clone()).unwrap();
        let sc = c.scenarios.iter().find(|s| s.id == f.scenario_id).expect("template scenario_id exists");
        assert!(matches!(judge_file(sc, &f, Options::default()), FileVerdict::Invalid(_)), "{:?}", e.path());
        assert!(!f.synthetic && f.physical_device.is_none());
        files.push((e.file_name().to_string_lossy().to_string(), f));
    }
    for s in &c.scenarios {
        assert!(covered.contains(&s.id), "no template covers {}", s.id);
    }
    assert!(summarize(&c, &files, Options::default()).iter().all(|r| r.status == Status::Pending));
}
