use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::models::ExtractedReceiptData;
use crate::utils::normalize_work_date;

pub struct OllamaService;

#[derive(Serialize)]
struct OllamaOptions {
    num_ctx: u32,
}

#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    images: Vec<String>,
    stream: bool,
    format: String,
    options: OllamaOptions,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Option<Vec<OllamaModelItem>>,
}

#[derive(Deserialize)]
pub struct OllamaModelItem {
    pub name: String,
}

#[derive(Deserialize)]
struct OllamaParsedReceipt {
    no: Option<serde_json::Value>,
    matainer: Option<serde_json::Value>,
    work_date: Option<serde_json::Value>,
    total_amount: Option<serde_json::Value>,
}

impl OllamaService {
    /// Test connection to Ollama server and list available models
    pub async fn test_connection(base_url: &str) -> Result<Vec<String>> {
        let clean_url = base_url.trim_end_matches('/');
        let url = format!("{}/api/tags", clean_url);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        let resp = client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("無法連線至 Ollama 伺服器 ({})，請確認 Ollama 服務是否已啟動", clean_url))?;

        if !resp.status().is_success() {
            anyhow::bail!("Ollama 伺服器回應錯誤狀態碼: {}", resp.status());
        }

        let tags: OllamaTagsResponse = resp.json().await.context("解析 Ollama 模型清單失敗")?;
        let names = tags
            .models
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.name)
            .collect();

        Ok(names)
    }

    /// Extract receipt fields from image using Ollama vision model
    pub async fn extract_receipt(
        base_url: &str,
        model: &str,
        image_bytes: &[u8],
    ) -> Result<ExtractedReceiptData> {
        let clean_url = base_url.trim_end_matches('/');
        let url = format!("{}/api/generate", clean_url);

        let b64_img = BASE64.encode(image_bytes);

        let prompt = r#"
你是一個專業的繁體中文工單與發票單據辨識助手。請仔細分析這張工單/收據圖片，辨識並提取下列四項關鍵資訊：
1. "no": 工單號碼或單號（請提取純數字，若有 "NO."、"No." 等前綴請務必去除，例如："NO.12345678" 請輸出 12345678）。
2. "matainer": 施工人員、保養者或服務工程師姓名。
3. "work_date": 施工日期（例如 "113年5月20日"、"民國113/05/20" 或 "2024-05-20"）。
4. "total_amount": 總計金額、應收金額或合計（純數字，不含貨幣符號）。

請直接以純 JSON 格式輸出，不要包含任何額外說明文字或 Markdown 標籤，格式範例：
{
  "no": 12345678,
  "matainer": "王小明",
  "work_date": "113年5月20日",
  "total_amount": 1500
}
"#;

        let req_body = OllamaGenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            images: vec![b64_img],
            stream: false,
            format: "json".to_string(),
            options: OllamaOptions {
                num_ctx: 16384,
            },
        };

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(90))
            .build()?;

        let resp = client
            .post(&url)
            .json(&req_body)
            .send()
            .await
            .with_context(|| format!("呼叫 Ollama 模型 {} 逾時或失敗", model))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Ollama 處理失敗 ({}): {}", status, err_text);
        }

        let gen_resp: OllamaGenerateResponse = resp.json().await.context("解析 Ollama 回應 JSON 失敗")?;
        let raw = gen_resp.response;

        // Parse extracted JSON
        let parsed: OllamaParsedReceipt = serde_json::from_str(&raw)
            .or_else(|_| {
                // If wrapped in ```json ... ```, extract it
                if let Some(start) = raw.find('{') {
                    if let Some(end) = raw.rfind('}') {
                        if start < end {
                            return serde_json::from_str(&raw[start..=end]);
                        }
                    }
                }
                serde_json::from_str("{}")
            })
            .with_context(|| format!("無法解析模型輸出的 JSON 內容: {}", raw))?;

        let no = parsed.no.and_then(|v| match v {
            serde_json::Value::Number(n) => n.as_i64(),
            serde_json::Value::String(s) => crate::utils::normalize_receipt_no(&s),
            _ => None,
        });

        let matainer = parsed.matainer.and_then(|v| match v {
            serde_json::Value::String(s) => Some(s),
            _ => None,
        });

        let work_date_raw = parsed.work_date.and_then(|v| match v {
            serde_json::Value::String(s) => Some(s),
            _ => None,
        });

        let work_date = work_date_raw.map(|d| normalize_work_date(&d));

        let total_amount = parsed.total_amount.and_then(|v| match v {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::String(s) => {
                let clean = s.replace(['$', '¥', '€', 'N', 'T', ',', ' '], "");
                clean.parse::<f64>().ok()
            }
            _ => None,
        });

        Ok(ExtractedReceiptData {
            no,
            matainer,
            work_date,
            total_amount,
            raw_response: Some(raw),
        })
    }
}
