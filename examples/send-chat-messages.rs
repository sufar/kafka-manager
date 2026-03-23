use rdkafka::producer::{FutureProducer, Producer};
use rdkafka::ClientConfig;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;
use rand::seq::SliceRandom;
use std::sync::Arc;
use tokio::sync::Semaphore;

const NAMES: &[&str] = &[
    "Alice", "Bob", "Charlie", "David", "Eve", "Frank", "Grace", "Henry",
    "Ivy", "Jack", "Kate", "Leo", "Mia", "Noah", "Olivia", "Peter",
    "Quinn", "Rose", "Sam", "Tom", "Uma", "Victor", "Wendy", "Xavier",
    "Yara", "Zack", "Anna", "Ben", "Cara", "Dan", "Emma", "Fred"
];

const MESSAGES: &[&str] = &[
    "Hello!", "How are you?", "Good morning!", "Good night!", "Thanks!",
    "See you later", "Have a nice day!", "What's up?", "Hey there", "Hi!",
    "How's it going?", "Long time no see", "Nice to meet you", "Take care",
    "Bye!", "Talk to you soon", "Have fun!", "Good luck!", "Congratulations!",
    "That's great!", "I agree", "I see", "Really?", "Wow!", "Amazing!",
    "Cool!", "Awesome!", "Perfect!", "Sounds good", "No problem",
    "You're welcome", "My pleasure", "Don't worry", "It's okay",
    "Let me think", "Good idea!", "I'll check", "Sure thing",
    "Absolutely", "Definitely", "Of course", "Maybe later", "Not sure",
    "I think so", "I hope so", "We'll see", "Stay tuned", "Keep going",
    "Well done", "Great job", "Impressive", "Fantastic work"
];

fn generate_message(index: u64, rng: &mut impl Rng) -> String {
    let name = NAMES.choose(rng).unwrap();
    let message = MESSAGES.choose(rng).unwrap();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let json = json!({
        "id": index,
        "sender": name,
        "content": format!("{} - Message #{}", message, index),
        "timestamp": timestamp,
        "room_id": rng.gen_range(1..=10),
        "message_type": if rng.gen_bool(0.1) { "image" } else { "text" }
    });

    json.to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bootstrap_servers = "localhost:9092";
    let topic = "chat-messages";
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
            let key = format!("chat-{}", i);
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
