use std::sync::Arc;
use std::time::Duration;
use reqwest::{Client};
use serde_json::{json, Value};
use tokio::spawn;

pub async fn send_requests(prompt: String, question: String) -> Vec<Option<String>> {
    let mut answers = vec![];
    let mut tasks = vec![];

    let arc_prompt = Arc::new(prompt);
    let arc_question = Arc::new(question);
    
    tasks.push(spawn(ask_gemini(arc_prompt.clone(), arc_question.clone())));
    tasks.push(spawn(ask_grok(arc_prompt.clone(), arc_question.clone())));
    tasks.push(spawn(ask_mistral(arc_prompt.clone(), arc_question.clone())));

    for i in tasks {
        if let Ok(ans) = i.await {
            answers.push(ans)
        }
        else {
            answers.push(None)
        }
    }

    answers
}

pub async fn ask_gemini(prompt: Arc<String>, text: Arc<String>) -> Option<String> {
    let request_text = format!("{} The question from user is: {}", prompt, text);
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("No build");

    let Ok(maybe_token) = std::fs::read("./gemini.token") else {
        return None
    };
    let token = String::from_utf8(maybe_token).unwrap();

    let response = client
        .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent")
        .header("x-goog-api-key", token.trim())
        .json(&json!({
        "contents": [{
            "parts": [{
                "text": request_text
            }]
        }]
    })).send()
        .await
        .expect("No send gemini");

    let future_json: Value = response.json().await.expect("No text");
    if let Some(ans) = future_json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
        Some(ans.to_string())
    }
    else {
        None
    }
}

pub async fn ask_grok(prompt: Arc<String>, text: Arc<String>) -> Option<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("No build");

    let Ok(maybe_token) = std::fs::read("./grok.token") else {
        return None
    };
    let token = String::from_utf8(maybe_token).unwrap();

    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", token.trim()))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": "x-ai/grok-4.1-fast",
            "messages": [
                {
                    "role": "system",
                    "content": &*prompt
                },
                {
                    "role": "user",
                    "content": &*text
                }
            ],
            "stream": false
        })).send()
        .await
        .expect("No send grok");

    let Ok(future_text) = response.text().await else {
        return None
    };

    let json_text: Value = serde_json::from_str(future_text.as_str()).expect("No json");
    if let Some(ans) = json_text["choices"][0]["message"]["content"].as_str() {
        Some(ans.to_string())
    }
    else {
        None
    }
}


pub async fn ask_mistral(prompt: Arc<String>, text: Arc<String>) -> Option<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("No build");

    let Ok(maybe_token) = std::fs::read("./mistral.token") else {
        return None
    };
    let token = String::from_utf8(maybe_token).unwrap();

    let response = client
        .post("https://api.mistral.ai/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", token.trim()))
        .json(&json!({
            "model": "mistral-large-latest",
            "messages": [
                {
                    "role": "system",
                    "content": &*prompt
                },
                {
                    "role": "user",
                    "content": &*text
                }
            ],
            "stream": false
        })).send()
        .await
        .expect("No send mistral");

    let Ok(future_text) = response.text().await else {
        return None
    };

    let json_text: Value = serde_json::from_str(future_text.as_str()).expect("No json");
    if let Some(ans) = json_text["choices"][0]["message"]["content"].as_str() {
        Some(ans.to_string())
    }
    else {
        None
    }
}
