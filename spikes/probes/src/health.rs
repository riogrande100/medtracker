//! Throwaway model of the rule "a health import or authorization never proves a dose taken".
//!
//! The classifier is deliberately total over (source, authorization, signal) so a test can
//! enumerate every combination. It models documented behavior only; it does not call HealthKit,
//! Health Connect, or any Google API.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    /// Apple HealthKit medication data (`HKMedicationDoseEvent`, iOS 26+ per Apple docs).
    HealthKit,
    /// Health Connect health-and-fitness records. Apple-style medication dose records are not in
    /// its documented data-type categories.
    HealthConnectFitness,
    /// Health Connect Medical Records (FHIR `Medication*` resources; experimental API).
    HealthConnectMedicalRecords,
    /// Google Health API / legacy Google Fit REST: cloud APIs that need accounts and network.
    GoogleCloudApi,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Auth {
    Granted,
    /// HealthKit limited-history authorization; earlier data is invisible to the app.
    LimitedWindow,
    Denied,
    NotDetermined,
    /// Platform or data store not present (no Health Connect, unsupported device/OS).
    Unavailable,
    /// User revoked, or the OS reset permission for an unused app.
    Revoked,
    /// Previously valid grant, token, or time window no longer applies.
    Expired,
    /// The source app/data was deleted or uninstalled.
    SourceRemoved,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Signal {
    /// Another app says the user logged a dose as taken (e.g. `LogStatus.taken`).
    ClaimTaken,
    ClaimSkipped,
    /// `notInteracted`, `notificationNotSent`, `snoozed`: reminder-pipeline statuses.
    ReminderPipelineStatus,
    /// A medication statement/request resource: what was prescribed or reported, not intake.
    MedicationStatement,
    /// The query returned nothing.
    NoRecords,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Effect {
    Ignore,
    /// Shown only as an unverified external claim that needs explicit user review.
    ExternalClaim,
    /// Blocked by product constraints (no network, no accounts).
    BlockedByConstraint,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Absence {
    /// "No data" must be shown as unknown, never as "no dose taken".
    Unknown,
    NotApplicable,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Outcome {
    pub effect: Effect,
    /// Dose events the import may create on its own. Always zero.
    pub ledger_doses_created: u32,
    pub may_change_occurrence_status: bool,
    pub absence: Absence,
}

pub fn classify(source: Source, auth: Auth, signal: Signal) -> Outcome {
    let absence = if signal == Signal::NoRecords { Absence::Unknown } else { Absence::NotApplicable };
    let effect = if source == Source::GoogleCloudApi {
        Effect::BlockedByConstraint
    } else {
        match (auth, signal) {
            (Auth::Granted | Auth::LimitedWindow, Signal::ClaimTaken | Signal::ClaimSkipped | Signal::MedicationStatement) => {
                Effect::ExternalClaim
            }
            _ => Effect::Ignore,
        }
    };
    Outcome { effect, ledger_doses_created: 0, may_change_occurrence_status: false, absence }
}
