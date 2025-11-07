use tokio::spawn;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicI16, Ordering};
use std::time::Duration;
use reqwest::{Client};
use serde_json::{json, Value};

pub async fn request_sum (apis: Arc<[&'static str; 3]>, answer: Arc<[Mutex<String>; 3]>, text: Arc<String>, counter: Arc<AtomicI16>, ready: Arc<[AtomicBool; 3]>) {
    spawn(summarize_text(apis.clone(), answer.clone(), text.clone(), ready.clone(), 0));
    spawn(summarize_text(apis.clone(), answer.clone(), text.clone(), ready.clone(), 1));
    spawn(summarize_text(apis.clone(), answer.clone(), text.clone(), ready.clone(), 2));
    spawn(sleeping(counter, ready.clone())).await.expect("No sleep");
}
async fn summarize_text (apis: Arc<[&'static str; 3]>, answer: Arc<[Mutex<String>; 3]>, text: Arc<String>, ready: Arc<[AtomicBool; 3]>, num: usize) {
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .expect("No build");

    let summary = match apis[num] {
        "apy" => create_req_apy(&client, text).await,
        "gemini" => create_req_gemini(&client, text).await,
        "huggingface" => create_req_hug(&client, text).await,
        "cohere" => create_req_cohere(&client, text).await,
        _ => "Unknown site name".to_string()
    };

    let mut ans = answer[num].lock().unwrap();
    *ans = summary;

    ready[num].store(true, Ordering::Release)
}

async fn sleeping (counter: Arc<AtomicI16>, ready: Arc<[AtomicBool; 3]>) {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        if ready.iter().all(|x| x.load(Ordering::Relaxed)) {
            return 
        }
        
        counter.fetch_add(1, Ordering::AcqRel);
    }
}

async fn create_req_gemini (client: &Client, text: Arc<String>) -> String {
    let response = client
        .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent")
        .header("x-goog-api-key", "token")
        .json(&json!({
        "contents": [{
            "parts": [{
                "text": format!("Summarize this text: {}", text)
            }]
        }]
    })).send()
        .await
        .expect("No send");

    let future_json: Value = response.json().await.expect("No text");
    future_json["candidates"][0]["content"]["parts"][0]["text"].as_str().expect("No str").to_string()
}

async fn create_req_hug(client: &Client, text: Arc<String>) -> String {
    let response = client
        .post("https://api-inference.huggingface.co/models/sshleifer/distilbart-cnn-12-6")
        .header("Authorization", "Bearer token")
        .json(&json!({
            "inputs": *text,
            "parameters": {
                "min_length": 30,
                "max_length": 150
            }
        })).send()
        .await
        .expect("No send");
    
    let future_json: Value = response.json().await.expect("No json");
    future_json["text"]
        .as_str()
        .unwrap_or_else(||{
            future_json[0]["summary_text"].as_str().expect("No str")
        })
        .to_string()
}

async fn create_req_apy(client: &Client, text: Arc<String>) -> String {
    let response = client
        .post("https://api.apyhub.com/sharpapi/api/v1/content/summarize")
        .header("apy-token", "token")
        .header("Content-Type", "application/json")
        .json(&json!({
            "content": *text,
            "min_length": 30,
            "max_length": 150,
            "language": "English"
        })).send()
        .await
        .expect("No send");
    
    response.text().await.expect("No text")
}

async fn create_req_cohere (client: &Client, text: Arc<String>) -> String {
    let response = client
        .post("https://api.cohere.ai/v2/chat")
        .header("Authorization", "Bearer token")
        .json(&json!({
            "model": "command-a-03-2025",
            "messages": [
                {
                    "role": "user",
                    "content": format!("Summarize the following text in 3 sentences:\n{}", text)
                }
            ]
        })).send()
        .await
        .expect("No send");
    
    let future_json: Value = response.json().await.expect("No json");
    future_json["message"]["content"][0]["text"].as_str().expect("No str").to_string()
}
