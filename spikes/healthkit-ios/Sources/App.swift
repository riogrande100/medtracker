import SwiftUI
import HealthKit

// THROWAWAY SPIKE (B0-09). No network code. Reads only; one optional write probe of a fake dose.
@main struct HKSpikeApp: App { var body: some Scene { WindowGroup { ContentView() } } }

@MainActor final class Spike: ObservableObject {
    @Published var log: [String] = []
    let store = HKHealthStore()
    func say(_ s: String) { log.append(s); print("SPIKE: \(s)") }

    func available() {
        say("isHealthDataAvailable = \(HKHealthStore.isHealthDataAvailable())")
        say("userAnnotatedMedicationType.requiresPerObjectAuthorization = \(HKObjectType.userAnnotatedMedicationType().requiresPerObjectAuthorization())")
        say("medicationDoseEventType.requiresPerObjectAuthorization = \(HKObjectType.medicationDoseEventType().requiresPerObjectAuthorization())")
        let t = HKObjectType.medicationDoseEventType()
        say("dose-event authorizationStatus(for:) = \(store.authorizationStatus(for: t).rawValue) (0 notDetermined,1 denied,2 authorized; write-status only)")
    }

    // Each step logs "about to" first so a crash (uncatchable NSException) is attributable from the console.
    func authPerObjectMeds() async {
        say("about to: requestPerObjectReadAuthorization(userAnnotatedMedicationType)")
        do {
            try await store.requestPerObjectReadAuthorization(for: HKObjectType.userAnnotatedMedicationType(), predicate: nil)
            say("-> returned OK (does not mean granted)")
        } catch { say("-> ERROR: \(error)") }
    }
    func authReadDose() async {
        say("about to: requestAuthorization(toShare: [], read: [doseEvent])")
        do {
            try await store.requestAuthorization(toShare: [], read: [HKObjectType.medicationDoseEventType()])
            say("-> returned OK")
        } catch { say("-> ERROR: \(error)") }
    }
    func authorize() async { await authPerObjectMeds() }  // dose events: no separate prompt allowed (see FINDINGS)

    func meds() async {
        do {
            let d = HKUserAnnotatedMedicationQueryDescriptor(predicate: nil, limit: nil)
            let r = try await d.result(for: store)
            say("medication query OK: \(r.count) medication(s)")
            for m in r { say(" med: \(m.medication.displayText) archived=\(m.isArchived) hasSchedule=\(m.hasSchedule)") }
        } catch { say("medication query ERROR: \(error)") }
    }

    func doses() async {
        do {
            let d = HKSampleQueryDescriptor(predicates: [.sample(type: HKObjectType.medicationDoseEventType())], sortDescriptors: [SortDescriptor(\.startDate, order: .reverse)], limit: 50)
            let r = try await d.result(for: store)
            say("dose-event query OK: \(r.count) event(s)")
            for case let e as HKMedicationDoseEvent in r { say(" dose: status=\(e.logStatus.rawValue) sched=\(e.scheduleType.rawValue) concept=\(e.medicationConceptIdentifier)") }
        } catch { say("dose-event query ERROR: \(error)") }
    }

    func writeProbe() async {
        // Needs a medication concept identifier; without a real one we cannot even construct a sample.
        say("about to: requestAuthorization(toShare: [doseEvent], read: [])  (HKMedicationDoseEvent has init NS_UNAVAILABLE; no public factory in SDK headers)")
        do {
            try await store.requestAuthorization(toShare: [HKObjectType.medicationDoseEventType()], read: [])
            say("requestAuthorization(share: [doseEvent]) returned OK")
        } catch { say("requestAuthorization(share: [doseEvent]) ERROR: \(error)") }
    }

    func runAll() async {
        available(); await meds(); await doses()
    }
}

struct ContentView: View {
    @StateObject var s = Spike()
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("HealthKit spike (fake data only)").font(.headline)
            ViewThatFits { HStack {
                Button("Check") { s.available() }
                Button("Authorize") { Task { await s.authorize() } }
                Button("Meds") { Task { await s.meds() } }
                Button("Doses") { Task { await s.doses() } }
                Button("Write") { Task { await s.writeProbe() } }
            }.buttonStyle(.bordered).lineLimit(1) }
            ScrollView {
                Text(s.log.joined(separator: "\n"))
                    .font(.system(.caption, design: .monospaced))
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .textSelection(.enabled)
                    .accessibilityIdentifier("log")
            }
        }.padding()
        .task {
            let a = CommandLine.arguments
            if a.contains("-autorun") { await s.runAll() }
            else if a.contains("-auth-read-dose") { await s.authReadDose() }
            else if a.contains("-write-probe") { await s.writeProbe() }
            else { s.available() }
        }
    }
}
