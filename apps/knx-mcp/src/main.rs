//! `knx-mcp`: serves saved KNXBench projects read-only over MCP stdio.
//!
//! stdout carries MCP messages only; every human-readable line goes to
//! stderr, as the stdio transport requires.

use std::process::ExitCode;
use std::sync::Arc;

use knx_mcp::args::{self, Invocation};
use knx_mcp::server::KnxMcp;
use knx_mcp::workspace::Workspace;
use rmcp::ServiceExt;

fn version_line() -> String {
    match option_env!("KNX_BUILD_SHA").filter(|sha| !sha.is_empty()) {
        Some(sha) => format!("knx-mcp {}+g{sha}", env!("CARGO_PKG_VERSION")),
        None => format!("knx-mcp {}", env!("CARGO_PKG_VERSION")),
    }
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let config = match args::parse(&argv) {
        Ok(Invocation::Serve(config)) => config,
        Ok(Invocation::Help) => {
            println!("{}", args::USAGE);
            return ExitCode::SUCCESS;
        }
        Ok(Invocation::Version) => {
            println!("{}", version_line());
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("knx-mcp: {error}\n\n{}", args::USAGE);
            return ExitCode::from(2);
        }
    };
    let ws = match Workspace::open(&config) {
        Ok(ws) => Arc::new(ws),
        Err(error) => {
            eprintln!("knx-mcp: {error}");
            return ExitCode::from(2);
        }
    };
    eprintln!(
        "{}: serving {} project(s) read-only over stdio ({}); product database: {}",
        version_line(),
        ws.aliases().len(),
        ws.aliases().join(", "),
        if ws.products().is_some() {
            "read-only"
        } else {
            "none"
        }
    );

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("knx-mcp: cannot start the async runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    let outcome = runtime.block_on(async move {
        let service = KnxMcp::new(ws)
            .serve(rmcp::transport::stdio())
            .await
            .map_err(|e| e.to_string())?;
        service.waiting().await.map_err(|e| e.to_string())?;
        Ok::<(), String>(())
    });
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("knx-mcp: {error}");
            ExitCode::FAILURE
        }
    }
}
