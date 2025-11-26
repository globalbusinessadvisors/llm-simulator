//! Throughput benchmarks for LLM-Simulator

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use llm_simulator::{SimulatorConfig, SimulationEngine};
use llm_simulator::types::{Message, ChatCompletionRequest};
use tokio::runtime::Runtime;

fn create_engine() -> SimulationEngine {
    let mut config = SimulatorConfig::default();
    // Disable latency for pure throughput testing
    config.latency.enabled = false;
    config.seed = Some(42);
    SimulationEngine::new(config)
}

fn bench_chat_completion(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = create_engine();

    let request = ChatCompletionRequest::new(
        "gpt-4",
        vec![
            Message::system("You are a helpful assistant."),
            Message::user("Hello, how are you?"),
        ],
    );

    let mut group = c.benchmark_group("chat_completion");
    group.throughput(Throughput::Elements(1));

    group.bench_function("single_request", |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(engine.chat_completion(&request).await.unwrap())
            })
        })
    });

    group.finish();
}

fn bench_concurrent_requests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = std::sync::Arc::new(create_engine());

    let request = ChatCompletionRequest::new(
        "gpt-4",
        vec![Message::user("Hello")],
    );

    let mut group = c.benchmark_group("concurrent_requests");

    for concurrency in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*concurrency as u64));
        group.bench_function(format!("concurrent_{}", concurrency), |b| {
            b.iter(|| {
                rt.block_on(async {
                    let tasks: Vec<_> = (0..*concurrency).map(|_| {
                        let engine = engine.clone();
                        let req = request.clone();
                        tokio::spawn(async move {
                            engine.chat_completion(&req).await.unwrap()
                        })
                    }).collect();

                    for task in tasks {
                        black_box(task.await.unwrap());
                    }
                })
            })
        });
    }

    group.finish();
}

fn bench_embeddings(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = create_engine();

    use llm_simulator::types::{EmbeddingsRequest, EmbeddingInput};

    let request = EmbeddingsRequest {
        model: "text-embedding-ada-002".to_string(),
        input: EmbeddingInput::Single("Hello, world!".to_string()),
        encoding_format: None,
        dimensions: None,
        user: None,
    };

    let mut group = c.benchmark_group("embeddings");
    group.throughput(Throughput::Elements(1));

    group.bench_function("single_embedding", |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(engine.embeddings(&request).await.unwrap())
            })
        })
    });

    // Batch embeddings
    let batch_request = EmbeddingsRequest {
        model: "text-embedding-ada-002".to_string(),
        input: EmbeddingInput::Multiple(vec![
            "First text".to_string(),
            "Second text".to_string(),
            "Third text".to_string(),
            "Fourth text".to_string(),
            "Fifth text".to_string(),
        ]),
        encoding_format: None,
        dimensions: None,
        user: None,
    };

    group.throughput(Throughput::Elements(5));
    group.bench_function("batch_5_embeddings", |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(engine.embeddings(&batch_request).await.unwrap())
            })
        })
    });

    group.finish();
}

fn bench_response_generation(c: &mut Criterion) {
    use llm_simulator::engine::ResponseGenerator;
    use llm_simulator::config::GenerationConfig;

    let generator = ResponseGenerator::with_seed(42);
    let messages = vec![Message::user("Hello!")];
    let config = GenerationConfig::default();

    let mut group = c.benchmark_group("response_generation");

    for token_count in [100, 500, 1000, 2000].iter() {
        group.throughput(Throughput::Elements(*token_count as u64));
        group.bench_function(format!("generate_{}_tokens", token_count), |b| {
            b.iter(|| {
                black_box(generator.generate_response(&messages, *token_count, &config))
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_chat_completion,
    bench_concurrent_requests,
    bench_embeddings,
    bench_response_generation,
);

criterion_main!(benches);
