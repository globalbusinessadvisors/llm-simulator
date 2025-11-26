//! Response generation implementation

use rand::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::config::GenerationConfig;
use crate::types::Message;

/// Generates simulated LLM responses
pub struct ResponseGenerator {
    rng: StdRng,
    templates: Vec<String>,
}

impl ResponseGenerator {
    /// Create a new generator with random seed
    pub fn new(seed: Option<u64>) -> Self {
        let rng = match seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_entropy(),
        };

        Self {
            rng,
            templates: default_templates(),
        }
    }

    /// Create with a specific seed
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
            templates: default_templates(),
        }
    }

    /// Generate a response based on input messages
    pub fn generate_response(
        &self,
        messages: &[Message],
        max_tokens: u32,
        config: &GenerationConfig,
    ) -> (String, u32) {
        let mut rng = self.rng.clone();

        // Determine target length
        let target_tokens = rng.gen_range(config.min_tokens..=config.max_tokens.min(max_tokens));

        // Generate based on strategy
        let content = match &config.strategy {
            crate::config::GenerationStrategy::Template => {
                self.generate_from_templates(messages, target_tokens as usize, &mut rng, config)
            }
            crate::config::GenerationStrategy::Lorem => {
                generate_lorem(target_tokens as usize, &mut rng)
            }
            crate::config::GenerationStrategy::Echo => {
                self.generate_echo(messages, target_tokens as usize)
            }
            crate::config::GenerationStrategy::Fixed => {
                config.templates.first()
                    .cloned()
                    .unwrap_or_else(|| "This is a simulated response.".to_string())
            }
            crate::config::GenerationStrategy::Random => {
                generate_random_text(target_tokens as usize, &mut rng)
            }
        };

        // Estimate actual tokens
        let actual_tokens = estimate_tokens(&content);

        (content, actual_tokens)
    }

    /// Generate response from templates
    fn generate_from_templates(
        &self,
        messages: &[Message],
        target_tokens: usize,
        rng: &mut StdRng,
        config: &GenerationConfig,
    ) -> String {
        let templates = if config.templates.is_empty() {
            &self.templates
        } else {
            &config.templates
        };

        // Select base template
        let template = templates.choose(rng).unwrap_or(&self.templates[0]);

        // Build response
        let mut response = template.clone();

        // Expand to target length
        let target_chars = target_tokens * 4; // ~4 chars per token
        while response.len() < target_chars {
            response.push_str("\n\n");
            response.push_str(self.generate_paragraph(messages, rng));
        }

        // Truncate if needed
        if response.len() > target_chars {
            response.truncate(target_chars);
            // Clean up at word boundary
            if let Some(last_space) = response.rfind(' ') {
                response.truncate(last_space);
            }
        }

        response
    }

    /// Generate a contextual paragraph
    fn generate_paragraph(&self, messages: &[Message], rng: &mut StdRng) -> &str {
        // Context-aware paragraph selection based on last message
        let last_message = messages.last().map(|m| m.text()).unwrap_or_default();

        let paragraphs = if last_message.contains('?') {
            QUESTION_RESPONSES
        } else if last_message.to_lowercase().contains("code")
            || last_message.to_lowercase().contains("program") {
            CODE_RESPONSES
        } else if last_message.to_lowercase().contains("explain") {
            EXPLANATION_RESPONSES
        } else {
            GENERAL_RESPONSES
        };

        paragraphs.choose(rng).unwrap_or(&GENERAL_RESPONSES[0])
    }

    /// Generate echo response
    fn generate_echo(&self, messages: &[Message], target_tokens: usize) -> String {
        let last_message = messages.last()
            .map(|m| m.text())
            .unwrap_or_default();

        let response = format!(
            "I understand you're asking about: \"{}\"\n\nHere's my response to that:",
            last_message
        );

        // Pad to target length
        let target_chars = target_tokens * 4;
        if response.len() < target_chars {
            let padding = generate_lorem(
                (target_chars - response.len()) / 4,
                &mut rand::thread_rng(),
            );
            format!("{}\n\n{}", response, padding)
        } else {
            response
        }
    }

    /// Generate an embedding vector
    pub fn generate_embedding(&self, dimensions: usize, input: &str) -> Vec<f32> {
        // Use input hash for determinism
        let mut hasher = DefaultHasher::new();
        input.hash(&mut hasher);
        let seed = hasher.finish();

        let mut rng = StdRng::seed_from_u64(seed);
        let mut embedding: Vec<f32> = (0..dimensions)
            .map(|_| rng.gen_range(-1.0..1.0))
            .collect();

        // Normalize to unit vector
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for x in &mut embedding {
                *x /= magnitude;
            }
        }

        embedding
    }

    /// Tokenize text for streaming (simple word-based)
    pub fn tokenize(&self, text: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();

        for c in text.chars() {
            current.push(c);
            // Split on spaces, punctuation, or every ~4 chars
            if c.is_whitespace() || c.is_ascii_punctuation() || current.len() >= 4 {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
        }

        if !current.is_empty() {
            tokens.push(current);
        }

        tokens
    }
}

impl Default for ResponseGenerator {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Estimate token count from text
fn estimate_tokens(text: &str) -> u32 {
    // Rough estimation: ~4 characters per token
    ((text.len() as f64 / 4.0).ceil() as u32).max(1)
}

/// Generate lorem ipsum text
fn generate_lorem<R: rand::Rng>(target_tokens: usize, rng: &mut R) -> String {
    let words = [
        "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit",
        "sed", "do", "eiusmod", "tempor", "incididunt", "ut", "labore", "et", "dolore",
        "magna", "aliqua", "enim", "ad", "minim", "veniam", "quis", "nostrud",
        "exercitation", "ullamco", "laboris", "nisi", "aliquip", "ex", "ea", "commodo",
        "consequat", "duis", "aute", "irure", "in", "reprehenderit", "voluptate",
        "velit", "esse", "cillum", "fugiat", "nulla", "pariatur", "excepteur", "sint",
        "occaecat", "cupidatat", "non", "proident", "sunt", "culpa", "qui", "officia",
        "deserunt", "mollit", "anim", "id", "est", "laborum",
    ];

    let mut result = Vec::new();
    let target_words = target_tokens; // Roughly 1 token per word

    for _ in 0..target_words {
        result.push(*words.choose(rng).unwrap());
    }

    // Capitalize first word and add punctuation
    let mut text = result.join(" ");
    if let Some(first) = text.get_mut(0..1) {
        first.make_ascii_uppercase();
    }

    // Add sentences
    let mut chars: Vec<char> = text.chars().collect();
    let mut word_count = 0;
    for i in 0..chars.len() {
        if chars[i] == ' ' {
            word_count += 1;
            if word_count % 12 == 0 && i + 1 < chars.len() {
                chars[i - 1] = '.';
                if let Some(c) = chars.get_mut(i + 1) {
                    *c = c.to_ascii_uppercase();
                }
            }
        }
    }

    chars.iter().collect()
}

/// Generate random text from a vocabulary
fn generate_random_text(target_tokens: usize, rng: &mut StdRng) -> String {
    let vocab = [
        "the", "a", "is", "are", "was", "were", "have", "has", "had", "do", "does",
        "did", "will", "would", "could", "should", "may", "might", "must", "can",
        "this", "that", "these", "those", "it", "they", "we", "you", "he", "she",
        "system", "data", "model", "process", "function", "result", "value", "type",
        "input", "output", "request", "response", "error", "success", "status",
        "configuration", "parameter", "option", "setting", "property", "attribute",
    ];

    let words: Vec<&str> = (0..target_tokens)
        .map(|_| *vocab.choose(rng).unwrap())
        .collect();

    words.join(" ")
}

/// Default response templates
fn default_templates() -> Vec<String> {
    vec![
        "I'd be happy to help you with that. Let me provide a detailed response.".to_string(),
        "Based on your request, here's what I can tell you.".to_string(),
        "That's a great question. Let me explain.".to_string(),
        "I understand what you're looking for. Here's my analysis.".to_string(),
        "Let me address your query comprehensively.".to_string(),
    ]
}

// Response paragraph collections
static QUESTION_RESPONSES: &[&str] = &[
    "To answer your question directly, the key consideration here is understanding the underlying principles involved.",
    "The answer depends on several factors that we should examine carefully.",
    "There are multiple perspectives to consider when addressing this question.",
    "Let me break down the answer into manageable parts for clarity.",
];

static CODE_RESPONSES: &[&str] = &[
    "Here's an implementation approach that follows best practices and maintains code clarity.",
    "The solution involves several components working together efficiently.",
    "This code pattern is commonly used in production systems for its reliability.",
    "Consider this implementation which balances performance with maintainability.",
];

static EXPLANATION_RESPONSES: &[&str] = &[
    "To understand this concept, we need to start with the fundamentals.",
    "This works by combining several mechanisms that interact in specific ways.",
    "The underlying principle is based on well-established patterns in the field.",
    "Let me walk you through the key components and how they relate to each other.",
];

static GENERAL_RESPONSES: &[&str] = &[
    "This is an important topic that deserves careful consideration.",
    "There are several aspects to explore in this area.",
    "The approach I recommend takes into account multiple factors.",
    "Based on the available information, here's what we can determine.",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GenerationStrategy;

    #[test]
    fn test_generator_creation() {
        let gen = ResponseGenerator::new(Some(42));
        assert!(!gen.templates.is_empty());
    }

    #[test]
    fn test_generate_response() {
        let gen = ResponseGenerator::with_seed(42);
        let messages = vec![Message::user("Hello!")];
        let config = GenerationConfig::default();

        let (response, tokens) = gen.generate_response(&messages, 100, &config);
        assert!(!response.is_empty());
        assert!(tokens > 0);
    }

    #[test]
    fn test_deterministic_generation() {
        let gen1 = ResponseGenerator::with_seed(42);
        let gen2 = ResponseGenerator::with_seed(42);
        let messages = vec![Message::user("Test")];
        let config = GenerationConfig::default();

        let (resp1, _) = gen1.generate_response(&messages, 50, &config);
        let (resp2, _) = gen2.generate_response(&messages, 50, &config);

        assert_eq!(resp1, resp2);
    }

    #[test]
    fn test_lorem_generation() {
        let gen = ResponseGenerator::with_seed(42);
        let messages = vec![Message::user("Test")];
        let config = GenerationConfig {
            strategy: GenerationStrategy::Lorem,
            min_tokens: 20,
            max_tokens: 30,
            ..Default::default()
        };

        let (response, _) = gen.generate_response(&messages, 100, &config);
        assert!(response.to_lowercase().contains("lorem") || response.len() > 0);
    }

    #[test]
    fn test_embedding_generation() {
        let gen = ResponseGenerator::with_seed(42);
        let embedding = gen.generate_embedding(1536, "test input");

        assert_eq!(embedding.len(), 1536);

        // Check normalization (magnitude should be ~1)
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_deterministic_embedding() {
        let gen = ResponseGenerator::new(None);

        let emb1 = gen.generate_embedding(100, "same input");
        let emb2 = gen.generate_embedding(100, "same input");

        // Same input should produce same embedding
        assert_eq!(emb1, emb2);
    }

    #[test]
    fn test_tokenize() {
        let gen = ResponseGenerator::new(None);
        let tokens = gen.tokenize("Hello, world! How are you?");

        assert!(!tokens.is_empty());
        assert!(tokens.len() > 3); // Should split into multiple tokens
    }

    #[test]
    fn test_token_estimation() {
        assert_eq!(estimate_tokens("test"), 1);
        assert_eq!(estimate_tokens("hello world"), 3);
        assert!(estimate_tokens("") >= 1);
    }
}
