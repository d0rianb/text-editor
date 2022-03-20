use std::{fs, thread};
use std::time::Duration;

use hyper_tls::HttpsConnector;
use hyper::body::Buf;
use tokio;
use serde::{Serialize, Deserialize};

use crate::EditorEvent;
use crate::editor::Editor;
use crate::contextual_menu::ContextualMenu;

const OAI_URI: &str = "https://api.openai.com/v1/engines/text-davinci-001/completions";

pub struct OpenAIWrapper;

#[derive(Serialize, Debug)]
struct OAIRequest {
    prompt: String,
    max_tokens: u64,
}

#[derive(Deserialize, Debug)]
struct OAIChoices {
    text: String,
    index: u8,
    logprobs: Option<u8>,
    finish_reason: String
}

#[derive(Deserialize, Debug)]
struct OAIResponse {
    id: Option<String>,
    object: Option<String>,
    created: Option<u64>,
    model: Option<String>,
    choices: Vec<OAIChoices>,
}

impl OpenAIWrapper {
    async fn request(request: OAIRequest) -> Result<OAIResponse, Box<dyn std::error::Error>> {
        let https = HttpsConnector::new();
        let client = hyper::Client::builder().build::<_, hyper::Body>(https);
        let bearer = format!("Bearer {}", Self::get_access_token());
        let body = hyper::Body::from(serde_json::to_vec(&request)?);
        let req = hyper::Request::post(OAI_URI)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .header(hyper::header::AUTHORIZATION, &bearer)
            .body(body)?;
        let res = client.request(req).await?;
        let body = hyper::body::aggregate(res).await?;
        let json: OAIResponse = serde_json::from_reader(body.reader())?;
        Ok(json)
    }

    fn get_access_token() -> String {
        let prefs_path = Editor::get_file_path("./resources/tokens.yaml");
        let prefs_str = fs::read_to_string(prefs_path).expect("Can't find the token file");
        let tokens: serde_yaml::Value = serde_yaml::from_str(&prefs_str).expect("Invalid tokens value");
        tokens
            .get("OAI")
            .unwrap()
            .as_str()
            .unwrap()
            .into()
    }

    async fn placeholder_request() -> OAIResponse {
        tokio::time::sleep(Duration::from_secs(1)).await;
        OAIResponse {
            id: Some("cmpl-4dOXSaQ5s4ZpSvvUHHgz817tmBpTr".to_string()),
            object: Some("text_completion".to_string()),
            created: Some(1645294466),
            model: Some("text-davinci:001".to_string()),
            choices: vec![
                OAIChoices {
                    text: "\nThere are 11 days until 21/03.".to_string(),
                    index: 0,
                    logprobs: None,
                    finish_reason: "stop".to_string(),
                },
            ],
        }
    }

    fn make_async_request(req: OAIRequest, menu: &ContextualMenu) {
        let es = menu.event_sender.as_ref().unwrap().clone();
        let menu_id = menu.id;
        thread::spawn(move || {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let res = Self::request(req).await.unwrap();
                    let choices: Vec<String> = res.choices.iter().map(|choice| choice.text.replace("\n", "")).collect();
                    es.send_event(EditorEvent::OAIResponse(menu_id, choices));
                });
        });
    }

    pub fn ask(question: &str, menu: &ContextualMenu) {
        if question.is_empty() { return; }
        let req = OAIRequest { prompt: question.to_string(), max_tokens: 200 };
        Self::make_async_request(req, menu);
    }

    pub fn correct(word: &str, menu: &ContextualMenu) {
        if word.is_empty() { return; }
        let req = OAIRequest { prompt: ("Correct in proper french : ").to_string() + word, max_tokens: 50 };
        Self::make_async_request(req, menu);
    }
}