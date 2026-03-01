/// 数据导入导出工具模块

use crate::error::{AppError, Result};
use crate::kafka::KafkaClients;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

/// 导出请求
#[derive(Debug, Deserialize)]
pub struct ExportMessagesRequest {
    pub cluster_id: String,
    pub topic: String,
    pub partition: Option<i32>,
    pub offset: Option<i64>,
    pub max_messages: usize,
    pub format: String,  // json, csv, text
    pub output_file: String,
}

/// 导入请求
#[derive(Debug, Deserialize)]
pub struct ImportMessagesRequest {
    pub cluster_id: String,
    pub topic: String,
    pub input_file: String,
    pub format: String,  // json, csv, text
    pub batch_size: Option<usize>,
}

/// 导出响应
#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub exported_count: usize,
    pub file_path: String,
    pub file_size_bytes: u64,
}

/// 导入响应
#[derive(Debug, Serialize)]
pub struct ImportResponse {
    pub imported_count: usize,
    pub failed_count: usize,
    pub errors: Vec<String>,
}

/// 消息记录（JSON 格式）
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageRecord {
    pub partition: i32,
    pub offset: i64,
    pub key: Option<String>,
    pub value: String,
    pub timestamp: Option<i64>,
}

/// 导出消息到文件
pub async fn export_messages_to_file(
    clients: &KafkaClients,
    req: ExportMessagesRequest,
) -> Result<ExportResponse> {
    let consumer = clients.get_consumer(&req.cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", req.cluster_id)))?;

    let config = clients.get_config(&req.cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", req.cluster_id)))?;

    // 获取消息
    let messages = consumer
        .fetch_messages_filtered(
            &config,
            &req.topic,
            req.partition,
            req.offset,
            req.max_messages,
            &|_| true,  // 不过滤
        )
        .await?;

    // 创建输出文件
    let file_path = PathBuf::from(&req.output_file);
    if let Some(parent) = file_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let file = File::create(&file_path).await?;
    let mut writer = BufWriter::new(file);

    let mut count = 0;
    for msg in messages {
        let record = MessageRecord {
            partition: msg.partition,
            offset: msg.offset,
            key: msg.key,
            value: msg.value.unwrap_or_default(),
            timestamp: msg.timestamp,
        };

        match req.format.as_str() {
            "json" => {
                let line = serde_json::to_string(&record)?;
                writer.write_all(line.as_bytes()).await?;
                writer.write_all(b"\n").await?;
            }
            "csv" => {
                let line = format!(
                    "{},{},{},\"{}\",\"{}\"\n",
                    record.partition,
                    record.offset,
                    record.timestamp.unwrap_or(0),
                    record.key.unwrap_or_default(),
                    record.value.replace('"', "\"\"")
                );
                writer.write_all(line.as_bytes()).await?;
            }
            _ => {
                // text 格式
                let line = format!("[{}] {}:{} - {}\n",
                    record.timestamp.unwrap_or(0),
                    record.partition,
                    record.offset,
                    record.value
                );
                writer.write_all(line.as_bytes()).await?;
            }
        }

        count += 1;
    }

    writer.flush().await?;

    // 获取文件大小
    let metadata = tokio::fs::metadata(&file_path).await?;

    Ok(ExportResponse {
        exported_count: count,
        file_path: req.output_file,
        file_size_bytes: metadata.len(),
    })
}

/// 从文件导入消息
pub async fn import_messages_from_file(
    clients: &KafkaClients,
    req: ImportMessagesRequest,
) -> Result<ImportResponse> {
    let producer = clients.get_producer(&req.cluster_id)
        .ok_or_else(|| AppError::NotFound(format!("Cluster '{}' not found", req.cluster_id)))?;

    let file_path = PathBuf::from(&req.input_file);
    let file = File::open(&file_path).await?;
    let reader = BufReader::new(file);

    let mut lines = reader.lines();
    let mut imported = 0;
    let mut failed = 0;
    let mut errors = Vec::new();

    let batch_size = req.batch_size.unwrap_or(100);

    loop {
        let mut batch = Vec::new();

        // 读取一批消息
        while batch.len() < batch_size {
            match lines.next_line().await {
                Ok(Some(line)) => batch.push(line),
                Ok(None) => break,
                Err(e) => {
                    errors.push(format!("Read error: {}", e));
                    failed += 1;
                }
            }
        }

        if batch.is_empty() {
            break;
        }

        // 解析并发送消息
        for line in batch {
            match parse_message(&line, &req.format) {
                Ok(record) => {
                    match producer.send(&req.topic, record.key.as_deref(), &record.value).await {
                        Ok(_) => imported += 1,
                        Err(e) => {
                            errors.push(format!("Send error: {}", e));
                            failed += 1;
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("Parse error: {}", e));
                    failed += 1;
                }
            }
        }
    }

    Ok(ImportResponse {
        imported_count: imported,
        failed_count: failed,
        errors,
    })
}

/// 解析消息行
fn parse_message(line: &str, format: &str) -> Result<MessageRecord> {
    match format {
        "json" => {
            Ok(serde_json::from_str(line)?)
        }
        "csv" => {
            // 简单的 CSV 解析
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() < 5 {
                return Err(AppError::BadRequest("Invalid CSV format".to_string()));
            }

            Ok(MessageRecord {
                partition: parts[0].parse().unwrap_or(0),
                offset: parts[1].parse().unwrap_or(0),
                key: if parts[3].is_empty() { None } else { Some(parts[3].to_string()) },
                value: parts[4].trim_matches('"').replace("\"\"", "\""),
                timestamp: Some(parts[2].parse().unwrap_or(0)),
            })
        }
        _ => {
            // text 格式解析
            Ok(MessageRecord {
                partition: 0,
                offset: 0,
                key: None,
                value: line.to_string(),
                timestamp: None,
            })
        }
    }
}

/// 数据迁移请求（从一个 topic 到另一个 topic）
#[derive(Debug, Deserialize)]
pub struct MigrateDataRequest {
    pub cluster_id: String,
    pub source_topic: String,
    pub target_topic: String,
    pub max_messages: usize,
}

/// 数据迁移响应
#[derive(Debug, Serialize)]
pub struct MigrateDataResponse {
    pub migrated_count: usize,
    pub failed_count: usize,
}
