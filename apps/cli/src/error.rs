use lightbookinput_dictionary::DictionaryError;
use lightbookinput_learning::LearningError;
use lightbookinput_lm::LmError;
use lightbookinput_neural::NeuralError;
use lightbookinput_platform::ConfigError;
use lightbookinput_predict::PredictError;
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error(transparent)]
    Cold(#[from] crate::cold::error::ColdError),

    #[error(transparent)]
    Dictionary(#[from] DictionaryError),

    #[error(transparent)]
    Neural(#[from] NeuralError),

    #[error(transparent)]
    Learning(#[from] LearningError),

    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error(transparent)]
    Predict(#[from] PredictError),

    #[error(transparent)]
    LanguageModel(#[from] LmError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Replay(#[from] crate::replay::ReplayError),

    #[error(transparent)]
    Eval(#[from] crate::eval::EvalError),

    #[error(transparent)]
    Tune(#[from] crate::tuning::TuneError),
}
