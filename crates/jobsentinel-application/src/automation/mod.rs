//! Candidate-controlled application-assistance facade.

pub use jobsentinel_assistance::{
    ApplicationAttempt, ApplicationProfile, ApplicationProfileInput, AtsDetector, AtsPlatform,
    AutomationError, AutomationPage, AutomationResult, AutomationStats, AutomationStatus,
    BrowserManager, FillResult, FormFiller, ScreeningAnswer,
};
pub use jobsentinel_domain::requires_user_answer;
pub use jobsentinel_storage::automation::{
    AnswerLearningManager, AnswerSource, AnswerStatistics, AnswerSuggestion, AutomationManager,
    ModificationExample, ProfileManager,
};
