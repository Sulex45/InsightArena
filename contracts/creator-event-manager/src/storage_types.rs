use soroban_sdk::{contracttype, Address, String, Symbol};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum length for event title (characters)
pub const MAX_TITLE_LEN: u32 = 200;
/// Maximum length for event description (characters)
pub const MAX_DESCRIPTION_LEN: u32 = 1000;
/// Maximum length for team names (characters)
pub const MAX_TEAM_NAME_LEN: u32 = 100;
/// Valid predicted outcome symbols
pub const OUTCOME_TEAM_A: &str = "TEAM_A";
pub const OUTCOME_TEAM_B: &str = "TEAM_B";
pub const OUTCOME_DRAW: &str = "DRAW";

// ---------------------------------------------------------------------------
// MatchResult
// ---------------------------------------------------------------------------

/// Possible outcomes of a prediction match.
///
/// Encoded as u8 on the wire: 0 = TeamA, 1 = TeamB, 2 = Draw.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchResult {
    /// First team / option A wins
    TeamA,
    /// Second team / option B wins
    TeamB,
    /// Match ends in a draw / tie
    Draw,
}

impl MatchResult {
    /// Encode to u8 for compact storage and prediction fields.
    pub fn to_u8(&self) -> u8 {
        match self {
            MatchResult::TeamA => 0,
            MatchResult::TeamB => 1,
            MatchResult::Draw => 2,
        }
    }

    /// Decode from u8.  Returns `None` for any value outside 0–2.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(MatchResult::TeamA),
            1 => Some(MatchResult::TeamB),
            2 => Some(MatchResult::Draw),
            _ => None,
        }
    }

    /// Convenience alias kept for callers that still use u32.
    pub fn to_u32(&self) -> u32 {
        self.to_u8() as u32
    }

    /// Convenience alias kept for callers that still use u32.
    pub fn from_u32(value: u32) -> Option<Self> {
        if value > u8::MAX as u32 {
            return None;
        }
        Self::from_u8(value as u8)
    }
}

// ---------------------------------------------------------------------------
// DataKey
// ---------------------------------------------------------------------------

/// Unified storage key enum for every piece of contract state.
///
/// Using a single enum keeps key namespacing explicit and avoids collisions
/// between different storage domains.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    // ── Global admin / config keys ──────────────────────────────────────────
    /// Contract administrator address
    Admin(Address),

    /// AI agent address authorised to submit match results
    AIAgent(Address),

    /// Treasury address that receives creation fees
    Treasury(Address),

    /// XLM creation fee in stroops (i128)
    CreationFee(i128),

    /// Emergency pause flag — when true, sensitive operations are halted
    Paused(bool),

    /// Native XLM token contract address
    XLMToken(Address),

    // ── Global counters ─────────────────────────────────────────────────────
    /// Monotonically increasing event counter → u64
    EventCounter(u64),

    /// Monotonically increasing match counter → u64
    MatchCounter(u64),

    /// Monotonically increasing prediction counter → u64
    PredictionCounter(u64),

    // ── Core entity keys ────────────────────────────────────────────────────
    /// Core event data keyed by event_id
    Event(u64),

    /// Individual match keyed by match_id
    Match(u64),

    /// A user's prediction keyed by prediction_id
    Prediction(u64),

    // ── Relationship / index keys ────────────────────────────────────────────
    /// Vec<u64> of match IDs belonging to an event  (event_id)
    EventMatches(u64),

    /// Vec<u64> of prediction IDs for a match  (match_id)
    MatchPredictions(u64),

    /// Vec<u64> of prediction IDs a user has placed in an event  (user, event_id)
    UserPredictions(Address, u64),

    /// Vec<Address> of participants for an event  (event_id)
    EventParticipants(u64),

    /// Vec<Winner> of verified winners for an event  (event_id)
    EventWinners(u64),

    // ── Initialization sentinel ──────────────────────────────────────────────
    /// Set to `true` once `initialize` has been called; prevents re-init.
    Initialized,

    // ── Canonical address lookups (no-payload keys for retrieval) ────────────
    /// Current treasury address — updated by set_treasury; used for fee routing.
    CurrentTreasury,

    /// Current AI agent address — updated by set_ai_agent; used for oracle auth.
    CurrentAIAgent,

    // ── Verification keys (#790–#793) ────────────────────────────────────────
    /// Verification status for an address — true = verified, false = not verified.
    VerifiedAddresses(Address),

    // ── Event invite code index (#795) ───────────────────────────────────────
    /// Maps an 8-character invite code Symbol → event_id (u64).
    InviteCode(Symbol),

    // ── Canonical XLM token key (#794) ───────────────────────────────────────
    /// Current XLM token contract address — set during initialize.
    CurrentXLMToken,
}

// ---------------------------------------------------------------------------
// Event
// ---------------------------------------------------------------------------

/// Core event struct — all information about a creator's prediction event.
///
/// Stored under `DataKey::Event(event_id)`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    /// Auto-incremented unique identifier
    pub event_id: u64,

    /// Address of the creator; only they can manage the event
    pub creator: Address,

    /// Human-readable title (max `MAX_TITLE_LEN` chars)
    pub title: String,

    /// Full description / rules (max `MAX_DESCRIPTION_LEN` chars)
    pub description: String,

    /// XLM fee (in stroops) the creator paid to create the event
    pub creation_fee_paid: i128,

    /// Unix timestamp when the event was created
    pub created_at: u64,

    /// Whether the event is open for new predictions
    pub is_active: bool,

    /// Whether the event has been cancelled
    pub is_cancelled: bool,

    /// 8-character invite code used for private events
    pub invite_code: Symbol,

    /// Hard cap on participants (0 = unlimited)
    pub max_participants: u32,

    /// Current number of confirmed participants
    pub participant_count: u32,

    /// Number of matches that belong to this event
    pub match_count: u32,
}

impl Event {
    /// Construct a new active, uncancelled event.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_id: u64,
        creator: Address,
        title: String,
        description: String,
        creation_fee_paid: i128,
        created_at: u64,
        invite_code: Symbol,
        max_participants: u32,
    ) -> Self {
        Self {
            event_id,
            creator,
            title,
            description,
            creation_fee_paid,
            created_at,
            is_active: true,
            is_cancelled: false,
            invite_code,
            max_participants,
            participant_count: 0,
            match_count: 0,
        }
    }

    /// Returns `true` when the event can still accept new participants.
    pub fn can_accept_participants(&self) -> bool {
        if !self.is_active || self.is_cancelled {
            return false;
        }
        // max_participants == 0 means unlimited
        self.max_participants == 0 || self.participant_count < self.max_participants
    }

    /// Close the event for new predictions without cancelling it.
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// Cancel the event entirely.
    pub fn cancel(&mut self) {
        self.is_active = false;
        self.is_cancelled = true;
    }

    /// Register a new participant.  Returns `Err` if the event is full or inactive.
    pub fn add_participant(&mut self) -> Result<(), &'static str> {
        if self.is_cancelled {
            return Err("Event is cancelled");
        }
        if !self.is_active {
            return Err("Event is not active");
        }
        if self.max_participants > 0 && self.participant_count >= self.max_participants {
            return Err("Event has reached maximum participants");
        }
        self.participant_count += 1;
        Ok(())
    }

    /// Increment the match counter when a new match is added.
    pub fn add_match(&mut self) {
        self.match_count += 1;
    }

    /// Age of the event in seconds relative to `current_timestamp`.
    pub fn get_age_seconds(&self, current_timestamp: u64) -> u64 {
        current_timestamp.saturating_sub(self.created_at)
    }

    /// Validate title and description lengths.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.title.len() == 0 {
            return Err("Title cannot be empty");
        }
        if self.title.len() > MAX_TITLE_LEN {
            return Err("Title exceeds maximum length");
        }
        if self.description.len() > MAX_DESCRIPTION_LEN {
            return Err("Description exceeds maximum length");
        }
        Ok(())
    }

    /// Returns true when the title length is within the 200-character limit.
    pub fn is_valid_title(title: &String) -> bool {
        title.len() <= MAX_TITLE_LEN
    }

    /// Returns true when the description length is within the 1000-character limit.
    pub fn is_valid_description(description: &String) -> bool {
        description.len() <= MAX_DESCRIPTION_LEN
    }
}

// ---------------------------------------------------------------------------
// Match
// ---------------------------------------------------------------------------

/// A single prediction match within an event.
///
/// Stored under `DataKey::Match(match_id)`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Match {
    /// Unique identifier (global, assigned via MatchCounter)
    pub match_id: u64,

    /// ID of the parent event
    pub event_id: u64,

    /// Name of the first team / option (max `MAX_TEAM_NAME_LEN` chars)
    pub team_a: String,

    /// Name of the second team / option (max `MAX_TEAM_NAME_LEN` chars)
    pub team_b: String,

    /// Scheduled start time (Unix timestamp in seconds)
    pub match_time: u64,

    /// Whether a result has been submitted
    pub result_submitted: bool,

    /// The winning outcome; `None` until a result is submitted.
    /// Stored as `Option<u32>` (0=TeamA, 1=TeamB, 2=Draw) because Soroban's
    /// `#[contracttype]` does not support `Option<EnumType>` directly.
    pub winning_team: Option<u32>,

    /// Address of the oracle / admin that submitted the result
    pub submitted_by: Option<Address>,

    /// Unix timestamp when the result was submitted
    pub submitted_at: Option<u64>,
}

impl Match {
    /// Create a new pending match.
    pub fn new(
        match_id: u64,
        event_id: u64,
        team_a: String,
        team_b: String,
        match_time: u64,
    ) -> Self {
        Self {
            match_id,
            event_id,
            team_a,
            team_b,
            match_time,
            result_submitted: false,
            winning_team: None,
            submitted_by: None,
            submitted_at: None,
        }
    }

    // -----------------------------------------------------------------------
    // Result management
    // -----------------------------------------------------------------------

    /// Submit a result for this match.
    ///
    /// # Errors
    /// Returns `Err` if a result has already been submitted.
    pub fn submit_result(
        &mut self,
        result: MatchResult,
        submitted_by: Address,
        timestamp: u64,
    ) -> Result<(), &'static str> {
        if self.result_submitted {
            return Err("Result already submitted for this match");
        }
        self.winning_team = Some(result.to_u32());
        self.submitted_by = Some(submitted_by);
        self.submitted_at = Some(timestamp);
        self.result_submitted = true;
        Ok(())
    }

    /// Return the winning `MatchResult`, or `None` if not yet submitted.
    pub fn get_winner(&self) -> Option<MatchResult> {
        self.winning_team.and_then(MatchResult::from_u32)
    }

    /// `true` once a result has been recorded.
    pub fn is_completed(&self) -> bool {
        self.result_submitted
    }

    // -----------------------------------------------------------------------
    // Timing helpers
    // -----------------------------------------------------------------------

    /// `true` if `current_time >= match_time`.
    pub fn has_started(&self, current_time: u64) -> bool {
        current_time >= self.match_time
    }

    /// `true` if the match has started but no result has been submitted yet.
    pub fn is_ready_for_result(&self, current_time: u64) -> bool {
        self.has_started(current_time) && !self.result_submitted
    }

    /// Seconds until the match starts; 0 if already started.
    pub fn time_until_start(&self, current_time: u64) -> u64 {
        if current_time >= self.match_time {
            0
        } else {
            self.match_time - current_time
        }
    }

    /// Seconds since the result was submitted; 0 if no result yet.
    pub fn time_since_result(&self, current_time: u64) -> u64 {
        match self.submitted_at {
            Some(t) => current_time.saturating_sub(t),
            None => 0,
        }
    }

    // -----------------------------------------------------------------------
    // Prediction window
    // -----------------------------------------------------------------------

    /// `true` if predictions are still open.
    ///
    /// Predictions close `prediction_cutoff_minutes` before `match_time` and
    /// are always closed once a result has been submitted.
    pub fn allows_predictions(&self, current_time: u64, prediction_cutoff_minutes: u64) -> bool {
        let cutoff = self
            .match_time
            .saturating_sub(prediction_cutoff_minutes * 60);
        current_time < cutoff && !self.result_submitted
    }

    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------

    /// Validate team names and internal state consistency.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.team_a.len() == 0 {
            return Err("Team A name cannot be empty");
        }
        if self.team_a.len() > MAX_TEAM_NAME_LEN {
            return Err("Team A name exceeds maximum length");
        }
        if self.team_b.len() == 0 {
            return Err("Team B name cannot be empty");
        }
        if self.team_b.len() > MAX_TEAM_NAME_LEN {
            return Err("Team B name exceeds maximum length");
        }
        if self.team_a == self.team_b {
            return Err("Team names must be different");
        }

        // Result consistency
        if self.result_submitted {
            if self.winning_team.is_none() {
                return Err("Result submitted but winning_team is None");
            }
            if self.submitted_by.is_none() {
                return Err("Result submitted but submitted_by is None");
            }
            if self.submitted_at.is_none() {
                return Err("Result submitted but submitted_at is None");
            }
            if let Some(v) = self.winning_team {
                if v > 2 {
                    return Err("winning_team value must be 0 (TeamA), 1 (TeamB), or 2 (Draw)");
                }
            }
        } else {
            if self.winning_team.is_some() {
                return Err("winning_team set but result_submitted is false");
            }
            if self.submitted_at.is_some() {
                return Err("submitted_at set but result_submitted is false");
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Prediction
// ---------------------------------------------------------------------------

/// A user's prediction for a single match inside an event.
///
/// Stored under `DataKey::Prediction(prediction_id)`.
///
/// The `predicted_outcome` field uses a `Symbol` with one of three values:
/// `"TEAM_A"`, `"TEAM_B"`, or `"DRAW"` (see `OUTCOME_*` constants).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Prediction {
    /// Global unique identifier assigned via PredictionCounter
    pub prediction_id: u64,

    /// Match this prediction is for
    pub match_id: u64,

    /// Parent event identifier
    pub event_id: u64,

    /// Address of the user who placed this prediction
    pub predictor: Address,

    /// Predicted outcome: Symbol of "TEAM_A", "TEAM_B", or "DRAW"
    pub predicted_outcome: Symbol,

    /// Unix timestamp when the prediction was submitted
    pub predicted_at: u64,

    /// `Some(true)` = correct, `Some(false)` = wrong, `None` = not yet graded
    pub is_correct: Option<bool>,
}

impl Prediction {
    /// Create a new ungraded prediction.
    pub fn new(
        prediction_id: u64,
        match_id: u64,
        event_id: u64,
        predictor: Address,
        predicted_outcome: Symbol,
        predicted_at: u64,
    ) -> Self {
        Self {
            prediction_id,
            match_id,
            event_id,
            predictor,
            predicted_outcome,
            predicted_at,
            is_correct: None,
        }
    }

    /// Validate that `predicted_outcome` is one of the three legal symbols.
    ///
    /// Valid values: `"TEAM_A"`, `"TEAM_B"`, `"DRAW"`.
    pub fn validate_outcome(env: &soroban_sdk::Env, outcome: &Symbol) -> Result<(), &'static str> {
        let team_a = Symbol::new(env, OUTCOME_TEAM_A);
        let team_b = Symbol::new(env, OUTCOME_TEAM_B);
        let draw = Symbol::new(env, OUTCOME_DRAW);

        if *outcome == team_a || *outcome == team_b || *outcome == draw {
            Ok(())
        } else {
            Err("predicted_outcome must be TEAM_A, TEAM_B, or DRAW")
        }
    }

    /// Grade this prediction against the actual match result symbol.
    pub fn grade(&mut self, actual_outcome: &Symbol) {
        self.is_correct = Some(self.predicted_outcome == *actual_outcome);
    }

    /// `true` if the prediction has been graded and was correct.
    pub fn is_winner(&self) -> bool {
        self.is_correct == Some(true)
    }

    /// Returns `true` if `predicted_at` is strictly before `match_time`.
    ///
    /// Used to verify the prediction was placed before the match started.
    pub fn is_before_match_time(&self, match_time: u64) -> bool {
        self.predicted_at < match_time
    }
}

// ---------------------------------------------------------------------------
// Winner
// ---------------------------------------------------------------------------

/// Records a user who correctly predicted matches in an event.
///
/// Stored inside the `Vec<Winner>` at `DataKey::EventWinners(event_id)`.
/// Used for leaderboard ranking and reward distribution.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Winner {
    /// Address of the winning predictor
    pub user: Address,

    /// Event they participated in
    pub event_id: u64,

    /// How many matches they predicted correctly
    pub total_correct: u32,

    /// Total number of matches in the event (denominator for accuracy)
    pub total_matches: u32,

    /// Unix timestamp when the user submitted their last prediction
    /// (used as tiebreaker — earlier completion ranks higher)
    pub completion_time: u64,

    /// Unix timestamp when winner status was verified on-chain
    pub verified_at: u64,
}

impl Winner {
    /// Construct a new verified winner record.
    pub fn new(
        user: Address,
        event_id: u64,
        total_correct: u32,
        total_matches: u32,
        completion_time: u64,
        verified_at: u64,
    ) -> Self {
        Self {
            user,
            event_id,
            total_correct,
            total_matches,
            completion_time,
            verified_at,
        }
    }

    /// Returns accuracy as an integer percentage in the range [0, 100].
    ///
    /// Returns 0 when `total_matches` is 0 to avoid division by zero.
    pub fn get_accuracy_percentage(&self) -> u32 {
        if self.total_matches == 0 {
            return 0;
        }
        (self.total_correct * 100) / self.total_matches
    }

    /// Returns `true` if this winner outranks `other` for leaderboard purposes.
    ///
    /// Primary sort: higher `total_correct` wins.
    /// Tiebreaker: earlier `completion_time` wins (submitted predictions sooner).
    pub fn outranks(&self, other: &Winner) -> bool {
        if self.total_correct != other.total_correct {
            return self.total_correct > other.total_correct;
        }
        // Earlier completion time is better (lower value = higher rank)
        self.completion_time < other.completion_time
    }
}
