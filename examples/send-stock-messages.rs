use rdkafka::producer::{FutureProducer, Producer};
use rdkafka::ClientConfig;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;
use rand::seq::SliceRandom;
use std::sync::Arc;
use tokio::sync::Semaphore;

const STOCK_SYMBOLS: &[&str] = &[
    "AAPL", "GOOGL", "MSFT", "AMZN", "META", "TSLA", "NVDA", "JPM",
    "V", "JNJ", "WMT", "PG", "MA", "UNH", "HD", "DIS", "PYPL",
    "BAC", "CMCSA", "ADBE", "NFLX", "XOM", "KO", "PEP", "INTC",
    "VZ", "T", "MRK", "ABT", "CRM", "CSCO", "PFE", "NKE", "TMO"
];

fn generate_message(index: u64, rng: &mut impl Rng) -> String {
    let symbol = STOCK_SYMBOLS.choose(rng).unwrap();
    let base_price = rng.gen_range(50.0..500.0);
    let change_percent = rng.gen_range(-5.0..5.0);
    let change = base_price * (change_percent / 100.0);
    let current_price = base_price + change;
    let volume = rng.gen_range(100_000..50_000_000);
    let high = current_price * rng.gen_range(1.01..1.05);
    let low = current_price * rng.gen_range(0.95..0.99);
    let open = current_price * rng.gen_range(0.98..1.02);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let json = json!({
        "id": index,
        "symbol": symbol,
        "price": format!("{:.2}", current_price),
        "change": format!("{:.2}", change),
        "change_percent": format!("{:.2}%", change_percent),
        "volume": volume,
        "high": format!("{:.2}", high),
        "low": format!("{:.2}", low),
        "open": format!("{:.2}", open),
        "timestamp": timestamp,
        "market": if rng.gen_bool(0.5) { "NASDAQ" } else { "NYSE" }
    });

    json.to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bootstrap_servers = "localhost:9092";
    let topic = "stock";
    let total_messages: u64 = 200000;
    let concurrent_sends: usize = 1000;

    println!("Connecting to Kafka at {}...", bootstrap_servers);

    let producer: Arc<FutureProducer> = Arc::new(
        ClientConfig::new()
            .set("bootstrap.servers", bootstrap_servers)
            .set("message.timeout.ms", "30000")
            .set("compression.type", "lz4")
            .set("batch.num.messages", "10000")
            .set("batch.size", "1048576")
            .set("linger.ms", "5")
            .set("queue.buffering.max.messages", "1000000")
            .create()?
    );

    println!("Producer created. Starting to send {} messages to topic '{}'...", total_messages, topic);

    let start = std::time::Instant::now();
    let semaphore = Arc::new(Semaphore::new(concurrent_sends));
    let mut last_report = 0u64;

    println!("Pre-generating {} messages...", total_messages);
    let mut rng = rand::thread_rng();
    let messages: Vec<(String, String)> = (0..total_messages)
        .map(|i| {
            let key = format!("stock-{}", i);
            let value = generate_message(i, &mut rng);
            (key, value)
        })
        .collect();
    println!("Messages generated. Starting to send...");

    let mut handles = Vec::new();

    for (i, (key, value)) in messages.into_iter().enumerate() {
        let producer = Arc::clone(&producer);
        let topic = topic.to_string();
        let permit = semaphore.clone().acquire_owned().await?;

        let handle = tokio::spawn(async move {
            let result = producer
                .send(
                    rdkafka::producer::FutureRecord::to(&topic)
                        .payload(value.as_bytes())
                        .key(key.as_bytes()),
                    None,
                )
                .await;
            drop(permit);
            result
        });

        handles.push(handle);

        let sent = (i + 1) as u64;

        if sent - last_report >= 50000 {
            let elapsed = start.elapsed();
            let rate = sent as f64 / elapsed.as_secs_f64();
            println!("Progress: {}/{} ({:.1}%) - {} msg/sec",
                     sent, total_messages,
                     (sent as f64 / total_messages as f64) * 100.0,
                     rate as i64);
            last_report = sent;
        }
    }

    println!("Waiting for all messages to be sent...");

    for result in futures::future::join_all(handles).await {
        match result {
            Ok(Ok(_)) => {}
            Ok(Err((e, _))) => eprintln!("Error: {:?}", e),
            Err(e) => eprintln!("Task failed: {}", e),
        }
    }

    println!("Flushing...");
    producer.flush(std::time::Duration::from_secs(60))?;

    let elapsed = start.elapsed();
    let rate = total_messages as f64 / elapsed.as_secs_f64();

    println!("\n=== Complete ===");
    println!("Total messages sent: {}", total_messages);
    println!("Time elapsed: {:.2?} ({:.2} seconds)", elapsed, elapsed.as_secs_f64());
    println!("Throughput: {:.0} messages/second", rate);

    Ok(())
}
