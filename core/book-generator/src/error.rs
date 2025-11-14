use thiserror::Error;

#[derive(Error, Debug)]
pub enum BookGeneratorError {
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    
    #[error("LLM error: {0}")]
    LLMError(String),
    
    #[error("Chain error: {0}")]
    ChainError(#[from] langchain_rust::chain::ChainError),
    
    #[error("Prompt error: {0}")]
    PromptError(#[from] langchain_rust::prompt::PromptError),
    
    #[error("Generation error: {0}")]
    Generation(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Config error: {0}")]
    ConfigError(String),
    
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),
    
    #[error("Unsupported LLM provider: {0}")]
    UnsupportedLLMProvider(String),
    
    #[error("Other error: {0}")]
    Other(String),
    
    #[error("Duplicate content error: {0}")]
    DuplicateContent(String),
    
    #[error("Missing context: {0}")]
    MissingContext(String),
    
    #[error("Duplicate chapter title: {0}")]
    DuplicateChapterTitle(String),
}

pub type Result<T> = std::result::Result<T, BookGeneratorError>;

impl From<langchain_rust::language_models::LLMError> for BookGeneratorError {
    fn from(err: langchain_rust::language_models::LLMError) -> Self {
        BookGeneratorError::LLMError(err.to_string())
    }
}

impl From<serde_json::Error> for BookGeneratorError {
    fn from(err: serde_json::Error) -> Self {
        BookGeneratorError::SerializationError(format!("JSON error: {}", err))
    }
}