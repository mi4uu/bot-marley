use std::time::Duration;

use mono_ai::{Message, MonoAI};
use futures_util::StreamExt;

#[tokio::main]
async fn main() ->Result<(), Box<dyn std::error::Error>>{
    println!("Hello, world!");
    dotenv::dotenv()?;
   let config= botmarley::config::Config::load();
   dbg!(&config);
   dbg!(&config.pairs());
   dbg!(&config.pairs_parts());
    dbg!(&config.symbols());



let mut client = MonoAI::openai_compatible("http://localhost:1234".to_string(), "xxx".to_string(),"openai/gpt-oss-20b".to_string());
client
.add_tool(botmarley::tools::get_prices::get_price_tool()).await?;

 if client.is_fallback_mode().await {
        println!("Using fallback mode for tool calling (model doesn't support native tools)");
    } else {
        println!("Using native tool calling support");
    }
    let mut messages = Vec::new();

      messages.push(Message {
            role: "user".to_string(),
            content: "what is the current BTC to USDC price?".to_string(),
            images: None,
            tool_calls: None,
        });

        loop{
            let mut stream = client.send_chat_request(&messages).await?;
            let mut full_response = String::new();
            let mut tool_calls = None;
            let mut final_usage = None;
        while let Some(item) = stream.next().await {
            let item = item.map_err(|e| format!("Stream error: {}", e))?;
            
            if !item.content.is_empty() {
                print!("{}", item.content);
                // io::stdout().flush()?;
                full_response.push_str(&item.content);
            }
            
            if let Some(tc) = item.tool_calls {
                tool_calls = Some(tc);
            }

            if let Some(usage) = item.usage {
                final_usage = Some(usage);
            }
            
            if item.done {
                break;
            }
        }

          // Add assistant response with tool calls to conversation
        messages.push(Message {
            role: "assistant".to_string(),
            content: full_response,
            images: None,
            tool_calls: tool_calls.clone(), // Include tool calls in the conversation history
        });

        // Handle tool calls
        if let Some(ref tc) = tool_calls {
            // Tool execution status (remove these prints for silent operation)
            for tool_call in tc {
                println!("\n{}", format!("Using {} tool...", tool_call.function.name));
            }
            
            let tool_responses = client.handle_tool_calls(tc.clone()).await;
            
            // Show tool results
            for (tool_call, response) in tc.iter().zip(tool_responses.iter()) {
                // Extract clean result from encoded format for display
                let clean_result = if response.content.starts_with("TOOL_RESULT:") {
                    // Parse "TOOL_RESULT:tool_id:actual_result" and extract actual_result
                    let parts: Vec<&str> = response.content.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        parts[2]
                    } else {
                        &response.content
                    }
                } else {
                    &response.content
                };
                println!("{}", format!("{} called, result: {}", tool_call.function.name, clean_result));
            }
            
            messages.extend(tool_responses);

            // Continue conversation after tool execution  
            print!("{}: ", client.model());
            // io::stdout().flush()?;
            let mut tool_stream = client.send_chat_request(&messages).await?;
            let mut final_response = String::new();
            let mut tool_usage = None;
            while let Some(item) = tool_stream.next().await {
                let item = item.map_err(|e| format!("Stream error: {}", e))?;
                if !item.content.is_empty() {
                    print!("{}", item.content);
                    // io::stdout().flush()?;
                    final_response.push_str(&item.content);
                }
                if let Some(usage) = item.usage {
                    tool_usage = Some(usage);
                }
                if item.done {
                    break;
                }
            }

              // Display tool follow-up usage
            if let Some(usage) = &tool_usage {
                 //   dbg!(usage);
            }
   // Add the final assistant response to conversation
            messages.push(Message {
                role: "assistant".to_string(),
                content: final_response,
                images: None,
                tool_calls: None,
            });
        }
tokio::time::sleep(Duration::from_secs(4)).await;
    }




Ok(())
}
