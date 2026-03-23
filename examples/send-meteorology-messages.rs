use rdkafka::producer::{FutureProducer, Producer};
use rdkafka::ClientConfig;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;
use rand::seq::SliceRandom;
use std::sync::Arc;
use tokio::sync::Semaphore;

const CITIES: &[&str] = &[
    "北京", "上海", "广州", "深圳", "成都", "杭州", "南京", "武汉",
    "西安", "重庆", "天津", "苏州", "郑州", "长沙", "青岛", "宁波",
    "沈阳", "大连", "厦门", "福州", "合肥", "济南", "哈尔滨", "长春"
];

const WEATHER_TYPES: &[&str] = &[
    "晴", "多云", "阴", "小雨", "中雨", "大雨", "暴雨",
    "小雪", "中雪", "大雪", "雾", "霾", "雷阵雨", "台风"
];

const WIND_DIRECTIONS: &[&str] = &[
    "东风", "南风", "西风", "北风", "东南风", "西南风", "西北风", "东北风"
];

fn generate_message(index: u64, rng: &mut impl Rng) -> String {
    let city = CITIES.choose(rng).unwrap();
    let weather = WEATHER_TYPES.choose(rng).unwrap();
    let temperature = rng.gen_range(-20.0..45.0);
    let feels_like = temperature + rng.gen_range(-5.0..5.0);
    let humidity = rng.gen_range(20..100);
    let pressure = rng.gen_range(980..1030);
    let wind_direction = WIND_DIRECTIONS.choose(rng).unwrap();
    let wind_speed = rng.gen_range(0.0..20.0);
    let visibility = rng.gen_range(1..20);
    let uv_index = rng.gen_range(0..11);
    let aqi = rng.gen_range(0..300);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let json = json!({
        "id": index,
        "city": city,
        "weather": weather,
        "temperature": format!("{:.1}", temperature),
        "feels_like": format!("{:.1}", feels_like),
        "humidity": humidity,
        "pressure": pressure,
        "wind_direction": wind_direction,
        "wind_speed": format!("{:.1}", wind_speed),
        "visibility": visibility,
        "uv_index": uv_index,
        "aqi": aqi,
        "timestamp": timestamp
    });

    json.to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bootstrap_servers = "localhost:9092";
    let topic = "meteorology";
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
            let key = format!("metro-{}", i);
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
