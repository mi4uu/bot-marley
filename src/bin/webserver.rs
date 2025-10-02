use botmarley::web_server;
use tracing::{info, error};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use color_eyre::eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Install color-eyre for better error reporting
    color_eyre::install()?;
    
    // Initialize basic logging
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .compact()
        )
        .with(EnvFilter::from_default_env().add_directive("botmarley=info".parse()?))
        .init();

    info!("🚀 BotMarley Web Server Only");
    
    // Start web server
    let web_port = 3040u16;
    info!("🌐 Starting web server on port {}", web_port);
    
    if let Err(e) = web_server::start_web_server(web_port).await {
        error!("❌ Web server error: {:?}", e);
        return Err(e);
    }
    
    Ok(())
}