//! Prints the Phase 0 acceptance board from the scenario catalog and collected evidence files.
//!
//! Usage: evidence_check [--scenarios FILE] [--evidence-dir DIR] [--json] [--require-go PLATFORM:AREA]
//! Exit codes: 0 no failures; 1 at least one scenario failed; 2 unreadable catalog/evidence;
//! 3 a --require-go gate is not met.

use std::path::{Path, PathBuf};
use std::{env, fs, process};

use medtracker_phase0_probes::evidence::{self, Gate, Options, Status};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if p.is_dir() {
            if name != "templates" && name != "fixtures" && !name.starts_with('.') {
                collect(&p, out);
            }
        } else if name.ends_with(".json") {
            out.push(p);
        }
    }
}

fn status_text(s: &Status) -> String {
    match s {
        Status::Pending => "PENDING".into(),
        Status::Incomplete { passed_devices, needed } => format!("INCOMPLETE ({passed_devices}/{needed} devices)"),
        Status::Pass => "PASS".into(),
        Status::Fail => "FAIL".into(),
    }
}

fn gate_text(g: &Gate) -> &'static str {
    match g {
        Gate::Pending => "PENDING (no valid physical-device evidence)",
        Gate::NoGo => "NO-GO (a required scenario failed)",
        Gate::Incomplete => "INCOMPLETE",
        Gate::CriteriaMet => "CRITERIA MET (human decision still required)",
    }
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut scenarios = root.join("scenarios/scenarios.json");
    let mut evidence_dir = root.join("evidence");
    let (mut json, mut require) = (false, Vec::<String>::new());
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--scenarios" => scenarios = args.next().expect("--scenarios FILE").into(),
            "--evidence-dir" => evidence_dir = args.next().expect("--evidence-dir DIR").into(),
            "--json" => json = true,
            "--require-go" => require.push(args.next().expect("--require-go PLATFORM:AREA")),
            other => {
                eprintln!("unknown argument {other}");
                process::exit(2);
            }
        }
    }

    let catalog = match fs::read_to_string(&scenarios).map_err(|e| e.to_string()).and_then(|s| evidence::load_catalog(&s)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cannot load catalog {}: {e}", scenarios.display());
            process::exit(2);
        }
    };

    let mut paths = Vec::new();
    collect(&evidence_dir, &mut paths);
    paths.sort();
    let mut files = Vec::new();
    let mut unreadable = Vec::new();
    for p in &paths {
        let label = p.strip_prefix(&evidence_dir).unwrap_or(p).display().to_string();
        match fs::read_to_string(p).map_err(|e| e.to_string()).and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string())) {
            Ok(f) => files.push((label, f)),
            Err(e) => unreadable.push(format!("{label}: {e}")),
        }
    }

    let reports = evidence::summarize(&catalog, &files, Options::default());
    let mut gates = Vec::new();
    for platform in ["ios", "android", "windows", "macos"] {
        for area in ["reminders", "timezone", "health"] {
            if reports.iter().any(|r| r.required_for_go && r.platform == platform && r.area == area) {
                gates.push((platform, area, evidence::gate(&reports, platform, area)));
            }
        }
    }

    if json {
        let v = serde_json::json!({
            "evidence_files": paths.len(),
            "unreadable": unreadable,
            "scenarios": reports.iter().map(|r| serde_json::json!({
                "id": r.id, "platform": r.platform, "area": r.area, "required_for_go": r.required_for_go,
                "status": status_text(&r.status), "files": r.files, "invalid_files": r.invalid_files, "notes": r.notes,
            })).collect::<Vec<_>>(),
            "gates": gates.iter().map(|(p, a, g)| serde_json::json!({"platform": p, "area": a, "gate": gate_text(g)})).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    } else {
        println!("Phase 0 physical-device acceptance board ({} scenarios, {} evidence files)\n", reports.len(), paths.len());
        println!("{:<12} {:<9} {:<10} {:<4} {}", "SCENARIO", "PLATFORM", "AREA", "GATE", "STATUS");
        for r in &reports {
            println!("{:<12} {:<9} {:<10} {:<4} {}", r.id, r.platform, r.area, if r.required_for_go { "req" } else { "info" }, status_text(&r.status));
            for n in &r.notes {
                println!("    - {n}");
            }
        }
        println!("\nGates (required scenarios only):");
        for (p, a, g) in &gates {
            println!("  {p:<8} {a:<10} {}", gate_text(g));
        }
        for u in &unreadable {
            println!("UNREADABLE: {u}");
        }
        println!("\nDocumentation, source reading, host tests, simulators, and synthetic fixtures never move a scenario off PENDING.");
    }

    if !unreadable.is_empty() {
        process::exit(2);
    }
    if reports.iter().any(|r| r.status == Status::Fail) {
        process::exit(1);
    }
    for want in require {
        let Some((p, a)) = want.split_once(':') else {
            eprintln!("--require-go expects PLATFORM:AREA");
            process::exit(2);
        };
        if evidence::gate(&reports, p, a) != Gate::CriteriaMet {
            eprintln!("gate {want} not met");
            process::exit(3);
        }
    }
}
