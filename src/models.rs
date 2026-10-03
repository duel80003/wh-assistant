use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct Receipt {
    pub id: i64,
    pub no: Option<i64>,
    pub matainer: Option<String>,
    pub work_date: String,
    pub due_date: String,
    pub total_amount: f64,
    pub currency: String,
    pub image_path: String,
    pub status: String, // "processing", "unconfirmed", "confirmed", "failed"
    pub payment_status: String, // "unpaid", "paid"
    pub payment_term_id: Option<i64>,
    pub paid_at: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct PaymentTerm {
    pub id: i64,
    pub name: String,
    pub duration_code: String,
    pub duration_days: i64,
    pub is_default: bool,
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProcessedFile {
    pub id: i64,
    pub file_path: String,
    pub file_hash: String,
    pub file_size: i64,
    pub receipt_id: Option<i64>,
    pub status: String,
    pub error_message: Option<String>,
    pub processed_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct AppSetting {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReceiptFilter {
    pub keyword: Option<String>,
    pub payment_status: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OverdueKpi {
    pub total_count: i64,
    pub total_amount: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExtractedReceiptData {
    pub no: Option<i64>,
    pub matainer: Option<String>,
    pub work_date: Option<String>,
    pub total_amount: Option<f64>,
    pub raw_response: Option<String>,
}
