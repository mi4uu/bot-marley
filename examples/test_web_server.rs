use botmarley::web_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting web server test on port 3040...");
    
    if let Err(e) = web_server::start_web_server(3040).await {
        eprintln!("Web server error: {}", e);
        return Err(e);
    }
    
    Ok(())
}