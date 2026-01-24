use std::thread::sleep;
use std::time::Duration;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::spawn;

pub async fn summary(texts: [String; 3]) -> Vec<Option<String>> {
    let mut answers = vec![];
    let mut tasks = vec![];
    let [text_1, text_2, text_3] = texts;

    tasks.push(spawn(req_sum(text_1)));
    sleep(Duration::from_millis(1010));
    tasks.push(spawn(req_sum(text_2)));
    sleep(Duration::from_millis(1010));
    tasks.push(spawn(req_sum(text_3)));

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

/// Request summarization. Done by requesting Mistral.
async fn req_sum(text: String) -> Option<String> {
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
                    "content": "Summarize this text in a couple of sentences. Do not write any other words"
                },
                {
                    "role": "user",
                    "content": text
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
