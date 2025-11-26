//! LLM-Simulator CLI
//!
//! Enterprise-grade offline LLM API simulator for testing and development.

use clap::Parser;

use llm_simulator::cli::{Cli, execute};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    execute(cli).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_simulator::cli::Commands;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::try_parse_from(["llm-simulator", "serve"]).unwrap();
        assert!(matches!(cli.command, Commands::Serve(_)));
    }

    #[test]
    fn test_cli_serve_with_args() {
        let cli = Cli::try_parse_from([
            "llm-simulator",
            "serve",
            "--port", "9090",
            "--chaos",
            "--seed", "42",
        ]).unwrap();

        if let Commands::Serve(cmd) = cli.command {
            assert_eq!(cmd.port, 9090);
            assert!(cmd.chaos);
            assert_eq!(cmd.seed, Some(42));
        } else {
            panic!("Expected Serve command");
        }
    }

    #[test]
    fn test_cli_generate() {
        let cli = Cli::try_parse_from([
            "llm-simulator",
            "generate",
            "chat",
            "--message", "Hello",
        ]).unwrap();

        assert!(matches!(cli.command, Commands::Generate(_)));
    }

    #[test]
    fn test_cli_health() {
        let cli = Cli::try_parse_from([
            "llm-simulator",
            "health",
            "--url", "http://localhost:8080",
        ]).unwrap();

        assert!(matches!(cli.command, Commands::Health(_)));
    }
}
