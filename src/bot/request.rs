
async fn make_llm_request(config: &Config, system_message: &str, user_message: &str) -> color_eyre::Result<TradingDecision> {
    let client = Client::new();
   let schema = serde_json::json!({
    "type": "object",
    "properties": {
          "thinking": {  
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "step": {
                        "type": "integer"
                    },
                    "thought": {
                        "type": "string"
                    },
                    "require_next_step":{
                        "type": "boolean"
                    },
                     "conclusion": {
                        "type": "string"
                    }
                },
                "required": ["step", "thought", "require_next_step","conclusion" ],
                "additionalProperties": false
            }
        },
         "reasoning": {
            "type": "string"
        },
        "action": {
            "type": "string",
            "enum": ["buy", "sell", "hold"]
        },
        "confidence": {
            "type": "number",
            "minimum": 0.0,
            "maximum": 1.0
        },
       
        
        "price_target": {
            "type": "number"
        },
        "stop_loss": {
            "type": "number"
        },
      
    },
    "required": ["thinking", "reasoning", "action", "confidence" ],  
    "additionalProperties": false
});
    let request = LLMRequest {
        model: config.openai_model.clone(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: system_message.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: user_message.to_string(),
            },
        ],
        // temperature: 0.1,
        response_format: ResponseFormat {
            format_type: "json_schema".to_string(),
            json_schema: JsonSchema {
                name: "trading_decision".to_string(),
                strict: true,
                schema,
            },
        },
    };

    let response = client
        .post(&format!("{}/v1/chat/completions", config.openai_base_url))
        .header("Authorization", format!("Bearer {}", config.openai_api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .wrap_err("Failed to send request to LLM")?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(color_eyre::eyre::eyre!("LLM API error: {}", error_text));
    }

    let llm_response: LLMResponse = response
        .json()
        .await
        .wrap_err("Failed to parse LLM response")?;

    let content = &llm_response.choices[0].message.content;
    let decision: TradingDecision = serde_json::from_str(content)
        .wrap_err("Failed to parse trading decision JSON")?;
    info!("DECISION: {}", serde_json::to_string_pretty(&decision).unwrap());
    Ok(decision)
}